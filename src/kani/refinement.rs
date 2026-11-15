//! The refinement run of a bounded shadow: its case plan, its evidence and the proof strength
//! the shadow run and the refinement run settle together (FR-028-AC-15 to FR-028-AC-17 and
//! FR-028-AC-24, ADR-003).
//!
//! The plan is pure. Two plans with equal inputs hold the same cases in the same order.

use serde::Serialize;

use crate::kani::{
    classify::KaniRunOutcome, identity::ProofCeilings, run::memory::MemoryObservation,
};

/// One inclusive bound of one drawn argument of the refinement domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainBound {
    /// The drawn argument's name.
    pub argument: String,
    /// Inclusive minimum.
    pub minimum: i64,
    /// Inclusive maximum.
    pub maximum: i64,
}

/// How many cases a refinement run executes and how it chose them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementClass {
    /// Every case of a domain of at most the case cap.
    Exhaustive,
    /// The boundary cases and a seeded sample of a larger domain.
    Sampled,
}

/// The number of cases of the domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "count", rename_all = "snake_case")]
pub enum CaseCount {
    /// The exact count.
    Exact(#[serde(serialize_with = "as_decimal")] u128),
    /// The count does not fit `u128`.
    Overflowed,
}

fn as_decimal<S: serde::Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

/// What a run is asked to execute: the case cap, the sample size and the seed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementBudget {
    /// The largest domain that is run exhaustively.
    pub case_cap: u128,
    /// The seeded cases a sampled run draws after the boundary cases.
    pub sample_size: u64,
    /// The seed every drawn case derives from.
    pub seed: u64,
}

/// The cases of one refinement run, in execution order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementPlan {
    /// Exhaustive or sampled.
    pub class: RefinementClass,
    /// The size of the whole domain.
    pub case_count: CaseCount,
    /// The seed the plan was built from.
    pub seed: u64,
    /// For a sampled plan, how many leading cases are boundary cases; zero otherwise.
    pub boundary_cases: usize,
    /// For a sampled plan, how many trailing cases are seeded draws; zero otherwise.
    pub sample_size: u64,
    /// One value per drawn argument, per case.
    pub cases: Vec<Vec<i64>>,
}

/// Why a plan could not be built.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefinementPlanError {
    /// An argument's minimum is above its maximum.
    EmptyBound {
        /// The argument.
        argument: String,
    },
    /// The domain has no argument.
    NoArguments,
}

fn span(bound: &DomainBound) -> u128 {
    (i128::from(bound.maximum) - i128::from(bound.minimum)) as u128 + 1
}

/// The cases of the domain `bounds` under `budget`.
///
/// A domain of at most `case_cap` cases is run in full. A larger one, or one whose count
/// overflows `u128`, runs the boundary cases and then exactly `sample_size` seeded cases drawn
/// uniformly over the whole domain. The boundary cases take each leaf's boundary values one leaf
/// at a time with every other leaf at its lower bound, plus the all-lower and the all-upper case;
/// they are not a cross product.
pub fn plan_refinement(
    bounds: &[DomainBound],
    budget: RefinementBudget,
) -> Result<RefinementPlan, RefinementPlanError> {
    if bounds.is_empty() {
        return Err(RefinementPlanError::NoArguments);
    }
    if let Some(bound) = bounds.iter().find(|bound| bound.minimum > bound.maximum) {
        return Err(RefinementPlanError::EmptyBound {
            argument: bound.argument.clone(),
        });
    }
    let count = bounds
        .iter()
        .try_fold(1_u128, |total, bound| total.checked_mul(span(bound)));
    let case_count = count.map_or(CaseCount::Overflowed, CaseCount::Exact);
    if let Some(count) = count.filter(|count| *count <= budget.case_cap) {
        return Ok(RefinementPlan {
            class: RefinementClass::Exhaustive,
            case_count,
            seed: budget.seed,
            boundary_cases: 0,
            sample_size: 0,
            cases: every_case(bounds, count),
        });
    }
    let mut cases = boundary_cases(bounds);
    let boundary = cases.len();
    let mut generator = SplitMix64(budget.seed);
    for _ in 0..budget.sample_size {
        cases.push(
            bounds
                .iter()
                .map(|bound| {
                    bound
                        .minimum
                        .wrapping_add(generator.below(span(bound)) as i64)
                })
                .collect(),
        );
    }
    Ok(RefinementPlan {
        class: RefinementClass::Sampled,
        case_count,
        seed: budget.seed,
        boundary_cases: boundary,
        sample_size: budget.sample_size,
        cases,
    })
}

