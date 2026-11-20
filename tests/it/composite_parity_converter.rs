//! FR-033/FR-030: bind public FSL composite reports before reading their terminal values.

use filament_contract_codegen::{
    composite_parity_terminal_value, proof_strength, run_terminal_value,
    verified_shadow_terminal_value, CompositeReportError, KaniExecutionEvidence,
    KaniInconclusiveReason, KaniRunOutcome, ProofStrength, ProofSubject, RefinementClass,
    RefinementSummary, ReplaySettlement, TerminalPairError,
};
use filament_contract_model::CheckedNodeId;
use filament_exact::PathSegment;
use fsl_package::ObligationIdentity;
use fsl_replay::{
    replay_composite_parity, settle_verified_shadow, Certification, CompositeEvidence,
    CompositeIdentity, CompositeParityClaim, CompositeParityResult, EqualityOperator,
    EqualityOutcome, FalsifiedParity, Identifier, InconclusiveCause, Integer,
    NativeParityObservation, NodeKey, OccurrenceKey, Origin, ProofBasis, Refinement, ReplayLimits,
    ReplayRefusal, ReplayRequest, ReplayRequestRefusal, ReplayRequestWire, ReplaySource,
    RequestTargetWire, Role, ScalarLimits, StateEnvironment, TerminalValue, VerifiedShadow,
    VerifiedShadowResult, WitnessValue,
};
use ix_trace_rs::trace;

fn limits() -> ScalarLimits {
    ScalarLimits {
        integer_bits: 1_000,
        decimal_digits: 1_000,
        scale_expansion: 1_000,
        text_input_bytes: 1_000,
        text_scalars: 1_000,
        normalized_scalars: 1_000,
        unit_edges: 1_000,
        value_occurrences: 1_000,
        work_units: 1_000,
        result_units: 1_000,
    }
}

struct PublicCompositeFixture {
    wire: ReplayRequestWire,
    claim: CompositeParityClaim,
}

fn sequence_value(value: i64) -> WitnessValue {
    WitnessValue::Sequence(vec![WitnessValue::ExactInteger(Integer::from(value))])
}

fn falsified_evidence() -> FalsifiedParity {
    let shadow = EqualityOutcome {
        equal: false,
        pair_count: 2,
    };
    FalsifiedParity {
        operands: [sequence_value(1), sequence_value(2)],
        shadow,
        native: NativeParityObservation::Completed(shadow),
        refinement: Refinement::NotExhausted,
    }
}

fn public_report(
    wire: ReplayRequestWire,
    claim: CompositeParityClaim,
    evidence: FalsifiedParity,
    replay_limits: ReplayLimits,
) -> (CompositeIdentity, fsl_replay::CompositeParityReport) {
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    let report = replay_composite_parity(wire, claim, evidence, replay_limits);
    (sent, report)
}

