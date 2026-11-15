//! The shadow machinery of FR-028-AC-13 to FR-028-AC-24 over a stand-in family: two bounded
//! integers compared for equality, with a stand-in production subject the tests mutate. The
//! composite family's own run is TC-025 steps 29 to 36.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use ix_trace_rs::trace;
use quire_exact::Cancel;

use super::*;
use crate::kani::{
    classify::KaniRunOutcome,
    refinement::{
        plan_refinement, proof_strength, CaseCount, ProofStrength, RefinementBudget,
        RefinementClass, RefinementEvidence, RefinementOutcome, RefinementPlanError,
        RefinementSummary, RefinementVersions,
    },
    run::{
        execute::{
            execute_kani_obligation, execute_kani_refinement, KaniExecutionRequest,
            KaniRefinementRefusal, KaniRefinementRequest,
        },
        memory::{MemoryUnit, PeakMemorySource, PeakReading},
        tool::KaniInstallation,
    },
};

fn ceilings(wall_seconds: u64) -> ProofCeilings {
    ProofCeilings {
        memory_bytes: std::num::NonZeroU64::new(16 << 30).unwrap(),
        wall_clock: Duration::from_secs(wall_seconds),
    }
}

fn claims() -> Vec<AbstractionClaim> {
    vec![
        AbstractionClaim {
            name: "representation".to_owned(),
            evidence: Some(AbstractionEvidence::RefinementComparison(
                "verdict".to_owned(),
            )),
        },
        AbstractionClaim {
            name: "integer_widening".to_owned(),
            evidence: Some(AbstractionEvidence::KaniCheck(
                "shadow_proof::prove_shadow arithmetic overflow check".to_owned(),
            )),
        },
    ]
}

fn helpers(read: ProductionRead) -> Vec<SharedHelperClaim> {
    vec![
        SharedHelperClaim {
            name: "pair_count".to_owned(),
            mark: HelperMark::ValueBearing,
            order_only_reason: None,
            production_read: read,
            independent_check: (read == ProductionRead::SameHelper)
                .then(|| "independent_helper_check against the declaration".to_owned()),
        },
        SharedHelperClaim {
            name: "leaf_order".to_owned(),
            mark: HelperMark::OrderOnly,
            order_only_reason: Some(
                "the verdict and the pair count do not read leaf order".to_owned(),
            ),
            production_read: ProductionRead::Independent,
            independent_check: None,
        },
    ]
}

fn leaf(name: &str, bound: Option<(i64, i64)>) -> ShadowLeaf {
    ShadowLeaf {
        node: format!("node-{name}"),
        name: name.to_owned(),
        declared: bound,
        narrowed: None,
        refinement: None,
    }
}

fn item() -> ShadowItem {
    item_over((-3, 3))
}

fn item_over(bound: (i64, i64)) -> ShadowItem {
    ShadowItem {
        node: "equality-node".to_owned(),
        shape: ShadowShape::TwoLeafEquality,
        leaves: vec![leaf("left", Some(bound)), leaf("right", Some(bound))],
        size: 1,
        subject: "stand_in".to_owned(),
        abstractions: claims(),
        unexercised: vec!["meter_limits".to_owned()],
        helpers: helpers(ProductionRead::Independent),
        ceilings: ceilings(600),
        unwind: 4,
    }
}

fn violation(hooks: &Hooks) -> String {
    match generate_with(&item(), hooks) {
        Err(ShadowRefusal::SourceCheck { violation }) => violation,
        other => panic!("expected a source-check refusal, got {other:?}"),
    }
}

