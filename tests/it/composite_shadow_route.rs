//! FR-029-AC-19 to AC-27, AC-30 and FR-033-AC-8, AC-19 (TC-048 steps 5 to 10): the production
//! route of a composite equality `bounded_shadow` run. One shadow-aware entry builds FSL's parity
//! request over the original proving context from the harness's own drawn leaves and settles by
//! the run's outcome; the installed-backend half is in `composite_shadow_kani.rs`.

use super::composite_equality_agreement::fixture;
use super::composite_equality_closure::{corpus, Corpus};
use super::composite_playback_decode::{flags, harness_path, transcript_for};
use super::composite_shadow::{entry, item_of, parameter_nodes, FLAGS_EQUAL};
use super::terminal_map::synthetic_evidence;
use filament_contract_codegen::{
    generate_composite_shadow, parse_assertion_playback, proof_strength,
    reconstruct_composite_operands, refinement_state, settle_composite_shadow,
    shadow_harness_bounds, CompositeDriverObservation, CompositeShadowError,
    CompositeShadowSettlement, DriverFalsification, KaniInconclusiveReason, KaniRunOutcome,
    KaniShadowHarness, LockedSource, ParityReport, ProofSubject, RefinementClass,
    RefinementSummary, ReplayInputs, ShadowClaim, ShadowOperand, ShadowRoute, WitnessDecodeLimits,
};
use filament_contract_model::{CheckedNodeId, CheckedOccurrence, CheckedPackageReadLimits};
use filament_exact::Outcome;
use fsl_replay::{
    Certification, CompositeParityResult, EqualityOperator, EqualityOutcome, IncompleteCause,
    IncompleteStage, InconclusiveCause, NativeParityObservation, ProofBasis, ReplayLimits,
    ReplayPackage, TerminalValue, WitnessValue,
};
use ix_trace_rs::trace;
use std::collections::BTreeMap;

pub(super) struct Fixture {
    pub(super) corpus: Corpus,
    pub(super) context: filament_contract_codegen::OriginalCompositeEqContext,
    pub(super) node: CheckedNodeId,
    pub(super) occurrence: CheckedOccurrence,
    pub(super) harness: KaniShadowHarness,
}

fn source() -> LockedSource {
    LockedSource {
        authority: "test".to_owned(),
        identity: "composite-agreement.native".to_owned(),
        bytes: fixture::source(fixture::BOUNDED_CASE).into_bytes(),
    }
}

fn limits(work: u64) -> filament_exact::ScalarLimits {
    filament_exact::ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: work,
        result_units: u64::MAX,
    }
}

pub(super) fn fixture_with(work: u64) -> Fixture {
    let corpus = corpus();
    let compiled = fixture::compile(fixture::BOUNDED_CASE);
    let context = filament_contract_codegen::OriginalCompositeEqContext::new(
        corpus.package.clone(),
        ReplayPackage::read(compiled.package().bytes(), u64::MAX).unwrap(),
        &source(),
        ReplayInputs {
            decode_limits: WitnessDecodeLimits {
                bytes: 1 << 20,
                values: 1024,
            },
            package_name: "composite-agreement".to_owned(),
            source: source(),
            dependencies: vec![],
            accounting_limits: limits(work),
            stage_limits: BTreeMap::new(),
            supplied_limits: None,
            declared_domains: vec![],
            replay_limits: ReplayLimits::default(),
        },
        "fr",
        CheckedPackageReadLimits::bounded(),
    )
    .unwrap();
    let node = super::admitted_enum_equality::application(&corpus.package, "fr");
    let occurrence = corpus
        .package
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == node)
        .unwrap()
        .occurrences[0]
        .clone();
    let claim = entry(&corpus, FLAGS_EQUAL);
    let (first, second) = parameter_nodes(&corpus, FLAGS_EQUAL);
    let mut item = item_of(&claim, "subject");
    item.left = ShadowOperand::Parameter {
        node: first,
        name: "a".to_owned(),
    };
    item.right = ShadowOperand::Parameter {
        node: second,
        name: "b".to_owned(),
    };
    let harness = generate_composite_shadow(&item).unwrap();
    Fixture {
        corpus,
        context,
        node,
        occurrence,
        harness,
    }
}