fn public_composite_fixture() -> PublicCompositeFixture {
    use filament_contract_model::{
        CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
        CheckedPackageV2ReadResult,
    };
    use fsl_replay::{
        compile_package, parity_obligation, BoundEntries, DependencyInput, Domain, DomainKey,
        DomainKind, FiniteBound, OperandIdentity, ParityArgument, ParityPreimage, ProofBound,
        SourceIdentity, StageLimits,
    };
    let source = b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Int[0, 9]>[0, 3], b: Sequence<Int[0, 9]>[0, 3]): Boolean pure { a = b }\n";
    let compiled = compile_package(
        fsl_replay::CompileUnit {
            source: SourceIdentity::new("test", "ir666"),
            package_name: "ir666".to_owned(),
            path: "ir666.native",
            bytes: source,
        },
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("compile package");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let CheckedPackageV2ReadResult::Admitted(package) = CheckedPackageV2::read(
        compiled.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("read package")
    };
    let nodes = &package.resolved_graph().nodes;
    let graph = fsl_replay::WireGraph::new(nodes.iter().map(|node| &node.node_id))
        .expect("admitted graph has structural keys");
    let named = |name: &str| {
        nodes
            .iter()
            .find(|node| {
                node.declaration.as_ref().is_some_and(|declaration| {
                    declaration.qualified_name.len() == 1
                        && declaration.qualified_name[0].as_ref() == name
                })
            })
            .expect("named declaration in compiled package")
    };
    let function = named("f");
    let function_body = serde_json::to_value(&function.body).expect("typed body serializes");
    let binding = |name: &str| {
        function_body["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == name)
            .unwrap()["value"]
            .clone()
    };
    let parameters = binding("parameters")["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let id: CheckedNodeId = serde_json::from_value(entry["target"].clone()).unwrap();
            graph.key(&id).expect("parameter target belongs to graph")
        })
        .collect::<Vec<_>>();
    assert_eq!(parameters.len(), 2);
    let application = nodes
        .iter()
        .find(|node| {
            node.node_tag.as_ref() == "expression"
                && matches!(&node.body, filament_contract_model::CheckedTerm::Application { operation, .. }
                    if operation.identity() == Some("filament.op.structural.eq"))
        })
        .expect("compiled structural equality");
    let node = graph
        .key(&application.node_id)
        .expect("application belongs to graph");
    let occurrence = &application.occurrences[0];
    let origin = Origin::new(Role::new("expression"), occurrence.ordinal);
    let bounds = parameters
        .iter()
        .flat_map(|parameter| {
            [
                ProofBound::new(
                    DomainKey::Node {
                        node: parameter.clone(),
                        path: Vec::new(),
                    },
                    Some(DomainKind::Collection),
                    FiniteBound::cardinality(3),
                )
                .expect("sequence cardinality pairs with collection"),
                ProofBound::new(
                    DomainKey::Node {
                        node: parameter.clone(),
                        path: vec![0],
                    },
                    Some(DomainKind::Integer),
                    FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64))
                        .expect("nonempty integer range"),
                )
                .expect("element range pairs with integer"),
            ]
        })
        .collect::<Vec<_>>();
    assert!(bounds.as_chunks::<2>().0.iter().all(|pair| {
        pair[0].kind() == Some(DomainKind::Collection)
            && pair[1].kind() == Some(DomainKind::Integer)
    }));
    let claim = CompositeParityClaim {
        node: node.clone(),
        occurrence: origin.clone(),
        operator: EqualityOperator::Equal,
        obligation_kind: "bounded_shadow".to_owned(),
        harness_bounds: bounds.clone(),
        limits: limits(),
    };
    let preimage = ParityPreimage {
        node: node.clone(),
        occurrence: origin,
        obligation_kind: claim.obligation_kind.clone(),
        arguments: parameters
            .iter()
            .map(|parameter| {
                let parameter_bounds = bounds
                    .iter()
                    .filter(|bound| {
                        matches!(bound.domain(), DomainKey::Node { node, .. } if node == parameter)
                    })
                    .cloned()
                    .collect();
                ParityArgument {
                    identity: OperandIdentity::GraphChild(parameter.clone()),
                    domain: Domain::Bounds(BoundEntries::new(parameter_bounds).unwrap()),
                }
            })
            .collect(),
    };
    let obligation = parity_obligation(&preimage).expect("the public preimage encodes");
    let wire = ReplayRequestWire {
        package: fsl_replay::ReplayPackage::read(compiled.bytes(), u64::MAX).unwrap(),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::function(
            fsl_replay::QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            OccurrenceKey::new(preimage.node.clone(), preimage.occurrence.clone()),
        ),
        source: ReplaySource::Input(Vec::new()),
        certificate: None,
        obligation_identity: obligation,
        backend: "test-backend".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: limits(),
        stage_limits: Default::default(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![("test".to_owned(), "ir666".to_owned(), source.to_vec())],
        documents: Vec::new(),
    };
    PublicCompositeFixture { wire, claim }
}

/// A valid public claim lets retained disagreement win over native faults and invalid operands;
/// either native fault kind wins over admission and both limit stages without disagreement.
#[trace(
    "FR-029-AC-20",
    "FR-030-AC-15",
    "FR-033-AC-12",
    "FR-033-AC-13",
    "TC-041",
    "TC-048"
)]
#[test]
fn tc_041_f1_and_f2_preserve_precedence_and_native_causes() {
    let fixture = public_composite_fixture();
    let mut evidence = falsified_evidence();
    evidence.refinement = Refinement::Disagreed;
    evidence.operands[0] = sequence_value(12);
    evidence.native = NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new(
        "native failed".to_owned(),
    ));
    let mut wire = fixture.wire.clone();
    wire.accounting_limits.value_occurrences = 0;
    let mut claim = fixture.claim.clone();
    claim.limits.value_occurrences = 0;
    let (sent, report) = public_report(wire, claim, evidence, ReplayLimits::default());
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(bound.result(), CompositeParityResult::Disagreed));
    assert_eq!(bound.terminal_value(), TerminalValue::Failed);

    for native in [
        NativeParityObservation::Incomplete(fsl_replay::NativeCause::new(
            "native limit".to_owned(),
        )),
        NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new(
            "native fault".to_owned(),
        )),
    ] {
        let mut evidence = falsified_evidence();
        evidence.refinement = Refinement::CeilingReached;
        evidence.operands[0] = sequence_value(12);
        evidence.native = native.clone();
        let mut wire = fixture.wire.clone();
        wire.accounting_limits.value_occurrences = 0;
        let mut claim = fixture.claim.clone();
        claim.limits.value_occurrences = 0;
        let (sent, report) = public_report(wire, claim, evidence, ReplayLimits::default());
        let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
        let CompositeParityResult::GeneratedFault { native: actual } = bound.result() else {
            panic!("expected F-2, got {:?}", bound.result());
        };
        assert_eq!(actual, &native);
        assert_eq!(bound.terminal_value(), TerminalValue::Failed);
    }
}

