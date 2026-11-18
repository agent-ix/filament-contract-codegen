//! The production route of a composite equality `bounded_shadow` run: the shadow-aware entry that
//! turns one Kani run of a shadow harness into a terminal value (FR-029-AC-19 to AC-27,
//! FR-029-AC-30, FR-033-AC-8, FR-033-AC-19).
//!
//! The entry builds FSL's parity request over the original proving context, derives the harness
//! bounds from the harness identity's own drawn leaves, and settles by outcome:
//!
//! - a retained refinement disagreement, of any outcome, through FSL's verified-shadow facade,
//!   whose entry carries no operand and no native observation, so nothing in the setup of either
//!   can precede it;
//! - a verified run through the same facade;
//! - a falsified run through FSL's falsified-parity facade, over the operands rebuilt from the
//!   playback and the driver's authenticated native observation of the same proved artifact;
//! - an inconclusive, cover-unsatisfied or cancelled run through the ordinary rows of
//!   [`run_terminal_value`], which never sees a verified or a falsified shadow outcome.
//!
//! A pre-invocation refusal returns an error and no terminal value.
// Implements: FR-029-AC-19, FR-029-AC-20, FR-029-AC-23, FR-029-AC-24, FR-029-AC-25,
// FR-029-AC-26, FR-029-AC-27, FR-029-AC-30, FR-033-AC-8, FR-033-AC-19

use filament_contract_model::{
    CheckedCompositeOperandDomain, CheckedNodeId, CheckedOccurrence, CheckedScalarOperandChild,
};
use filament_exact::Outcome;
use fsl_replay::{
    CompositeEvidence, CompositeIdentity, DomainKey, DomainKind, EqualityOperator, EqualityOutcome,
    FalsifiedParity, FiniteBound, Integer, NativeParityObservation, ProofBound, Refinement,
    TerminalValue, VerifiedShadow, WitnessValue,
};

use super::{
    composite_artifact::AuthenticatedCompositeArtifact,
    composite_builder::{
        wire_node, CompositeBuildError, OriginalCompositeEqContext, OriginalCompositeEqReport,
        OriginalCompositeEqRequest, OriginalCompositeFalsifiedReport,
    },
    composite_leaves::reconstruct_composite_operands,
    witness::DecodeFailure,
};
use crate::{
    core::ir::node_key::render_checked_node_id,
    kani::{
        classify::KaniRunOutcome,
        identity::{
            leaf_binding::{integer_positions, LeafBindingRefusal},
            refinement::{ProofSubject, RefinementClass, RefinementSummary},
            shadow::{KaniShadowHarness, OperandSource, ShadowIdentity},
        },
        output::playback::parse_assertion_playback,
        run::execute::KaniExecutionEvidence,
        terminal::{run_terminal_value, ReplaySettlement, TerminalPairError},
    },
};

/// The claim a shadow run is about, over the original proving context.
#[derive(Debug)]
pub struct ShadowClaim<'a> {
    /// The original proving context.
    pub context: &'a OriginalCompositeEqContext,
    /// The claimed equality or inequality application.
    pub node: &'a CheckedNodeId,
    /// The claimed occurrence of the node.
    pub occurrence: &'a CheckedOccurrence,
    /// The claimed operation.
    pub operator: EqualityOperator,
    /// The work ceiling of the owning operand projection.
    pub work_limit: u64,
}

/// One executed run of the claim's shadow harness, with the refinement evidence of the same
/// harness. `refinement` is `None` when no refinement ran.
#[derive(Debug)]
pub struct ShadowRun<'a> {
    /// The harness that was run.
    pub harness: &'a KaniShadowHarness,
    /// The run's evidence.
    pub evidence: &'a KaniExecutionEvidence,
    /// What the native refinement run established.
    pub refinement: Option<RefinementSummary>,
}

/// What the driver observed for a falsified run: the retained shadow comparison at the rebuilt
/// operands, the driver's native observation of the proved artifact there, and the authenticated
/// artifact binding that observation is tied to.
#[derive(Debug)]
pub struct DriverFalsification<'a> {
    /// The original generated artifact and native context, authenticated by the driver's capture.
    pub artifact: &'a AuthenticatedCompositeArtifact,
    /// The shadow's verdict and occurrence-pair count at the rebuilt operands.
    pub shadow: EqualityOutcome,
    /// The driver's native observation at those operands; `None` when it made none.
    pub native: Option<NativeParityObservation>,
}

