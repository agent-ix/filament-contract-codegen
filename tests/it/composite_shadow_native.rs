//! FR-015-AC-74 and FR-015-AC-75 (TC-025), FR-028-AC-14 to FR-028-AC-16 over the composite
//! family: the obligation's native refinement program, built against the generated oracle crate
//! of the TC-029 bounded corpus and run through `execute_kani_refinement`.
//!
//! The program's production values are built by the oracle's own environment; the shadow, its
//! expectation and the proof never import it.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use filament_contract_codegen::{
    execute_kani_refinement, plan_refinement, KaniInstallation, KaniRefinementRequest,
    KaniShadowHarness, RefinementBudget, RefinementClass, RefinementEvidence, RefinementOutcome,
    RefinementVersions,
};
use ix_trace_rs::trace;
use quire_exact::Cancel;

use super::composite_equality_agreement::fixture;
use super::composite_equality_closure::{corpus, generated, Corpus};
use super::composite_shadow::{
    generate, DUO_EQUAL, FLAGS_EQUAL, FLAGS_NOT_EQUAL, NESTED_OPTION_EQUAL, OPTION_EQUAL,
};
use super::exact_scalar_agreement::{
    composite_manifest_for_shared_target, AGREEMENT_DEV_DEPENDENCIES,
};
use super::scratch_crate::{
    agreement_target, lock_agreement_build, resolve_seeded_lock, TemporaryDirectory,
};

/// The subject the generated program names: the oracle crate's own environment and oracle
/// function behind the interface the program calls.
const SUBJECT_ADAPTER: &str = r#"
    pub mod subject {
        use super::*;
        pub use quire_exact as exact;

        pub struct Environment {
            prepared: PreparedEqualityEnvironment<'static>,
        }

        pub enum Outcome {
            Completed { verdict: bool, pairs: u64 },
            Refused(String),
            Incomplete(String),
        }

        pub fn environment() -> Environment {
            Environment {
                prepared: environment_subject(Some(crate::support::context()))
                    .expect("the original admitted context prepares the environment"),
            }
        }

        pub fn type_environment(environment: &Environment) -> &sv::TypeEnvironment {
            &environment.prepared.environment
        }

        pub fn declaration_key(
            environment: &Environment,
            key: &str,
        ) -> Result<exact::NodeKey, String> {
            environment
                .prepared
                .environment
                .composites()
                .find(|declaration| declaration.key().to_string() == key)
                .map(|declaration| declaration.key().clone())
                .ok_or_else(|| format!("the environment declares no {key}"))
        }

        pub fn compare(
            environment: &Environment,
            left: &exact::Value,
            right: &exact::Value,
        ) -> Outcome {
            let mut meter = exact::Meter::new(crate::support::rt_side::UNLIMITED);
            match oracle_subject(&environment.prepared, left, right, &mut meter) {
                exact::Outcome::Completed(verdict) => {
                    let pairs = meter
                        .admitted_charges()
                        .iter()
                        .filter(|charge| charge.point == exact::ChargePoint::EqualityPair)
                        .count() as u64;
                    Outcome::Completed { verdict, pairs }
                }
                exact::Outcome::Refused(refusal) => Outcome::Refused(format!("{refusal:?}")),
                other => Outcome::Incomplete(format!("{other:?}")),
            }
        }
    }
"#;

fn support_path(file: &str) -> String {
    format!(
        "{}/tests/composite_equality_support/{file}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn refine_source(harness: &KaniShadowHarness, names_file: &str) -> String {
    format!(
        "#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
#[macro_use]
#[path = {support:?}]
mod support;
#[path = {package:?}]
mod package;
#[path = {fixture:?}]
mod fixture;
mod generated {{
    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/lib.rs\"));
    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{names_file}\"));
{SUBJECT_ADAPTER}}}

{driver}",
        support = support_path("agreement.rs"),
        package = support_path("codes.rs"),
        fixture = support_path("authentic_native.rs"),
        driver = harness.obligation.source.contents,
    )
}

/// One refinement program: the generated driver of `harness` over the oracle of `symbol_of`.
pub(super) struct Example {
    pub(super) label: String,
    pub(super) harness: KaniShadowHarness,
    pub(super) names: String,
}

/// One scratch package: the corpus oracle crate, possibly edited, and its programs.
pub(super) struct Member {
    pub(super) label: String,
    pub(super) artifacts: Vec<(String, String)>,
    pub(super) examples: Vec<Example>,
}

pub(super) fn example(label: &str, corpus: &Corpus, index: usize) -> Example {
    let claim = generated(corpus, index);
    Example {
        label: label.to_owned(),
        harness: super::composite_shadow::generate_for(corpus, index, "generated::subject"),
        names: format!(
            "pub use self::{{{} as environment_subject, {} as oracle_subject}};\n",
            claim.environment_symbol, claim.oracle_symbol
        ),
    }
}

/// The corpus oracle crate with `edit` applied to every emitted file.
pub(super) fn artifacts(
    corpus: &Corpus,
    edit: &dyn Fn(&str, String) -> String,
) -> Vec<(String, String)> {
    corpus
        .oracles
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                edit(&artifact.path, artifact.contents.clone()),
            )
        })
        .collect()
}