/// FR-028-AC-13: the unmutated shadow harness names no crate but `core`, `alloc`, `std` and
/// `kani`, holds no stub and assumes only its drawn domain; each mutated generator fails the
/// check it breaks.
#[trace("TC-039", "FR-028-AC-13")]
#[test]
fn tc_039_the_shadow_source_has_no_runtime_path_no_stub_and_no_assumption_beyond_its_domain() {
    let harness = generate_shadow(&item()).unwrap();
    let drawn = source_check::check_shadow_source(&harness.rust.contents, None).unwrap();
    assert_eq!(
        drawn,
        vec![
            source_check::DrawnBound {
                argument: "left".to_owned(),
                minimum: -3,
                maximum: 3
            },
            source_check::DrawnBound {
                argument: "right".to_owned(),
                minimum: -3,
                maximum: 3
            },
        ]
    );
    assert!(!source_check::contains_loop(&harness.rust.contents));
    assert!(!harness.rust.contents.contains("kani::stub"));

    let runtime = violation(&Hooks {
        proof_extra: "    use filament_contract_runtime::Outcome;\n".to_owned(),
        ..Hooks::default()
    });
    assert!(runtime.contains("ForeignCrate") && runtime.contains("filament_contract_runtime"));
    let exact = violation(&Hooks {
        proof_extra: "    use filament_exact::Value;\n".to_owned(),
        ..Hooks::default()
    });
    assert!(exact.contains("filament_exact"));
    let inline = violation(&Hooks {
        body_extra: "        let _ = filament_exact::Value::default();\n".to_owned(),
        ..Hooks::default()
    });
    assert!(inline.contains("ForeignCrate"));
    let stub = violation(&Hooks {
        proof_extra: "    #[kani::stub(shadow, shadow)]\n".to_owned(),
        ..Hooks::default()
    });
    assert!(stub.contains("Stub"));
    let verified_stub = violation(&Hooks {
        proof_extra: "    #[kani::stub_verified(shadow)]\n".to_owned(),
        ..Hooks::default()
    });
    assert!(verified_stub.contains("Stub"));
    let relation = violation(&Hooks {
        body_extra: "        kani::assume(left == right);\n".to_owned(),
        ..Hooks::default()
    });
    assert!(relation.contains("Assumption"));
    let one_sided = violation(&Hooks {
        body_extra: "        kani::assume(left >= 0_i64);\n".to_owned(),
        ..Hooks::default()
    });
    assert!(one_sided.contains("Assumption"));
}

/// FR-028-AC-14: the result carries a refinement obligation naming subject, shadow, domain,
/// abstraction function and observables, with the abstraction in a module the shadow, the
/// expectation and the harness do not import.
#[trace("TC-039", "FR-028-AC-14")]
#[test]
fn tc_039_the_obligation_names_its_parts_and_the_abstraction_module_is_imported_by_none() {
    let harness = generate_shadow(&item()).unwrap();
    let obligation = &harness.obligation;
    assert_eq!(obligation.subject, "stand_in");
    assert_eq!(obligation.shadow, "shadow_two_leaf::shadow");
    assert_eq!(obligation.abstraction, "abstraction::lift");
    assert_eq!(obligation.observables, ["verdict", "pair_count"]);
    assert_eq!(
        obligation.domain,
        vec![
            DomainBound {
                argument: "left".to_owned(),
                minimum: -3,
                maximum: 3
            },
            DomainBound {
                argument: "right".to_owned(),
                minimum: -3,
                maximum: 3
            },
        ]
    );
    // The harness source, which holds the shadow, the expectation and the proof, does not name
    // the abstraction module; the obligation source, which holds it, does.
    assert!(source_check::check_shadow_source(&harness.rust.contents, Some("abstraction")).is_ok());
    assert!(!harness.rust.contents.contains("abstraction"));
    assert!(obligation.source.contents.contains("mod abstraction"));
    assert!(obligation.abstraction_source.contains("subject::build"));
    // A harness that imports the abstraction module fails the check.
    let imported = generate_with(
        &item(),
        &Hooks {
            proof_extra: "    use super::abstraction::lift;\n".to_owned(),
            ..Hooks::default()
        },
    );
    assert!(
        matches!(imported, Err(ShadowRefusal::SourceCheck { violation }) if violation.contains("ImportsAbstraction"))
    );
    // An obligation naming no production subject is not returned.
    let mut nameless = item();
    nameless.subject = String::new();
    assert_eq!(
        generate_shadow(&nameless).unwrap_err(),
        ShadowRefusal::NoProductionSubject
    );
}

// ---- the stand-in production subject ----------------------------------------------------

fn subject(build: &str, compare: &str) -> String {
    format!(
        "pub mod stand_in {{
    pub struct Environment;
    pub struct Value(pub i64);
    pub enum Outcome {{
        Completed {{ verdict: bool, pairs: u64 }},
        Refused(String),
        Incomplete(String),
    }}
    pub fn environment() -> Environment {{ Environment }}
    pub fn build(_e: &Environment, _leaf: &str, value: i64) -> Result<Value, String> {{
        {build}
    }}
    pub fn compare(_e: &Environment, l: &Value, r: &Value) -> Outcome {{
        {compare}
    }}
}}
"
    )
}

const BUILD_OK: &str = "Ok(Value(value))";
const COMPARE_OK: &str = "Outcome::Completed { verdict: l.0 == r.0, pairs: 1 }";

fn scratch(name: &str) -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"),
        PathBuf::from,
    );
    let path = base
        .join("shadow-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

/// Compiles the obligation's program against `subject_source`, natively.
fn compile(harness: &KaniShadowHarness, subject_source: &str, directory: &Path) -> PathBuf {
    let source = directory.join("refinement.rs");
    fs::write(
        &source,
        format!("{subject_source}\n{}", harness.obligation.source.contents),
    )
    .unwrap();
    let program = directory.join("refinement");
    let status = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "-o"])
        .arg(&program)
        .arg(&source)
        .status()
        .expect("rustc runs");
    assert!(
        status.success(),
        "the generated refinement program compiles"
    );
    program
}