/// A refusal before any terminal value exists.
#[derive(Debug)]
pub enum CompositeShadowError {
    /// The harness is not a composite bounded-shadow harness.
    NotCompositeShadow,
    /// The harness belongs to another node.
    NodeMismatch,
    /// The harness compares under another operation.
    OperatorMismatch,
    /// The harness's operand sources are not the claim's operands.
    OperandMismatch,
    /// The harness's leaves cannot be keyed as FSL positions.
    Bounds(LeafBindingRefusal),
    /// The request or the claim could not be built from the original proving context.
    Build(Box<CompositeBuildError>),
    /// A falsified run has no driver observation to settle through.
    MissingDriverEvidence,
    /// No native observation was made, and no retained disagreement settles the claim first.
    MissingNativeObservation,
    /// The authenticated artifact is not the one the claim, its request or its operands name.
    ArtifactMismatch,
    /// The native observation contradicts the authenticated artifact's own run.
    NativeContradictsArtifact,
    /// The ordinary map refused the pair; it is built here from the outcome, so this is a defect.
    Pair(TerminalPairError),
}

impl std::fmt::Display for CompositeShadowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composite shadow route refused: {self:?}")
    }
}

impl std::error::Error for CompositeShadowError {}

impl From<CompositeBuildError> for CompositeShadowError {
    fn from(cause: CompositeBuildError) -> Self {
        Self::Build(Box::new(cause))
    }
}

/// Which map produced the terminal value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowRoute {
    /// FSL's parity facade over a CG-built request.
    Parity,
    /// The ordinary rows, for an outcome that carries no shadow evidence.
    Ordinary,
    /// A playback the leaf bindings could not type: a CG defect, `Failed`.
    DecodeDefect,
}

/// FSL's genuine report, with the identity CG sent for it.
#[derive(Debug)]
pub enum ParityReport {
    /// The verified-shadow facade settled the claim.
    Verified(Box<OriginalCompositeEqReport>),
    /// The falsified-parity facade settled the claim.
    Falsified(Box<OriginalCompositeFalsifiedReport>),
}

/// A settled composite shadow run.
#[derive(Debug)]
pub struct CompositeShadowSettlement {
    /// The terminal value.
    pub terminal: TerminalValue,
    /// Which map produced it.
    pub route: ShadowRoute,
    /// FSL's report, present exactly on the parity route.
    pub report: Option<ParityReport>,
    /// The decode failure behind a [`ShadowRoute::DecodeDefect`].
    pub defect: Option<DecodeFailure>,
}

/// The FSL refinement state a refinement run's summary is evidence of. One match, no wildcard arm.
#[must_use]
pub const fn refinement_state(summary: Option<RefinementSummary>) -> Refinement {
    match summary {
        None => Refinement::NotExhausted,
        Some(RefinementSummary::Agreed(RefinementClass::Exhaustive)) => Refinement::Exhausted,
        Some(RefinementSummary::Agreed(RefinementClass::Sampled)) => Refinement::NotExhausted,
        Some(RefinementSummary::ReachedCeiling) => Refinement::CeilingReached,
        Some(RefinementSummary::Disagreed) => Refinement::Disagreed,
    }
}

fn operator_text(operator: EqualityOperator) -> &'static str {
    match operator {
        EqualityOperator::Equal => "equality.equal",
        EqualityOperator::NotEqual => "equality.not_equal",
    }
}

/// The harness bounds a shadow harness ran, keyed by FSL's positions: one integer range per
/// bounded-integer leaf of each drawn parameter, the harness's drawn bound. FSL derives which
/// positions the claim has and which bounds cover them; this list names only what was drawn.
///
/// # Errors
///
/// [`CompositeShadowError::NotCompositeShadow`] for another kind of harness,
/// [`CompositeShadowError::OperandMismatch`] when a drawn parameter is no operand of the claim, and
/// [`CompositeShadowError::Bounds`] when the identity's leaves are not the shape's.
pub fn shadow_harness_bounds(
    identity: &ShadowIdentity,
    request: &OriginalCompositeEqRequest,
    package: &filament_contract_model::CheckedPackageV2,
) -> Result<Vec<ProofBound>, CompositeShadowError> {
    let composite = identity
        .composite
        .as_ref()
        .ok_or(CompositeShadowError::NotCompositeShadow)?;
    let positions = integer_positions(composite).map_err(CompositeShadowError::Bounds)?;
    let mut bounds = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for source in &composite.operands {
        let OperandSource::Parameter { node } = source else {
            continue;
        };
        if seen.contains(&node.as_str()) {
            continue;
        }
        seen.push(node);
        let child = request
            .operands()
            .operands
            .iter()
            .find_map(|operand| match &operand.child {
                CheckedScalarOperandChild::GraphChild(child)
                    if render_checked_node_id(child).as_deref() == Some(node.as_str()) =>
                {
                    Some(child)
                }
                CheckedScalarOperandChild::GraphChild(_) => None,
                _ => None,
            })
            .ok_or(CompositeShadowError::OperandMismatch)?;
        let key = wire_node(package, child)?;
        for (path, indexes) in &positions {
            let leaf = identity
                .leaves
                .iter()
                .find(|leaf| &leaf.node == node && &leaf.path == path)
                .ok_or(CompositeShadowError::OperandMismatch)?;
            let range = FiniteBound::integer_range(
                Integer::from(leaf.drawn.0),
                Integer::from(leaf.drawn.1),
            )
            .map_err(|_| CompositeShadowError::OperandMismatch)?;
            bounds.push(
                ProofBound::new(
                    DomainKey::Node {
                        node: key.clone(),
                        path: indexes.clone(),
                    },
                    Some(DomainKind::Integer),
                    range,
                )
                .map_err(|_| CompositeShadowError::OperandMismatch)?,
            );
        }
    }
    Ok(bounds)
}

