//! The value types settlement is written in: what a backend advertises, what an item
//! requires, and how an item settles (FSpec FR-290, FR-331, FR-711).
//!
//! They sit below both [`super::generic`] and [`super::capability`], which each read
//! them, so neither module imports the other's types.

use std::collections::{BTreeMap, BTreeSet};

use fsl_replay::{DomainKey, DomainKind, ProofBound};
use serde::Serialize;

fn serialize_domain_kind<S: serde::Serializer>(
    kind: &DomainKind,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(kind.to_wire())
}

/// The twelve FR-290 capability kinds.
///
/// The vocabulary is closed. A label outside it is refused with its exact
/// received bytes and never mapped onto a member.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityKind {
    /// Whether a Boolean clause holds for every assignment of its domain.
    ValueValidity,
    /// Whether an operation's contract clauses hold for every admitted pre-state.
    OperationContract,
    /// Whether one finite execution conforms step by step.
    FiniteReplay,
    /// Whether a temporal requirement holds over its profile's traces.
    TemporalSatisfaction,
    /// Finite global protocol conformance under the selected closure premises.
    GlobalConformance,
    /// Whether the selected fragment can be monitored under the declared observations.
    Monitorability,
    /// Whether one participant projection is defined under its premises.
    LocalProjection,
    /// Whether one protocol refines another under a selected relation.
    Refinement,
    /// Whether the protocol admits a causal implementation strategy.
    Realizability,
    /// Whether selected components compose under independently discharged assumptions.
    Composition,
    /// Whether a probabilistic claim's bound holds, with statistical or exact evidence.
    ProbabilisticSatisfaction,
    /// Whether a task set is schedulable under a named scheduling policy.
    Schedulability,
}

impl CapabilityKind {
    /// Every member, in the order FR-290 declares them.
    ///
    /// Declaration order carries no strength and no dispatch precedence; this
    /// is a census, not a ranking.
    pub const ALL: [Self; 12] = [
        Self::ValueValidity,
        Self::OperationContract,
        Self::FiniteReplay,
        Self::TemporalSatisfaction,
        Self::GlobalConformance,
        Self::Monitorability,
        Self::LocalProjection,
        Self::Refinement,
        Self::Realizability,
        Self::Composition,
        Self::ProbabilisticSatisfaction,
        Self::Schedulability,
    ];

    /// The exact label this kind serializes as.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ValueValidity => "value-validity",
            Self::OperationContract => "operation-contract",
            Self::FiniteReplay => "finite-replay",
            Self::TemporalSatisfaction => "temporal-satisfaction",
            Self::GlobalConformance => "global-conformance",
            Self::Monitorability => "monitorability",
            Self::LocalProjection => "local-projection",
            Self::Refinement => "refinement",
            Self::Realizability => "realizability",
            Self::Composition => "composition",
            Self::ProbabilisticSatisfaction => "probabilistic-satisfaction",
            Self::Schedulability => "schedulability",
        }
    }

    /// The member whose label is byte-equal to `label`.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.label() == label)
    }

    /// This member's position in [`Self::ALL`].
    ///
    /// The match is exhaustive, so a new member cannot be added without giving
    /// it an index; TC-030 then reads every index back out of `ALL` and fails if
    /// the array was not extended to match. Without it `ALL` is a hand-kept list
    /// whose length happens to be right, and a member missing from it is a label
    /// `from_label` refuses while the vocabulary contains it.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::ValueValidity => 0,
            Self::OperationContract => 1,
            Self::FiniteReplay => 2,
            Self::TemporalSatisfaction => 3,
            Self::GlobalConformance => 4,
            Self::Monitorability => 5,
            Self::LocalProjection => 6,
            Self::Refinement => 7,
            Self::Realizability => 8,
            Self::Composition => 9,
            Self::ProbabilisticSatisfaction => 10,
            Self::Schedulability => 11,
        }
    }
}

impl serde::Serialize for CapabilityKind {
    /// Serialises through [`Self::label`], so the emitted label and the label a
    /// request is read against cannot drift apart.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.label())
    }
}