/// `from` replaced by `to` inside the body of the oracle function of `symbol` only.
pub(super) fn in_oracle(source: &str, symbol: &str, from: &str, to: &str) -> String {
    let start = source
        .find(&format!("pub fn {symbol}("))
        .expect("the oracle function is emitted");
    let end = start + source[start..].find("\n}\n").expect("the function ends");
    let body = &source[start..end];
    assert!(
        body.contains(from),
        "the mutation target is in the oracle: {from}"
    );
    format!(
        "{}{}{}",
        &source[..start],
        body.replacen(from, to, 1),
        &source[end..]
    )
}

/// Builds every member's programs in one workspace; the programs' paths follow `members`.
pub(super) fn build(workspace: &Path, members: &[Member]) -> Vec<Vec<PathBuf>> {
    let mut names = Vec::new();
    for member in members {
        let root = workspace.join(&member.label);
        let mut wrote_manifest = false;
        for (path, contents) in &member.artifacts {
            wrote_manifest |= path == "Cargo.toml";
            let destination = root.join(path);
            fs::create_dir_all(destination.parent().expect("relative artifact path")).unwrap();
            fs::write(destination, contents).unwrap();
        }
        assert!(wrote_manifest, "{}: no Cargo.toml", member.label);
        fs::write(root.join("case-mode"), fixture::BOUNDED_CASE).unwrap();
        let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        let mut manifest = composite_manifest_for_shared_target(&manifest, &member.label);
        assert_eq!(manifest.matches("[workspace]\n").count(), 1);
        manifest = manifest.replacen("[workspace]\n", "", 1);
        for example in &member.examples {
            fs::write(
                root.join(format!("{}_names.rs", example.label)),
                &example.names,
            )
            .unwrap();
            fs::write(
                root.join(format!("{}.rs", example.label)),
                refine_source(&example.harness, &format!("{}_names.rs", example.label)),
            )
            .unwrap();
            write!(
                manifest,
                "\n[[example]]\nname = \"{0}\"\npath = \"{0}.rs\"\n",
                example.label
            )
            .unwrap();
        }
        write!(
            manifest,
            "\n[dev-dependencies]\n{AGREEMENT_DEV_DEPENDENCIES}"
        )
        .unwrap();
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        names.push(format!("\"{}\"", member.label));
    }
    fs::write(
        workspace.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [{}]\nresolver = \"2\"\n",
            names.join(", ")
        ),
    )
    .unwrap();
    resolve_seeded_lock(workspace);
    let guard = lock_agreement_build();
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "--locked",
            "--offline",
            "--workspace",
            "--examples",
        ])
        .env("CARGO_TARGET_DIR", agreement_target())
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .current_dir(workspace)
        .output()
        .expect("build the refinement programs");
    drop(guard);
    assert!(
        output.status.success(),
        "the refinement programs must build:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    members
        .iter()
        .map(|member| {
            member
                .examples
                .iter()
                .map(|example| {
                    agreement_target()
                        .join("debug/examples")
                        .join(&example.label)
                })
                .collect()
        })
        .collect()
}

pub(super) fn run(
    harness: &KaniShadowHarness,
    program: &Path,
    work: &Path,
    budget: RefinementBudget,
) -> RefinementEvidence {
    let cancel = Cancel::new();
    execute_kani_refinement(&KaniRefinementRequest {
        cancel: &cancel,
        installation: &KaniInstallation::new("/nonexistent/launcher"),
        harness,
        program,
        work_directory: work,
        budget,
        versions: RefinementVersions {
            rust_compiler: "rustc-under-test".to_owned(),
            subject_crate: filament_contract_codegen::COMPOSITE_EQUALITY_CRATE_NAME.to_owned(),
        },
    })
    .expect("the refinement program runs to a result")
}

pub(super) fn budget(case_cap: u128) -> RefinementBudget {
    RefinementBudget {
        case_cap,
        sample_size: 64,
        seed: 11,
    }
}

const CAP: u128 = 1 << 20;

const CORPUS: [(&str, usize); 5] = [
    ("refine_flags_equal", FLAGS_EQUAL),
    ("refine_flags_not_equal", FLAGS_NOT_EQUAL),
    ("refine_duo_equal", DUO_EQUAL),
    ("refine_option_equal", OPTION_EQUAL),
    ("refine_nested_option_equal", NESTED_OPTION_EQUAL),
];

