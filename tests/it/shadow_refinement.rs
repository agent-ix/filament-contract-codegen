//! FR-028-AC-15 to FR-028-AC-18, AC-22 and AC-24 over the stand-in family (two bounded integers
//! compared for equality) and a mutable stand-in production subject, run natively through
//! `execute_kani_refinement`. The generator-side checks are in `src/kani/generate/shadow/tests.rs`
//! and the installed-backend runs in `shadow_kani.rs`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use filament_contract_codegen::{
    execute_kani_refinement, generate_shadow, independent_helper_check, plan_refinement,
    proof_strength, AbstractionClaim, AbstractionEvidence, CaseCount, DomainBound, HelperMark,
    KaniInstallation, KaniRefinementRefusal, KaniRefinementRequest, KaniRunOutcome,
    KaniShadowHarness, MemoryUnit, PeakMemorySource, PeakReading, ProductionRead, ProofCeilings,
    ProofStrength, ProofSubject, RefinementBudget, RefinementClass, RefinementEvidence,
    RefinementOutcome, RefinementPlanError, RefinementSummary, RefinementVersions, ShadowItem,
    ShadowLeaf, ShadowShape, SharedHelperClaim, MAX_PLANNED_CASES,
};
use filament_exact::Cancel;
use ix_trace_rs::trace;

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

pub(super) fn item() -> ShadowItem {
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

/// The shadow harness and the refinement program with the order helper permuted.
fn with_leaf_order(harness: &mut KaniShadowHarness) {
    let healthy = "[\"left\", \"right\"]";
    assert!(harness.rust.contents.contains(healthy));
    harness.rust.contents = harness
        .rust
        .contents
        .replace(healthy, "[\"right\", \"left\"]");
    harness.obligation.source.contents = harness
        .obligation
        .source
        .contents
        .replace(healthy, "[\"right\", \"left\"]");
}

fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("shadow-refinement")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}
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

/// FR-028-AC-18 and FR-028-AC-22: a mutation of the value-bearing `pair_count` helper that the
/// production subject reads independently is a refinement disagreement; one it reads through the
/// same helper is caught by the independent check; an order-only permutation is exempt.
#[trace("TC-039", "FR-028-AC-18")]
#[test]
fn tc_039_a_value_bearing_helper_mutant_fails_a_check_and_an_order_only_permutation_none() {
    // Independent read: the shadow and its expectation share the mutated helper, so the proof
    // cannot see it; the refinement run can.
    let mut harness = generate_shadow(&item()).unwrap();
    with_pair_count(&mut harness, 2);
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
    let mut harness = generate_shadow(&shared_item).unwrap();
    with_pair_count(&mut harness, 2);
    let evidence = refine("helper-same", &harness, &same, budget(1000)).unwrap();
    assert_eq!(evidence.outcome, RefinementOutcome::Agreed);
    assert_eq!(
        independent_helper_check(&harness, &[("pair_count", 1)]),
        Err("pair_count".to_owned())
    );
    // The unmutated helper passes the independent check.
    let healthy = generate_shadow(&shared_item).unwrap();
    assert_eq!(
        independent_helper_check(&healthy, &[("pair_count", 1)]),
        Ok(())
    );

    // An order-only permutation changes no observable: refinement agrees, no check is required.
    let mut harness = generate_shadow(&item()).unwrap();
    with_leaf_order(&mut harness);
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

    // The aborting subject ends itself at once, so its wall ceiling is one it cannot reach even
    // when a loaded machine delays the process start; the 2 s ceiling belongs to the slow case.
    let mut patient = item();
    patient.ceilings = ceilings(600);
    let patient = generate_shadow(&patient).unwrap();
    let killed = subject(BUILD_OK, "std::process::abort()");
    let ended = refine_with("signal", &patient, &killed, budget(1000), &installation).unwrap();
    assert_eq!(ended.outcome, RefinementOutcome::EndedBySignal);
    assert_eq!(ended.ceilings, ceilings(600));
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
        refine_with("panic", &patient, &panicking, budget(1000), &installation),
        Err(KaniRefinementRefusal::ProcessFailed {
            exit_code: Some(101)
        })
    ));
    // The agreeing run records the peak beside its class and count.
    let agreed = refine_with(
        "peak",
        &patient,
        &subject(BUILD_OK, COMPARE_OK),
        budget(1000),
        &installation,
    )
    .unwrap();
    assert_eq!(agreed.memory.peak_resident_bytes, Some(2 * 1024 * 1024));
    assert_eq!(agreed.case_count, CaseCount::Exact(49));
}

