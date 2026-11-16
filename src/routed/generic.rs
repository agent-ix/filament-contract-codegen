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
use serde::Serialize;

use crate::routed::capability::{
    BackendDescriptor, CapabilityKind, Cause, Disposition, ExtentClassification, Mode,
};

/// The dynamics a backend decides (FSpec FR-193 "Routing a hybrid item").
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dynamics {
    /// Temporal items whose subject has no hybrid dynamics.
    Discrete,
    /// Items that carry the hybrid-dynamics requirement.
    Hybrid,
}

/// A fairness kind a backend decides (FSpec FR-368).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FairnessKind {
    /// Weak fairness.
    Weak,
    /// Strong fairness.
    Strong,
}

/// A state-graph claim form a backend decides (FSpec FR-390).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StateGraphForm {
    /// `possible`.
    Possible,
    /// `always possible`.
    AlwaysPossible,
    /// `unique path`.
    UniquePath,
}

/// A state-space reduction a backend applies (FSpec FR-382).
///
/// Declaration order is the order in which negotiation names the first
/// unadvertised reduction of an item.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reduction {
    /// Symmetry reduction.
    Symmetry,
    /// Partial-order reduction.
    PartialOrder,
    /// A state constraint.
    Constraint,
}

/// An obligation form of the SMT backend's declared set (FSpec FR-349).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObligationForm {
    /// An FR-346 quantification obligation.
    Quantification,
    /// An FR-347 induction obligation.
    Induction,
    /// An FR-348 composition obligation.
    Composition,
    /// An FR-147 mathematical-integer obligation.
    MathematicalInteger,
}

/// The evidence kind of a probabilistic item (FSpec FR-290 "Evidence kinds").
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    /// Statistical evidence.
    Statistical,
    /// Exact evidence.
    Exact,
}

/// The evidence kind an item names when it is not one of the two.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestedEvidence {
    /// The item names one of the two evidence kinds.
    Known(EvidenceKind),
    /// The item names a label that is neither, reported verbatim.
    Unknown(String),
}

/// What a manifest entry declares beyond identity, modes and domains (FSpec FR-331).
///
/// Every member defaults to the empty set, which is how the manifest spells
/// an absent member: a backend that declares none decides `discrete`
/// dynamics only, no fairness kind, no state-graph form, no reduction, no
/// obligation form and no evidence kind.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendProfile {
    /// The dynamics decided; empty means `discrete` only.
    pub dynamics: BTreeSet<Dynamics>,
    /// The fairness kinds decided.
    pub fairness_kinds: BTreeSet<FairnessKind>,
    /// The state-graph claim forms decided.
    pub state_graph_forms: BTreeSet<StateGraphForm>,
    /// The state-space reductions applied.
    pub reductions: BTreeSet<Reduction>,
    /// The obligation forms discharged.
    pub obligation_forms: BTreeSet<ObligationForm>,
    /// The evidence kinds advertised for `probabilistic-satisfaction`.
    pub evidence_kinds: BTreeSet<EvidenceKind>,
}

impl BackendProfile {
    fn decides(&self, dynamics: Dynamics) -> bool {
        if self.dynamics.is_empty() {
            dynamics == Dynamics::Discrete
        } else {
            self.dynamics.contains(&dynamics)
        }
    }
}

/// The requirements an item carries beyond kind and extent (FSpec FR-331).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ItemRequirements {
    /// The evidence kind a `probabilistic-satisfaction` item names; absent when it names none.
    pub evidence: Option<RequestedEvidence>,
    /// Whether the item carries the `hybrid-dynamics` requirement.
    pub hybrid_dynamics: bool,
    /// The item's resolved fairness set.
    pub fairness: BTreeSet<FairnessKind>,
    /// The state-graph form of a state-graph item.
    pub state_graph_form: Option<StateGraphForm>,
    /// The reductions the item selects.
    pub reductions: BTreeSet<Reduction>,
    /// The obligation form of an SMT-discharged item.
    pub obligation_form: Option<ObligationForm>,
}

/// One requirement a candidate does not advertise, named by an `unsupported` warning.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "requirement", content = "name")]
pub enum Requirement {
    /// A fairness kind.
    Fairness(FairnessKind),
    /// A state-graph claim form.
    StateGraphForm(StateGraphForm),
    /// A state-space reduction.
    Reduction(Reduction),
    /// An SMT obligation form.
    ObligationForm(ObligationForm),
}

impl Requirement {
    /// The requirement as the warning spells it.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Self::Fairness(kind) => format!("fairness kind {}", wire(kind)),
            Self::StateGraphForm(form) => format!("state-graph form {}", wire(form)),
            Self::Reduction(reduction) => format!("reduction {}", wire(reduction)),
            Self::ObligationForm(form) => format!("obligation form {}", wire(form)),
        }
    }
}

fn wire<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

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
            candidates: vec![crate::routed::capability::Candidate {
                identity: backend.identity.clone(),
            }],
        },
    }
}