/// F-3, F-4, F-5 and F-6 remain separate typed rows with the exact FSL terminal value.
#[trace("FR-029-AC-20", "FR-030-AC-16", "FR-033-AC-13", "TC-041", "TC-048")]
#[test]
fn tc_041_f3_to_f6_keep_operand_and_limit_stages_distinct() {
    let fixture = public_composite_fixture();
    let mut refused = falsified_evidence();
    refused.operands[0] = sequence_value(12);
    refused.refinement = Refinement::CeilingReached;
    let (sent, report) = public_report(
        fixture.wire.clone(),
        fixture.claim.clone(),
        refused,
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::RefusedInput(refusal) = bound.result() else {
        panic!("expected F-3, got {:?}", bound.result());
    };
    assert_eq!(refusal.index, 0);
    assert_eq!(refusal.code(), fsl_replay::Code::InvalidRuntimeInput);
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
    );

    let mut ceiling = falsified_evidence();
    ceiling.refinement = Refinement::CeilingReached;
    let mut wire = fixture.wire.clone();
    wire.accounting_limits.value_occurrences = 0;
    let (sent, report) = public_report(
        wire,
        fixture.claim.clone(),
        ceiling.clone(),
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::Incomplete {
        stage: fsl_replay::IncompleteStage::Admission(incomplete),
    } = bound.result()
    else {
        panic!("expected F-4 admission, got {:?}", bound.result());
    };
    assert_eq!(
        incomplete.limit().map(|limit| (
            limit.limit,
            limit.limit_kind.field(),
            incomplete.context.consumed,
            limit.next_charge,
        )),
        Some((0, "value_occurrences", 0, 2))
    );
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted)
    );

    let mut claim = fixture.claim.clone();
    claim.limits.value_occurrences = 0;
    let (sent, report) = public_report(
        fixture.wire.clone(),
        claim,
        ceiling.clone(),
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::Incomplete {
        stage: fsl_replay::IncompleteStage::ExactEvaluation(incomplete),
    } = bound.result()
    else {
        panic!("expected F-5 exact evaluation, got {:?}", bound.result());
    };
    assert_eq!(incomplete.limit, 0);
    assert_eq!(incomplete.limit_kind.as_str(), "value_occurrences");
    assert_eq!(incomplete.consumed, 0);
    assert_eq!(incomplete.next_charge, Integer::from(2_i64));
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted)
    );

    let (sent, report) = public_report(
        fixture.wire,
        fixture.claim,
        ceiling,
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        CompositeParityResult::Incomplete {
            stage: fsl_replay::IncompleteStage::RefinementCeiling
        }
    ));
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted)
    );
}

/// Exact verdict and pair-count divergence are distinct F-7 inputs; agreement carries the full
/// composite claim and equality outcome, and none of the three becomes Refuted.
#[trace("FR-029-AC-20", "FR-030-AC-17", "FR-033-AC-7", "TC-041", "TC-048")]
#[test]
fn tc_041_f7_compares_verdict_and_pair_count_then_preserves_agreement() {
    let fixture = public_composite_fixture();
    for shadow in [
        EqualityOutcome {
            equal: true,
            pair_count: 2,
        },
        EqualityOutcome {
            equal: false,
            pair_count: 3,
        },
    ] {
        let mut evidence = falsified_evidence();
        evidence.shadow = shadow;
        let (sent, report) = public_report(
            fixture.wire.clone(),
            fixture.claim.clone(),
            evidence,
            ReplayLimits::default(),
        );
        let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
        let CompositeParityResult::Diverged {
            exact,
            shadow: kept,
            ..
        } = bound.result()
        else {
            panic!("expected F-7 divergence, got {:?}", bound.result());
        };
        assert_eq!(
            *exact,
            EqualityOutcome {
                equal: false,
                pair_count: 2
            }
        );
        assert_eq!(*kept, shadow);
        assert_eq!(bound.terminal_value(), TerminalValue::Failed);
    }
    let (sent, report) = public_report(
        fixture.wire,
        fixture.claim,
        falsified_evidence(),
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::Agrees { agreement, .. } = bound.result() else {
        panic!("expected F-7 agreement, got {:?}", bound.result());
    };
    assert!(
        matches!(agreement.claim(), fsl_replay::ScalarClaim::CompositeEquality(claim) if **claim == sent)
    );
    assert_eq!(
        agreement.outcome(),
        &fsl_replay::ScalarOutcome::Equality(EqualityOutcome {
            equal: false,
            pair_count: 2
        })
    );
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement.clone()))
    );
}