/// FR-028-AC-22: a production subject that never compares one leaf (it reads the left leaf on both
/// sides) settles `refinement_failed` naming the first disagreeing case.
#[trace("TC-039", "FR-028-AC-22")]
#[test]
fn tc_039_a_subject_that_never_compares_one_leaf_settles_refinement_failed() {
    let harness = generate_shadow(&item()).unwrap();
    let never = subject(
        BUILD_OK,
        "Outcome::Completed { verdict: l.0 == l.0, pairs: 1 }",
    );
    let evidence = refine("never-compares", &harness, &never, budget(1000)).unwrap();
    let (case, production, shadow) = disagreement(&evidence);
    assert_eq!(case, [-3, -2]);
    assert_eq!(production, "Completed(verdict=true,pairs=1)");
    assert_eq!(shadow, "Completed(verdict=false,pairs=1)");
    assert_eq!(
        proof_strength(
            ProofSubject::BoundedShadow,
            &KaniRunOutcome::Verified,
            Some(evidence.summary())
        ),
        Some(ProofStrength::RefinementFailed)
    );
}

/// The plan is bounded before the run launches: a huge case cap over a domain above the limit is
/// sampled, and a sample size above the limit is refused, so planning cannot outrun the ceiling.
#[trace("TC-039", "FR-028-AC-16")]
#[test]
fn tc_039_planning_is_bounded_whatever_the_case_cap() {
    let domain = bounds(&[(0, (MAX_PLANNED_CASES as i64) / 2), (0, 3)]);
    let huge = RefinementBudget {
        case_cap: u128::MAX,
        sample_size: 5,
        seed: 1,
    };
    let plan = plan_refinement(&domain, huge).unwrap();
    assert_eq!(plan.class, RefinementClass::Sampled);
    assert_eq!(plan.cases.len(), plan.boundary_cases + 5);
    let trillion = bounds(&[(0, 1 << 20), (0, 1 << 20)]);
    let plan = plan_refinement(&trillion, huge).unwrap();
    assert_eq!(plan.class, RefinementClass::Sampled);
    assert_eq!(
        plan_refinement(
            &domain,
            RefinementBudget {
                sample_size: MAX_PLANNED_CASES as u64 + 1,
                ..huge
            }
        )
        .unwrap_err(),
        RefinementPlanError::SampleTooLarge {
            sample_size: MAX_PLANNED_CASES as u64 + 1
        }
    );
}

/// FR-028-AC-24: a caller cancellation is neither a ceiling nor a signal, so it is its own typed
/// refusal and yields no strength.
#[trace("TC-039", "FR-028-AC-24")]
#[test]
fn tc_039_a_cancelled_refinement_is_a_typed_refusal_not_an_inconclusive_strength() {
    let harness = generate_shadow(&item()).unwrap();
    let slow = subject(
        BUILD_OK,
        "{ std::thread::sleep(std::time::Duration::from_secs(60)); Outcome::Completed { verdict: l.0 == r.0, pairs: 1 } }",
    );
    let directory = scratch("cancelled");
    let program = compile(&harness, &slow, &directory);
    let cancel = Cancel::new();
    let installation = KaniInstallation::new("/nonexistent/launcher");
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_secs(1));
            cancel.cancel(filament_exact::CancelCause::Requested);
        });
        execute_kani_refinement(&KaniRefinementRequest {
            cancel: &cancel,
            installation: &installation,
            harness: &harness,
            program: &program,
            work_directory: &directory,
            budget: budget(1000),
            versions: versions(),
        })
    });
    assert!(matches!(
        result,
        Err(KaniRefinementRefusal::Cancelled { .. })
    ));
}

/// The shadow harness the cover-last inspection drives (FR-015-AC-58).
pub(crate) fn guard_sources() -> Vec<(&'static str, String)> {
    vec![(
        "bounded shadow",
        generate_shadow(&item()).unwrap().rust.contents,
    )]
}
