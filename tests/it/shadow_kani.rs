//! FR-028-AC-22 and FR-028-AC-23 under the installed backend: a shadow harness through
//! `execute_kani_obligation`. Run serially through `make kani-gate`; plain `cargo test` skips it.

use std::{fs, path::PathBuf};

use filament_contract_codegen::{
    execute_kani_obligation, generate_shadow, AbstractionClaim, AbstractionEvidence, HelperMark,
    KaniExecutionEvidence, KaniExecutionRequest, KaniInstallation, KaniRunOutcome,
    KaniShadowHarness, ProductionRead, ProofCeilings, ShadowItem, ShadowLeaf, ShadowShape,
    SharedHelperClaim,
};
use ix_trace_rs::trace;
use quire_exact::Cancel;
use std::time::Duration;

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

/// The shadow harness and the refinement program with the shadow's one function body replaced.
fn with_shadow_body(harness: &mut KaniShadowHarness, body: &str) {
    let healthy = "(difference == 0, pair_count())";
    assert!(harness.rust.contents.contains(healthy));
    harness.rust.contents = harness.rust.contents.replace(healthy, body);
    harness.obligation.source.contents = harness.obligation.source.contents.replace(healthy, body);
}

/// The shadow harness and the refinement program with the shared `pair_count` helper changed.
fn with_pair_count(harness: &mut KaniShadowHarness, count: u64) {
    let healthy = "pub fn pair_count() -> u64 {\n        1\n    }";
    assert!(harness.rust.contents.contains(healthy));
    let changed = format!("pub fn pair_count() -> u64 {{\n        {count}\n    }}");
    harness.rust.contents = harness.rust.contents.replace(healthy, &changed);
    harness.obligation.source.contents = harness
        .obligation
        .source
        .contents
        .replace(healthy, &changed);
}

fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("shadow-kani")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
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

fn prove(harness: &KaniShadowHarness, name: &str) -> KaniExecutionEvidence {
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
#[ignore = "kani lane: run serially through make kani-gate"]
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

    let mut always_true = generate_shadow(&item()).unwrap();
    with_shadow_body(&mut always_true, "(true, pair_count())");
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

    let mut shared_mutant = generate_shadow(&item()).unwrap();
    with_pair_count(&mut shared_mutant, 2);
    assert_eq!(
        prove(&shared_mutant, "kani-shared-helper").outcome,
        KaniRunOutcome::Verified
    );
}