/// Whether the harness names the claim's node, operation and operands.
fn check_harness(
    claim: &ShadowClaim<'_>,
    harness: &KaniShadowHarness,
    request: &OriginalCompositeEqRequest,
) -> Result<(), CompositeShadowError> {
    let identity = &harness.identity;
    let composite = identity
        .composite
        .as_ref()
        .filter(|_| identity.proof_subject == ProofSubject::BoundedShadow)
        .ok_or(CompositeShadowError::NotCompositeShadow)?;
    if render_checked_node_id(claim.node).as_deref() != Some(identity.node.as_str()) {
        return Err(CompositeShadowError::NodeMismatch);
    }
    if composite.operator != operator_text(claim.operator) {
        return Err(CompositeShadowError::OperatorMismatch);
    }
    for (source, operand) in composite.operands.iter().zip(&request.operands().operands) {
        let matches = match (source, &operand.domain, &operand.child) {
            (
                OperandSource::Parameter { node },
                CheckedCompositeOperandDomain::Parameter { .. },
                CheckedScalarOperandChild::GraphChild(child),
            ) => render_checked_node_id(child).as_deref() == Some(node.as_str()),
            (OperandSource::Literal, CheckedCompositeOperandDomain::Literal, _) => true,
            _ => false,
        };
        if !matches {
            return Err(CompositeShadowError::OperandMismatch);
        }
    }
    Ok(())
}

fn ordinary(
    evidence: &KaniExecutionEvidence,
    settlement: Option<ReplaySettlement<'_>>,
    route: ShadowRoute,
    defect: Option<DecodeFailure>,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    Ok(CompositeShadowSettlement {
        terminal: run_terminal_value(evidence, settlement).map_err(CompositeShadowError::Pair)?,
        route,
        report: None,
        defect,
    })
}

/// Whether the native observation is the one the authenticated artifact's own run made.
fn consistent(artifact: &Outcome<bool>, native: &NativeParityObservation) -> bool {
    match (artifact, native) {
        (Outcome::Completed(verdict), NativeParityObservation::Completed(outcome)) => {
            outcome.equal == *verdict
        }
        (Outcome::Completed(_), _) => false,
        (_, NativeParityObservation::Completed(_)) => false,
        (_, _) => true,
    }
}