fn versions() -> RefinementVersions {
    RefinementVersions {
        rust_compiler: "rustc-under-test".to_owned(),
        subject_crate: "0.0.0-stand-in".to_owned(),
    }
}

fn budget(case_cap: u128) -> RefinementBudget {
    RefinementBudget {
        case_cap,
        sample_size: 20,
        seed: 7,
    }
}

fn refine(
    name: &str,
    harness: &KaniShadowHarness,
    subject_source: &str,
    budget: RefinementBudget,
) -> Result<RefinementEvidence, KaniRefinementRefusal> {
    refine_with(
        name,
        harness,
        subject_source,
        budget,
        &KaniInstallation::new("/nonexistent/launcher"),
    )
}

fn refine_with(
    name: &str,
    harness: &KaniShadowHarness,
    subject_source: &str,
    budget: RefinementBudget,
    installation: &KaniInstallation,
) -> Result<RefinementEvidence, KaniRefinementRefusal> {
    let directory = scratch(name);
    let program = compile(harness, subject_source, &directory);
    let cancel = Cancel::new();
    execute_kani_refinement(&KaniRefinementRequest {
        cancel: &cancel,
        installation,
        harness,
        program: &program,
        work_directory: &directory,
        budget,
        versions: versions(),
    })
}

fn disagreement(evidence: &RefinementEvidence) -> (Vec<i64>, String, String) {
    match &evidence.outcome {
        RefinementOutcome::Disagreed { first, .. } => (
            first.case.clone(),
            first.production.clone(),
            first.shadow.clone(),
        ),
        other => panic!("expected a disagreement, got {other:?}"),
    }
}

/// FR-028-AC-15: an unmutated subject agrees on every case; a charge of one extra pair, a refused
/// legal pair, the other verdict and a construction refusal are each a recorded disagreement
/// carrying the case and both results.
#[trace("TC-039", "FR-028-AC-15")]
#[test]
fn tc_039_each_production_divergence_is_a_recorded_disagreement_never_skipped() {
    let harness = generate_shadow(&item()).unwrap();
    let agreed = refine(
        "agree",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(1000),
    )
    .unwrap();
    assert_eq!(agreed.outcome, RefinementOutcome::Agreed);
    assert_eq!((agreed.cases_run, agreed.cases_planned), (49, 49));

    let extra = subject(
        BUILD_OK,
        "Outcome::Completed { verdict: l.0 == r.0, pairs: 2 }",
    );
    let (case, production, shadow) =
        disagreement(&refine("extra", &harness, &extra, budget(1000)).unwrap());
    assert_eq!(case, [-3, -3]);
    assert_eq!(production, "Completed(verdict=true,pairs=2)");
    assert_eq!(shadow, "Completed(verdict=true,pairs=1)");

    let refused = subject(
        BUILD_OK,
        "if l.0 == 1 && r.0 == 1 { Outcome::Refused(\"legal pair\".into()) } else { Outcome::Completed { verdict: l.0 == r.0, pairs: 1 } }",
    );
    let (case, production, _) =
        disagreement(&refine("refused", &harness, &refused, budget(1000)).unwrap());
    assert_eq!(
        (case, production.as_str()),
        (vec![1, 1], "Refused(legal pair)")
    );

    let incomplete = subject(BUILD_OK, "Outcome::Incomplete(\"meter\".into())");
    let (_, production, _) =
        disagreement(&refine("incomplete", &harness, &incomplete, budget(1000)).unwrap());
    assert_eq!(production, "Incomplete(meter)");

    let other = subject(
        BUILD_OK,
        "Outcome::Completed { verdict: l.0 != r.0, pairs: 1 }",
    );
    let (case, production, shadow) =
        disagreement(&refine("verdict", &harness, &other, budget(1000)).unwrap());
    assert_eq!(case, [-3, -3]);
    assert_eq!(
        (production.as_str(), shadow.as_str()),
        (
            "Completed(verdict=false,pairs=1)",
            "Completed(verdict=true,pairs=1)"
        )
    );

    let construction = subject(
        "if value == 2 { Err(\"NullForRequiredField\".into()) } else { Ok(Value(value)) }",
        COMPARE_OK,
    );
    let evidence = refine("construction", &harness, &construction, budget(1000)).unwrap();
    let (case, production, _) = disagreement(&evidence);
    assert_eq!(case, [-3, 2]);
    assert_eq!(production, "ConstructionRefused(NullForRequiredField)");
    // Every case ran: the disagreeing ones are counted, not skipped.
    assert_eq!(evidence.cases_run, 49);
    assert!(
        matches!(&evidence.outcome, RefinementOutcome::Disagreed { disagreements, .. } if *disagreements == 13)
    );
}