fn every_case(bounds: &[DomainBound], count: u128) -> Vec<Vec<i64>> {
    let mut cases = Vec::new();
    for index in 0..count {
        let mut rest = index;
        let mut case = vec![0; bounds.len()];
        for (slot, bound) in case.iter_mut().zip(bounds).rev() {
            let width = span(bound);
            *slot = bound.minimum.wrapping_add((rest % width) as i64);
            rest /= width;
        }
        cases.push(case);
    }
    cases
}

fn boundary_cases(bounds: &[DomainBound]) -> Vec<Vec<i64>> {
    let lower: Vec<i64> = bounds.iter().map(|bound| bound.minimum).collect();
    let mut cases = vec![lower.clone()];
    for (index, bound) in bounds.iter().enumerate() {
        let mut values = vec![bound.minimum];
        if bound.minimum < bound.maximum {
            values.push(bound.minimum + 1);
            values.push(bound.maximum - 1);
        }
        values.push(bound.maximum);
        if bound.minimum <= 0 && 0 <= bound.maximum {
            values.push(0);
        }
        for value in values {
            let mut case = lower.clone();
            case[index] = value;
            cases.push(case);
        }
    }
    cases.push(bounds.iter().map(|bound| bound.maximum).collect());
    let mut seen = std::collections::BTreeSet::new();
    cases.retain(|case| seen.insert(case.clone()));
    cases
}

/// A small, fixed generator: equal seeds draw equal cases on every platform.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// A value uniform over `0..width`, by rejection so no residue is favoured.
    fn below(&mut self, width: u128) -> u128 {
        if width > u128::from(u64::MAX) {
            return u128::from(self.next());
        }
        let width = width as u64;
        let zone = u64::MAX - (u64::MAX % width + 1) % width;
        loop {
            let draw = self.next();
            if draw <= zone {
                return u128::from(draw % width);
            }
        }
    }
}

/// One case on which the production subject and the shadow disagree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementDisagreement {
    /// The drawn value of each argument, in argument order.
    pub case: Vec<i64>,
    /// The production subject's result, or its construction refusal.
    pub production: String,
    /// The shadow's result.
    pub shadow: String,
}

/// How the native refinement process ended.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RefinementOutcome {
    /// Every planned case ran and agreed.
    Agreed,
    /// At least one case disagreed. `first` is the first disagreeing case in plan order.
    Disagreed {
        /// The first disagreeing case with both results.
        first: RefinementDisagreement,
        /// How many cases disagreed.
        disagreements: u64,
    },
    /// The wall-clock ceiling of the harness identity stopped the run.
    ReachedWallClockCeiling,
    /// The environment ended the process by a signal, which is how it enforces its memory cap.
    EndedBySignal,
    /// The caller's cancellation handle stopped the run.
    Cancelled,
}

/// The tool versions a refinement evidence names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementVersions {
    /// The Rust compiler version string of the build that produced the native process.
    pub rust_compiler: String,
    /// The production subject's crate version.
    pub subject_crate: String,
}

