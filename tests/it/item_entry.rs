//! FSpec FR-711: one item entry settles an item on every admitted registration.
//!
//! The generic arm reads the manifest entry and the item alone, so a linked
//! engine, a process provider and a Kani backend settle the same item the same
//! way; a backend-specific arm only adds to that.

use std::collections::BTreeSet;

use filament_contract_codegen::{
    negotiate_item, BackendDescriptor, BackendProfile, Candidate, Candidates, CapabilityKind,
    Cause, Disposition, Dynamics, EvidenceKind, ExtentClassification, ExtentDomain, FairnessKind,
    ItemRequirements, Mode, ObligationForm, ProviderOrigin, Reduction, RequestItem,
    RequestedEvidence, RequestedKind, Requirement, StateGraphForm,
};
use filament_exact::PathSegment;
use fsl_replay::{DomainKey, DomainKind, NodeKey};
use ix_trace_rs::trace;

const KIND: CapabilityKind = CapabilityKind::TemporalSatisfaction;

fn engine(
    identity: &str,
    origin: ProviderOrigin,
    modes: &[Mode],
    domains: &[DomainKind],
) -> BackendDescriptor {
    BackendDescriptor {
        identity: identity.to_owned(),
        advertised: modes.iter().map(|mode| (KIND, *mode)).collect(),
        origin,
        domains: Some(domains.iter().copied().collect::<BTreeSet<_>>()),
        bounds: None,
        profile: BackendProfile::default(),
    }
}

fn extent(
    mode: Mode,
    finite_bound_available: bool,
    domains: Vec<ExtentDomain>,
) -> ExtentClassification {
    ExtentClassification {
        extent: mode,
        finite_bound_available,
        domains,
        bounds: Vec::new(),
    }
}

fn item_for(backend: &str, extent: ExtentClassification) -> RequestItem {
    RequestItem {
        kind: RequestedKind::Known(KIND),
        extent: Some(extent),
        named_backend: None,
        candidates: Candidates::Set(vec![Candidate {
            identity: backend.to_owned(),
        }]),
        kani_form: None,
        requirements: ItemRequirements::default(),
    }
}

fn settle(manifest: &[BackendDescriptor], item: &RequestItem) -> Disposition {
    negotiate_item(manifest, item).expect("an item over a well-formed manifest is not refused")
}

fn domain(kind: DomainKind) -> ExtentDomain {
    ExtentDomain {
        domain: DomainKey::Node {
            node: NodeKey::new("fixture", vec![PathSegment::Text("d".to_owned())], 0)
                .expect("fixture node"),
            path: Vec::new(),
        },
        kind,
    }
}

fn supported(backend: &str) -> Disposition {
    Disposition::Supported {
        backend: backend.to_owned(),
    }
}

fn with_profile(mut backend: BackendDescriptor, profile: BackendProfile) -> BackendDescriptor {
    backend.profile = profile;
    backend
}

/// A linked engine advertising both modes settles bounded and unbounded items, never `unknown-backend`.
#[trace("FR-711-AC-1", "TC-1741")]
#[test]
fn linked_engine_with_both_modes_settles_bounded_and_unbounded_items() {
    let manifest = [engine(
        "en-1",
        ProviderOrigin::Linked,
        &[Mode::Unbounded, Mode::Bounded],
        &[DomainKind::Integer],
    )];
    for mode in [Mode::Bounded, Mode::Unbounded] {
        let item = item_for("en-1", extent(mode, false, Vec::new()));
        assert_eq!(settle(&manifest, &item), supported("en-1"), "{mode:?}");
    }
}

