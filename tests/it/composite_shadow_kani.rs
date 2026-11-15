//! FR-015-AC-74 and FR-015-AC-75 (TC-025) under the installed backend: the composite family's
//! shadow harness through `execute_kani_obligation`. Run serially through `make kani-gate`;
//! plain `cargo test` skips it.

use std::{fs, path::PathBuf, time::Instant};

use filament_contract_codegen::{
    execute_kani_obligation, generate_composite_shadow_with_budget, KaniExecutionEvidence,
    KaniExecutionRequest, KaniInstallation, KaniRunOutcome, KaniShadowHarness,
};
use ix_trace_rs::trace;
use quire_exact::Cancel;

use super::composite_equality_closure::corpus;
use super::composite_shadow::{
    boolean, entry, generate, int, item_of, with_record, FLAGS_EQUAL, FLAGS_NOT_EQUAL, OPTION_EQUAL,
};
use super::composite_shadow::{DUO_EQUAL, NESTED_OPTION_EQUAL};
use filament_contract_codegen::{
    KaniCheckClass, KaniCheckStatus, RecordedMember, RecordedPresence,
};

/// Whether the row `label` is run: every row when `COMPOSITE_KANI_ROW` is unset, so that a
/// measuring script can run one row to a scope of its own.
fn selected(label: &str) -> bool {
    std::env::var("COMPOSITE_KANI_ROW").map_or(true, |row| row == label)
}

fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("composite-shadow-kani")
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

fn prove(harness: &KaniShadowHarness, name: &str) -> (KaniExecutionEvidence, f64) {
    let directory = crate_with(harness, name);
    let installation = KaniInstallation::discover().expect("cargo-kani is installed");
    let cancel = Cancel::new();
    let target = directory.join("target");
    let started = Instant::now();
    let evidence = execute_kani_obligation(&KaniExecutionRequest {
        cancel: &cancel,
        installation: &installation,
        harness: harness.into(),
        crate_directory: &directory,
        target_directory: &target,
        home_directory: None,
    })
    .expect("the run is not refused");
    let wall = started.elapsed().as_secs_f64();
    eprintln!(
        "EVIDENCE exit={:?} success_checks={} checks={} args={:?}",
        evidence.exit_code,
        evidence.success_checks,
        evidence.checks.len(),
        evidence.arguments
    );
    eprintln!(
        "VERDICT row={name} outcome={:?} wall_s={wall:.1}",
        short(&evidence.outcome)
    );
    (evidence, wall)
}

fn short(outcome: &KaniRunOutcome) -> &'static str {
    match outcome {
        KaniRunOutcome::Verified => "Verified",
        KaniRunOutcome::Falsified { .. } => "Falsified",
        _ => "Other",
    }
}

/// A record of `nodes - 1` members cycling Boolean, bounded integer, optional bounded integer:
/// a closure whose pair-node count is exactly `nodes`.
fn mixed_record(nodes: u32) -> filament_contract_codegen::CompositeEqualityClaim {
    let corpus = corpus();
    let base = entry(&corpus, FLAGS_EQUAL);
    let members = (0..nodes - 1)
        .map(|index| match index % 3 {
            0 => boolean(&format!("f{index}")),
            1 => int(&format!("f{index}"), "0", "2"),
            _ => RecordedMember {
                presence: RecordedPresence::Optional,
                ..int(&format!("f{index}"), "0", "2")
            },
        })
        .collect();
    with_record(&base, members)
}