pub(super) fn fixture_default() -> Fixture {
    fixture_with(u64::MAX)
}

impl Fixture {
    pub(super) fn claim(&self) -> ShadowClaim<'_> {
        ShadowClaim {
            context: &self.context,
            node: &self.node,
            occurrence: &self.occurrence,
            operator: EqualityOperator::Equal,
            work_limit: 100_000,
        }
    }

    pub(super) fn bounds(&self, harness: &KaniShadowHarness) -> Vec<fsl_replay::ProofBound> {
        let packet = self
            .context
            .parity_packet(
                &self.node,
                &self.occurrence,
                EqualityOperator::Equal,
                &[],
                100_000,
            )
            .unwrap();
        let request = self
            .context
            .request_from_packet(
                packet,
                &self.node,
                &self.occurrence,
                EqualityOperator::Equal,
                &[],
                100_000,
            )
            .unwrap();
        shadow_harness_bounds(&harness.identity, &request, &self.corpus.package).unwrap()
    }
}

fn settle_with(
    fx: &Fixture,
    harness: &KaniShadowHarness,
    outcome: KaniRunOutcome,
    checks: u32,
    refinement: Option<RefinementSummary>,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    let evidence = synthetic_evidence(outcome, checks);
    settle_composite_shadow(
        &fx.claim(),
        &filament_contract_codegen::ShadowRun {
            harness,
            evidence: &evidence,
            refinement,
        },
        None,
    )
}

fn settle(
    fx: &Fixture,
    outcome: KaniRunOutcome,
    checks: u32,
    refinement: Option<RefinementSummary>,
) -> CompositeShadowSettlement {
    settle_with(fx, &fx.harness, outcome, checks, refinement).unwrap()
}

const EXHAUSTIVE: Option<RefinementSummary> =
    Some(RefinementSummary::Agreed(RefinementClass::Exhaustive));
const SAMPLED: Option<RefinementSummary> =
    Some(RefinementSummary::Agreed(RefinementClass::Sampled));

fn proved(count: u32) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::Checks {
            success_checks: count,
        },
        certification: Certification::Certified,
    }
}