/// An advertised mode. A backend advertises a set of (kind, mode) pairs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// The backend discharges the kind over a finite bound.
    Bounded,
    /// The backend discharges the kind without one.
    Unbounded,
}

/// A claim's extent, classified by the boundedness architecture.
///
/// `filament-spec-language#222` owns the representation, the classification and
/// the finite-bound predicate; this module reads all three and computes none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtentClassification {
    /// The classification itself.
    pub extent: Mode,
    /// Whether a finite bound is available for an unbounded extent.
    ///
    /// It separates `requires-bound` from `unsupported` and is read from #222,
    /// never inferred from the extent.
    pub finite_bound_available: bool,
    /// Kinds of the item's unbounded domains, supplied by FSL.
    pub domains: Vec<ExtentDomain>,
    /// Substituted proof bounds, including each explicit kind and numeric bound.
    pub bounds: Vec<ProofBound>,
}

/// One unbounded domain and its explicit FSL kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtentDomain {
    /// Domain identity from the request.
    pub domain: DomainKey,
    /// Its declared domain kind.
    pub kind: DomainKind,
}

/// The registry-owned provider origin projected into CG's in-process descriptor.
///
/// FSL layer R owns this closed vocabulary (FR-288, ADR-029 PV-1). The driver
/// maps its registry value exhaustively; CG does not infer it from identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderOrigin {
    /// A provider linked into the host at compile time.
    Linked,
    /// A process provider admitted through the registry.
    Process,
}

/// A registered backend: its identity, advertised capabilities, and origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
    /// Backend identity, unique within a registry.
    pub identity: String,
    /// Advertised (kind, mode) pairs.
    pub advertised: Vec<(CapabilityKind, Mode)>,
    /// Origin supplied by the FSL registry, never derived from identity.
    pub origin: ProviderOrigin,
    /// Domain kinds admitted for finite proof bounds by the manifest.
    pub domains: Option<BTreeSet<DomainKind>>,
    /// Optional manifest run-limit defaults; absence differs from a present empty map.
    /// Negotiation retains but never compares these values.
    pub bounds: Option<BTreeMap<String, u64>>,
    /// What the manifest entry declares beyond modes and domains: dynamics, fairness kinds,
    /// state-graph forms, reductions, obligation forms and evidence kinds.
    pub profile: BackendProfile,
}

impl BackendDescriptor {
    /// The modes this backend advertises for `kind`, ascending.
    pub(crate) fn modes_for(&self, kind: CapabilityKind) -> BTreeSet<Mode> {
        self.advertised
            .iter()
            .filter(|(advertised, _)| *advertised == kind)
            .map(|(_, mode)| *mode)
            .collect()
    }

    pub(crate) fn advertises(&self, kind: CapabilityKind) -> bool {
        !self.modes_for(kind).is_empty()
    }
}

/// A candidate: the identity of a registered backend.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Candidate {
    /// Backend identity.
    pub identity: String,
}

/// Stable identity fields used when a Kani form is refused.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KaniFormIdentity {
    /// The finite family vocabulary from FR-019 AC-25.
    pub family: KaniFormFamily,
    /// The typed form within that family.
    pub form: KaniFormName,
    /// The exact operation identity, where the form has one.
    pub operation_identity: Option<String>,
    /// The obligation role, where the form has one.
    pub role: Option<KaniObligationRole>,
    /// The selected contract or frame harness role for a V2 state clause.
    pub state_frame_role: Option<KaniStateFrameRole>,
}

/// The selected harness role for a V2 state clause.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniStateFrameRole {
    /// The operation-contract harness.
    Contract,
    /// The frame-effect harness.
    Frame,
}