/// Every row of the advertised-mode table settles its listed disposition, equally for a linked and a process candidate.
#[trace("FR-711-AC-2", "TC-1741")]
#[test]
fn every_advertised_mode_row_settles_alike_for_linked_and_process_candidates() {
    let rows: [(Mode, bool, &[Mode], Disposition); 6] = [
        (Mode::Bounded, false, &[Mode::Bounded], supported("en-1")),
        (
            Mode::Bounded,
            false,
            &[Mode::Unbounded],
            unsupported_capability(),
        ),
        (
            Mode::Unbounded,
            false,
            &[Mode::Unbounded],
            supported("en-1"),
        ),
        (
            Mode::Unbounded,
            true,
            &[Mode::Bounded],
            Disposition::RequiresBound {
                backend: "en-1".to_owned(),
            },
        ),
        (
            Mode::Unbounded,
            false,
            &[Mode::Bounded],
            Disposition::Unsupported {
                cause: Cause::UnboundedExtent {
                    kind: KIND,
                    backend: "en-1".to_owned(),
                },
            },
        ),
        (
            Mode::Unbounded,
            true,
            &[Mode::Bounded, Mode::Unbounded],
            supported("en-1"),
        ),
    ];
    for (mode, finite_bound_available, advertised, expected) in rows {
        let item = item_for("en-1", extent(mode, finite_bound_available, Vec::new()));
        for origin in [ProviderOrigin::Linked, ProviderOrigin::Process] {
            let manifest = [engine("en-1", origin, advertised, &[DomainKind::Integer])];
            assert_eq!(
                settle(&manifest, &item),
                expected,
                "{mode:?} bound={finite_bound_available} advertised={advertised:?} {origin:?}"
            );
        }
    }
}

fn unsupported_capability() -> Disposition {
    Disposition::Unsupported {
        cause: Cause::UnsupportedRequestedCapability {
            kind: KIND,
            backend: Some("en-1".to_owned()),
        },
    }
}

/// The domain-kind check runs before the mode table and names the uncovered domain kind.
#[trace("FR-711-AC-2", "TC-1741")]
#[test]
fn domain_kind_check_precedes_the_mode_table_for_linked_and_process_candidates() {
    let item = item_for(
        "en-1",
        extent(Mode::Unbounded, true, vec![domain(DomainKind::Population)]),
    );
    for origin in [ProviderOrigin::Linked, ProviderOrigin::Process] {
        let manifest = [engine(
            "en-1",
            origin,
            &[Mode::Bounded],
            &[DomainKind::Integer],
        )];
        assert_eq!(
            settle(&manifest, &item),
            Disposition::Unsupported {
                cause: Cause::UnsupportedDomain {
                    kind: KIND,
                    domain: DomainKind::Population,
                    backend: "en-1".to_owned(),
                }
            },
            "{origin:?}"
        );
    }
}

/// An item with no backend-specific form settles on a linked and a process candidate; a Kani candidate that reads a form refuses its absence.
#[trace("FR-711-AC-3", "TC-1741")]
#[test]
fn item_without_a_form_settles_except_on_a_kani_candidate() {
    let item = |backend: &str| item_for(backend, extent(Mode::Bounded, false, Vec::new()));
    for origin in [ProviderOrigin::Linked, ProviderOrigin::Process] {
        let manifest = [engine(
            "en-1",
            origin,
            &[Mode::Bounded],
            &[DomainKind::Integer],
        )];
        assert_eq!(settle(&manifest, &item("en-1")), supported("en-1"));
    }
    let kani = [engine(
        "kani",
        ProviderOrigin::Linked,
        &[Mode::Bounded],
        &[DomainKind::Integer],
    )];
    let settled = settle(&kani, &item("kani"));
    assert_eq!(
        settled,
        Disposition::InvalidRequest {
            cause: Cause::InconsistentCandidates {
                candidates: vec![Candidate {
                    identity: "kani".to_owned()
                }]
            }
        }
    );
    let Disposition::InvalidRequest { cause } = settled else {
        unreachable!("asserted above");
    };
    assert_eq!(cause.code(), "invalid_capability");
}