/// FR-029-AC-19 and FR-033-AC-8: FSL derives the claim's positions and decides coverage from the
/// bounds the harness drew. Exhaustive refinement over the declared bounds is `Proved n`; a
/// sampled one is `Tested`; a refinement ceiling is `Incomplete(ResourceExhausted)`; zero checks
/// are the vacuous record, never promoted by the refinement.
#[trace("TC-048", "FR-029-AC-19")]
#[trace("TC-048", "FR-029-AC-23")]
#[trace("TC-048", "FR-029-AC-26")]
#[trace("TC-048", "FR-033-AC-8")]
#[test]
fn tc_048_a_verified_shadow_settles_through_fsl_by_refinement_and_coverage() {
    let fx = fixture_default();
    let verified = |checks, refinement| settle(&fx, KaniRunOutcome::Verified, checks, refinement);
    let covered = verified(5, EXHAUSTIVE);
    assert_eq!(covered.terminal, proved(5));
    assert_eq!(covered.route, ShadowRoute::Parity);
    assert!(matches!(covered.report, Some(ParityReport::Verified(_))));
    assert_eq!(verified(5, SAMPLED).terminal, TerminalValue::Tested);
    assert_eq!(verified(5, None).terminal, TerminalValue::Tested);
    assert_eq!(
        verified(5, Some(RefinementSummary::ReachedCeiling)).terminal,
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
    for refinement in [EXHAUSTIVE, SAMPLED, None] {
        assert_eq!(
            verified(0, refinement).terminal,
            proved(0),
            "{refinement:?}: zero checks stay the vacuous record"
        );
    }
    // A ceiling wins over zero checks.
    assert_eq!(
        verified(0, Some(RefinementSummary::ReachedCeiling)).terminal,
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
}

/// FR-033-AC-8: a range the harness drew inside the declared one does not cover, so an exhaustive
/// refinement over it is `Tested`, not `Proved`; the bounds are the harness's drawn ones.
#[trace("TC-048", "FR-033-AC-8")]
#[test]
fn tc_048_a_tightened_drawn_range_does_not_cover_the_declared_one() {
    let fx = fixture_default();
    let exact = fx.bounds(&fx.harness);
    assert_eq!(exact.len(), 4, "b and c of each of the two parameters");
    let mut tight = fx.harness.clone();
    let leaf = tight
        .identity
        .leaves
        .iter_mut()
        .find(|leaf| leaf.kind == filament_contract_codegen::LeafKind::Integer)
        .unwrap();
    leaf.drawn = (leaf.drawn.0, leaf.drawn.1 - 1);
    assert_ne!(
        fx.bounds(&tight),
        exact,
        "the bound follows the drawn range"
    );
    let settled =
        settle_with(&fx, &tight, KaniRunOutcome::Verified, 5, EXHAUSTIVE).expect("settles");
    assert_eq!(settled.terminal, TerminalValue::Tested);
}

/// FR-029-AC-24: a retained refinement disagreement is `Failed` for every backend outcome, zero
/// checks and an unreadable playback included, and no operand or native setup can precede it.
#[trace("TC-048", "FR-029-AC-24")]
#[test]
fn tc_048_a_retained_disagreement_fails_every_backend_outcome() {
    let fx = fixture_default();
    let disagreed = Some(RefinementSummary::Disagreed);
    let outcomes = [
        (KaniRunOutcome::Verified, 5),
        (KaniRunOutcome::Verified, 0),
        (
            KaniRunOutcome::Falsified {
                counterexample: "not a playback".into(),
            },
            0,
        ),
        (
            KaniRunOutcome::CoverUnsatisfied {
                satisfied: 0,
                total: 1,
            },
            0,
        ),
        (
            KaniRunOutcome::Inconclusive {
                reason: KaniInconclusiveReason::TimedOut,
            },
            0,
        ),
        (
            KaniRunOutcome::Inconclusive {
                reason: KaniInconclusiveReason::VacuousProof,
            },
            0,
        ),
    ];
    for (outcome, checks) in outcomes {
        let settled = settle(&fx, outcome.clone(), checks, disagreed);
        assert_eq!(settled.terminal, TerminalValue::Failed, "{outcome:?}");
        assert_eq!(settled.route, ShadowRoute::Parity, "{outcome:?}");
    }
    // An operand outside its declared range (`b: 12`, declared `Int[0, 2]`) does not hide it.
    let low = flags(true, 12, 1, 0);
    let falsified = KaniRunOutcome::Falsified {
        counterexample: transcript_for(
            &harness_path(&fx.harness.identity),
            &[low, flags(true, 1, 1, 0)].concat(),
        ),
    };
    assert_eq!(
        settle(&fx, falsified.clone(), 0, disagreed).terminal,
        TerminalValue::Failed
    );
    // Without the disagreement the same playback is a CG defect of the decode, also `Failed`.
    let defect = settle(&fx, falsified, 0, None);
    assert_eq!(defect.terminal, TerminalValue::Failed);
    assert_eq!(defect.route, ShadowRoute::DecodeDefect);
    assert!(defect.defect.is_some() && defect.report.is_none());
}

/// FR-029-AC-25 and AC-26: with no disagreement every shadow-inconclusive reason maps through the
/// ordinary rows, and none is coerced into verified evidence.
#[trace("TC-048", "FR-029-AC-25")]
#[trace("TC-048", "FR-029-AC-26")]
#[trace("TC-048", "FR-029-AC-30")]
#[test]
fn tc_048_a_shadow_inconclusive_outcome_maps_through_the_ordinary_rows() {
    let fx = fixture_default();
    let reason = |reason| KaniRunOutcome::Inconclusive { reason };
    let cases = [
        (reason(KaniInconclusiveReason::VacuousProof), proved(0)),
        (
            KaniRunOutcome::CoverUnsatisfied {
                satisfied: 0,
                total: 1,
            },
            proved(0),
        ),
        (
            reason(KaniInconclusiveReason::TimedOut),
            TerminalValue::Incomplete(IncompleteCause::TimedOut),
        ),
        (
            reason(KaniInconclusiveReason::UnwindBoundExhausted),
            TerminalValue::Incomplete(IncompleteCause::ResourceExhausted),
        ),
        (
            reason(KaniInconclusiveReason::NoVerdict),
            TerminalValue::Failed,
        ),
        (
            reason(KaniInconclusiveReason::MissingCoverSummary),
            TerminalValue::Failed,
        ),
        (
            reason(KaniInconclusiveReason::WitnessDisagreed),
            TerminalValue::Failed,
        ),
        (
            reason(KaniInconclusiveReason::FailedWithoutCounterexample),
            TerminalValue::Failed,
        ),
    ];
    for refinement in [
        None,
        EXHAUSTIVE,
        SAMPLED,
        Some(RefinementSummary::ReachedCeiling),
    ] {
        for (outcome, expected) in &cases {
            let settled = settle(&fx, outcome.clone(), 3, refinement);
            assert_eq!(settled.terminal, *expected, "{outcome:?} {refinement:?}");
            assert_eq!(settled.route, ShadowRoute::Ordinary);
            assert!(settled.report.is_none());
        }
    }
    // The cause row: out of memory is resource exhaustion.
    let mut evidence = synthetic_evidence(
        reason(KaniInconclusiveReason::FailedWithoutCounterexample),
        0,
    );
    evidence.failure_cause = Some(filament_contract_codegen::KaniFailureCause::OutOfMemory);
    let settled = settle_composite_shadow(
        &fx.claim(),
        &filament_contract_codegen::ShadowRun {
            harness: &fx.harness,
            evidence: &evidence,
            refinement: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(
        settled.terminal,
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
    // A verified shadow never reads as the ordinary production `Proved` (FR-028-AC-17): its value
    // is FSL's, so a sampled refinement over a verified run is `Tested`.
    assert_eq!(
        settle(&fx, KaniRunOutcome::Verified, 5, SAMPLED).terminal,
        TerminalValue::Tested
    );
}

/// FR-029-AC-27: another claim's node, operation or operands refuse with no value.
#[trace("TC-048", "FR-029-AC-27")]
#[test]
fn tc_048_a_harness_of_another_claim_refuses_with_no_value() {
    let fx = fixture_default();
    let mut other_node = fx.harness.clone();
    other_node.identity.node.push('x');
    assert!(matches!(
        settle_with(&fx, &other_node, KaniRunOutcome::Verified, 5, EXHAUSTIVE),
        Err(CompositeShadowError::NodeMismatch)
    ));
    let mut other_operator = fx.harness.clone();
    other_operator.identity.composite.as_mut().unwrap().operator = "equality.not_equal".to_owned();
    assert!(matches!(
        settle_with(
            &fx,
            &other_operator,
            KaniRunOutcome::Verified,
            5,
            EXHAUSTIVE
        ),
        Err(CompositeShadowError::OperatorMismatch)
    ));
    let mut other_operand = fx.harness.clone();
    other_operand.identity.composite.as_mut().unwrap().operands[0] =
        filament_contract_codegen::OperandSource::Literal;
    assert!(matches!(
        settle_with(&fx, &other_operand, KaniRunOutcome::Verified, 5, EXHAUSTIVE),
        Err(CompositeShadowError::OperandMismatch)
    ));
    let mut production = fx.harness.clone();
    production.identity.proof_subject = ProofSubject::Production;
    assert!(matches!(
        settle_with(&fx, &production, KaniRunOutcome::Verified, 5, EXHAUSTIVE),
        Err(CompositeShadowError::NotCompositeShadow)
    ));
    // The request itself names the original operation: the other operator refuses first.
    let mut claim = fx.claim();
    claim.operator = EqualityOperator::NotEqual;
    let evidence = synthetic_evidence(KaniRunOutcome::Verified, 5);
    assert!(matches!(
        settle_composite_shadow(
            &claim,
            &filament_contract_codegen::ShadowRun {
                harness: &fx.harness,
                evidence: &evidence,
                refinement: EXHAUSTIVE,
            },
            None
        ),
        Err(CompositeShadowError::Build(_))
    ));
}

/// The playback of the two parameters, the lower node's entries first.
fn playback(
    fx: &Fixture,
    low: Vec<super::composite_playback_decode::E>,
    high: Vec<super::composite_playback_decode::E>,
) -> String {
    transcript_for(&harness_path(&fx.harness.identity), &[low, high].concat())
}

fn decoded(fx: &Fixture, text: &str) -> [WitnessValue; 2] {
    let parsed = parse_assertion_playback(text, "t", "ctx", text.len(), 1024).unwrap();
    reconstruct_composite_operands(
        &fx.harness.identity,
        &parsed,
        WitnessDecodeLimits {
            bytes: 1 << 20,
            values: 1024,
        },
    )
    .unwrap()
}

fn authenticated(
    fx: &Fixture,
    operands: [WitnessValue; 2],
    outcome: Outcome<bool>,
) -> filament_contract_codegen::AuthenticatedCompositeArtifact {
    let binding = fx
        .context
        .retain_generated_artifact(
            &fx.corpus.oracles,
            &fx.node,
            &fx.occurrence,
            EqualityOperator::Equal,
            &fx.bounds(&fx.harness),
            operands.clone(),
            100_000,
        )
        .expect("the original generation backs the claim");
    let body = binding.body().clone();
    let observation = CompositeDriverObservation {
        staged_entries: binding
            .artifacts()
            .iter()
            .map(|artifact| (artifact.path.clone(), artifact.contents.as_bytes().to_vec()))
            .collect(),
        oracle_symbol: binding.oracle_symbol().to_owned(),
        environment_symbol: binding.environment_symbol().to_owned(),
        operands,
        limits: binding.limits(),
        source: source(),
        package: fx.corpus.package.clone(),
        outcome,
    };
    binding.complete(&body, observation).expect("authentic")
}

fn falsified_outcome(fx: &Fixture) -> (KaniRunOutcome, [WitnessValue; 2]) {
    // The lower parameter has b = 1, the other b = 2: the operands differ on the field b.
    let text = playback(fx, flags(true, 1, 1, 0), flags(true, 2, 1, 0));
    let operands = decoded(fx, &text);
    (
        KaniRunOutcome::Falsified {
            counterexample: text,
        },
        operands,
    )
}

fn falsify(
    fx: &Fixture,
    outcome: &KaniRunOutcome,
    refinement: Option<RefinementSummary>,
    driver: Option<&DriverFalsification<'_>>,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    let evidence = synthetic_evidence(outcome.clone(), 0);
    settle_composite_shadow(
        &fx.claim(),
        &filament_contract_codegen::ShadowRun {
            harness: &fx.harness,
            evidence: &evidence,
            refinement,
        },
        driver,
    )
}

fn result_of(settled: &CompositeShadowSettlement) -> &CompositeParityResult {
    let Some(ParityReport::Falsified(report)) = &settled.report else {
        panic!("a falsified parity report, not {settled:?}");
    };
    report.report().result()
}

/// FR-029-AC-20 (rows F-1 to F-7 from a CG-built request) and FR-033-AC-19: a falsified shadow
/// run is rebuilt from its playback and replayed through FSL against the authenticated artifact's
/// native observation. A shadow that calls unequal operands equal diverges from the exact
/// comparison; the shadow that agrees with it does not reproduce; nothing is `Refuted`.
#[trace("TC-048", "FR-029-AC-20")]
#[trace("TC-048", "FR-033-AC-19")]
#[trace("TC-048", "FR-033-AC-7")]
#[test]
fn tc_048_a_falsified_shadow_replays_through_fsl_from_a_cg_built_request() {
    let fx = fixture_default();
    let (outcome, operands) = falsified_outcome(&fx);
    assert_ne!(operands[0], operands[1], "the playback differs on b");
    let artifact = authenticated(&fx, operands.clone(), Outcome::Completed(false));
    let driver = |shadow, native: Option<NativeParityObservation>| DriverFalsification {
        artifact: &artifact,
        shadow,
        native,
    };
    let completed = |equal, pair_count| {
        Some(NativeParityObservation::Completed(EqualityOutcome {
            equal,
            pair_count,
        }))
    };

    // F-7 divergence: the mutant shadow says equal where FSL's exact comparison says unequal.
    let lying = EqualityOutcome {
        equal: true,
        pair_count: 0,
    };
    let probe = falsify(
        &fx,
        &outcome,
        None,
        Some(&driver(lying, completed(false, 0))),
    )
    .unwrap();
    assert_eq!(probe.terminal, TerminalValue::Failed);
    let CompositeParityResult::Diverged { exact, .. } = result_of(&probe) else {
        panic!("diverged, not {:?}", result_of(&probe));
    };
    let exact = *exact;
    assert!(!exact.equal && exact.pair_count > 0);

    // F-7 agreement: the shadow that matches the exact comparison does not reproduce.
    let agreeing = falsify(
        &fx,
        &outcome,
        EXHAUSTIVE,
        Some(&driver(exact, completed(exact.equal, exact.pair_count))),
    )
    .unwrap();
    assert!(matches!(
        agreeing.terminal,
        TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(_))
    ));
    // The pair count is compared too, with no early exit after an unequal pair.
    let short = EqualityOutcome {
        pair_count: exact.pair_count - 1,
        ..exact
    };
    let counted = falsify(
        &fx,
        &outcome,
        None,
        Some(&driver(short, completed(false, 1))),
    )
    .unwrap();
    assert!(matches!(
        result_of(&counted),
        CompositeParityResult::Diverged { .. }
    ));

    // F-2: a native stop wins and is a generated-artifact fault.
    for native in [
        NativeParityObservation::Incomplete(fsl_replay::NativeCause::new("limit".into())),
        NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new("fault".into())),
    ] {
        let stopped_artifact = authenticated(
            &fx,
            operands.clone(),
            Outcome::Undefined(filament_exact::Undefined::DivisionByZero),
        );
        let settled = falsify(
            &fx,
            &outcome,
            EXHAUSTIVE,
            Some(&DriverFalsification {
                artifact: &stopped_artifact,
                shadow: exact,
                native: Some(native),
            }),
        )
        .unwrap();
        assert_eq!(settled.terminal, TerminalValue::Failed);
        assert!(matches!(
            result_of(&settled),
            CompositeParityResult::GeneratedFault { .. }
        ));
    }

    // F-6: with sufficient limits a refinement ceiling is incomplete at its own stage.
    let ceiling = falsify(
        &fx,
        &outcome,
        Some(RefinementSummary::ReachedCeiling),
        Some(&driver(exact, completed(exact.equal, exact.pair_count))),
    )
    .unwrap();
    assert_eq!(
        ceiling.terminal,
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
    assert!(matches!(
        result_of(&ceiling),
        CompositeParityResult::Incomplete {
            stage: IncompleteStage::RefinementCeiling
        }
    ));

    // F-1: a retained disagreement settles first, with no driver evidence at all.
    let disagreed = falsify(&fx, &outcome, Some(RefinementSummary::Disagreed), None).unwrap();
    assert_eq!(disagreed.terminal, TerminalValue::Failed);

    // Nothing is a refutation.
    for settled in [&probe, &agreeing, &counted, &ceiling, &disagreed] {
        assert_ne!(settled.terminal, TerminalValue::Refuted);
    }
}

/// FR-029-AC-20 (F-4 and F-5): the original accounting limit stops the settlement at its own
/// stage, as `Incomplete(ResourceExhausted)`, even before the refinement ceiling is read.
#[trace("TC-048", "FR-029-AC-20")]
#[test]
fn tc_048_the_original_accounting_limit_stops_the_settlement_at_its_stage() {
    let fx = fixture_with(1);
    let (outcome, operands) = falsified_outcome(&fx);
    let artifact = authenticated(&fx, operands, Outcome::Completed(false));
    let settled = falsify(
        &fx,
        &outcome,
        Some(RefinementSummary::ReachedCeiling),
        Some(&DriverFalsification {
            artifact: &artifact,
            shadow: EqualityOutcome {
                equal: false,
                pair_count: 3,
            },
            native: Some(NativeParityObservation::Completed(EqualityOutcome {
                equal: false,
                pair_count: 3,
            })),
        }),
    )
    .unwrap();
    assert_eq!(
        settled.terminal,
        TerminalValue::Incomplete(IncompleteCause::ResourceExhausted)
    );
    assert!(
        matches!(
            result_of(&settled),
            CompositeParityResult::Incomplete {
                stage: IncompleteStage::Admission(_) | IncompleteStage::ExactEvaluation(_)
            }
        ),
        "a resource stage that is not the refinement ceiling: {:?}",
        result_of(&settled)
    );
}

/// FR-033-AC-19: before any falsified invocation the artifact, operands, claim and native
/// observation are tied; each failure refuses with no report and no terminal value, and a valid
/// retained disagreement is not lost to a missing observation.
#[trace("TC-048", "FR-033-AC-19")]
#[trace("TC-048", "FR-029-AC-24")]
#[test]
fn tc_048_the_native_observation_is_tied_to_the_authenticated_artifact() {
    let fx = fixture_default();
    let (outcome, operands) = falsified_outcome(&fx);
    let artifact = authenticated(&fx, operands.clone(), Outcome::Completed(false));
    let shadow = EqualityOutcome {
        equal: false,
        pair_count: 3,
    };
    let observed = Some(NativeParityObservation::Completed(shadow));
    let with = |artifact, native| DriverFalsification {
        artifact,
        shadow,
        native,
    };
    assert!(matches!(
        falsify(&fx, &outcome, EXHAUSTIVE, None),
        Err(CompositeShadowError::MissingDriverEvidence)
    ));
    assert!(matches!(
        falsify(&fx, &outcome, EXHAUSTIVE, Some(&with(&artifact, None))),
        Err(CompositeShadowError::MissingNativeObservation)
    ));
    // The native run found equal operands where the authenticated artifact's run said unequal.
    let contradicting = Some(NativeParityObservation::Completed(EqualityOutcome {
        equal: true,
        pair_count: 3,
    }));
    assert!(matches!(
        falsify(
            &fx,
            &outcome,
            EXHAUSTIVE,
            Some(&with(&artifact, contradicting))
        ),
        Err(CompositeShadowError::NativeContradictsArtifact)
    ));
    // An artifact authenticated at other operands is another run's.
    let swapped = [operands[1].clone(), operands[0].clone()];
    let other = authenticated(&fx, swapped, Outcome::Completed(false));
    assert!(matches!(
        falsify(
            &fx,
            &outcome,
            EXHAUSTIVE,
            Some(&with(&other, observed.clone()))
        ),
        Err(CompositeShadowError::ArtifactMismatch)
    ));
    // The same checks do not run ahead of a retained disagreement.
    let disagreed = falsify(&fx, &outcome, Some(RefinementSummary::Disagreed), None).unwrap();
    assert_eq!(disagreed.terminal, TerminalValue::Failed);
    // The healthy driver evidence settles.
    assert!(falsify(&fx, &outcome, EXHAUSTIVE, Some(&with(&artifact, observed))).is_ok());
}

/// FR-029-AC-17 and AC-22 stay the projection: the route's refinement state agrees with the
/// strength projection for a verified run, and has one row per summary.
#[trace("TC-048", "FR-029-AC-17")]
#[test]
fn tc_048_the_route_refinement_state_agrees_with_the_strength_projection() {
    for summary in [
        None,
        EXHAUSTIVE,
        SAMPLED,
        Some(RefinementSummary::ReachedCeiling),
        Some(RefinementSummary::Disagreed),
    ] {
        let strength = proof_strength(
            ProofSubject::BoundedShadow,
            &KaniRunOutcome::Verified,
            summary,
        )
        .unwrap();
        assert_eq!(
            strength.parity_refinement(),
            Some(refinement_state(summary)),
            "{summary:?}"
        );
    }
}

/// FR-033-AC-10 and FR-029-AC-19/AC-20 under the installed backend: the composite family's real
/// verified control and a real falsified mutant, each settled through the production route over
/// the original proving context. The verified shadow ran with no refinement run, so FSL's value
/// is `Tested`, never `Proved`. The mutant leaves field `b` out of the comparison and says equal;
/// the operands rebuilt from Kani's own playback differ on `b`, so FSL's exact comparison
/// diverges from the retained shadow verdict: `Failed`, never `Refuted`.
#[trace("TC-048", "FR-033-AC-10")]
#[trace("TC-048", "FR-029-AC-30")]
#[test]
#[ignore = "kani lane: run serially through make kani-gate"]
fn tc_048_installed_kani_runs_settle_through_the_production_route() {
    use super::composite_shadow_kani::{
        failed_lines, line_of, mutate, prove, selected, VERDICT_ASSERT,
    };
    if !selected("route") {
        return;
    }
    let fx = fixture_default();
    let (evidence, _) = prove(&fx.harness, "route-verified");
    assert_eq!(evidence.outcome, KaniRunOutcome::Verified);
    eprintln!("TOOLS {:?}", evidence.tool_versions);
    let settled = settle_composite_shadow(
        &fx.claim(),
        &filament_contract_codegen::ShadowRun {
            harness: &fx.harness,
            evidence: &evidence,
            refinement: None,
        },
        None,
    )
    .unwrap();
    eprintln!("ROUTE verified -> {:?}", settled.terminal);
    assert_eq!(settled.terminal, TerminalValue::Tested);
    assert_eq!(settled.route, ShadowRoute::Parity);

    let mut mutant = fx.harness.clone();
    let line = line_of(&mutant, VERDICT_ASSERT);
    mutate(&mut mutant, "p0.0 && p1.0 && p2.0,", "p0.0 && p2.0,");
    let (evidence, _) = prove(&mutant, "route-mutant");
    assert_eq!(
        failed_lines(&evidence),
        [line],
        "falsified by the verdict check only"
    );
    let KaniRunOutcome::Falsified { counterexample } = &evidence.outcome else {
        panic!("falsified, not {:?}", evidence.outcome);
    };
    let parsed = parse_assertion_playback(counterexample, "t", "ctx", 1 << 20, 1024).unwrap();
    let operands = reconstruct_composite_operands(
        &mutant.identity,
        &parsed,
        WitnessDecodeLimits {
            bytes: 1 << 20,
            values: 1024,
        },
    )
    .unwrap();
    assert_ne!(
        operands[0], operands[1],
        "the omitted field is where they differ"
    );
    let artifact = authenticated(&fx, operands, Outcome::Completed(false));
    let run = filament_contract_codegen::ShadowRun {
        harness: &mutant,
        evidence: &evidence,
        refinement: None,
    };
    let driver = |pair_count| DriverFalsification {
        artifact: &artifact,
        shadow: EqualityOutcome {
            equal: true,
            pair_count,
        },
        native: Some(NativeParityObservation::Completed(EqualityOutcome {
            equal: false,
            pair_count,
        })),
    };
    let probe = settle_composite_shadow(&fx.claim(), &run, Some(&driver(0))).unwrap();
    let CompositeParityResult::Diverged { exact, shadow, .. } = result_of(&probe) else {
        panic!("diverged, not {:?}", result_of(&probe));
    };
    eprintln!(
        "ROUTE mutant -> {:?} exact={exact:?} shadow={shadow:?}",
        probe.terminal
    );
    assert!(!exact.equal && shadow.equal);
    assert_eq!(probe.terminal, TerminalValue::Failed);
    let exact = *exact;
    let counted =
        settle_composite_shadow(&fx.claim(), &run, Some(&driver(exact.pair_count))).unwrap();
    assert!(matches!(
        result_of(&counted),
        CompositeParityResult::Diverged { .. }
    ));
}