/// The budget probe: the shadow of a mixed closure of `PROBE_NODES` pair nodes, under the
/// explicit budget so that the family's own budget does not stand in the way. Run once per
/// size by the measuring script; the largest size that verifies inside the probe wall sets
/// `SHADOW_PAIR_NODE_BUDGET`.
#[trace("TC-025", "FR-015-AC-72")]
#[test]
#[ignore = "kani lane: run serially through make kani-gate"]
fn tc_025_budget_probe_verifies_a_mixed_closure_of_the_requested_size() {
    let nodes: u32 = std::env::var("PROBE_NODES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4);
    let claim = mixed_record(nodes);
    let harness = generate_composite_shadow_with_budget(&item_of(&claim, "subject"), nodes)
        .expect("the mixed closure generates under its own size");
    let (evidence, wall) = prove(&harness, &format!("probe-{nodes}"));
    eprintln!("PROBE nodes={nodes} wall_s={wall:.1}");
    assert_eq!(evidence.outcome, KaniRunOutcome::Verified);
}

/// The lines of the generated crate's source at which a check failed.
fn failed_lines(evidence: &KaniExecutionEvidence) -> Vec<u32> {
    let mut lines: Vec<u32> = evidence
        .checks
        .iter()
        .filter(|check| {
            check.status == KaniCheckStatus::Failure && check.class != KaniCheckClass::Cover
        })
        .filter_map(|check| check.location.line)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// The line of the written crate source on which `needle` first appears.
fn line_of(harness: &KaniShadowHarness, needle: &str) -> u32 {
    let source = format!(
        "//! Generated shadow check crate.\n\n{}",
        harness.rust.contents
    );
    let offset = source.find(needle).expect("the assertion is in the proof");
    u32::try_from(source[..offset].matches('\n').count() + 1).unwrap()
}

const VERDICT_ASSERT: &str = "assert!(verdict == expected_verdict);";
const PAIRS_ASSERT: &str = "assert!(pairs == expected_pairs);";

/// Replaces `from` by `to` in the shadow source and in the refinement program, which carries the
/// same shadow.
fn mutate(harness: &mut KaniShadowHarness, from: &str, to: &str) {
    assert!(
        harness.rust.contents.contains(from),
        "the mutation target is emitted: {from}"
    );
    harness.rust.contents = harness.rust.contents.replacen(from, to, 1);
    harness.obligation.source.contents = harness.obligation.source.contents.replacen(from, to, 1);
}

/// FR-015-AC-74: the unmutated shadow of each supported corpus shape verifies with its cover
/// satisfied.
#[trace("TC-025", "FR-015-AC-74")]
#[test]
#[ignore = "kani lane: run serially through make kani-gate"]
fn tc_025_installed_kani_verifies_each_supported_corpus_shadow_with_its_cover_satisfied() {
    let corpus = corpus();
    for (label, index) in [
        ("flags-equal", FLAGS_EQUAL),
        ("flags-not-equal", FLAGS_NOT_EQUAL),
        ("duo-equal", DUO_EQUAL),
        ("option-equal", OPTION_EQUAL),
        ("nested-option-equal", NESTED_OPTION_EQUAL),
    ] {
        if !selected(label) {
            continue;
        }
        let harness = generate(&corpus, index);
        let (evidence, _) = prove(&harness, label);
        assert_eq!(evidence.outcome, KaniRunOutcome::Verified, "{label}");
        assert!(
            evidence
                .checks
                .iter()
                .any(|check| check.class == KaniCheckClass::Cover
                    && check.status == KaniCheckStatus::Satisfied),
            "{label}: the cover is satisfied"
        );
    }
}

/// FR-015-AC-75 and FR-028-AC-22: each shadow mutant is falsified, and by the check it claims.
#[trace("TC-025", "FR-015-AC-75")]
#[trace("TC-039", "FR-028-AC-22")]
#[test]
#[ignore = "kani lane: run serially through make kani-gate"]
fn tc_025_installed_kani_falsifies_each_shadow_mutant_by_the_check_it_claims() {
    let corpus = corpus();
    type Mutation = (
        &'static str,
        usize,
        &'static str,
        &'static str,
        &'static str,
    );
    let mutants: [Mutation; 6] = [
        (
            "absent-equals-null",
            FLAGS_EQUAL,
            "(Slot::Absent, Slot::Absent) | (Slot::Null, Slot::Null) =>",
            "(Slot::Absent, Slot::Absent) | (Slot::Null, Slot::Null) | (Slot::Absent, Slot::Null) | (Slot::Null, Slot::Absent) =>",
            VERDICT_ASSERT,
        ),
        (
            "right-field-never-read",
            FLAGS_EQUAL,
            "let p1 = (l.f1 == r.f1, 1);",
            "let p1 = (l.f1 == l.f1, 1);",
            VERDICT_ASSERT,
        ),
        (
            "option-payload-pair-not-counted",
            OPTION_EQUAL,
            "(inner.0, 1 + inner.1)",
            "(inner.0, 1)",
            PAIRS_ASSERT,
        ),
        (
            "count-stops-at-first-unequal-field",
            FLAGS_EQUAL,
            "1 + p0.1 + p1.1 + p2.1)",
            "1 + p0.1 + if p0.0 { p1.1 + p2.1 } else { 0 })",
            PAIRS_ASSERT,
        ),
        (
            "not-equal-rendered-as-equal",
            FLAGS_NOT_EQUAL,
            "(!equal, pairs)",
            "(equal, pairs)",
            VERDICT_ASSERT,
        ),
        (
            "field-left-out-of-the-comparison",
            FLAGS_EQUAL,
            "p0.0 && p1.0 && p2.0,",
            "p0.0 && p2.0,",
            VERDICT_ASSERT,
        ),
    ];
    for (label, index, from, to, claimed) in mutants {
        if !selected(label) {
            continue;
        }
        let mut harness = generate(&corpus, index);
        let line = line_of(&harness, claimed);
        mutate(&mut harness, from, to);
        let (evidence, _) = prove(&harness, label);
        assert!(
            matches!(&evidence.outcome, KaniRunOutcome::Falsified { counterexample } if !counterexample.is_empty()),
            "{label}: {:?}",
            evidence.outcome
        );
        assert_eq!(
            failed_lines(&evidence),
            [line],
            "{label}: falsified by the {claimed} check only"
        );
    }
}

/// The array literal `member_order` returns, reversed: a permutation of the order the members
/// are walked and reported in, which no compared observable reads (FR-028-AC-18).
fn permute_member_order(harness: &mut KaniShadowHarness) {
    let permute = |text: &str| -> String {
        let lines: Vec<&str> = text.split('\n').collect();
        let at = lines
            .iter()
            .position(|line| line.contains("pub fn member_order()"))
            .expect("the order helper is emitted");
        let literal = lines[at + 1].trim();
        let inner = literal
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .expect("the order is an array literal");
        let mut items: Vec<&str> = inner.split(", ").collect();
        assert!(items.len() > 1);
        items.reverse();
        let mut changed: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        changed[at + 1] = format!("        [{}]", items.join(", "));
        changed.join("\n")
    };
    let rust = permute(&harness.rust.contents);
    assert_ne!(rust, harness.rust.contents);
    harness.rust.contents = rust;
    harness.obligation.source.contents = permute(&harness.obligation.source.contents);
}

/// FR-015-AC-75: a permutation of the member order is not a mutant. It verifies, with its
/// cover satisfied, exactly as the unmutated shadow does.
#[trace("TC-025", "FR-015-AC-75")]
#[test]
#[ignore = "kani lane: run serially through make kani-gate"]
fn tc_025_installed_kani_verifies_a_shadow_with_its_member_order_permuted() {
    if !selected("member-order-permuted") {
        return;
    }
    let corpus = corpus();
    let mut harness = generate(&corpus, FLAGS_EQUAL);
    permute_member_order(&mut harness);
    let (evidence, _) = prove(&harness, "member-order-permuted");
    assert_eq!(evidence.outcome, KaniRunOutcome::Verified);
}