/// The generic arm settles before the Kani arm reads its form: an unadvertised mode is not hidden by a missing form.
#[trace("FR-711-AC-3", "TC-1741")]
#[test]
fn generic_arm_settles_before_the_backend_specific_arm() {
    let kani = [engine(
        "kani",
        ProviderOrigin::Linked,
        &[Mode::Unbounded],
        &[DomainKind::Integer],
    )];
    let item = item_for("kani", extent(Mode::Bounded, false, Vec::new()));
    assert_eq!(
        settle(&kani, &item),
        Disposition::Unsupported {
            cause: Cause::UnsupportedRequestedCapability {
                kind: KIND,
                backend: Some("kani".to_owned())
            }
        }
    );
}

/// A state-graph form in the candidate's list settles; one absent from it settles unsupported, and an empty candidate set does too.
#[trace("FR-711-AC-4", "TC-1741")]
#[test]
fn state_graph_form_is_settled_against_the_candidates_declared_forms() {
    let profile = BackendProfile {
        state_graph_forms: BTreeSet::from([StateGraphForm::Possible]),
        ..BackendProfile::default()
    };
    let manifest = [with_profile(
        engine("en-1", ProviderOrigin::Linked, &[Mode::Unbounded], &[]),
        profile,
    )];
    let mut item = item_for("en-1", extent(Mode::Unbounded, false, Vec::new()));
    item.requirements.state_graph_form = Some(StateGraphForm::Possible);
    assert_eq!(settle(&manifest, &item), supported("en-1"));

    item.requirements.state_graph_form = Some(StateGraphForm::AlwaysPossible);
    let settled = settle(&manifest, &item);
    let Disposition::Unsupported { cause } = &settled else {
        panic!("expected unsupported, got {settled:?}");
    };
    assert_eq!(
        cause,
        &Cause::UnsupportedRequirement {
            kind: KIND,
            requirement: Requirement::StateGraphForm(StateGraphForm::AlwaysPossible),
            backend: "en-1".to_owned(),
        }
    );
    assert_eq!(cause.code(), "unsupported_projection");
    assert_eq!(
        serde_json::to_value(cause).expect("cause serializes")["cause"],
        "unsupported-requested-capability"
    );

    item.candidates = Candidates::Set(Vec::new());
    assert_eq!(
        settle(&manifest, &item),
        Disposition::Unsupported {
            cause: Cause::UnsupportedRequestedCapability {
                kind: KIND,
                backend: None
            }
        }
    );
}

/// Fairness, reduction and obligation-form rules name the first requirement the candidate does not advertise, in rule order.
#[trace("FR-711-AC-2", "TC-1741")]
#[test]
fn fairness_reduction_and_obligation_rules_name_the_first_missing_requirement() {
    let profile = BackendProfile {
        fairness_kinds: BTreeSet::from([FairnessKind::Weak]),
        reductions: BTreeSet::from([Reduction::Symmetry]),
        obligation_forms: BTreeSet::from([ObligationForm::Induction]),
        ..BackendProfile::default()
    };
    let manifest = [with_profile(
        engine("en-1", ProviderOrigin::Linked, &[Mode::Unbounded], &[]),
        profile,
    )];
    let missing = |requirement| Disposition::Unsupported {
        cause: Cause::UnsupportedRequirement {
            kind: KIND,
            requirement,
            backend: "en-1".to_owned(),
        },
    };
    let mut item = item_for("en-1", extent(Mode::Unbounded, false, Vec::new()));
    item.requirements = ItemRequirements {
        fairness: BTreeSet::from([FairnessKind::Weak]),
        reductions: BTreeSet::from([Reduction::Symmetry]),
        obligation_form: Some(ObligationForm::Induction),
        ..ItemRequirements::default()
    };
    assert_eq!(settle(&manifest, &item), supported("en-1"));

    // Every requirement missing: the fairness rule runs first.
    item.requirements = ItemRequirements {
        fairness: BTreeSet::from([FairnessKind::Weak, FairnessKind::Strong]),
        reductions: BTreeSet::from([Reduction::PartialOrder]),
        obligation_form: Some(ObligationForm::Composition),
        ..ItemRequirements::default()
    };
    assert_eq!(
        settle(&manifest, &item),
        missing(Requirement::Fairness(FairnessKind::Strong))
    );
    item.requirements.fairness.clear();
    assert_eq!(
        settle(&manifest, &item),
        missing(Requirement::Reduction(Reduction::PartialOrder))
    );
    item.requirements.reductions.clear();
    assert_eq!(
        settle(&manifest, &item),
        missing(Requirement::ObligationForm(ObligationForm::Composition))
    );
}

