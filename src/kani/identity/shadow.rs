//! The identity of a bounded-shadow harness, its refinement obligation and the harness record
//! (FR-028-AC-14 and FR-028-AC-18).

use serde::Serialize;

use crate::{
    core::artifact::Artifact,
    kani::{
        abi::KaniSolver,
        identity::{
            refinement::{DomainBound, ProofSubject},
            ProofCeilings,
        },
    },
    oracle::equality::closure::{RecordedDeclaration, RecordedOperandTypes},
};

/// The evidence that discharges one abstraction the shadow makes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum AbstractionEvidence {
    /// A Kani check of the harness, by name.
    KaniCheck(String),
    /// A comparison of the refinement run, by the observable it reads.
    RefinementComparison(String),
}

/// Whether a shared helper's result is read by the compared observables.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperMark {
    /// It returns a bound, a presence, a member set or a count the comparison reads.
    ValueBearing,
    /// It returns only an order the compared observables do not read.
    OrderOnly,
}

/// Whether the production subject reads a value-bearing helper's value through the same helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionRead {
    /// The production subject reads its own value, so the refinement run sees a mutant.
    Independent,
    /// The production subject calls the same helper, so only an independent check sees a mutant.
    SameHelper,
}

/// One helper the shadow shares with its expectation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedHelperClaim {
    /// The helper's function name.
    pub name: String,
    /// Its mark.
    pub mark: HelperMark,
    /// Why it is `order_only`; required exactly for that mark.
    pub order_only_reason: Option<String>,
    /// How the production subject obtains the same fact.
    pub production_read: ProductionRead,
    /// The family's independent check of the helper; required when the production subject reads
    /// it through the same helper.
    pub independent_check: Option<String>,
}

/// One abstraction of the identity, with its discharging evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AbstractionRecord {
    /// The name.
    pub name: String,
    /// The evidence.
    pub evidence: AbstractionEvidence,
}

/// What one drawn leaf stands for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeafKind {
    /// A bounded integer.
    Integer,
    /// A Boolean.
    Boolean,
    /// Whether an option is present.
    Presence,
    /// The state of an optional field: 0 present, 1 absent, 2 null.
    SlotState,
}

/// One leaf of the identity: its declared and drawn domains.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeafRecord {
    /// The leaf name.
    pub name: String,
    /// The node of the parameter the leaf belongs to.
    pub node: String,
    /// The leaf's path inside the parameter; empty for a stand-in leaf.
    pub path: String,
    /// What the leaf stands for.
    pub kind: LeafKind,
    /// The declared inclusive bound.
    pub declared: (i64, i64),
    /// The inclusive bound the harness draws from.
    pub drawn: (i64, i64),
}

/// Whether the criterion that covers an unexercised behaviour is covered in the test matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackingState {
    /// The matrix lists the criterion covered.
    Backed,
    /// The matrix lists it in any other state.
    Unbacked,
}

/// A behaviour the proof does not exercise, with the native criterion that covers it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnexercisedCoverage {
    /// The behaviour.
    pub behaviour: String,
    /// The native criterion that covers it.
    pub criterion: String,
    /// The criterion's backing state.
    pub backing: BackingState,
}

/// A literal operand's leaf, pinned to exactly one value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PinnedLeaf {
    /// The leaf name.
    pub name: String,
    /// The value, in the encoding of the refinement domain.
    pub value: i64,
}

/// What the composite-equality family adds to the identity (FR-015-AC-76).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeIdentity {
    /// `=` or `!=`.
    pub operator: String,
    /// The closure's declarations, each with its leaves' declared domains.
    pub declarations: Vec<RecordedDeclaration>,
    /// The comparison types of both operands.
    pub operand_types: RecordedOperandTypes,
    /// The closure's pair-node count.
    pub pair_nodes: u32,
    /// Literal operand leaves.
    pub pinned: Vec<PinnedLeaf>,
    /// Each unexercised behaviour with its covering criterion and its backing state.
    pub coverage: Vec<UnexercisedCoverage>,
    /// Whether any covering criterion is unbacked, so the result closes no family row.
    pub closes_family_row: bool,
}

/// The identity of a bounded-shadow harness (FR-028-AC-18).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowIdentity {
    /// The family.
    pub family: String,
    /// Always `bounded_shadow`.
    pub proof_subject: ProofSubject,
    /// The claimed node.
    pub node: String,
    /// The request ceilings.
    pub ceilings: ProofCeilings,
    /// Harness module.
    pub module_symbol: String,
    /// Harness function.
    pub harness_symbol: String,
    /// The drawn arguments.
    pub leaves: Vec<LeafRecord>,
    /// The family's size budget and the item's size.
    pub size_budget: u32,
    /// The item's size.
    pub size: u32,
    /// Each abstraction with its discharging evidence.
    pub abstractions: Vec<AbstractionRecord>,
    /// Each behaviour of the production subject the proof does not exercise.
    pub unexercised: Vec<String>,
    /// Each helper the shadow shares with its expectation.
    pub shared_helpers: Vec<SharedHelperClaim>,
    /// Solver.
    pub solver: KaniSolver,
    /// Unwind bound.
    pub unwind: u32,
    /// The proof option vector.
    pub options: Vec<String>,
    /// The witness option vector.
    pub witness_options: Vec<String>,
    /// The composite-equality family's members.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite: Option<CompositeIdentity>,
}

/// The refinement obligation: what the production subject owes the shadow (FR-028-AC-14).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementObligation {
    /// The production subject's symbol.
    pub subject: String,
    /// The shadow's symbol.
    pub shadow: String,
    /// The bounded domain, one inclusive bound per drawn argument.
    pub domain: Vec<DomainBound>,
    /// The abstraction function's symbol.
    pub abstraction: String,
    /// The observables the comparison reads.
    pub observables: Vec<String>,
    /// The generated Rust source of the native refinement program, in the same result.
    pub source: Artifact,
    /// The source of the abstraction module alone.
    pub abstraction_source: String,
}

/// One generated shadow harness with its identity and its refinement obligation.
#[derive(Clone, Debug)]
pub struct KaniShadowHarness {
    /// Identity.
    pub identity: ShadowIdentity,
    /// The shadow, the expectation and the Kani harness, self-contained.
    pub rust: Artifact,
    /// The obligation generated beside it.
    pub obligation: RefinementObligation,
}

impl KaniShadowHarness {
    /// The `module::harness` path of the proof.
    #[must_use]
    pub fn harness_path(&self) -> String {
        format!(
            "{}::{}",
            self.identity.module_symbol, self.identity.harness_symbol
        )
    }
}