fn bounds(spans: &[(i64, i64)]) -> Vec<DomainBound> {
    spans
        .iter()
        .enumerate()
        .map(|(index, (minimum, maximum))| DomainBound {
            argument: format!("leaf{index}"),
            minimum: *minimum,
            maximum: *maximum,
        })
        .collect()
}

/// FR-028-AC-16: a domain of exactly the cap is exhaustive; one case larger, and a domain whose
/// count overflows `u128`, are sampled with the boundary cases and exactly the sample size of
/// seeded cases; equal inputs run equal cases.
#[trace("TC-039", "FR-028-AC-16")]
#[test]
fn tc_039_a_refinement_domain_is_exhaustive_up_to_the_cap_and_sampled_beyond_it() {
    let domain = bounds(&[(-3, 3), (-3, 3)]);
    let exact = plan_refinement(
        &domain,
        RefinementBudget {
            case_cap: 49,
            sample_size: 5,
            seed: 1,
        },
    )
    .unwrap();
    assert_eq!(exact.class, RefinementClass::Exhaustive);
    assert_eq!(exact.case_count, CaseCount::Exact(49));
    assert_eq!(exact.cases.len(), 49);
    let distinct: std::collections::BTreeSet<_> = exact.cases.iter().collect();
    assert_eq!(distinct.len(), 49);

    let larger = RefinementBudget {
        case_cap: 48,
        sample_size: 5,
        seed: 1,
    };
    let sampled = plan_refinement(&domain, larger).unwrap();
    assert_eq!(sampled.class, RefinementClass::Sampled);
    // One leaf at a time with the other at its lower bound (-3, -2, 2, 3, 0 each), the
    // all-lower and the all-upper case: ten distinct cases, not the forty-nine of a product.
    assert_eq!(sampled.boundary_cases, 10);
    assert_eq!(sampled.sample_size, 5);
    assert_eq!(sampled.cases.len(), 15);
    let boundary: std::collections::BTreeSet<Vec<i64>> =
        sampled.cases[..10].iter().cloned().collect();
    let expected: std::collections::BTreeSet<Vec<i64>> = [
        [-3, -3],
        [-2, -3],
        [2, -3],
        [3, -3],
        [0, -3],
        [-3, -2],
        [-3, 2],
        [-3, 3],
        [-3, 0],
        [3, 3],
    ]
    .iter()
    .map(|case| case.to_vec())
    .collect();
    assert_eq!(boundary, expected);
    assert!(sampled
        .cases
        .iter()
        .all(|case| case.iter().all(|value| (-3..=3).contains(value))));
    assert_eq!(plan_refinement(&domain, larger).unwrap(), sampled);
    let other_seed = RefinementBudget { seed: 2, ..larger };
    assert_ne!(
        plan_refinement(&domain, other_seed).unwrap().cases,
        sampled.cases
    );

    let full = bounds(&[
        (i64::MIN, i64::MAX),
        (i64::MIN, i64::MAX),
        (i64::MIN, i64::MAX),
    ]);
    let overflow = plan_refinement(
        &full,
        RefinementBudget {
            case_cap: u128::MAX,
            sample_size: 9,
            seed: 3,
        },
    )
    .unwrap();
    assert_eq!(overflow.class, RefinementClass::Sampled);
    assert_eq!(overflow.case_count, CaseCount::Overflowed);
    assert_eq!(overflow.cases.len(), overflow.boundary_cases + 9);

    assert_eq!(
        plan_refinement(&bounds(&[(2, 1)]), larger).unwrap_err(),
        RefinementPlanError::EmptyBound {
            argument: "leaf0".to_owned()
        }
    );
}

/// FR-028-AC-16 through the native run: the evidence records the class, the count, the cases run
/// and the seed, and a sampled run ran the boundary cases plus the sample.
#[trace("TC-039", "FR-028-AC-16")]
#[test]
fn tc_039_a_native_sampled_run_records_class_count_cases_and_seed() {
    let harness = generate_shadow(&item()).unwrap();
    let evidence = refine(
        "sampled",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(48),
    )
    .unwrap();
    assert_eq!(evidence.class, RefinementClass::Sampled);
    assert_eq!(evidence.case_count, CaseCount::Exact(49));
    assert_eq!((evidence.boundary_cases, evidence.sample_size), (10, 20));
    assert_eq!(evidence.cases_run, 30);
    assert_eq!(evidence.seed, 7);
    let exhaustive = refine(
        "exhaustive",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(49),
    )
    .unwrap();
    assert_eq!(exhaustive.class, RefinementClass::Exhaustive);
    assert_eq!(exhaustive.cases_run, 49);
}

