//! FR-015-AC-72 (TC-025): the run evidence of a shadow harness carries the family's size figures,
//! and no other family's evidence carries them. A launcher stand-in verifies every harness, so the
//! default lane reads the evidence without the installed backend.

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use filament_contract_codegen::{
    execute_kani_obligation, generate_shadow, KaniExecutionEvidence, KaniExecutionRequest,
    KaniInstallation, KaniRunOutcome, KaniShadowHarness,
};
use filament_exact::Cancel;
use ix_trace_rs::trace;

use super::composite_equality_closure::corpus;
use super::composite_shadow::{generate, FLAGS_EQUAL};
use super::shadow_refinement::item;

const LAUNCHER: &str = r#"#!/bin/sh
last=; ids=
while [ $# -gt 0 ]; do
  last="$1"
  if [ "$1" = --harness ]; then ids="$ids $2"; last="$2"; shift; fi
  shift
done
results=; sep=
for id in $ids; do
  results="$results$sep{\"harness_id\":\"$id\",\"status\":\"Success\",\"checks\":[{\"id\":1,\"status\":\"Success\",\"category\":\"assertion\",\"location\":{\"file\":\"src/lib.rs\",\"line\":\"1\"}},{\"id\":2,\"status\":\"Satisfied\",\"category\":\"cover\",\"location\":{\"file\":\"src/lib.rs\",\"line\":\"2\"}}]}"
  sep=,
done
printf '{"metadata":{"version":"1.0"},"verification_results":{"results":[%s]}}' "$results" > "$last"
exit 0
"#;

fn run(harness: &KaniShadowHarness, name: &str) -> KaniExecutionEvidence {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("shadow-evidence")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::create_dir_all(directory.join("target")).unwrap();
    fs::write(directory.join("src/lib.rs"), &harness.rust.contents).unwrap();
    let launcher = directory.join("cargo-kani");
    fs::write(&launcher, LAUNCHER).unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
    let installation = KaniInstallation::new(&launcher);
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
    .expect("the stand-in run is not refused")
}

/// The stand-in family records its budget and size and no pair-node count; the composite family
/// records all three; each equals the identity's figures.
#[trace("TC-025", "FR-015-AC-72")]
#[test]
fn tc_025_a_shadow_run_records_its_size_figures_and_the_stand_in_family_no_pair_nodes() {
    let stand_in = generate_shadow(&item()).unwrap();
    let evidence = run(&stand_in, "stand-in");
    assert_eq!(evidence.outcome, KaniRunOutcome::Verified);
    assert_eq!(evidence.shadow_size, Some(stand_in.identity.size_record()));
    let wire = serde_json::to_value(&evidence).unwrap();
    assert_eq!(
        wire["shadowSize"]["sizeBudget"],
        stand_in.identity.size_budget
    );
    assert_eq!(wire["shadowSize"]["size"], stand_in.identity.size);
    assert!(wire["shadowSize"].get("pairNodes").is_none(), "{wire}");

    let composite = generate(&corpus(), FLAGS_EQUAL);
    let evidence = run(&composite, "composite");
    assert_eq!(evidence.outcome, KaniRunOutcome::Verified);
    let nodes = composite.identity.composite.as_ref().unwrap().pair_nodes;
    let wire = serde_json::to_value(&evidence).unwrap();
    assert_eq!(
        wire["shadowSize"]["sizeBudget"],
        composite.identity.size_budget
    );
    assert_eq!(wire["shadowSize"]["size"], composite.identity.size);
    assert_eq!(wire["shadowSize"]["pairNodes"], nodes);
}
