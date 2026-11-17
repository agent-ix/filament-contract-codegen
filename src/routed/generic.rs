//! The generic negotiation arm every admitted registration has (FSpec FR-711).
//!
//! An item is compared with what a registration advertises in its FR-331
//! manifest entry. The rules here read the manifest entry and the item alone,
//! so the same item over the same manifest settles the same whether the
//! candidate is a linked engine, a process provider or a Kani backend. A
//! backend whose IR forms need their own check adds it after this arm in
//! `capability.rs` and never changes what this arm settled.

use std::collections::BTreeSet;

use fsl_replay::ProofBound;

use crate::routed::vocabulary::{
    BackendDescriptor, CapabilityKind, Cause, Disposition, Dynamics, ExtentClassification,
    ItemRequirements, Mode, RequestedEvidence, Requirement,
};

/// Settle one item on one candidate from the manifest entry and the item alone.
///
/// The rules run in FSpec FR-711's order: the domain-kind check, the
/// advertised-mode table, the evidence-kind rules, the hybrid-dynamics rule,
/// the fairness-kind rule, the state-graph form rule, the reduction rule and
/// the obligation-form rule. The first rule that does not pass settles the
/// item; when all pass the item is `supported` on the candidate.
pub(crate) fn negotiate_generic(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    extent: &ExtentClassification,
    needs: &ItemRequirements,
) -> Disposition {
    let settled = negotiate_extent(backend, kind, extent);
    if !matches!(settled, Disposition::Supported { .. }) {
        return settled;
    }
    negotiate_requirements(backend, kind, needs).unwrap_or(settled)
}

/// The domain-kind check and the advertised-mode table (FSpec FR-290).
fn negotiate_extent(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    extent: &ExtentClassification,
) -> Disposition {
    let modes = backend.modes_for(kind);
    let advertised_domains = backend.domains.as_ref();
    let unsupported_domain = match extent.extent {
        Mode::Bounded => extent
            .bounds
            .iter()
            .filter_map(ProofBound::kind)
            .find(|domain| {
                !advertised_domains.is_some_and(|advertised| advertised.contains(domain))
            }),
        Mode::Unbounded if modes.contains(&Mode::Unbounded) => None,
        Mode::Unbounded => extent
            .domains
            .iter()
            .map(|domain| domain.kind)
            .find(|domain| {
                domain.finite_kind().is_some()
                    && !advertised_domains.is_some_and(|advertised| advertised.contains(domain))
            }),
    };
    if let Some(domain) = unsupported_domain {
        return Disposition::Unsupported {
            cause: Cause::UnsupportedDomain {
                kind,
                domain,
                backend: backend.identity.clone(),
            },
        };
    }
    match extent.extent {
        Mode::Bounded if modes.contains(&Mode::Bounded) => supported(backend),
        Mode::Bounded => Disposition::Unsupported {
            cause: Cause::UnsupportedRequestedCapability {
                kind,
                backend: Some(backend.identity.clone()),
            },
        },
        Mode::Unbounded if modes.contains(&Mode::Unbounded) => supported(backend),
        Mode::Unbounded if extent.finite_bound_available => Disposition::RequiresBound {
            backend: backend.identity.clone(),
        },
        Mode::Unbounded => Disposition::Unsupported {
            cause: Cause::UnboundedExtent {
                kind,
                backend: backend.identity.clone(),
            },
        },
    }
}

fn supported(backend: &BackendDescriptor) -> Disposition {
    Disposition::Supported {
        backend: backend.identity.clone(),
    }
}

/// The rules after the mode table; `None` when every one passes.
fn negotiate_requirements(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    needs: &ItemRequirements,
) -> Option<Disposition> {
    negotiate_evidence(backend, kind, needs)
        .or_else(|| negotiate_dynamics(backend, kind, needs))
        .or_else(|| {
            first_missing(&needs.fairness, &backend.profile.fairness_kinds)
                .map(Requirement::Fairness)
                .map(|requirement| unsupported_requirement(backend, kind, requirement))
        })
        .or_else(|| {
            needs
                .state_graph_form
                .filter(|form| !backend.profile.state_graph_forms.contains(form))
                .map(Requirement::StateGraphForm)
                .map(|requirement| unsupported_requirement(backend, kind, requirement))
        })
        .or_else(|| {
            first_missing(&needs.reductions, &backend.profile.reductions)
                .map(Requirement::Reduction)
                .map(|requirement| unsupported_requirement(backend, kind, requirement))
        })
        .or_else(|| {
            needs
                .obligation_form
                .filter(|form| !backend.profile.obligation_forms.contains(form))
                .map(Requirement::ObligationForm)
                .map(|requirement| unsupported_requirement(backend, kind, requirement))
        })
}

fn first_missing<T: Ord + Copy>(wanted: &BTreeSet<T>, held: &BTreeSet<T>) -> Option<T> {
    wanted.iter().copied().find(|item| !held.contains(item))
}

fn unsupported_requirement(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    requirement: Requirement,
) -> Disposition {
    Disposition::Unsupported {
        cause: Cause::UnsupportedRequirement {
            kind,
            requirement,
            backend: backend.identity.clone(),
        },
    }
}

/// A `probabilistic-satisfaction` item names exactly one evidence kind and a
/// candidate advertises it (FSpec FR-290 "Evidence kinds").
fn negotiate_evidence(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    needs: &ItemRequirements,
) -> Option<Disposition> {
    if kind != CapabilityKind::ProbabilisticSatisfaction {
        return None;
    }
    match &needs.evidence {
        None => Some(Disposition::InvalidRequest {
            cause: Cause::AbsentEvidenceKind,
        }),
        Some(RequestedEvidence::Unknown(received)) => Some(Disposition::InvalidRequest {
            cause: Cause::UnknownEvidenceKind {
                received: received.clone(),
            },
        }),
        Some(RequestedEvidence::Known(evidence)) => {
            (!backend.profile.evidence_kinds.contains(evidence)).then(|| inconsistent(backend))
        }
    }
}

/// A candidate decides `hybrid` for an item carrying the hybrid-dynamics
/// requirement and `discrete` for a temporal item carrying none (FSpec FR-193).
fn negotiate_dynamics(
    backend: &BackendDescriptor,
    kind: CapabilityKind,
    needs: &ItemRequirements,
) -> Option<Disposition> {
    if kind != CapabilityKind::TemporalSatisfaction {
        return None;
    }
    let required = if needs.hybrid_dynamics {
        Dynamics::Hybrid
    } else {
        Dynamics::Discrete
    };
    (!backend.profile.decides(required)).then(|| inconsistent(backend))
}

fn inconsistent(backend: &BackendDescriptor) -> Disposition {
    Disposition::InvalidRequest {
        cause: Cause::InconsistentCandidates {
            candidates: vec![crate::routed::vocabulary::Candidate {
                identity: backend.identity.clone(),
            }],
        },
    }
}