/// The verified entry retains its distinct V-row outcomes through the same full-identity check.
#[trace("TC-048")]
#[test]
fn tc_048_verified_public_rows_keep_their_fsl_terminal_values() {
    let fixture = public_composite_fixture();
    for (verified, expected) in [
        (
            VerifiedShadow {
                success_checks: 3,
                refinement: Refinement::Disagreed,
            },
            TerminalValue::Failed,
        ),
        (
            VerifiedShadow {
                success_checks: 3,
                refinement: Refinement::CeilingReached,
            },
            TerminalValue::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted),
        ),
        (
            VerifiedShadow {
                success_checks: 0,
                refinement: Refinement::Exhausted,
            },
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                certification: Certification::Certified,
            },
        ),
        (
            VerifiedShadow {
                success_checks: 3,
                refinement: Refinement::Exhausted,
            },
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 3 },
                certification: Certification::Certified,
            },
        ),
        (
            VerifiedShadow {
                success_checks: 3,
                refinement: Refinement::NotExhausted,
            },
            TerminalValue::Tested,
        ),
    ] {
        let sent = CompositeIdentity::new(
            fixture.wire.obligation_identity.clone(),
            &fixture.claim,
            CompositeEvidence::Verified(verified),
        );
        let report = settle_verified_shadow(
            fixture.wire.clone(),
            fixture.claim.clone(),
            verified,
            ReplayLimits::default(),
        );
        let bound = verified_shadow_terminal_value(&sent, Some(&report)).unwrap();
        match verified.refinement {
            Refinement::Disagreed => {
                assert!(matches!(bound.result(), VerifiedShadowResult::Disagreed))
            }
            Refinement::CeilingReached => assert!(matches!(
                bound.result(),
                VerifiedShadowResult::Incomplete {
                    stage: fsl_replay::IncompleteStage::RefinementCeiling
                }
            )),
            Refinement::Exhausted if verified.success_checks == 0 => {
                assert!(matches!(bound.result(), VerifiedShadowResult::Vacuous))
            }
            Refinement::Exhausted => assert!(matches!(
                bound.result(),
                VerifiedShadowResult::Proved { success_checks: 3 }
            )),
            Refinement::NotExhausted => {
                assert!(matches!(bound.result(), VerifiedShadowResult::Tested))
            }
        }
        assert_eq!(bound.terminal_value(), expected);
    }
}

/// A real FSL prepare refusal remains a bound report and carries FSL's own code before F-1.
#[trace("FR-029-AC-28", "FR-030-AC-17", "TC-041", "TC-048")]
#[test]
fn tc_041_bound_prepare_refusal_precedes_disagreed() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    ReplayRequest::decode(wire.clone(), ReplayLimits::default())
        .expect("the wire request itself is valid at the caller's ordinary limit");
    let claim = fixture.claim;
    let mut evidence = falsified_evidence();
    evidence.refinement = Refinement::Disagreed;
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    let report = replay_composite_parity(
        wire,
        claim,
        evidence,
        ReplayLimits::default().with_input_bytes(1),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::Refused(refusal) = bound.result() else {
        panic!("FSL prepare should refuse before F-1: {:?}", bound.result());
    };
    assert!(matches!(
        &**refusal,
        ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(_))
    ));
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
    );
    assert_eq!(bound.terminal_value(), report.terminal_value());
}

/// A changed claim body keeps its structural obligation but FSL refuses the wrong operation.
#[trace("FR-033-AC-18", "TC-048")]
#[test]
fn structural_obligation_does_not_admit_a_changed_operator_body() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    let mut claim = fixture.claim;
    assert_eq!(wire.obligation_identity.node(), &claim.node);
    assert_eq!(wire.obligation_identity.model(), None);
    assert_eq!(
        wire.obligation_identity.ordinal(),
        u32::try_from(claim.occurrence.ordinal()).expect("admitted occurrence ordinal fits")
    );
    claim.operator = EqualityOperator::NotEqual;
    let (sent, report) = public_report(wire, claim, falsified_evidence(), ReplayLimits::default());
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        CompositeParityResult::Refused(refusal)
            if matches!(&**refusal, ReplayRefusal::ScalarIdentity(_))
    ));
}