/// The complete V1 family vocabulary named by FR-019 AC-25.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniFormFamily {
    /// Boolean scalar claims.
    Boolean,
    /// Integer scalar claims.
    Integer,
    /// Rational scalar claims.
    Rational,
    /// Decimal scalar claims.
    Decimal,
    /// IEEE floating-point scalar claims.
    IeeeFloat,
    /// Text scalar claims.
    Text,
    /// Enumeration scalar claims.
    Enum,
    /// Quantity scalar claims.
    Quantity,
    /// Record or tuple composite equality.
    CompositeEquality,
    /// Function application.
    FunctionApplication,
    /// Precondition, postcondition, invariant or frame clauses.
    Clause,
    /// State before/after and frame obligations.
    StateFrame,
    /// Bounded collection forms.
    BoundedCollection,
    /// Bounded graph forms.
    BoundedGraph,
    /// A typed IR form outside the normative V1/V2 set.
    OutsideSet,
}

/// Form names used in stable Kani form identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniFormName {
    /// V2 exact scalar claim.
    V2ScalarClaim,
    /// V2 state-clause contract/frame role.
    V2StateFrame,
    /// V1 composite equality.
    V1CompositeEquality,
    /// V1 function application.
    V1FunctionApplication,
    /// V2 function application.
    V2FunctionApplication,
    /// Bounded collection operation.
    BoundedCollection,
    /// Bounded graph operation.
    BoundedGraph,
    /// An IR form outside the normative set.
    OutsideSet,
}

/// Obligation role carried by a typed Kani form.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniObligationRole {
    /// A precondition obligation.
    Precondition,
    /// A postcondition obligation.
    Postcondition,
    /// An invariant obligation.
    Invariant,
    /// A frame-effect obligation.
    Frame,
}

impl KaniFormName {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::V2ScalarClaim => "v2_scalar_claim",
            Self::V2StateFrame => "v2_state_frame",
            Self::V1CompositeEquality => "v1_composite_equality",
            Self::V1FunctionApplication => "v1_function_application",
            Self::V2FunctionApplication => "v2_function_application",
            Self::BoundedCollection => "bounded_collection",
            Self::BoundedGraph => "bounded_graph",
            Self::OutsideSet => "outside_set",
        }
    }
}