/// What one refinement run did (FR-028-AC-24).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementEvidence {
    /// The `module::harness` path of the shadow harness this refinement accompanies.
    pub harness_path: String,
    /// Exhaustive or sampled, as planned.
    pub class: RefinementClass,
    /// The size of the domain.
    pub case_count: CaseCount,
    /// How many cases the process executed.
    pub cases_run: u64,
    /// How many cases the plan held.
    pub cases_planned: u64,
    /// Boundary cases in a sampled plan.
    pub boundary_cases: u64,
    /// Seeded cases in a sampled plan.
    pub sample_size: u64,
    /// The seed.
    pub seed: u64,
    /// The two ceilings the run was held to: the identity wall clock, and the memory ceiling
    /// the caller's environment is expected to enforce.
    pub ceilings: ProofCeilings,
    /// The peak the environment provided, when it provided one.
    pub memory: MemoryObservation,
    /// How the run ended.
    pub outcome: RefinementOutcome,
    /// The versions the run used.
    pub versions: RefinementVersions,
}

impl RefinementEvidence {
    /// The summary strength mapping reads.
    #[must_use]
    pub fn summary(&self) -> RefinementSummary {
        match &self.outcome {
            RefinementOutcome::Agreed => RefinementSummary::Agreed(self.class),
            RefinementOutcome::Disagreed { .. } => RefinementSummary::Disagreed,
            RefinementOutcome::ReachedWallClockCeiling
            | RefinementOutcome::EndedBySignal
            | RefinementOutcome::Cancelled => RefinementSummary::ReachedCeiling,
        }
    }
}

/// What a refinement run established, with nothing else a strength reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefinementSummary {
    /// It ran and agreed on every case, in this class.
    Agreed(RefinementClass),
    /// A case disagreed.
    Disagreed,
    /// It was stopped or ended before it finished.
    ReachedCeiling,
}

/// What a harness's proof is a statement about.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofSubject {
    /// The production code itself.
    Production,
    /// A smaller program, with a refinement obligation to the production code.
    BoundedShadow,
}

/// How strongly a verified harness, with its refinement run if it has one, proves the production
/// subject (FR-028-AC-17). A `bounded_shadow` result is never `production_proved`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofStrength {
    /// A verified `production` harness.
    ProductionProved,
    /// A verified shadow whose refinement run was exhaustive and agreed.
    ShadowProvedRefinementExhaustive,
    /// A verified shadow whose refinement run was sampled and agreed.
    ShadowProvedRefinementSampled,
    /// A verified shadow with no refinement run.
    ShadowProvedRefinementNotRun,
    /// A verified shadow whose refinement run reached a ceiling.
    ShadowProvedRefinementInconclusive,
    /// The refinement run found a disagreement.
    RefinementFailed,
}

/// The strength of one harness's result, or none where the harness itself did not verify and
/// no refinement disagreement was found.
#[must_use]
pub fn proof_strength(
    subject: ProofSubject,
    outcome: &KaniRunOutcome,
    refinement: Option<RefinementSummary>,
) -> Option<ProofStrength> {
    if subject == ProofSubject::BoundedShadow && refinement == Some(RefinementSummary::Disagreed) {
        return Some(ProofStrength::RefinementFailed);
    }
    if !matches!(outcome, KaniRunOutcome::Verified) {
        return None;
    }
    Some(match (subject, refinement) {
        (ProofSubject::Production, _) => ProofStrength::ProductionProved,
        (ProofSubject::BoundedShadow, None) => ProofStrength::ShadowProvedRefinementNotRun,
        (ProofSubject::BoundedShadow, Some(RefinementSummary::Agreed(class))) => match class {
            RefinementClass::Exhaustive => ProofStrength::ShadowProvedRefinementExhaustive,
            RefinementClass::Sampled => ProofStrength::ShadowProvedRefinementSampled,
        },
        (ProofSubject::BoundedShadow, Some(RefinementSummary::ReachedCeiling)) => {
            ProofStrength::ShadowProvedRefinementInconclusive
        }
        (ProofSubject::BoundedShadow, Some(RefinementSummary::Disagreed)) => {
            ProofStrength::RefinementFailed
        }
    })
}