/// FR-028-AC-17: the strength each result reports.
#[trace("TC-039", "FR-028-AC-17")]
#[test]
fn tc_039_proof_strength_follows_the_subject_and_the_refinement_run() {
    use ProofSubject::{BoundedShadow, Production};
    let verified = KaniRunOutcome::Verified;
    let falsified = KaniRunOutcome::Falsified {
        counterexample: String::new(),
    };
    assert_eq!(
        proof_strength(Production, &verified, None),
        Some(ProofStrength::ProductionProved)
    );
    let shadow = |refinement| proof_strength(BoundedShadow, &verified, refinement);
    assert_eq!(
        shadow(Some(RefinementSummary::Agreed(RefinementClass::Exhaustive))),
        Some(ProofStrength::ShadowProvedRefinementExhaustive)
    );
    assert_eq!(
        shadow(Some(RefinementSummary::Agreed(RefinementClass::Sampled))),
        Some(ProofStrength::ShadowProvedRefinementSampled)
    );
    assert_eq!(
        shadow(None),
        Some(ProofStrength::ShadowProvedRefinementNotRun)
    );
    assert_eq!(
        shadow(Some(RefinementSummary::ReachedCeiling)),
        Some(ProofStrength::ShadowProvedRefinementInconclusive)
    );
    // A disagreement reports `refinement_failed` whatever the shadow harness settled.
    assert_eq!(
        shadow(Some(RefinementSummary::Disagreed)),
        Some(ProofStrength::RefinementFailed)
    );
    assert_eq!(
        proof_strength(
            BoundedShadow,
            &falsified,
            Some(RefinementSummary::Disagreed)
        ),
        Some(ProofStrength::RefinementFailed)
    );
    // A shadow harness that did not verify has no strength, and no shadow result is production_proved.
    assert_eq!(proof_strength(BoundedShadow, &falsified, None), None);
    for refinement in [
        None,
        Some(RefinementSummary::Disagreed),
        Some(RefinementSummary::ReachedCeiling),
        Some(RefinementSummary::Agreed(RefinementClass::Exhaustive)),
        Some(RefinementSummary::Agreed(RefinementClass::Sampled)),
    ] {
        assert_ne!(shadow(refinement), Some(ProofStrength::ProductionProved));
    }
    assert_eq!(
        serde_json::to_string(&ProofStrength::ShadowProvedRefinementExhaustive).unwrap(),
        "\"shadow_proved_refinement_exhaustive\""
    );
}

/// FR-028-AC-18: the identity lists abstractions with evidence, unexercised behaviours and shared
/// helpers with their marks; an abstraction with no evidence is refused naming it.
#[trace("TC-039", "FR-028-AC-18")]
#[test]
fn tc_039_the_identity_lists_what_the_shadow_abstracts_and_an_undischarged_one_is_refused() {
    let harness = generate_shadow(&item()).unwrap();
    let identity = &harness.identity;
    assert_eq!(identity.family, TWO_LEAF_FAMILY);
    assert_eq!(identity.proof_subject, ProofSubject::BoundedShadow);
    assert_eq!(
        identity
            .abstractions
            .iter()
            .map(|record| record.name.as_str())
            .collect::<Vec<_>>(),
        ["representation", "integer_widening"]
    );
    assert_eq!(identity.unexercised, ["meter_limits"]);
    let marks: Vec<_> = identity
        .shared_helpers
        .iter()
        .map(|helper| (helper.name.as_str(), helper.mark))
        .collect();
    assert_eq!(
        marks,
        [
            ("pair_count", HelperMark::ValueBearing),
            ("leaf_order", HelperMark::OrderOnly)
        ]
    );
    assert!(identity.shared_helpers[1].order_only_reason.is_some());
    let json = serde_json::to_value(identity).unwrap();
    assert_eq!(json["proofSubject"], "bounded_shadow");

    let mut undischarged = item();
    undischarged.abstractions[1].evidence = None;
    assert_eq!(
        generate_shadow(&undischarged).unwrap_err(),
        ShadowRefusal::UndischargedAbstraction {
            name: "integer_widening".to_owned()
        }
    );
    let mut missing = item();
    missing.abstractions.remove(0);
    assert_eq!(
        generate_shadow(&missing).unwrap_err(),
        ShadowRefusal::UndischargedAbstraction {
            name: "representation".to_owned()
        }
    );
    // An order-only helper without a reason, and a same-helper value-bearing helper with no
    // independent check, are refused.
    let mut reasonless = item();
    reasonless.helpers[1].order_only_reason = None;
    assert_eq!(
        generate_shadow(&reasonless).unwrap_err(),
        ShadowRefusal::HelperClaim {
            name: "leaf_order".to_owned()
        }
    );
    let mut unchecked = item();
    unchecked.helpers[0].production_read = ProductionRead::SameHelper;
    assert_eq!(
        generate_shadow(&unchecked).unwrap_err(),
        ShadowRefusal::HelperClaim {
            name: "pair_count".to_owned()
        }
    );
    // Changing a bound changes the identity; regeneration is byte-identical.
    assert_ne!(
        generate_shadow(&item_over((-3, 4))).unwrap().identity,
        harness.identity
    );
    assert_eq!(generate_shadow(&item()).unwrap().rust, harness.rust);
}