fn disagreement(evidence: &RefinementEvidence) -> (&str, &str, u64) {
    match &evidence.outcome {
        RefinementOutcome::Disagreed {
            first,
            disagreements,
        } => (&first.production, &first.shadow, *disagreements),
        other => panic!("expected a disagreement, got {other:?}"),
    }
}

/// The production oracle mutants of FR-015-AC-75, each a package of its own.
fn production_mutants(corpus: &Corpus) -> Vec<Member> {
    let flags = generated(corpus, FLAGS_EQUAL).oracle_symbol.clone();
    let option = generated(corpus, OPTION_EQUAL).oracle_symbol.clone();
    let evaluate = "checked.evaluate(environment, left, right, meter)";
    let helper = r#"
fn mutant_absent_is_null(left: &exact::Value, right: &exact::Value) -> bool {
    match (left, right) {
        (exact::Value::Composite(a), exact::Value::Composite(b)) => {
            a.slots().len() == b.slots().len()
                && a.slots().iter().zip(b.slots()).all(|pair| match pair {
                    (
                        exact::FieldValue::Present(exact::Value::Boolean(x)),
                        exact::FieldValue::Present(exact::Value::Boolean(y)),
                    ) => x == y,
                    (
                        exact::FieldValue::Present(exact::Value::Integer(x)),
                        exact::FieldValue::Present(exact::Value::Integer(y)),
                    ) => x == y,
                    (exact::FieldValue::Present(_), _) | (_, exact::FieldValue::Present(_)) => false,
                    _ => true,
                })
        }
        _ => false,
    }
}
"#;
    let lib = |edit: &dyn Fn(String) -> String| {
        artifacts(corpus, &|path, contents| {
            if path == "src/lib.rs" {
                edit(contents)
            } else {
                contents
            }
        })
    };
    vec![
        Member {
            label: "mutant_opposite_operator".to_owned(),
            artifacts: lib(&|s| {
                in_oracle(
                    &s,
                    &flags,
                    "sv::EqualityOperator::Equal",
                    "sv::EqualityOperator::NotEqual",
                )
            }),
            examples: vec![example("run_opposite_operator", corpus, FLAGS_EQUAL)],
        },
        Member {
            label: "mutant_absent_equals_null".to_owned(),
            artifacts: lib(&|s| {
                let s = in_oracle(
                    &s,
                    &flags,
                    evaluate,
                    "match checked.evaluate(environment, left, right, meter) { exact::Outcome::Completed(false) if mutant_absent_is_null(left, right) => exact::Outcome::Completed(true), other => other }",
                );
                format!("{s}{helper}")
            }),
            examples: vec![example("run_absent_equals_null", corpus, FLAGS_EQUAL)],
        },
        Member {
            label: "mutant_extra_pair_charge".to_owned(),
            artifacts: lib(&|s| {
                in_oracle(
                    &s,
                    &option,
                    evaluate,
                    "{ let outcome = checked.evaluate(environment, left, right, meter); if let (exact::Value::Option(a), exact::Value::Option(b)) = (left, right) { if a.payload().is_some() && b.payload().is_some() { let _ = meter.charge(exact::Charge::new(exact::ChargePoint::EqualityPair)); } } outcome }",
                )
            }),
            examples: vec![example("run_extra_pair_charge", corpus, OPTION_EQUAL)],
        },
        Member {
            label: "mutant_optional_declared_required".to_owned(),
            artifacts: artifacts(corpus, &|_, contents| {
                contents.replace("exact::Presence::Optional", "exact::Presence::Required")
            }),
            examples: vec![example("run_optional_as_required", corpus, FLAGS_EQUAL)],
        },
        Member {
            label: "mutant_legal_pair_refused".to_owned(),
            artifacts: lib(&|s| {
                in_oracle(
                    &s,
                    &flags,
                    evaluate,
                    "match checked.evaluate(environment, left, right, meter) { exact::Outcome::Completed(true) => exact::Outcome::Refused(exact::Refusal::CheckedInvariant { cause: exact::CheckedInvariantCause::GeneratedDescriptorReconstructionFailed }), other => other }",
                )
            }),
            examples: vec![example("run_legal_pair_refused", corpus, FLAGS_EQUAL)],
        },
    ]
}