/// Each retained identity member is checked against the real FSL report, including the native
/// cause and refinement. A missing report has no terminal value.
#[trace("FR-033-AC-18", "TC-048")]
#[test]
fn tc_048_falsified_report_requires_every_sent_identity_member() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    let claim = fixture.claim;
    let evidence = falsified_evidence();
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    let report = replay_composite_parity(wire, claim, evidence, ReplayLimits::default());
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        CompositeParityResult::Agrees { .. }
    ));
    assert!(matches!(
        bound.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(_))
    ));
    assert_eq!(
        composite_parity_terminal_value(&sent, None).unwrap_err(),
        CompositeReportError::MissingReport
    );

    let mut changed = Vec::new();
    let mut other = sent.clone();
    other.obligation = ObligationIdentity::new(other.node.clone(), None, 1);
    changed.push(other);
    let mut other = sent.clone();
    other.node = NodeKey::new("ir666", vec![PathSegment::Text("other".to_owned())], 0).unwrap();
    changed.push(other);
    let mut other = sent.clone();
    other.occurrence = Origin::new(Role::new("expression"), 1);
    changed.push(other);
    let mut other = sent.clone();
    other.operator = EqualityOperator::NotEqual;
    assert_eq!(other.obligation, sent.obligation);
    changed.push(other);
    let mut other = sent.clone();
    other.obligation_kind = "other".to_owned();
    changed.push(other);
    let mut other = sent.clone();
    let sequence_domain = other
        .harness_bounds
        .first()
        .expect("the fixture declares a sequence bound")
        .domain()
        .clone();
    other.harness_bounds.push(
        fsl_replay::ProofBound::new(
            sequence_domain,
            Some(fsl_replay::DomainKind::Collection),
            fsl_replay::FiniteBound::cardinality(1),
        )
        .expect("sequence cardinality pairs with collection"),
    );
    changed.push(other);
    let mut other = sent.clone();
    other.limits.work_units += 1;
    changed.push(other);
    let CompositeEvidence::Falsified(_) = &sent.evidence else {
        panic!("fixture is falsified");
    };
    for edit in [
        |e: &mut FalsifiedParity| e.operands[0] = sequence_value(3),
        |e: &mut FalsifiedParity| e.operands[1] = sequence_value(4),
        |e: &mut FalsifiedParity| e.shadow.equal = true,
        |e: &mut FalsifiedParity| e.shadow.pair_count += 1,
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::Completed(EqualityOutcome {
                equal: true,
                pair_count: 2,
            })
        },
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::Completed(EqualityOutcome {
                equal: false,
                pair_count: 3,
            })
        },
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::Incomplete(fsl_replay::NativeCause::new(
                "stopped".to_owned(),
            ))
        },
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::Incomplete(fsl_replay::NativeCause::new(
                "other stop".to_owned(),
            ))
        },
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new(
                "execution fault".to_owned(),
            ))
        },
        |e: &mut FalsifiedParity| {
            e.native = NativeParityObservation::Refused(fsl_replay::Code::InvalidRuntimeInput)
        },
        |e: &mut FalsifiedParity| e.refinement = Refinement::CeilingReached,
    ] {
        let mut other = sent.clone();
        let CompositeEvidence::Falsified(ref mut evidence) = other.evidence else {
            unreachable!()
        };
        edit(evidence);
        changed.push(other);
    }
    for other in changed {
        assert_eq!(
            composite_parity_terminal_value(&other, Some(&report)).unwrap_err(),
            CompositeReportError::ClaimMismatch
        );
    }
    let mut other = sent.clone();
    other.evidence = CompositeEvidence::Verified(VerifiedShadow {
        success_checks: 3,
        refinement: Refinement::Exhausted,
    });
    assert_eq!(
        composite_parity_terminal_value(&other, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );
}

/// The report binds to the identity of a request changed before send, with no earlier unsent
/// identity influencing the result.
#[trace("TC-048")]
#[test]
fn tc_048_changed_request_binds_to_its_actual_sent_identity() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    let mut claim = fixture.claim;
    let evidence = falsified_evidence();
    let unsent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    claim.limits.work_units += 1;
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    let report = replay_composite_parity(wire, claim, evidence, ReplayLimits::default());
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        CompositeParityResult::Agrees { .. }
    ));
    assert_eq!(
        composite_parity_terminal_value(&unsent, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );
}