/// FR-028-AC-18 and FR-028-AC-22: a mutation of the value-bearing `pair_count` helper that the
/// production subject reads independently is a refinement disagreement; one it reads through the
/// same helper is caught by the independent check; an order-only permutation is exempt.
#[trace("TC-039", "FR-028-AC-18")]
#[test]
fn tc_039_a_value_bearing_helper_mutant_fails_a_check_and_an_order_only_permutation_none() {
    let mutated = Hooks {
        pair_count_body: Some("2".to_owned()),
        ..Hooks::default()
    };
    // Independent read: the shadow and its expectation share the mutated helper, so the proof
    // cannot see it; the refinement run can.
    let harness = generate_with(&item(), &mutated).unwrap();
    let evidence = refine(
        "helper-independent",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(1000),
    )
    .unwrap();
    let (case, production, shadow) = disagreement(&evidence);
    assert_eq!(case, [-3, -3]);
    assert_eq!(production, "Completed(verdict=true,pairs=1)");
    assert_eq!(shadow, "Completed(verdict=true,pairs=2)");
    assert_eq!(
        proof_strength(
            ProofSubject::BoundedShadow,
            &KaniRunOutcome::Verified,
            Some(evidence.summary())
        ),
        Some(ProofStrength::RefinementFailed)
    );

    // Same helper: the production subject calls the helper that was mutated, so refinement agrees.
    let same = subject(
        BUILD_OK,
        "Outcome::Completed { verdict: l.0 == r.0, pairs: crate::shadow_two_leaf::pair_count() }",
    );
    let mut shared_item = item();
    shared_item.helpers = helpers(ProductionRead::SameHelper);
    let harness = generate_with(&shared_item, &mutated).unwrap();
    let evidence = refine("helper-same", &harness, &same, budget(1000)).unwrap();
    assert_eq!(evidence.outcome, RefinementOutcome::Agreed);
    assert_eq!(
        independent_helper_check(&harness, &[("pair_count", 1)]),
        Err("pair_count".to_owned())
    );
    // The unmutated helper passes the independent check.
    let healthy = generate_with(&shared_item, &Hooks::default()).unwrap();
    assert_eq!(
        independent_helper_check(&healthy, &[("pair_count", 1)]),
        Ok(())
    );

    // An order-only permutation changes no observable: refinement agrees, no check is required.
    let permuted = Hooks {
        leaf_order_body: Some("[\"right\", \"left\"]".to_owned()),
        ..Hooks::default()
    };
    let harness = generate_with(&item(), &permuted).unwrap();
    let evidence = refine(
        "order-only",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(1000),
    )
    .unwrap();
    assert_eq!(evidence.outcome, RefinementOutcome::Agreed);
    assert_eq!(
        independent_helper_check(&harness, &[("pair_count", 1)]),
        Ok(())
    );
}

/// FR-028-AC-19: a refinement domain narrower than the drawn one is refused naming the argument;
/// the drawn domain equals the declared one or the narrowed bound.
#[trace("TC-039", "FR-028-AC-19")]
#[test]
fn tc_039_a_refinement_domain_narrower_than_the_drawn_domain_is_refused() {
    let mut narrower = item();
    narrower.leaves[1].refinement = Some((-3, 2));
    assert_eq!(
        generate_shadow(&narrower).unwrap_err(),
        ShadowRefusal::RefinementDomainNarrower {
            argument: "right".to_owned()
        }
    );
    let mut wider = item();
    wider.leaves[0].refinement = Some((-4, 4));
    let harness = generate_shadow(&wider).unwrap();
    assert_eq!(
        (
            harness.obligation.domain[0].minimum,
            harness.obligation.domain[0].maximum
        ),
        (-4, 4)
    );
    assert_eq!(harness.identity.leaves[0].drawn, (-3, 3));
    // A narrowed bound inside the declared one is what the harness draws from and what the
    // refinement covers; a narrowed bound outside it is refused.
    let mut narrowed = item();
    narrowed.leaves[0].narrowed = Some((-1, 2));
    let harness = generate_shadow(&narrowed).unwrap();
    let drawn = source_check::check_shadow_source(&harness.rust.contents, None).unwrap();
    assert_eq!((drawn[0].minimum, drawn[0].maximum), (-1, 2));
    assert_eq!(harness.identity.leaves[0].declared, (-3, 3));
    assert_eq!(
        (
            harness.obligation.domain[0].minimum,
            harness.obligation.domain[0].maximum
        ),
        (-1, 2)
    );
    let mut outside = item();
    outside.leaves[0].narrowed = Some((-4, 2));
    assert_eq!(
        generate_shadow(&outside).unwrap_err(),
        ShadowRefusal::NarrowedOutsideDeclared {
            argument: "left".to_owned()
        }
    );
}