/// The evidence-kind rules refuse an absent or unknown evidence kind and an unadvertised one, whatever the provider's origin.
#[trace("FR-711-AC-2", "TC-1741")]
#[test]
fn probabilistic_items_name_one_advertised_evidence_kind() {
    let profile = BackendProfile {
        evidence_kinds: BTreeSet::from([EvidenceKind::Exact]),
        ..BackendProfile::default()
    };
    let mut backend = engine("en-1", ProviderOrigin::Linked, &[Mode::Unbounded], &[]);
    backend.advertised = vec![(CapabilityKind::ProbabilisticSatisfaction, Mode::Unbounded)];
    let manifest = [with_profile(backend, profile)];
    let mut item = item_for("en-1", extent(Mode::Unbounded, false, Vec::new()));
    item.kind = RequestedKind::Known(CapabilityKind::ProbabilisticSatisfaction);

    let invalid = |cause| Disposition::InvalidRequest { cause };
    assert_eq!(settle(&manifest, &item), invalid(Cause::AbsentEvidenceKind));
    item.requirements.evidence = Some(RequestedEvidence::Unknown("sampled".to_owned()));
    assert_eq!(
        settle(&manifest, &item),
        invalid(Cause::UnknownEvidenceKind {
            received: "sampled".to_owned()
        })
    );
    item.requirements.evidence = Some(RequestedEvidence::Known(EvidenceKind::Statistical));
    assert_eq!(
        settle(&manifest, &item),
        invalid(Cause::InconsistentCandidates {
            candidates: vec![Candidate {
                identity: "en-1".to_owned()
            }]
        })
    );
    item.requirements.evidence = Some(RequestedEvidence::Known(EvidenceKind::Exact));
    assert_eq!(settle(&manifest, &item), supported("en-1"));
}

/// A candidate with no `dynamics` decides discrete items only; one holding `hybrid` decides hybrid items.
#[trace("FR-711-AC-2", "TC-1741")]
#[test]
fn hybrid_dynamics_requirement_routes_by_the_candidates_dynamics() {
    let inconsistent = Disposition::InvalidRequest {
        cause: Cause::InconsistentCandidates {
            candidates: vec![Candidate {
                identity: "en-1".to_owned(),
            }],
        },
    };
    let discrete_only = engine("en-1", ProviderOrigin::Linked, &[Mode::Unbounded], &[]);
    let hybrid_only = with_profile(
        discrete_only.clone(),
        BackendProfile {
            dynamics: BTreeSet::from([Dynamics::Hybrid]),
            ..BackendProfile::default()
        },
    );
    let both = with_profile(
        discrete_only.clone(),
        BackendProfile {
            dynamics: BTreeSet::from([Dynamics::Discrete, Dynamics::Hybrid]),
            ..BackendProfile::default()
        },
    );
    let mut item = item_for("en-1", extent(Mode::Unbounded, false, Vec::new()));
    for (backend, discrete, hybrid) in [
        (discrete_only, supported("en-1"), inconsistent.clone()),
        (hybrid_only, inconsistent, supported("en-1")),
        (both, supported("en-1"), supported("en-1")),
    ] {
        let manifest = [backend];
        item.requirements.hybrid_dynamics = false;
        assert_eq!(settle(&manifest, &item), discrete);
        item.requirements.hybrid_dynamics = true;
        assert_eq!(settle(&manifest, &item), hybrid);
    }
}