/// Native fault descriptors are part of the full observation on a generated-fault report.
#[trace("TC-048")]
#[test]
fn tc_048_native_cause_is_part_of_the_bound_observation() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    let claim = fixture.claim;
    let mut evidence = falsified_evidence();
    evidence.native = NativeParityObservation::Incomplete(fsl_replay::NativeCause::new(
        "actual native stop".to_owned(),
    ));
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Falsified(Box::new(evidence.clone())),
    );
    let report = replay_composite_parity(wire, claim, evidence, ReplayLimits::default());
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    let CompositeParityResult::GeneratedFault { native } = bound.result() else {
        panic!("expected F-2, got {:?}", bound.result());
    };
    assert!(
        matches!(native, NativeParityObservation::Incomplete(cause) if cause.as_str() == "actual native stop")
    );
    assert_eq!(bound.terminal_value(), TerminalValue::Failed);
    let mut other = sent.clone();
    let CompositeEvidence::Falsified(ref mut other_evidence) = other.evidence else {
        panic!("fixture is falsified");
    };
    other_evidence.native = NativeParityObservation::Incomplete(fsl_replay::NativeCause::new(
        "different stop".to_owned(),
    ));
    assert_eq!(
        composite_parity_terminal_value(&other, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );

    let fixture = public_composite_fixture();
    let mut evidence = falsified_evidence();
    evidence.native = NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new(
        "actual execution fault".to_owned(),
    ));
    let (sent, report) = public_report(
        fixture.wire.clone(),
        fixture.claim.clone(),
        evidence,
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(
        matches!(bound.result(), CompositeParityResult::GeneratedFault { native: NativeParityObservation::ExecutionFault(cause) } if cause.as_str() == "actual execution fault")
    );
    let mut other = sent.clone();
    let CompositeEvidence::Falsified(ref mut other_evidence) = other.evidence else {
        panic!("fixture is falsified");
    };
    other_evidence.native = NativeParityObservation::ExecutionFault(fsl_replay::NativeCause::new(
        "changed execution fault".to_owned(),
    ));
    assert_eq!(
        composite_parity_terminal_value(&other, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );

    let mut evidence = falsified_evidence();
    evidence.native = NativeParityObservation::Refused(fsl_replay::Code::InvalidRuntimeInput);
    let (sent, report) = public_report(
        fixture.wire,
        fixture.claim,
        evidence,
        ReplayLimits::default(),
    );
    let bound = composite_parity_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        CompositeParityResult::Agrees { .. }
    ));
    let mut other = sent;
    let CompositeEvidence::Falsified(ref mut other_evidence) = other.evidence else {
        panic!("fixture is falsified");
    };
    other_evidence.native = NativeParityObservation::Refused(fsl_replay::Code::InvalidRequest);
    assert_eq!(
        composite_parity_terminal_value(&other, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );
}

/// Verified reports use their own observation shape and preserve FSL's settlement.
#[trace("TC-048")]
#[test]
fn tc_048_verified_report_requires_success_count_and_refinement() {
    let fixture = public_composite_fixture();
    let wire = fixture.wire;
    let claim = fixture.claim;
    let verified = VerifiedShadow {
        success_checks: 3,
        refinement: Refinement::Exhausted,
    };
    let sent = CompositeIdentity::new(
        wire.obligation_identity.clone(),
        &claim,
        CompositeEvidence::Verified(verified),
    );
    let report = settle_verified_shadow(wire, claim, verified, ReplayLimits::default());
    let bound = verified_shadow_terminal_value(&sent, Some(&report)).unwrap();
    assert!(matches!(
        bound.result(),
        VerifiedShadowResult::Proved { success_checks: 3 }
    ));
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Proved {
            basis: ProofBasis::Checks { success_checks: 3 },
            certification: Certification::Certified,
        }
    );
    assert_eq!(
        verified_shadow_terminal_value(&sent, None).unwrap_err(),
        CompositeReportError::MissingReport
    );
    for evidence in [
        VerifiedShadow {
            success_checks: 4,
            ..verified
        },
        VerifiedShadow {
            refinement: Refinement::NotExhausted,
            ..verified
        },
    ] {
        let mut other = sent.clone();
        other.evidence = CompositeEvidence::Verified(evidence);
        assert_eq!(
            verified_shadow_terminal_value(&other, Some(&report)).unwrap_err(),
            CompositeReportError::ClaimMismatch
        );
    }
    let mut other = sent;
    other.evidence = CompositeEvidence::Falsified(Box::new(falsified_evidence()));
    assert_eq!(
        verified_shadow_terminal_value(&other, Some(&report)).unwrap_err(),
        CompositeReportError::ClaimMismatch
    );
}