/// FR-028-AC-20: an unbounded node, a shape no shadow covers and a shape over the size budget are
/// refused with their disposition and no harness; a supported sibling settles as it does alone.
#[trace("TC-039", "FR-028-AC-20")]
#[test]
fn tc_039_unsupported_shapes_are_refused_without_a_harness_or_a_production_fallback() {
    let mut unbounded = item();
    unbounded.leaves[1] = leaf("right", None);
    let mut no_shadow = item();
    no_shadow.shape = ShadowShape::NoShadow {
        family: "composite_ordering".to_owned(),
        capability: "ordered comparison".to_owned(),
    };
    let mut oversize = item();
    oversize.size = SHADOW_PAIR_NODE_BUDGET + 1;
    let mut at_budget = item();
    at_budget.size = SHADOW_PAIR_NODE_BUDGET;
    let results = generate_shadows(&[unbounded, no_shadow, oversize, item(), at_budget]);
    assert_eq!(
        results[0].as_ref().unwrap_err(),
        &ShadowRefusal::RequiresBound {
            node: "node-right".to_owned()
        }
    );
    assert_eq!(
        results[0].as_ref().unwrap_err().disposition(),
        ShadowDisposition::RequiresBound
    );
    assert_eq!(
        results[1].as_ref().unwrap_err(),
        &ShadowRefusal::Unsupported {
            family: "composite_ordering".to_owned(),
            capability: "ordered comparison".to_owned()
        }
    );
    assert_eq!(
        results[1].as_ref().unwrap_err().disposition(),
        ShadowDisposition::Unsupported
    );
    assert_eq!(
        results[2].as_ref().unwrap_err(),
        &ShadowRefusal::ShapeOverBudget {
            size: SHADOW_PAIR_NODE_BUDGET + 1,
            budget: SHADOW_PAIR_NODE_BUDGET
        }
    );
    assert_eq!(
        results[2].as_ref().unwrap_err().disposition(),
        ShadowDisposition::Unsupported
    );
    let alone = generate_shadow(&item()).unwrap();
    assert_eq!(results[3].as_ref().unwrap().rust, alone.rust);
    assert_eq!(results[3].as_ref().unwrap().identity, alone.identity);
    assert!(results[4].is_ok());
}

#[derive(Debug)]
struct TestPeak;

impl PeakMemorySource for TestPeak {
    fn name(&self) -> &str {
        "test_peak_source"
    }
    fn peak(&self, _launcher: u32) -> Option<PeakReading> {
        Some(PeakReading {
            value: 2048,
            unit: MemoryUnit::Kibibytes,
        })
    }
}

/// FR-028-AC-24: a refinement stand-in that outruns its wall-clock ceiling is stopped and recorded
/// at it; one the environment ends by a signal is recorded as ended; each yields the inconclusive
/// strength, with the ceilings, the peak the environment provided and the versions recorded.
#[trace("TC-039", "FR-028-AC-24")]
#[test]
fn tc_039_a_refinement_at_a_ceiling_or_ended_by_a_signal_is_inconclusive_with_its_evidence() {
    let mut short = item();
    short.ceilings = ceilings(2);
    let harness = generate_shadow(&short).unwrap();
    let installation =
        KaniInstallation::new("/nonexistent/launcher").with_peak_memory_source(Arc::new(TestPeak));
    let slow = subject(
        BUILD_OK,
        "{ std::thread::sleep(std::time::Duration::from_secs(60)); Outcome::Completed { verdict: l.0 == r.0, pairs: 1 } }",
    );
    let started = std::time::Instant::now();
    let stopped = refine_with("slow", &harness, &slow, budget(1000), &installation).unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "stopped at the ceiling, not at the end"
    );
    assert_eq!(stopped.outcome, RefinementOutcome::ReachedWallClockCeiling);
    assert_eq!(stopped.ceilings, ceilings(2));
    assert_eq!(stopped.versions, versions());
    assert_eq!(
        (stopped.class, stopped.seed),
        (RefinementClass::Exhaustive, 7)
    );
    assert_eq!(
        proof_strength(
            ProofSubject::BoundedShadow,
            &KaniRunOutcome::Verified,
            Some(stopped.summary())
        ),
        Some(ProofStrength::ShadowProvedRefinementInconclusive)
    );
    assert_eq!(stopped.memory.source.as_deref(), Some("test_peak_source"));
    assert_eq!(stopped.memory.peak_resident_bytes, Some(2 * 1024 * 1024));

    let killed = subject(BUILD_OK, "std::process::abort()");
    let ended = refine_with("signal", &harness, &killed, budget(1000), &installation).unwrap();
    assert_eq!(ended.outcome, RefinementOutcome::EndedBySignal);
    assert_eq!(ended.ceilings.memory_bytes, ceilings(2).memory_bytes);
    assert_eq!(
        proof_strength(
            ProofSubject::BoundedShadow,
            &KaniRunOutcome::Verified,
            Some(ended.summary())
        ),
        Some(ProofStrength::ShadowProvedRefinementInconclusive)
    );

    // A program that exits unsuccessfully on its own is a refusal, not an outcome.
    let panicking = subject(BUILD_OK, "panic!(\"stand-in failure\")");
    assert!(matches!(
        refine_with("panic", &harness, &panicking, budget(1000), &installation),
        Err(KaniRefinementRefusal::ProcessFailed {
            exit_code: Some(101)
        })
    ));
    // The agreeing run records the peak beside its class and count.
    let agreed = refine_with(
        "peak",
        &harness,
        &subject(BUILD_OK, COMPARE_OK),
        budget(1000),
        &installation,
    )
    .unwrap();
    assert_eq!(agreed.memory.peak_resident_bytes, Some(2 * 1024 * 1024));
    assert_eq!(agreed.case_count, CaseCount::Exact(49));
}