/// Why an item settled as something other than `supported`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
// FR-290 spells every item-level cause in kebab-case (`absent-kind`,
// `unknown-backend`, `inconsistent-candidates`) while the codes that type them
// are snake (`invalid_capability`, `unsupported_projection`). A consumer
// matching the spec's spelling never matched `absent_kind`.
#[serde(rename_all = "kebab-case", tag = "cause")]
pub enum Cause {
    /// The item's kind is absent or `null`.
    AbsentKind,
    /// The item's kind is a label outside the vocabulary, reported verbatim.
    UnknownKind {
        /// The exact received label.
        received: String,
    },
    /// The item carries no extent classification.
    AbsentExtent,
    /// A `probabilistic-satisfaction` item names no evidence kind.
    AbsentEvidenceKind,
    /// A `probabilistic-satisfaction` item names an evidence kind outside the two.
    UnknownEvidenceKind {
        /// The exact received label.
        received: String,
    },
    /// The request named an unregistered backend, or one with no negotiation arm.
    UnknownBackend {
        /// The backend identity the request named.
        backend: String,
    },
    /// A candidate is absent from the manifest, does not advertise the kind, or
    /// disagrees with the named backend.
    InconsistentCandidates {
        /// Every offending candidate, in candidate order.
        candidates: Vec<Candidate>,
    },
    /// More than one candidate, and the request names no backend.
    AmbiguousBackend {
        /// Every candidate, in candidate order.
        candidates: Vec<Candidate>,
    },
    /// No registered backend advertises the item's kind.
    UnsupportedRequestedCapability {
        /// The kind no candidate advertises.
        kind: CapabilityKind,
        /// The backend the request named, when it named one.
        backend: Option<String>,
    },
    /// The single candidate does not advertise a requirement the item carries.
    #[serde(rename = "unsupported-requested-capability")]
    UnsupportedRequirement {
        /// The capability kind requested.
        kind: CapabilityKind,
        /// The fairness kind, state-graph form, reduction or obligation form missing.
        requirement: Requirement,
        /// The candidate that does not advertise it.
        backend: String,
    },
    /// A process descriptor cannot discharge one explicit domain kind.
    #[serde(rename = "unsupported-requested-capability")]
    UnsupportedDomain {
        /// The capability kind requested.
        kind: CapabilityKind,
        /// The uncovered domain kind.
        #[serde(serialize_with = "serialize_domain_kind")]
        domain: DomainKind,
        /// The candidate that cannot discharge it.
        backend: String,
    },
    /// A Kani form is outside the renderer's current typed capability set.
    #[serde(rename = "unsupported-requested-capability")]
    UnsupportedKaniForm {
        /// The form's stable typed identity.
        form: KaniFormIdentity,
        /// The sole Kani candidate.
        backend: String,
    },
    /// An in-set Kani form was refused by its FR-015 typed validation/lowering path.
    #[serde(rename = "unsupported-requested-capability")]
    KaniFormRefused {
        /// The in-set form's stable typed identity.
        form: KaniFormIdentity,
        /// The exact typed FR-015 refusal.
        refusal: crate::kani::generate::outcome::UnsupportedObligation,
        /// The sole Kani candidate.
        backend: String,
    },
    /// The typed IR form is malformed or does not belong to its admitted package.
    #[serde(rename = "invalid-capability")]
    KaniFormInvalid {
        /// The form's stable typed identity.
        form: KaniFormIdentity,
        /// The FR-015 invalid-item reason.
        refusal: crate::kani::generate::outcome::InvalidObligationItem,
        /// The sole Kani candidate.
        backend: String,
    },
    /// A checked state-frame form was refused by its source-backed FR-015 classifier.
    #[serde(rename = "unsupported-requested-capability")]
    KaniStateFrameRefused {
        /// The form's stable typed identity.
        form: KaniFormIdentity,
        /// The exact typed state-frame refusal.
        refusal: crate::kani::generate::outcome::StateFrameRefusal,
        /// The sole Kani candidate.
        backend: String,
    },
    /// An unbounded extent against a bounded-only advertisement, with no finite
    /// bound available.
    UnboundedExtent {
        /// The kind whose extent could not be bounded.
        kind: CapabilityKind,
        /// The single candidate that advertises `bounded` only.
        backend: String,
    },
}

impl Cause {
    /// The FR-272 code this cause is typed under.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::AbsentKind
            | Self::UnknownKind { .. }
            | Self::AbsentExtent
            | Self::AbsentEvidenceKind
            | Self::UnknownEvidenceKind { .. }
            | Self::UnknownBackend { .. }
            | Self::InconsistentCandidates { .. }
            | Self::AmbiguousBackend { .. } => "invalid_capability",
            Self::UnsupportedRequestedCapability { .. }
            | Self::UnsupportedDomain { .. }
            | Self::UnsupportedRequirement { .. }
            | Self::UnsupportedKaniForm { .. }
            | Self::KaniFormRefused { .. }
            | Self::KaniStateFrameRefused { .. }
            | Self::UnboundedExtent { .. } => "unsupported_projection",
            Self::KaniFormInvalid { .. } => "invalid_capability",
        }
    }
}

/// How one item settled.
///
/// The four dispositions are FR-331's. `RequiresBound` is a negotiation
/// disposition and never a verification result; nothing in this module converts
/// one into the other.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "disposition")]
pub enum Disposition {
    /// The single candidate's arm discharges the claim as requested.
    Supported {
        /// The backend that settled it.
        backend: String,
    },
    /// The claim needs a finite bound before the single candidate can discharge it.
    RequiresBound {
        /// The backend that settled it.
        backend: String,
    },
    /// No candidate discharges the claim; the warning names what was requested.
    Unsupported {
        /// Why, typed under `unsupported_projection`.
        cause: Cause,
    },
    /// The request itself is wrong, and the cause names what is wrong with it.
    InvalidRequest {
        /// Why, typed under `invalid_capability`.
        cause: Cause,
    },
}

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
    pub(crate) fn decides(&self, dynamics: Dynamics) -> bool {
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