/// A verified prepare refusal is a bound FSL report, with the same refusal code FSL emitted.
#[trace("TC-048")]
#[test]
fn tc_048_verified_prepare_refusal_passes_through() {
    let fixture = public_composite_fixture();
    let verified = VerifiedShadow {
        success_checks: 3,
        refinement: Refinement::Disagreed,
    };
    let sent = CompositeIdentity::new(
        fixture.wire.obligation_identity.clone(),
        &fixture.claim,
        CompositeEvidence::Verified(verified),
    );
    let report = settle_verified_shadow(
        fixture.wire,
        fixture.claim,
        verified,
        ReplayLimits::default().with_input_bytes(1),
    );
    let bound = verified_shadow_terminal_value(&sent, Some(&report)).unwrap();
    let VerifiedShadowResult::Refused(refusal) = bound.result() else {
        panic!("expected FSL prepare refusal, got {:?}", bound.result());
    };
    assert!(matches!(
        &**refusal,
        ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(_))
    ));
    assert_eq!(
        bound.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
    );
}

/// The two CG binding refusals stay distinct and human-readable.
#[trace("TC-048")]
#[test]
fn tc_048_binding_refusals_have_distinct_display() {
    assert_eq!(
        CompositeReportError::MissingReport.to_string(),
        "composite parity report is missing"
    );
    assert_eq!(
        CompositeReportError::ClaimMismatch.to_string(),
        "composite parity report claim differs from sent claim"
    );
}

/// Every strength, built by a match with no wildcard arm: a new `ProofStrength` stops this
/// compiling until it has a row here (FR-029-AC-22).
fn every_strength() -> [ProofStrength; 6] {
    let all = [
        ProofStrength::ProductionProved,
        ProofStrength::ShadowProvedRefinementExhaustive,
        ProofStrength::ShadowProvedRefinementSampled,
        ProofStrength::ShadowProvedRefinementNotRun,
        ProofStrength::ShadowProvedRefinementInconclusive,
        ProofStrength::RefinementFailed,
    ];
    for strength in all {
        match strength {
            ProofStrength::ProductionProved
            | ProofStrength::ShadowProvedRefinementExhaustive
            | ProofStrength::ShadowProvedRefinementSampled
            | ProofStrength::ShadowProvedRefinementNotRun
            | ProofStrength::ShadowProvedRefinementInconclusive
            | ProofStrength::RefinementFailed => {}
        }
    }
    all
}

/// FR-029-AC-17: each strength projects onto the FSL refinement state the spec states, and
/// production proof is never composite shadow evidence.
#[trace("FR-029-AC-17", "TC-048")]
#[test]
fn tc_048_every_strength_projects_onto_its_refinement_state() {
    let expected = [
        (ProofStrength::ProductionProved, None),
        (
            ProofStrength::ShadowProvedRefinementExhaustive,
            Some(Refinement::Exhausted),
        ),
        (
            ProofStrength::ShadowProvedRefinementSampled,
            Some(Refinement::NotExhausted),
        ),
        (
            ProofStrength::ShadowProvedRefinementNotRun,
            Some(Refinement::NotExhausted),
        ),
        (
            ProofStrength::ShadowProvedRefinementInconclusive,
            Some(Refinement::CeilingReached),
        ),
        (ProofStrength::RefinementFailed, Some(Refinement::Disagreed)),
    ];
    for (strength, refinement) in expected {
        assert_eq!(strength.parity_refinement(), refinement, "{strength:?}");
    }
}

/// FR-029-AC-17: the strength producer, fed a bounded-shadow outcome that did not verify and
/// whose refinement did not disagree, yields no strength, so no verified strength is invented
/// for a backend-inconclusive or cover-unsatisfied run.
#[trace("FR-029-AC-17", "TC-048")]
#[test]
fn tc_048_a_non_verified_shadow_outcome_acquires_no_strength() {
    let reasons = [
        KaniInconclusiveReason::FailedWithoutCounterexample,
        KaniInconclusiveReason::NoVerdict,
        KaniInconclusiveReason::MissingCoverSummary,
        KaniInconclusiveReason::VacuousProof,
        KaniInconclusiveReason::UnwindBoundExhausted,
        KaniInconclusiveReason::TimedOut,
        KaniInconclusiveReason::WitnessDisagreed,
    ];
    let mut outcomes: Vec<KaniRunOutcome> = reasons
        .into_iter()
        .map(|reason| KaniRunOutcome::Inconclusive { reason })
        .collect();
    outcomes.push(KaniRunOutcome::CoverUnsatisfied {
        satisfied: 0,
        total: 1,
    });
    outcomes.push(KaniRunOutcome::Falsified {
        counterexample: "synthetic playback".into(),
    });
    for outcome in &outcomes {
        for refinement in [
            None,
            Some(RefinementSummary::Agreed(RefinementClass::Exhaustive)),
            Some(RefinementSummary::Agreed(RefinementClass::Sampled)),
            Some(RefinementSummary::ReachedCeiling),
        ] {
            assert_eq!(
                proof_strength(ProofSubject::BoundedShadow, outcome, refinement),
                None,
                "{outcome:?} with {refinement:?}"
            );
        }
        assert_eq!(
            proof_strength(
                ProofSubject::BoundedShadow,
                outcome,
                Some(RefinementSummary::Disagreed)
            ),
            Some(ProofStrength::RefinementFailed),
            "a disagreement is the strength whatever the backend settled: {outcome:?}"
        );
    }
}