/// FR-015-AC-74 and FR-028-AC-14 to FR-028-AC-16: the unmutated corpus agrees on every case
/// of an exhaustive domain, and the same case under a smaller cap is sampled and agrees.
#[trace("TC-025", "FR-015-AC-74")]
#[test]
fn tc_025_each_corpus_refinement_agrees_exhaustively_and_a_smaller_cap_samples() {
    let corpus = corpus();
    let scratch = TemporaryDirectory::new("composite-shadow-native");
    let member = Member {
        label: "composite_shadow_base".to_owned(),
        artifacts: artifacts(&corpus, &|_, contents| contents),
        examples: CORPUS
            .iter()
            .map(|(label, index)| example(label, &corpus, *index))
            .collect(),
    };
    let programs = build(&scratch.path, std::slice::from_ref(&member));
    let mut counts = Vec::new();
    for (example, program) in member.examples.iter().zip(&programs[0]) {
        let evidence = run(&example.harness, program, &scratch.path, budget(CAP));
        assert_eq!(
            evidence.outcome,
            RefinementOutcome::Agreed,
            "{}",
            example.label
        );
        assert_eq!(
            evidence.class,
            RefinementClass::Exhaustive,
            "{}",
            example.label
        );
        assert_eq!(
            evidence.cases_run, evidence.cases_planned,
            "{}",
            example.label
        );
        assert!(evidence.cases_run > 1, "{}", example.label);
        counts.push((example.label.as_str(), evidence.cases_run));
    }
    eprintln!("EXHAUSTIVE {counts:?}");

    let flags = &member.examples[0];
    let sampled = run(&flags.harness, &programs[0][0], &scratch.path, budget(100));
    assert_eq!(sampled.outcome, RefinementOutcome::Agreed);
    assert_eq!(sampled.class, RefinementClass::Sampled);
    assert!(sampled.boundary_cases > 0 && sampled.sample_size == 64);
    assert!(sampled.cases_run < counts[0].1);
    eprintln!("SAMPLED cases_run={}", sampled.cases_run);
}

/// FR-015-AC-75: each production mutant settles `refinement_failed` naming its first
/// disagreeing case, and the disagreement states why.
#[trace("TC-025", "FR-015-AC-75")]
#[test]
fn tc_025_each_production_mutant_disagrees_with_the_shadow_for_the_reason_it_claims() {
    let corpus = corpus();
    let scratch = TemporaryDirectory::new("composite-shadow-mutants");
    let members = production_mutants(&corpus);
    let programs = build(&scratch.path, &members);
    for (member, programs) in members.iter().zip(&programs) {
        let example = &member.examples[0];
        let evidence = run(&example.harness, &programs[0], &scratch.path, budget(CAP));
        let (production, shadow, count) = disagreement(&evidence);
        eprintln!(
            "MUTANT {} disagreements={count} production={production} shadow={shadow}",
            member.label
        );
        assert!(count > 0);
        match member.label.as_str() {
            "mutant_opposite_operator"
            | "mutant_legal_pair_refused"
            | "mutant_absent_equals_null" => {
                assert!(
                    shadow.starts_with("Completed(verdict="),
                    "{}: {shadow}",
                    member.label
                );
            }
            "mutant_extra_pair_charge" => {
                let verdict = |text: &str| text.split("pairs=").next().unwrap().to_owned();
                assert_eq!(
                    verdict(production),
                    verdict(shadow),
                    "only the count differs"
                );
                assert_ne!(production, shadow);
            }
            "mutant_optional_declared_required" => {
                assert!(
                    production.starts_with("ConstructionRefused("),
                    "{production}"
                );
                assert!(
                    production.contains("MissingField")
                        || production.contains("NullForRequiredField"),
                    "{production}"
                );
            }
            other => panic!("{other}"),
        }
        if member.label == "mutant_legal_pair_refused" {
            assert!(production.starts_with("Refused("), "{production}");
        }
        if member.label == "mutant_absent_equals_null" {
            assert!(production.contains("verdict=true") && shadow.contains("verdict=false"));
        }
    }
}

/// FR-028-AC-16: the Boolean, presence and slot-state leaves are encoded as the small integer
/// ranges 0..=1 and 0..=2, so a sampled plan still draws every state of each of them.
#[trace("TC-025", "FR-028-AC-16")]
#[test]
fn tc_025_a_sampled_plan_draws_every_boolean_presence_and_slot_state() {
    let corpus = corpus();
    let harness = generate(&corpus, FLAGS_EQUAL);
    let domain = &harness.obligation.domain;
    let plan = plan_refinement(domain, budget(100)).expect("plans");
    assert_eq!(plan.class, RefinementClass::Sampled);
    for (position, leaf) in harness.identity.leaves.iter().enumerate() {
        use filament_contract_codegen::LeafKind;
        let (low, high) = match leaf.kind {
            LeafKind::Boolean | LeafKind::Presence => (0, 1),
            LeafKind::SlotState => (0, 2),
            LeafKind::Integer => continue,
        };
        for state in low..=high {
            assert!(
                plan.cases.iter().any(|case| case[position] == state),
                "{}: state {state} is drawn",
                leaf.name
            );
        }
    }
}