fn crate_with(harness: &KaniShadowHarness, name: &str) -> PathBuf {
    let directory = scratch(name);
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::write(
        directory.join("src/lib.rs"),
        format!(
            "//! Generated shadow check crate.\n\n{}",
            harness.rust.contents
        ),
    )
    .unwrap();
    fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = \"generated-shadow\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        directory.join("build.rs"),
        "fn main() { println!(\"cargo:rustc-check-cfg=cfg(kani)\"); }\n",
    )
    .unwrap();
    directory
}

fn prove(
    harness: &KaniShadowHarness,
    name: &str,
) -> crate::kani::run::execute::KaniExecutionEvidence {
    let directory = crate_with(harness, name);
    let installation = KaniInstallation::discover().expect("cargo-kani is installed");
    let cancel = Cancel::new();
    let target = directory.join("target");
    execute_kani_obligation(&KaniExecutionRequest {
        cancel: &cancel,
        installation: &installation,
        harness: harness.into(),
        crate_directory: &directory,
        target_directory: &target,
        home_directory: None,
    })
    .expect("the run is not refused")
}

/// FR-028-AC-22 and FR-028-AC-23 under the installed backend: the unmutated shadow verifies with
/// its cover satisfied and its proof vector only; a shadow that always answers `true` is
/// falsified with a concrete playback, retaining both phases' vectors under one identity; a
/// shared-helper mutant verifies, which is why it is the refinement's and the independent check's.
#[trace("TC-039", "FR-028-AC-22")]
#[test]
fn tc_039_installed_kani_verifies_the_shadow_and_falsifies_its_mutant_in_two_phases() {
    let harness = generate_shadow(&item()).unwrap();
    let verified = prove(&harness, "kani-healthy");
    assert_eq!(
        verified.outcome,
        KaniRunOutcome::Verified,
        "the cover is satisfied and every check holds"
    );
    assert!(verified.proof_phase.is_none());
    let options = &harness.identity.options;
    assert_eq!(&verified.arguments[1..=options.len()], &options[..]);
    assert_eq!(verified.unwind, 4);

    let always_true = generate_with(
        &item(),
        &Hooks {
            shadow_body: Some("(true, pair_count())".to_owned()),
            ..Hooks::default()
        },
    )
    .unwrap();
    let falsified = prove(&always_true, "kani-always-true");
    assert!(
        matches!(&falsified.outcome, KaniRunOutcome::Falsified { counterexample } if !counterexample.is_empty())
    );
    let proof_phase = falsified
        .proof_phase
        .as_ref()
        .expect("the proof phase is retained");
    assert!(!proof_phase
        .arguments
        .iter()
        .any(|argument| argument == "concrete-playback"));
    let witness = &always_true.identity.witness_options;
    assert_eq!(&falsified.arguments[1..=witness.len()], &witness[..]);
    assert!(falsified
        .arguments
        .iter()
        .any(|argument| argument == "concrete-playback"));
    assert_eq!(falsified.harness_path, proof_phase.harness_path);

    let shared_mutant = generate_with(
        &item(),
        &Hooks {
            pair_count_body: Some("2".to_owned()),
            ..Hooks::default()
        },
    )
    .unwrap();
    assert_eq!(
        prove(&shared_mutant, "kani-shared-helper").outcome,
        KaniRunOutcome::Verified
    );
}