/// FR-029-AC-22: the strengths the map enumerates are exactly the six FR-028-AC-17 states, and
/// each has a projection and a settled terminal value.
#[trace("FR-029-AC-22", "TC-048")]
#[test]
fn tc_048_the_enumerated_strengths_equal_the_closed_set_of_the_spec() {
    let spec = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/spec/kani/functional/FR-028-bounded-proof-ceilings.md"
    ))
    .expect("FR-028 is in this repository");
    let row = spec
        .lines()
        .find(|line| line.starts_with("| FR-028-AC-17 |"))
        .expect("FR-028-AC-17 row");
    let mut stated: Vec<String> = row
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| {
            *token == "production_proved"
                || *token == "refinement_failed"
                || token.starts_with("shadow_proved_")
        })
        .map(str::to_owned)
        .collect();
    stated.sort();
    stated.dedup();
    let mut enumerated: Vec<String> = every_strength()
        .into_iter()
        .map(|strength| {
            serde_json::to_value(strength)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    enumerated.sort();
    assert_eq!(enumerated, stated);
    assert_eq!(stated.len(), 6);
}

/// FR-029-AC-17, AC-22 and AC-31: every projected strength, settled by FSL's delivered
/// verified-shadow facade, gives the terminal value FSL's own report carries, and CG adds no
/// cause of its own.
#[trace("FR-029-AC-17", "FR-029-AC-22", "FR-029-AC-31", "TC-048")]
#[test]
fn tc_048_each_strength_settles_to_the_terminal_value_fsl_reports() {
    let fixture = public_composite_fixture();
    let expected = |strength: ProofStrength| match strength {
        ProofStrength::ProductionProved => None,
        ProofStrength::ShadowProvedRefinementExhaustive => Some(TerminalValue::Proved {
            basis: ProofBasis::Checks { success_checks: 3 },
            certification: Certification::Certified,
        }),
        ProofStrength::ShadowProvedRefinementSampled
        | ProofStrength::ShadowProvedRefinementNotRun => Some(TerminalValue::Tested),
        ProofStrength::ShadowProvedRefinementInconclusive => Some(TerminalValue::Incomplete(
            fsl_replay::IncompleteCause::ResourceExhausted,
        )),
        ProofStrength::RefinementFailed => Some(TerminalValue::Failed),
    };
    for strength in every_strength() {
        let Some(refinement) = strength.parity_refinement() else {
            assert_eq!(expected(strength), None);
            continue;
        };
        let verified = VerifiedShadow {
            success_checks: 3,
            refinement,
        };
        let sent = CompositeIdentity::new(
            fixture.wire.obligation_identity.clone(),
            &fixture.claim,
            CompositeEvidence::Verified(verified),
        );
        let report = settle_verified_shadow(
            fixture.wire.clone(),
            fixture.claim.clone(),
            verified,
            ReplayLimits::default(),
        );
        let bound = verified_shadow_terminal_value(&sent, Some(&report)).unwrap();
        assert_eq!(
            Some(bound.terminal_value()),
            expected(strength),
            "{strength:?}"
        );
        assert_eq!(
            bound.terminal_value(),
            report.terminal_value(),
            "{strength:?}"
        );
    }
}

/// FR-029-AC-21: the ordinary map takes the run's evidence and an optional replay settlement
/// and nothing about a proof subject or strength, and its refusal has no bounded-shadow
/// variant. Adding an input or a variant stops this compiling.
#[trace("FR-029-AC-21", "TC-040", "TC-048")]
#[test]
fn tc_040_the_ordinary_map_has_no_shadow_input_or_variant() {
    let _: fn(
        &KaniExecutionEvidence,
        Option<ReplaySettlement<'_>>,
    ) -> Result<TerminalValue, TerminalPairError> = run_terminal_value;
    let name = |error: &TerminalPairError| match error {
        TerminalPairError::MissingSettlement => "missing_settlement",
        TerminalPairError::UnexpectedSettlement => "unexpected_settlement",
        TerminalPairError::MissingProvedCount => "missing_proved_count",
        TerminalPairError::ProvedCountOutOfRange { .. } => "proved_count_out_of_range",
        TerminalPairError::InvalidCancellation(_) => "invalid_cancellation",
    };
    assert_eq!(
        name(&TerminalPairError::MissingSettlement),
        "missing_settlement"
    );
}