/// Settle one run of a composite shadow harness against the claim it is about.
///
/// # Errors
///
/// A [`CompositeShadowError`] before any terminal value exists: a harness that is not the claim's,
/// a request the original proving context cannot build, a falsified run with no driver
/// observation, a missing native observation (unless a retained disagreement settles the claim
/// first), an artifact that is not the claim's, or a native observation that contradicts the
/// artifact's run.
pub fn settle_composite_shadow(
    claim: &ShadowClaim<'_>,
    run: &ShadowRun<'_>,
    driver: Option<&DriverFalsification<'_>>,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    let context = claim.context;
    let original = context.original_context();
    let package = original.original.as_ref();
    // The request is built from the harness's own bounds, so one request is built to name the
    // operands the bounds are keyed by, and the settled one is built over the bounds.
    let probe = claim_request(claim, &[])?;
    check_harness(claim, run.harness, &probe)?;
    let bounds = shadow_harness_bounds(&run.harness.identity, &probe, package)?;
    let request = claim_request(claim, &bounds)?;
    let evidence = run.evidence;
    let refinement = refinement_state(run.refinement);

    if refinement == Refinement::Disagreed
        && !matches!(evidence.outcome, KaniRunOutcome::Cancelled { .. })
    {
        return verified_entry(request, evidence, refinement);
    }
    match &evidence.outcome {
        KaniRunOutcome::Verified => verified_entry(request, evidence, refinement),
        KaniRunOutcome::Falsified { counterexample } => {
            let limits = original.inputs.decode_limits;
            let operands = parse_assertion_playback(
                counterexample,
                &run.harness.identity.harness_symbol,
                "composite shadow playback",
                limits.bytes,
                limits.values,
            )
            .map_err(|refusal| {
                DecodeFailure::new(
                    "kani_playback_refused",
                    &run.harness.identity.harness_symbol,
                    &format!("{refusal:?}"),
                )
            })
            .and_then(|playback| {
                reconstruct_composite_operands(&run.harness.identity, &playback, limits)
            });
            let operands = match operands {
                Ok(operands) => operands,
                Err(failure) => {
                    return ordinary(
                        evidence,
                        Some(ReplaySettlement::CgDefect),
                        ShadowRoute::DecodeDefect,
                        Some(failure),
                    );
                }
            };
            let driver = driver.ok_or(CompositeShadowError::MissingDriverEvidence)?;
            falsified_entry(request, operands, refinement, driver)
        }
        KaniRunOutcome::CoverUnsatisfied { .. }
        | KaniRunOutcome::Inconclusive { .. }
        | KaniRunOutcome::Cancelled { .. } => ordinary(evidence, None, ShadowRoute::Ordinary, None),
    }
}

// The 192-byte refusal is retained inline to preserve the public PackageRead payload.
#[allow(clippy::result_large_err)]
fn claim_request(
    claim: &ShadowClaim<'_>,
    bounds: &[ProofBound],
) -> Result<OriginalCompositeEqRequest, CompositeShadowError> {
    let packet = claim.context.parity_packet(
        claim.node,
        claim.occurrence,
        claim.operator,
        bounds,
        claim.work_limit,
    )?;
    Ok(claim.context.request_from_packet(
        packet,
        claim.node,
        claim.occurrence,
        claim.operator,
        bounds,
        claim.work_limit,
    )?)
}

fn verified_entry(
    request: OriginalCompositeEqRequest,
    evidence: &KaniExecutionEvidence,
    refinement: Refinement,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    let report = request.settle_verified(VerifiedShadow {
        success_checks: evidence.success_checks,
        refinement,
    })?;
    let terminal = report
        .settlement()
        .map_err(|cause| CompositeShadowError::from(CompositeBuildError::Report(cause)))?
        .terminal_value();
    Ok(CompositeShadowSettlement {
        terminal,
        route: ShadowRoute::Parity,
        report: Some(ParityReport::Verified(Box::new(report))),
        defect: None,
    })
}

fn falsified_entry(
    request: OriginalCompositeEqRequest,
    operands: [WitnessValue; 2],
    refinement: Refinement,
    driver: &DriverFalsification<'_>,
) -> Result<CompositeShadowSettlement, CompositeShadowError> {
    let artifact = driver.artifact;
    let (_, artifact_claim, artifact_limits) = artifact.request();
    let tie = |claim: &fsl_replay::CompositeParityClaim| {
        CompositeIdentity::new(
            request.wire().obligation_identity.clone(),
            claim,
            CompositeEvidence::Verified(VerifiedShadow {
                success_checks: 0,
                refinement: Refinement::NotExhausted,
            }),
        )
    };
    if tie(&artifact_claim) != tie(request.claim())
        || artifact.operands() != &operands
        || artifact
            .check_sent_context(request.wire(), artifact_limits)
            .is_err()
    {
        return Err(CompositeShadowError::ArtifactMismatch);
    }
    let native = driver
        .native
        .clone()
        .ok_or(CompositeShadowError::MissingNativeObservation)?;
    if !consistent(artifact.native_outcome(), &native) {
        return Err(CompositeShadowError::NativeContradictsArtifact);
    }
    let report = request.settle_falsified(FalsifiedParity {
        operands,
        shadow: driver.shadow,
        native,
        refinement,
    })?;
    let terminal = report
        .settlement()
        .map_err(|cause| CompositeShadowError::from(CompositeBuildError::Report(cause)))?
        .terminal_value();
    Ok(CompositeShadowSettlement {
        terminal,
        route: ShadowRoute::Parity,
        report: Some(ParityReport::Falsified(Box::new(report))),
        defect: None,
    })
}
