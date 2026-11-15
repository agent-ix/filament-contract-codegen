//! Bounded harnesses for admitted V2 scalar clauses (FR-015).

#[cfg(test)]
#[allow(dead_code, clippy::duplicate_mod)]
#[path = "../../../../tests/state_frame_support/model.rs"]
mod model;
#[cfg(test)]
#[allow(dead_code, clippy::duplicate_mod)]
#[path = "../../../../tests/state_frame_support/native_twin.rs"]
mod native_twin;

use filament_contract_model::{
    ApplicationOperator, CheckedMemberType, CheckedModelOperations, CheckedNodeId, CheckedNodeKind,
    CheckedOperationParameter, CheckedOperationReadError, CheckedPackageV2, CheckedTerm,
    CompositeTypeForm, ContractSourceV1, LiteralKind, ScalarTypeForm,
};
use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use fsl_replay::ObligationIdentity;

use crate::kani::tractability::{
    ClauseFeatures, Decision, InputBound, IntegerRoute, OperationWidth, TractabilityDecider,
};
use crate::{
    core::ir::{
        checked_term::{application, bound_members, literal, reference},
        clause::{
            option_shape, read_clause, union_members, ClauseCondition, ClauseGraphView,
            ClauseInput, ClauseInputSide, ClauseLeaf, ClauseTerm, OptionLeaf, OptionShape,
            CONTROL_CASE, CONTROL_IF, CONTROL_LET, MAX_CONDITION_TERMS, OPTION_PRESENT,
            OPTION_VALUE,
        },
    },
    kani::{
        abi::{KaniPrimitiveType, KaniSolver},
        census::{dependency_readiness, ProofDependencyEdge, ProofExecutionState},
        generate::{
            outcome::{
                node_label, KaniObligationRequest, OptionNotLoweredReason, UnsupportedObligation,
                V2ClauseControlReason,
            },
            record::{harness_path, record},
            render,
            spec::{
                Argument, Cover, ExprText, HarnessSpec, Init, LetBinding, LetType,
                NativeAdmissionStage, ProofKind, Statement,
            },
        },
        identity::{
            KaniV2ClauseObligationHarness, ObligationKind, ScalarObligationArgument,
            V2ClauseObligationClaim, V2ClauseObligationIdentity, V2ClauseParameter, V2ClauseRead,
            V2ClauseSelection, V2ClauseStateField, V2_INVARIANT_ASSERTION_MESSAGE,
        },
    },
    oracle::clause::{
        clause_operation_widths, generate_v2_clause_native_oracle,
        generate_v2_clause_native_oracle_with_view, generate_v2_clause_oracle_with_view,
        V2ClauseOracle,
    },
};

pub(in crate::kani::generate) struct LoweredV2Clause {
    dependencies: Vec<ProofDependencyEdge>,
    pub(super) package: ContractSourceV1,
    pub(super) node_id: CheckedNodeId,
    kind: ObligationKind,
    oracle: V2ClauseOracle,
    native_integer: bool,
    native_expectation: Option<String>,
    arithmetic: Vec<ArithmeticProbe>,
    symbolic_name: Option<String>,
    result_postcondition: Option<ResultPostcondition>,
    state_postcondition: Option<StatePostcondition>,
    obligation: ObligationIdentity,
    pub(super) module_symbol: String,
    pub(super) harness_symbol: String,
}

#[derive(Clone)]
enum MachineScalar {
    Boolean,
    Integer {
        minimum: i64,
        maximum: i64,
    },
    /// An option parameter (FR-054): a presence draw and the payload leaves in ascending path order.
    Option {
        leaves: Vec<OptionLeaf>,
    },
}

struct ResultParameter {
    name: String,
    scalar: MachineScalar,
}

struct ResultPostcondition {
    result: Option<MachineScalar>,
    parameters: Vec<ResultParameter>,
}

/// Draw one option parameter: a presence `bool` then each payload leaf as a native `Copy` value,
/// with every leaf canonical (`0`, `false`) when absent and inside its bound when present
/// (FR-054). Each draw is a harness argument in that order.
fn draw_option(
    native: bool,
    name: &str,
    leaves: &[OptionLeaf],
    body: &mut Vec<Statement>,
    arguments: &mut Vec<ScalarObligationArgument>,
    oracle_bindings: &mut BTreeSet<String>,
) -> Result<(), UnsupportedObligation> {
    let refusal = || UnsupportedObligation::RenderFailed;
    let draws = option_symbolics(name, leaves);
    let (_, presence) = draws.first().ok_or_else(refusal)?;
    let mut canonical = Vec::new();
    let mut bounded = Vec::new();
    let mut statements = Vec::new();
    let mut identities = Vec::new();
    let mut oracle_names = Vec::new();
    for ((leaf, symbolic), draw) in draws.iter().skip(1).zip(leaves) {
        let oracle_name = symbolic
            .strip_suffix("_current")
            .ok_or_else(refusal)?
            .to_owned();
        if syn::parse_str::<syn::Ident>(symbolic).is_err() || draw.path != payload_path(leaf) {
            return Err(refusal());
        }
        let (ty, domain, init) = if draw.boolean {
            canonical.push(format!("!{symbolic}"));
            bounded.push("true".to_owned());
            (KaniPrimitiveType::Boolean, (0, 1), symbolic.clone())
        } else {
            let minimum = i64::try_from(draw.lower).map_err(|_| refusal())?;
            let maximum = i64::try_from(draw.upper).map_err(|_| refusal())?;
            canonical.push(format!("{symbolic} == 0"));
            bounded.push(format!(
                "{symbolic} >= {} && {symbolic} <= {}",
                crate::kani::abi::i64_literal(minimum),
                crate::kani::abi::i64_literal(maximum)
            ));
            (
                KaniPrimitiveType::I64,
                // The draw's declared domain also holds the canonical absent payload.
                (i128::from(minimum.min(0)), i128::from(maximum.max(0))),
                if native {
                    format!("nx::NativeInteger::new(i128::from({symbolic}))")
                } else {
                    format!("rt::Integer::from({symbolic})")
                },
            )
        };
        statements.push(Statement::Input {
            name: symbolic.clone(),
            ty,
            domain,
            then: vec![LetBinding {
                name: oracle_name.clone(),
                ty: None,
                mutable: false,
                init: Init::Expression(expression(init)?),
            }],
        });
        identities.push(ScalarObligationArgument {
            identifier: symbolic.clone(),
            minimum: i64::try_from(domain.0).map_err(|_| refusal())?,
            maximum: i64::try_from(domain.1).map_err(|_| refusal())?,
        });
        oracle_names.push(oracle_name);
    }
    let oracle_presence = presence
        .strip_suffix("_current")
        .ok_or_else(refusal)?
        .to_owned();
    body.push(Statement::Input {
        name: presence.clone(),
        ty: KaniPrimitiveType::Boolean,
        domain: (0, 1),
        then: vec![LetBinding {
            name: oracle_presence.clone(),
            ty: None,
            mutable: false,
            init: Init::Expression(expression(presence)?),
        }],
    });
    arguments.push(ScalarObligationArgument {
        identifier: presence.clone(),
        minimum: 0,
        maximum: 1,
    });
    body.extend(statements);
    arguments.extend(identities);
    oracle_bindings.insert(oracle_presence);
    oracle_bindings.extend(oracle_names);
    if !canonical.is_empty() {
        body.push(Statement::Assume {
            condition: expression(format!("{presence} || ({})", canonical.join(" && ")))?,
        });
        body.push(Statement::Assume {
            condition: expression(format!("!{presence} || ({})", bounded.join(" && ")))?,
        });
    }
    Ok(())
}

fn payload_path(leaf: &ClauseLeaf) -> &str {
    match leaf {
        ClauseLeaf::Payload(path) => path,
        _ => "",
    }
}

/// A postcondition that reads state: the state fields the subject receives by `&mut`, the operation
/// parameters it receives by value, and the sibling preconditions assumed as its `requires`.
struct StatePostcondition {
    reads: Vec<StateRead>,
    parameters: Vec<StateParameter>,
    requires: Vec<StateRequire>,
    /// Each oracle input name and the variable that supplies it.
    oracle_inputs: Vec<(String, String)>,
}

struct StateRead {
    declaring_node_id: CheckedNodeId,
    field_name: String,
    minimum: i64,
    maximum: i64,
    /// The drawn pre-state.
    pre: String,
    /// The state the subject produced.
    post: String,
}

struct StateParameter {
    name: String,
    minimum: i64,
    maximum: i64,
    drawn: String,
}

/// A same-anchor precondition assumed through its own native oracle, embedded in a nested module.
struct StateRequire {
    precondition: CheckedNodeId,
    oracle: V2ClauseOracle,
    /// Each oracle input name and the drawn pre-state or parameter variable that supplies it.
    inputs: Vec<(String, String)>,
}

/// Encode the complete structural key for a generated message. Escaping `%` keeps
/// this reversible even when a source name itself contains an escape-looking span.
pub(in crate::kani::generate) fn message_safe_node_label(node: &CheckedNodeId) -> String {
    escape_message_label(&node_label(node).to_string())
}

fn escape_message_label(raw: &str) -> String {
    let mut label = String::new();
    for character in raw.chars() {
        match character {
            '%' | '"' | '\\' | '{' | '}' | '\n' | '\r' => {
                let _ = write!(&mut label, "%{:02X}", u32::from(character));
            }
            other => label.push(other),
        }
    }
    label
}

pub(in crate::kani::generate) fn lower(
    view: &ClauseGraphView<'_>,
    clause: &CheckedNodeId,
    kind: ObligationKind,
    claim: &V2ClauseObligationClaim,
    route: IntegerRoute,
    decider: &dyn TractabilityDecider,
) -> Result<LoweredV2Clause, UnsupportedObligation> {
    lower_with_census(
        view,
        clause,
        kind,
        claim,
        route,
        decider,
        |package, context| package.model_operations(context, 65_536),
    )
}

fn lower_with_census<'a>(
    view: &ClauseGraphView<'a>,
    clause: &CheckedNodeId,
    kind: ObligationKind,
    claim: &V2ClauseObligationClaim,
    route: IntegerRoute,
    decider: &dyn TractabilityDecider,
    census: impl FnOnce(
        &'a CheckedPackageV2,
        &CheckedNodeId,
    ) -> Result<CheckedModelOperations<'a>, CheckedOperationReadError>,
) -> Result<LoweredV2Clause, UnsupportedObligation> {
    let package = view.package();
    let refusal = || UnsupportedObligation::V2ClauseConditionNotRendered {
        node_id: clause.clone(),
    };
    match unsupported_checked_clause_operator(view, clause) {
        Some(ClauseRefusal::Operator(operation_identity)) => {
            return Err(UnsupportedObligation::V2ClauseOperatorNotSupported {
                node_id: clause.clone(),
                operation_identity,
            });
        }
        Some(ClauseRefusal::Control {
            reason,
            operation_identity,
            node,
        }) => {
            return Err(UnsupportedObligation::V2ClauseControlNotSupported {
                node_id: clause.clone(),
                control_node_id: node.unwrap_or_else(|| clause.clone()),
                operation_identity,
                reason,
            });
        }
        Some(ClauseRefusal::Option {
            reason,
            detail,
            node,
        }) => {
            return Err(UnsupportedObligation::OptionNotLowered {
                node_id: clause.clone(),
                option_node_id: node.unwrap_or_else(|| clause.clone()),
                reason,
                detail,
            });
        }
        None => {}
    }
    let condition = read_clause(clause, view).ok_or_else(refusal)?;
    if let Some(operation_identity) = unsupported_clause_operator(&condition.term) {
        return Err(UnsupportedObligation::V2ClauseOperatorNotSupported {
            node_id: clause.clone(),
            operation_identity: operation_identity.to_owned(),
        });
    }
    let result_postcondition = if kind == ObligationKind::Postcondition {
        Some(read_postcondition_signature(view, clause, census)?)
    } else {
        None
    };
    let oracle = generate_v2_clause_oracle_with_view(clause, view).map_err(|_| refusal())?;
    // An option operand is drawn as a presence and its payload leaves only for a Boolean-result
    // postcondition (FR-054); a precondition, invariant or state postcondition reading one is
    // not rendered.
    if condition
        .inputs
        .iter()
        .any(|input| input.leaf != ClauseLeaf::Whole)
        && !result_postcondition
            .as_ref()
            .is_some_and(|signature| matches!(signature.result, Some(MachineScalar::Boolean)))
    {
        return Err(refusal());
    }
    let (state_postcondition, state_expectation, state_arithmetic) = match &result_postcondition {
        Some(signature)
            if kind == ObligationKind::Postcondition
                && signature.result.is_none()
                && oracle
                    .descriptor
                    .inputs
                    .iter()
                    .any(|input| input.state_field.is_some()) =>
        {
            let (state, expectation, arithmetic) =
                read_state_postcondition(view, clause, &condition, &oracle, signature)?;
            (Some(state), Some(expectation), arithmetic)
        }
        _ => (None, None, Vec::new()),
    };
    let input_count_supported = match kind {
        ObligationKind::Precondition => true,
        ObligationKind::Postcondition => {
            result_postcondition
                .as_ref()
                .is_some_and(|signature| signature.result.is_some())
                || oracle.descriptor.inputs.len() == 1
        }
        ObligationKind::Invariant => true,
        ObligationKind::Frame => false,
    };
    if state_postcondition.is_none()
        && (!input_count_supported
            || oracle.descriptor.inputs.iter().any(|input| {
                let supported = match kind {
                    ObligationKind::Precondition => {
                        input.name.starts_with("parameter_") && input.state_field.is_none()
                            || input.name.starts_with("current_") && input.state_field.is_some()
                    }
                    ObligationKind::Postcondition => {
                        input.state_field.is_none()
                            && (input.name.starts_with("parameter_")
                                || result_postcondition
                                    .as_ref()
                                    .is_some_and(|signature| signature.result.is_some())
                                    && input.name == "result_result")
                    }
                    ObligationKind::Invariant => {
                        input.name.starts_with("current_") && input.state_field.is_some()
                    }
                    ObligationKind::Frame => false,
                };
                !supported
                    || (input.boolean
                        && result_postcondition
                            .as_ref()
                            .is_some_and(|signature| signature.result.is_none()))
                    || input.lower > input.upper
                    || i64::try_from(input.lower).is_err()
                    || i64::try_from(input.upper).is_err()
            }))
    {
        return Err(refusal());
    }
    // Arithmetic clauses require independent subterm probes before they can be asserted.
    if kind == ObligationKind::Invariant
        && oracle
            .descriptor
            .operation_identities
            .iter()
            .any(|identity| {
                matches!(
                    identity.as_str(),
                    "filament.op.integer.add"
                        | "filament.op.integer.sub"
                        | "filament.op.integer.mul"
                        | "filament.op.integer.negate"
                )
            })
    {
        return Err(refusal());
    }
    let (native_expectation, arithmetic, symbolic_name) = if state_postcondition.is_some() {
        (state_expectation, state_arithmetic, None)
    } else if kind == ObligationKind::Postcondition
        && result_postcondition
            .as_ref()
            .is_some_and(|signature| signature.result.is_none())
    {
        let signature = result_postcondition.as_ref().ok_or_else(refusal)?;
        let [parameter] = signature.parameters.as_slice() else {
            return Err(refusal());
        };
        let input = oracle.descriptor.inputs.first().ok_or_else(refusal)?;
        let input_name = input.name.strip_prefix("parameter_").ok_or_else(refusal)?;
        if input_name != parameter.name
            || !matches!(
                parameter.scalar,
                MachineScalar::Integer { minimum, maximum }
                    if input.lower == i128::from(minimum) && input.upper == i128::from(maximum)
            )
        {
            return Err(refusal());
        }
        let symbolic_name = format!("{input_name}_current");
        let mut arithmetic = Vec::new();
        let expectation = native_term(
            &condition.term,
            &[NativeBinding::parameter(input_name, &symbolic_name)],
            &[],
            &mut arithmetic,
        )
        .and_then(NativeTerm::boolean)
        .ok_or_else(refusal)?;
        (Some(expectation), arithmetic, Some(symbolic_name))
    } else if kind == ObligationKind::Postcondition
        && result_postcondition
            .as_ref()
            .is_some_and(|signature| matches!(signature.result, Some(MachineScalar::Boolean)))
    {
        let signature = result_postcondition.as_ref().ok_or_else(refusal)?;
        if let [parameter] = signature.parameters.as_slice() {
            let symbolic_name = format!("parameter_{}_current", parameter.name);
            let mut arithmetic = Vec::new();
            let expectation = match (&parameter.scalar, result_expectation_term(&condition.term)) {
                (MachineScalar::Option { leaves }, Some(term)) => {
                    let names = option_symbolics(&parameter.name, leaves);
                    let bindings = names
                        .iter()
                        .map(|(leaf, symbolic)| NativeBinding {
                            side: ClauseInputSide::Parameter,
                            name: &parameter.name,
                            leaf: leaf.clone(),
                            symbolic,
                        })
                        .collect::<Vec<_>>();
                    native_term(term, &bindings, &[], &mut arithmetic).and_then(NativeTerm::boolean)
                }
                (MachineScalar::Integer { .. }, Some(term)) => native_term(
                    term,
                    &[NativeBinding::parameter(&parameter.name, &symbolic_name)],
                    &[],
                    &mut arithmetic,
                )
                .and_then(NativeTerm::boolean),
                _ => None,
            };
            (expectation, arithmetic, Some(symbolic_name))
        } else {
            (None, Vec::new(), None)
        }
    } else {
        (None, Vec::new(), None)
    };
    refuse_if_not_tractable(view, clause, &oracle, route, decider)?;
    // Kani includes the module in generated filenames. Use the full admitted pair so two
    // revisions of one declaration cannot collide on a readable clause name.
    let package_identity = ContractSourceV1 {
        name: package.name().into(),
        lock: package.lock().clone(),
    };
    let node_key = node_label(clause).to_string();
    let preimage = crate::core::canonical::preimage_value(&(&package_identity, &node_key))
        .map_err(|_| UnsupportedObligation::RenderFailed)?;
    let digest = crate::core::canonical::content_digest(&preimage)
        .map_err(|_| UnsupportedObligation::RenderFailed)?;
    let module_symbol = format!(
        "{}_{}",
        if kind == ObligationKind::Postcondition {
            "kob_v2_clause"
        } else if kind == ObligationKind::Invariant {
            "kob_v2_inv"
        } else {
            "kob_v2_pre"
        },
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    Ok(LoweredV2Clause {
        dependencies: Vec::new(),
        package: package_identity,
        node_id: clause.clone(),
        kind,
        oracle,
        native_integer: false,
        native_expectation,
        arithmetic,
        symbolic_name,
        result_postcondition,
        state_postcondition,
        obligation: claim.obligation.clone(),
        module_symbol,
        harness_symbol: if kind == ObligationKind::Postcondition {
            "prove_clause"
        } else if kind == ObligationKind::Invariant {
            "assert_invariant"
        } else {
            "cover_precondition"
        }
        .to_owned(),
    })
}

/// The FR-056 refusal point. It runs after the operator check, so it sees only operations the
/// V2 clause path accepts. An operation without a known width never matches a row.
fn refuse_if_not_tractable(
    view: &ClauseGraphView<'_>,
    clause: &CheckedNodeId,
    oracle: &V2ClauseOracle,
    route: IntegerRoute,
    decider: &dyn TractabilityDecider,
) -> Result<(), UnsupportedObligation> {
    let Ok(widths) = clause_operation_widths(clause, view) else {
        return Ok(());
    };
    let features = ClauseFeatures::new(
        route,
        widths
            .into_iter()
            .map(|(operation_id, bits)| OperationWidth { operation_id, bits })
            .collect(),
        oracle
            .descriptor
            .inputs
            .iter()
            .map(|input| InputBound {
                lower: input.lower,
                upper: input.upper,
                boolean: input.boolean,
            })
            .collect(),
        None,
    );
    match decider.decide(&features) {
        Decision::Run => Ok(()),
        Decision::Refuse {
            reason,
            evidence_row,
        } => Err(UnsupportedObligation::V2ClauseNotTractable {
            node_id: clause.clone(),
            operation_identity: reason.operation_id,
            width_bits: reason.width_bits,
            route,
            row: evidence_row,
        }),
    }
}

/// The declared operation parameters of a state-reading postcondition, each an integer.
fn state_parameters(signature: &ResultPostcondition) -> Option<Vec<StateParameter>> {
    signature
        .parameters
        .iter()
        .map(|parameter| {
            let MachineScalar::Integer { minimum, maximum } = parameter.scalar else {
                return None;
            };
            let drawn = format!("parameter_{}_current", parameter.name);
            syn::parse_str::<syn::Ident>(&drawn).ok()?;
            Some(StateParameter {
                name: parameter.name.clone(),
                minimum,
                maximum,
                drawn,
            })
        })
        .collect()
}

type StateFieldDomains = BTreeMap<(String, CheckedNodeId), (i64, i64)>;

/// Admit one clause input: a state field read on the pre or current side with one `i64` integer
/// domain per field, or a declared integer parameter read over its declared domain.
fn record_state_input(
    fields: &mut StateFieldDomains,
    parameters: &[StateParameter],
    input: &ClauseInput,
) -> Option<()> {
    if input.boolean {
        return None;
    }
    match (input.side, &input.declaring_node_id) {
        (ClauseInputSide::Pre | ClauseInputSide::Current, Some(declaring)) => {
            let bounds = (
                i64::try_from(input.lower).ok()?,
                i64::try_from(input.upper).ok()?,
            );
            if bounds.0 > bounds.1 {
                return None;
            }
            let previous = fields
                .entry((input.name.clone(), declaring.clone()))
                .or_insert(bounds);
            (*previous == bounds).then_some(())
        }
        (ClauseInputSide::Parameter, None) => {
            let declared = parameters
                .iter()
                .find(|parameter| parameter.name == input.name)?;
            ((input.lower, input.upper) == (declared.minimum.into(), declared.maximum.into()))
                .then_some(())
        }
        _ => None,
    }
}

/// The drawn state fields in field-name order. Two fields of one name would share their draws.
fn state_reads(fields: StateFieldDomains) -> Option<Vec<StateRead>> {
    let mut reads: Vec<StateRead> = Vec::new();
    for ((field_name, declaring_node_id), (minimum, maximum)) in fields {
        let (pre, post) = (
            format!("pre_{field_name}_current"),
            format!("post_{field_name}_current"),
        );
        syn::parse_str::<syn::Ident>(&pre).ok()?;
        if reads
            .last()
            .is_some_and(|last| last.field_name == field_name)
        {
            return None;
        }
        reads.push(StateRead {
            declaring_node_id,
            field_name,
            minimum,
            maximum,
            pre,
            post,
        });
    }
    Some(reads)
}

/// The state-reading scalar postcondition shape: its draws, its `requires` and its independent
/// native expectation. A shape this renderer cannot represent refuses as a not-rendered condition,
/// and a sibling precondition it cannot assume refuses naming that precondition.
fn read_state_postcondition(
    view: &ClauseGraphView<'_>,
    clause: &CheckedNodeId,
    condition: &ClauseCondition,
    oracle: &V2ClauseOracle,
    signature: &ResultPostcondition,
) -> Result<(StatePostcondition, String, Vec<ArithmeticProbe>), UnsupportedObligation> {
    let refusal = || UnsupportedObligation::V2ClauseConditionNotRendered {
        node_id: clause.clone(),
    };
    let parameters = state_parameters(signature).ok_or_else(refusal)?;
    let mut fields = StateFieldDomains::new();
    for input in &condition.inputs {
        record_state_input(&mut fields, &parameters, input).ok_or_else(refusal)?;
    }
    let siblings = super::super::negotiate::same_anchor_preconditions(view.package(), clause);
    for precondition in &siblings {
        let not_rendered = || UnsupportedObligation::V2ClauseSiblingRequiresNotRendered {
            node_id: clause.clone(),
            precondition: precondition.clone(),
        };
        let read = read_clause(precondition, view).ok_or_else(not_rendered)?;
        for input in &read.inputs {
            if input.side == ClauseInputSide::Pre {
                return Err(not_rendered());
            }
            record_state_input(&mut fields, &parameters, input).ok_or_else(not_rendered)?;
        }
    }
    let reads = state_reads(fields).ok_or_else(refusal)?;
    let mut bindings = Vec::new();
    for read in &reads {
        bindings.push(NativeBinding {
            side: ClauseInputSide::Pre,
            name: &read.field_name,
            leaf: ClauseLeaf::Whole,
            symbolic: &read.pre,
        });
        bindings.push(NativeBinding {
            side: ClauseInputSide::Current,
            name: &read.field_name,
            leaf: ClauseLeaf::Whole,
            symbolic: &read.post,
        });
    }
    for parameter in &parameters {
        bindings.push(NativeBinding::parameter(&parameter.name, &parameter.drawn));
    }
    let mut arithmetic = Vec::new();
    let expectation = native_term(&condition.term, &bindings, &[], &mut arithmetic)
        .and_then(NativeTerm::boolean)
        .ok_or_else(refusal)?;
    // A sibling is assumed through its own native oracle on the state the subject receives. An
    // arithmetic sibling has no independent probes here, so it refuses.
    let mut requires = Vec::new();
    for precondition in siblings {
        let not_rendered = || UnsupportedObligation::V2ClauseSiblingRequiresNotRendered {
            node_id: clause.clone(),
            precondition: precondition.clone(),
        };
        let sibling = generate_v2_clause_native_oracle_with_view(&precondition, view)
            .map_err(|_| not_rendered())?;
        if sibling
            .descriptor
            .operation_identities
            .iter()
            .any(|identity| {
                matches!(
                    identity.as_str(),
                    "filament.op.integer.add"
                        | "filament.op.integer.sub"
                        | "filament.op.integer.mul"
                        | "filament.op.integer.negate"
                )
            })
        {
            return Err(not_rendered());
        }
        let mut inputs = Vec::new();
        for input in &sibling.descriptor.inputs {
            let source = if let Some(field) = input.state_field.as_ref().and_then(|field| {
                reads.iter().find(|read| {
                    read.field_name == field.field_name
                        && read.declaring_node_id == field.declaring_node_id
                })
            }) {
                input
                    .name
                    .starts_with("current_")
                    .then(|| field.pre.clone())
            } else {
                input.name.strip_prefix("parameter_").and_then(|_| {
                    parameters
                        .iter()
                        .find(|parameter| parameter.name == input.declared_name)
                        .map(|parameter| parameter.drawn.clone())
                })
            };
            if input.boolean {
                return Err(not_rendered());
            }
            inputs.push((input.name.clone(), source.ok_or_else(not_rendered)?));
        }
        requires.push(StateRequire {
            precondition,
            oracle: sibling,
            inputs,
        });
    }
    let mut oracle_inputs = Vec::new();
    for input in &oracle.descriptor.inputs {
        let declared = input.declared_name.as_str();
        let source = if let Some(parameter) = input.name.strip_prefix("parameter_").and_then(|_| {
            parameters
                .iter()
                .find(|parameter| parameter.name == declared)
        }) {
            parameter.drawn.clone()
        } else if let Some(field) = input.state_field.as_ref().and_then(|field| {
            reads.iter().find(|read| {
                read.field_name == field.field_name
                    && read.declaring_node_id == field.declaring_node_id
            })
        }) {
            if input.name.starts_with("pre_") {
                field.pre.clone()
            } else if input.name.starts_with("current_") {
                field.post.clone()
            } else {
                return Err(refusal());
            }
        } else {
            return Err(refusal());
        };
        oracle_inputs.push((input.name.clone(), source));
    }
    Ok((
        StatePostcondition {
            reads,
            parameters,
            requires,
            oracle_inputs,
        },
        expectation,
        arithmetic,
    ))
}

fn machine_scalar(member: &CheckedMemberType) -> Option<MachineScalar> {
    match member {
        CheckedMemberType::Boolean => Some(MachineScalar::Boolean),
        CheckedMemberType::IntRange { lower, upper } if lower <= upper => {
            Some(MachineScalar::Integer {
                minimum: i64::try_from(*lower).ok()?,
                maximum: i64::try_from(*upper).ok()?,
            })
        }
        _ => None,
    }
}

fn binder_matches_scalar(
    view: &ClauseGraphView<'_>,
    binder: &CheckedNodeId,
    scalar: &MachineScalar,
) -> bool {
    let Some(node) = view.node(binder) else {
        return false;
    };
    match scalar {
        MachineScalar::Option { leaves } => matches!(
            option_shape(view, &node.semantic_type),
            Some(OptionShape::Leaves(actual)) if actual == *leaves
        ),
        MachineScalar::Boolean => {
            view.kind(&node.semantic_type)
                == Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean))
        }
        MachineScalar::Integer { minimum, maximum } => {
            let Some(CheckedTerm::Aggregate { members }) = view.package().term(&node.semantic_type)
            else {
                return false;
            };
            let number = |name| {
                members.iter().find_map(|entry| match entry {
                    CheckedTerm::Binding {
                        name: actual,
                        value,
                    } if actual.as_ref() == name => {
                        literal(value, LiteralKind::Integer)?.parse::<i64>().ok()
                    }
                    _ => None,
                })
            };
            view.kind(&node.semantic_type)
                == Some(CheckedNodeKind::BoundedDomain(
                    filament_contract_model::BoundedDomainForm::IntegerRange,
                ))
                && number("min") == Some(*minimum)
                && number("max") == Some(*maximum)
        }
    }
}

fn self_binder_matches_context(
    view: &ClauseGraphView<'_>,
    binder: &CheckedNodeId,
    context: &CheckedNodeId,
) -> bool {
    let Some(node) = view.node(binder) else {
        return false;
    };
    view.kind(&node.semantic_type)
        == Some(CheckedNodeKind::CompositeType(CompositeTypeForm::Reference))
        && matches!(
            view.package().term(&node.semantic_type),
            Some(CheckedTerm::Aggregate { members })
                if matches!(members.as_slice(), [CheckedTerm::Reference { target }] if target == context)
        )
}

fn read_postcondition_signature<'a>(
    view: &ClauseGraphView<'a>,
    clause: &CheckedNodeId,
    census_read: impl FnOnce(
        &'a CheckedPackageV2,
        &CheckedNodeId,
    ) -> Result<CheckedModelOperations<'a>, CheckedOperationReadError>,
) -> Result<ResultPostcondition, UnsupportedObligation> {
    let mismatch = || UnsupportedObligation::V2ClauseSignatureMismatch {
        node_id: clause.clone(),
    };
    let unsupported = || UnsupportedObligation::V2ClauseResultDomainUnsupported {
        node_id: clause.clone(),
    };
    let package = view.package();
    let [binders, anchor, _] = application(
        package.term(clause).ok_or_else(mismatch)?,
        ApplicationOperator::StateClause,
        "filament.op.state.clause",
    )
    .ok_or_else(mismatch)?
    else {
        return Err(mismatch());
    };
    let CheckedTerm::Aggregate { members } = binders else {
        return Err(mismatch());
    };
    let anchor = reference(anchor).ok_or_else(mismatch)?;
    let Some(CheckedTerm::OperationAnchorBody(body)) = package.term(anchor) else {
        return Err(mismatch());
    };
    let [context, operation_name, _] =
        bound_members(&body.members, ["context", "operation", "frame"]).ok_or_else(mismatch)?;
    let context = reference(context).ok_or_else(mismatch)?;
    let operation_name = literal(operation_name, LiteralKind::Text).ok_or_else(mismatch)?;
    let census = census_read(package, context).map_err(|source| {
        UnsupportedObligation::V2ClauseOperationRead {
            node_id: clause.clone(),
            source,
        }
    })?;
    let mut matches = census
        .operations()
        .iter()
        .filter(|operation| operation.name() == operation_name);
    let operation = matches.next().ok_or_else(mismatch)?;
    if matches.next().is_some() {
        return Err(mismatch());
    }
    let result = operation
        .result()
        .map(|member| machine_scalar(member.ok_or_else(unsupported)?).ok_or_else(unsupported))
        .transpose()?;
    validate_signature_binders(
        view,
        clause,
        context,
        members,
        operation.parameters(),
        result,
    )
}

fn validate_signature_binders(
    view: &ClauseGraphView<'_>,
    clause: &CheckedNodeId,
    context: &CheckedNodeId,
    members: &[CheckedTerm],
    signature_parameters: &[CheckedOperationParameter<'_>],
    result: Option<MachineScalar>,
) -> Result<ResultPostcondition, UnsupportedObligation> {
    let mismatch = || UnsupportedObligation::V2ClauseSignatureMismatch {
        node_id: clause.clone(),
    };
    let package = view.package();
    let expected = signature_parameters
        .len()
        .checked_add(if result.is_some() { 2 } else { 1 })
        .ok_or_else(mismatch)?;
    if members.len() != expected {
        return Err(mismatch());
    }
    let mut ids = members.iter().map(reference);
    let self_id = ids.next().flatten().ok_or_else(mismatch)?;
    if package
        .parameter_declaration(self_id)
        .map_err(|_| mismatch())?
        .name
        .as_ref()
        != "self"
        || !self_binder_matches_context(view, self_id, context)
    {
        return Err(mismatch());
    }
    if let Some(result) = &result {
        let result_id = ids.next().flatten().ok_or_else(mismatch)?;
        if package
            .parameter_declaration(result_id)
            .map_err(|_| mismatch())?
            .name
            .as_ref()
            != "result"
            || !binder_matches_scalar(view, result_id, result)
        {
            return Err(mismatch());
        }
    }
    let mut parameters = Vec::new();
    let mut parameter_names = BTreeSet::new();
    for (id, parameter) in ids.zip(signature_parameters) {
        let id = id.ok_or_else(mismatch)?;
        if matches!(parameter.name(), "self" | "result")
            || !parameter_names.insert(parameter.name())
        {
            return Err(mismatch());
        }
        let member = parameter.member_type().ok_or_else(mismatch)?;
        let scalar = if matches!(member, CheckedMemberType::Option(_)) {
            let node = view.node(id).ok_or_else(mismatch)?;
            let Some(OptionShape::Leaves(leaves)) = option_shape(view, &node.semantic_type) else {
                return Err(mismatch());
            };
            MachineScalar::Option { leaves }
        } else {
            machine_scalar(member).ok_or_else(mismatch)?
        };
        if package
            .parameter_declaration(id)
            .map_err(|_| mismatch())?
            .name
            .as_ref()
            != parameter.name()
            || !binder_matches_scalar(view, id, &scalar)
        {
            return Err(mismatch());
        }
        parameters.push(ResultParameter {
            name: parameter.name().to_owned(),
            scalar,
        });
    }
    Ok(ResultPostcondition { result, parameters })
}

/// The first refusal the depth-first, left-to-right walk of a clause body meets.
enum ClauseRefusal {
    Operator(String),
    Control {
        reason: V2ClauseControlReason,
        operation_identity: String,
        node: Option<CheckedNodeId>,
    },
    Option {
        reason: OptionNotLoweredReason,
        detail: String,
        node: Option<CheckedNodeId>,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Family {
    Boolean,
    Integer,
    Other,
}

impl Family {
    fn native_scalar(self) -> bool {
        self != Self::Other
    }
}

fn type_family(view: &ClauseGraphView<'_>, type_node: &CheckedNodeId) -> Family {
    match view.kind(type_node) {
        Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean)) => Family::Boolean,
        Some(
            CheckedNodeKind::ScalarType(ScalarTypeForm::Integer)
            | CheckedNodeKind::BoundedDomain(
                filament_contract_model::BoundedDomainForm::IntegerRange,
            ),
        ) => Family::Integer,
        _ => Family::Other,
    }
}

/// The value family of a checked term: a literal's own kind, an application's result type and a
/// referenced node's semantic type.
fn term_family(view: &ClauseGraphView<'_>, term: &CheckedTerm) -> Family {
    match term {
        CheckedTerm::Literal { value_kind, .. } => match value_kind {
            LiteralKind::Boolean => Family::Boolean,
            LiteralKind::Integer => Family::Integer,
            _ => Family::Other,
        },
        CheckedTerm::Reference { target } => view
            .node(target)
            .map_or(Family::Other, |node| type_family(view, &node.semantic_type)),
        CheckedTerm::Application { result_type, .. } => type_family(view, result_type),
        _ => Family::Other,
    }
}

/// The shape check of one control application, run on entering its node before its operands are
/// walked. Returns the reason it is refused, or the terms its operands contribute to the walk in
/// source order.
fn control_operands<'a>(
    view: &ClauseGraphView<'a>,
    identity: &str,
    arguments: &'a [CheckedTerm],
    parameters: &[CheckedNodeId],
) -> Result<Vec<&'a CheckedTerm>, V2ClauseControlReason> {
    match (identity, arguments) {
        (CONTROL_LET, [CheckedTerm::Binding { value, .. }, body]) => {
            if !term_family(view, value).native_scalar() {
                return Err(V2ClauseControlReason::LetValueNotNativeScalar);
            }
            Ok(vec![value.as_ref(), body])
        }
        (CONTROL_IF, [_, then, other]) => {
            if !term_family(view, then).native_scalar() || !term_family(view, other).native_scalar()
            {
                return Err(V2ClauseControlReason::IfBranchNotNativeScalar);
            }
            Ok(arguments.iter().collect())
        }
        (CONTROL_CASE, [scrutinee, arms @ ..]) => {
            let declared = match scrutinee {
                CheckedTerm::Reference { target } if parameters.contains(target) => view
                    .node(target)
                    .and_then(|node| union_members(view.package(), view, &node.semantic_type)),
                _ => None,
            };
            let Some(members) = declared else {
                return Err(V2ClauseControlReason::CaseScrutineeNotParameter);
            };
            if members.iter().any(|(_, arity)| *arity != 0) {
                return Err(V2ClauseControlReason::CaseUnionPayloadNotSupported);
            }
            let mut bodies = Vec::with_capacity(arms.len());
            for arm in arms {
                let CheckedTerm::Binding { value, .. } = arm else {
                    continue;
                };
                let CheckedTerm::Aggregate { members: parts } = value.as_ref() else {
                    continue;
                };
                let [_, body] = parts.as_slice() else {
                    continue;
                };
                if !term_family(view, body).native_scalar() {
                    return Err(V2ClauseControlReason::CaseArmNotNativeScalar);
                }
                bodies.push(body);
            }
            Ok(bodies)
        }
        _ => Ok(arguments.iter().collect()),
    }
}

fn unsupported_checked_clause_operator(
    view: &ClauseGraphView<'_>,
    clause: &CheckedNodeId,
) -> Option<ClauseRefusal> {
    let [declared, _, condition] = application(
        view.package().term(clause)?,
        ApplicationOperator::StateClause,
        "filament.op.state.clause",
    )?
    else {
        return None;
    };
    // The operation parameters the clause declares: every declared binder but `self` and `result`.
    let parameters = match declared {
        CheckedTerm::Aggregate { members } => members
            .iter()
            .skip(1)
            .filter_map(reference)
            .filter(|id| {
                view.package()
                    .parameter_declaration(id)
                    .is_ok_and(|declaration| declaration.name.as_ref() != "result")
            })
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut terms = vec![(condition, None::<&CheckedNodeId>)];
    let mut visited = BTreeSet::new();
    let mut visits = 0;
    while let Some((term, node)) = terms.pop() {
        if visits >= MAX_CONDITION_TERMS {
            return None;
        }
        visits += 1;
        match term {
            CheckedTerm::Reference { target } => {
                if visited.insert(target) {
                    terms.push((view.package().term(target)?, Some(target)));
                }
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => {
                let identity = operation.identity()?;
                if identity == "filament.op.record.project" {
                    // A projection of an option payload (FR-054-AC-8) walks its option read.
                    if let [argument] = arguments.as_slice() {
                        let read = match argument {
                            CheckedTerm::Reference { target } => view.package().term(target),
                            other => Some(other),
                        };
                        if matches!(
                            read,
                            Some(CheckedTerm::Application { operation, .. })
                                if operation.identity() == Some(OPTION_VALUE)
                        ) {
                            terms.push((argument, None));
                        }
                    }
                    continue;
                }
                if matches!(
                    identity,
                    "filament.op.state.pre" | "filament.op.model.deref"
                ) {
                    continue;
                }
                if identity.starts_with("filament.op.option.") {
                    if let Some((reason, detail)) = option_refusal(view, identity, arguments) {
                        return Some(ClauseRefusal::Option {
                            reason,
                            detail,
                            node: node.cloned(),
                        });
                    }
                }
                if matches!(identity, CONTROL_LET | CONTROL_IF | CONTROL_CASE) {
                    match control_operands(view, identity, arguments, &parameters) {
                        Ok(operands) => {
                            terms.extend(operands.into_iter().rev().map(|term| (term, None)));
                            continue;
                        }
                        Err(reason) => {
                            return Some(ClauseRefusal::Control {
                                reason,
                                operation_identity: identity.to_owned(),
                                node: node.cloned(),
                            });
                        }
                    }
                }
                if !supported_clause_operator(identity) {
                    return Some(ClauseRefusal::Operator(identity.to_owned()));
                }
                terms.extend(arguments.iter().rev().map(|term| (term, None)));
            }
            _ => {}
        }
    }
    None
}

/// The harness variable of each physical draw of an option parameter: the presence draw, then the
/// payload leaves in ascending path order.
fn option_symbolics(name: &str, leaves: &[OptionLeaf]) -> Vec<(ClauseLeaf, String)> {
    let mut draws = vec![(
        ClauseLeaf::Presence,
        format!("parameter_{name}_present_current"),
    )];
    draws.extend(leaves.iter().map(|leaf| {
        let suffix = if leaf.path.is_empty() {
            "_value".to_owned()
        } else {
            format!("_value_{}", leaf.path)
        };
        (
            ClauseLeaf::Payload(leaf.path.clone()),
            format!("parameter_{name}{suffix}_current"),
        )
    }));
    draws
}

/// The refusal of one `filament.op.option.*` application, from its operation and the shape of the
/// option it reads (FR-054-AC-10 to FR-054-AC-13). `None` when it is lowered, or when its refusal
/// belongs to another route (an unbounded integer payload keeps its bound refusal).
fn option_refusal(
    view: &ClauseGraphView<'_>,
    identity: &str,
    arguments: &[CheckedTerm],
) -> Option<(OptionNotLoweredReason, String)> {
    if !matches!(identity, OPTION_PRESENT | OPTION_VALUE) {
        return Some((
            OptionNotLoweredReason::OptionOperationNotLowered,
            identity.to_owned(),
        ));
    }
    let [operand] = arguments else {
        return None;
    };
    // A record field read through a projection admits the `null` state besides present and absent.
    let read = match operand {
        CheckedTerm::Reference { target } => view.package().term(target)?,
        other => other,
    };
    if let CheckedTerm::Application { operation, .. } = read {
        if operation.identity()? == "filament.op.record.project" {
            return Some((
                OptionNotLoweredReason::NullablePresence,
                "filament.op.record.project".to_owned(),
            ));
        }
    }
    let target = reference(operand)?;
    let node = view.node(target)?;
    match option_shape(view, &node.semantic_type)? {
        OptionShape::Leaves(_) | OptionShape::Unbounded => None,
        OptionShape::NotScalar(family) => Some((OptionNotLoweredReason::PayloadNotScalar, family)),
        OptionShape::Nested => Some((OptionNotLoweredReason::NestedOption, "option".to_owned())),
    }
}

fn unsupported_clause_operator(term: &ClauseTerm) -> Option<&str> {
    match term {
        ClauseTerm::Operation {
            identity,
            arguments,
            ..
        } => {
            if !supported_clause_operator(identity) {
                return Some(identity);
            }
            arguments.iter().find_map(unsupported_clause_operator)
        }
        ClauseTerm::Let { value, body, .. } => {
            unsupported_clause_operator(value).or_else(|| unsupported_clause_operator(body))
        }
        ClauseTerm::If {
            condition,
            then,
            other,
        } => [condition, then, other]
            .into_iter()
            .find_map(|term| unsupported_clause_operator(term)),
        ClauseTerm::Case { arms, .. } => arms.iter().find_map(unsupported_clause_operator),
        _ => None,
    }
}

fn supported_clause_operator(identity: &str) -> bool {
    matches!(
        identity,
        "filament.op.boolean.and"
            | "filament.op.boolean.or"
            | "filament.op.boolean.not"
            | "filament.op.boolean.implies"
            | "filament.op.boolean.eq"
            | "filament.op.boolean.ne"
            | "filament.op.integer.eq"
            | "filament.op.integer.ne"
            | "filament.op.integer.lt"
            | "filament.op.integer.le"
            | "filament.op.integer.gt"
            | "filament.op.integer.ge"
            | "filament.op.integer.add"
            | "filament.op.integer.sub"
            | "filament.op.integer.mul"
            | "filament.op.integer.negate"
            | "filament.op.option.present"
            | "filament.op.option.value"
    )
}

fn result_expectation_term(term: &ClauseTerm) -> Option<&ClauseTerm> {
    let ClauseTerm::Operation {
        identity,
        arguments,
        ..
    } = term
    else {
        return None;
    };
    if identity != "filament.op.boolean.eq" {
        return None;
    }
    let [left, right] = arguments.as_slice() else {
        return None;
    };
    match (left, right) {
        (ClauseTerm::Input(input), expression) | (expression, ClauseTerm::Input(input))
            if input.side == ClauseInputSide::Result && input.boolean =>
        {
            Some(expression)
        }
        _ => None,
    }
}

impl LoweredV2Clause {
    /// The clause's parameters and state reads by declared name, as a claim states them.
    pub(in crate::kani::generate) fn selection(
        &self,
    ) -> Result<V2ClauseSelection, UnsupportedObligation> {
        let bound = |lower: i128, upper: i128| {
            Ok::<_, UnsupportedObligation>((
                i64::try_from(lower).map_err(|_| UnsupportedObligation::RenderFailed)?,
                i64::try_from(upper).map_err(|_| UnsupportedObligation::RenderFailed)?,
            ))
        };
        let mut selection = V2ClauseSelection::default();
        if let Some(state) = &self.state_postcondition {
            for read in &state.reads {
                selection.state_reads.push(V2ClauseRead {
                    declaring_node_id: read.declaring_node_id.clone(),
                    field_name: read.field_name.clone(),
                    minimum: read.minimum,
                    maximum: read.maximum,
                    element: None,
                });
            }
            for parameter in &state.parameters {
                selection.parameters.push(V2ClauseParameter {
                    name: parameter.name.clone(),
                    minimum: parameter.minimum,
                    maximum: parameter.maximum,
                });
            }
            return Ok(selection);
        }
        if let Some(signature) = &self.result_postcondition {
            if signature.result.is_some() {
                for parameter in &signature.parameters {
                    let (minimum, maximum) = match parameter.scalar {
                        MachineScalar::Boolean | MachineScalar::Option { .. } => (0, 1),
                        MachineScalar::Integer { minimum, maximum } => (minimum, maximum),
                    };
                    selection.parameters.push(V2ClauseParameter {
                        name: parameter.name.clone(),
                        minimum,
                        maximum,
                    });
                }
                return Ok(selection);
            }
        }
        for input in &self.oracle.descriptor.inputs {
            let (minimum, maximum) = bound(input.lower, input.upper)?;
            if let Some(field) = &input.state_field {
                selection.state_reads.push(V2ClauseRead {
                    declaring_node_id: field.declaring_node_id.clone(),
                    field_name: field.field_name.clone(),
                    minimum,
                    maximum,
                    element: None,
                });
            } else {
                selection.parameters.push(V2ClauseParameter {
                    name: input.declared_name.clone(),
                    minimum,
                    maximum,
                });
            }
        }
        Ok(selection)
    }

    /// Whether the harness assumes every sibling precondition as its `requires`.
    pub(in crate::kani::generate) fn renders_requires(&self) -> bool {
        self.state_postcondition.is_some()
    }

    pub(in crate::kani::generate) fn set_dependencies(
        &mut self,
        dependencies: Vec<ProofDependencyEdge>,
    ) {
        self.dependencies = dependencies;
    }

    pub(in crate::kani::generate) fn select_native_integer(
        &mut self,
        package: &CheckedPackageV2,
    ) -> Result<(), UnsupportedObligation> {
        let oracle = generate_v2_clause_native_oracle(package, &self.node_id)
            .map_err(|refusal| UnsupportedObligation::NativeV2ClauseCapability { refusal })?;
        let selection = oracle
            .native_integer_selection
            .as_ref()
            .ok_or(UnsupportedObligation::RenderFailed)?;
        bind_native_arithmetic_probes(&mut self.arithmetic, selection)?;
        self.oracle = oracle;
        self.native_integer = true;
        Ok(())
    }
}

/// Which clause outcome a generated assertion or cover observes.
#[derive(Clone, Copy)]
enum OutcomeKind {
    CompletedTrue,
    Completed,
    OutOfDomain,
}

/// The pattern over `&outcome`. The native proof oracle returns `Result<bool, ClauseStop>`
/// with a payload-free stop; the general oracle retains the full runtime outcome.
fn outcome_pattern(native: bool, kind: OutcomeKind) -> &'static str {
    match (native, kind) {
        (true, OutcomeKind::CompletedTrue) => "Ok(true)",
        (true, OutcomeKind::Completed) => "Ok(_)",
        (true, OutcomeKind::OutOfDomain) => "Err(ClauseStop::IntegerOutOfDomain)",
        (false, OutcomeKind::CompletedTrue) => "Ok(rt::Outcome::Completed(true))",
        (false, OutcomeKind::Completed) => "Ok(rt::Outcome::Completed(_))",
        (false, OutcomeKind::OutOfDomain) => {
            "Ok(rt::Outcome::Refused(rt::Refusal::IntegerOutOfDomain { .. }))"
        }
    }
}

pub(in crate::kani::generate) fn render_clause(
    request: &KaniObligationRequest<'_>,
    lowered: &LoweredV2Clause,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    if let Some(state) = &lowered.state_postcondition {
        return render_state_postcondition(request, lowered, state);
    }
    if let Some(signature) = &lowered.result_postcondition {
        if signature.result.is_some() {
            return render_result_postcondition(request, lowered, signature);
        }
    }
    if lowered.kind == ObligationKind::Postcondition {
        return render_postcondition(request, lowered);
    }
    let inputs = &lowered.oracle.descriptor.inputs;
    let invariant = lowered.kind == ObligationKind::Invariant;
    let direct_boolean = lowered.oracle.descriptor.operation_identities.is_empty();
    let mut body = Vec::new();
    let mut arguments = Vec::new();
    let mut state_fields = Vec::new();
    let mut reachable = Vec::new();
    // A harness draws its state fields, then its parameters: the order the playback decoder
    // reads `state_fields` and `arguments` in. The oracle takes its inputs in its own order.
    let mut drawn = inputs.iter().enumerate().collect::<Vec<_>>();
    drawn.sort_by_key(|(_, input)| input.state_field.is_none());
    for (index, input) in drawn {
        let minimum =
            i64::try_from(input.lower).map_err(|_| UnsupportedObligation::RenderFailed)?;
        let maximum =
            i64::try_from(input.upper).map_err(|_| UnsupportedObligation::RenderFailed)?;
        let mut then = Vec::new();
        if invariant {
            let domain = format!("invariant_domain_{index}");
            then.push(LetBinding {
                name: domain.clone(),
                ty: None,
                mutable: false,
                init: Init::Expression(ExprText::new(format!(
                    "{} >= {minimum} && {} <= {maximum}",
                    input.name, input.name
                ))?),
            });
            reachable.push(domain);
        }
        then.push(LetBinding {
            name: input.name.clone(),
            ty: None,
            mutable: false,
            init: Init::Expression(ExprText::new(if lowered.native_integer {
                format!("nx::NativeInteger::new(i128::from({}))", input.name)
            } else {
                format!("rt::Integer::from({})", input.name)
            })?),
        });
        body.push(Statement::Input {
            name: input.name.clone(),
            ty: KaniPrimitiveType::I64,
            domain: (input.lower, input.upper),
            then,
        });
        if let Some(field) = &input.state_field {
            state_fields.push(V2ClauseStateField {
                declaring_node_id: field.declaring_node_id.clone(),
                field_name: field.field_name.clone(),
                identifier: input.name.clone(),
                minimum,
                maximum,
            });
        } else {
            arguments.push(ScalarObligationArgument {
                identifier: input.name.clone(),
                minimum,
                maximum,
            });
        }
    }
    if invariant && inputs.is_empty() {
        // The projection onto no read fields has exactly one empty valuation.
        body.push(binding(
            "invariant_domain",
            Init::Expression(expression("()")?),
            false,
        ));
    }
    let mut oracle_arguments = inputs
        .iter()
        .map(|input| Argument::Borrow(input.name.clone()))
        .collect::<Vec<_>>();
    if !direct_boolean {
        body.push(Statement::Let(LetBinding {
            name: "meter".to_owned(),
            ty: None,
            mutable: true,
            init: Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
        }));
        oracle_arguments.push(Argument::BorrowMut("meter".to_owned()));
    }
    body.push(Statement::Call {
        result: "outcome".to_owned(),
        callee: "oracle_clause".to_owned(),
        args: oracle_arguments,
    });
    if invariant {
        body.push(Statement::Assert {
            condition: ExprText::new(if direct_boolean {
                "outcome".to_owned()
            } else {
                format!(
                    "matches!(&outcome, {})",
                    outcome_pattern(lowered.native_integer, OutcomeKind::CompletedTrue)
                )
            })?,
            message: V2_INVARIANT_ASSERTION_MESSAGE.to_owned(),
        });
    }
    let (label, cover) = if invariant {
        (
            "invariant",
            Cover {
                condition: ExprText::new(if inputs.is_empty() {
                    "matches!(invariant_domain, ())".to_owned()
                } else {
                    reachable.join(" && ")
                })?,
                message: "the V2 invariant's admitted state domain is reachable".to_owned(),
            },
        )
    } else {
        (
            "precondition",
            Cover {
                condition: ExprText::new(if direct_boolean {
                    "outcome".to_owned()
                } else {
                    format!(
                        "matches!(outcome, {})",
                        outcome_pattern(lowered.native_integer, OutcomeKind::CompletedTrue)
                    )
                })?,
                message: "the V2 precondition is satisfiable within admitted input bounds"
                    .to_owned(),
            },
        )
    };
    let path = harness_path(&lowered.module_symbol, &lowered.harness_symbol)?;
    if !direct_boolean {
        body.extend(forget_fat_values(lowered.native_integer));
    }
    let spec = HarnessSpec::new(
        format!("V2 {label} {}", message_safe_node_label(&lowered.node_id)),
        path,
        lowered.oracle.source.clone(),
        ProofKind::Proof,
        body,
        vec![cover],
        request.unwind,
        KaniSolver::Cadical,
    )?;
    let rust = render::render(&spec)?;
    let identity = V2ClauseObligationIdentity {
        dependencies: lowered.dependencies.clone(),
        readiness: dependency_readiness(&lowered.dependencies),
        proof_execution_state: ProofExecutionState::NotRun,
        integer_representation: if lowered.native_integer {
            crate::kani::identity::ScalarIntegerRepresentation::Native
        } else {
            crate::kani::identity::ScalarIntegerRepresentation::General
        },
        native_integer_selection: lowered.oracle.native_integer_selection.clone(),
        ceilings: request.ceilings,
        package: lowered.package.clone(),
        clause: lowered.node_id.clone(),
        collection_precondition: None,
        collection_operation: None,
        kind: lowered.kind,
        requires: Vec::new(),
        oracle_symbol: "oracle_clause".to_owned(),
        subject_path: request.subject_path.to_owned(),
        arguments,
        state_fields,
        module_symbol: spec.path().module.clone(),
        harness_symbol: spec.path().harness.clone(),
        solver: spec.solver(),
        unwind: spec.unwind(),
        options: spec.options().to_vec(),
        witness_options: spec.witness_options().to_vec(),
    };
    let record = record(&lowered.module_symbol, &identity, &rust)?;
    Ok(KaniV2ClauseObligationHarness::generated(
        identity,
        lowered.obligation.clone(),
        rust,
        record,
    ))
}

fn expression(text: impl Into<String>) -> Result<ExprText, UnsupportedObligation> {
    ExprText::new(text).map_err(Into::into)
}

/// A generated harness never drops a fat runtime value. The meter and a general oracle's
/// outcome hold symbolic-discriminant sum types whose drop glue the proof would otherwise
/// execute. The native oracle's `Result<bool, ClauseStop>` is `Copy` and has none.
fn forget_fat_values(native: bool) -> Vec<Statement> {
    let mut names = vec!["meter"];
    if !native {
        names.push("outcome");
    }
    names
        .into_iter()
        .map(|name| Statement::Forget(name.to_owned()))
        .collect()
}

fn binding(name: &str, init: Init, mutable: bool) -> Statement {
    Statement::Let(LetBinding {
        name: name.to_owned(),
        ty: None,
        mutable,
        init,
    })
}

fn native_expected_expression(text: &str) -> Result<String, UnsupportedObligation> {
    let mut expected =
        syn::parse_str::<syn::Expr>(text).map_err(|_| UnsupportedObligation::RenderFailed)?;
    while let syn::Expr::Paren(parenthesized) = expected {
        expected = *parenthesized.expr;
    }
    Ok(expected.into_token_stream().to_string())
}

fn i128_literal(value: i128) -> String {
    if value == i128::MIN {
        "i128::MIN".to_owned()
    } else {
        format!("{value}_i128")
    }
}

fn render_result_postcondition(
    request: &KaniObligationRequest<'_>,
    lowered: &LoweredV2Clause,
    signature: &ResultPostcondition,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    let refusal = || UnsupportedObligation::V2ClauseSignatureMismatch {
        node_id: lowered.node_id.clone(),
    };
    let mut body = Vec::new();
    let mut arguments = Vec::new();
    let mut oracle_bindings = BTreeSet::new();
    let mut drawn_parameters = signature.parameters.iter().collect::<Vec<_>>();
    drawn_parameters.sort_by(|left, right| left.name.cmp(&right.name));
    for parameter in drawn_parameters {
        let oracle_name = format!("parameter_{}", parameter.name);
        let symbolic = format!("{oracle_name}_current");
        if syn::parse_str::<syn::Ident>(&symbolic).is_err() {
            return Err(refusal());
        }
        if let MachineScalar::Option { leaves } = &parameter.scalar {
            draw_option(
                lowered.native_integer,
                &parameter.name,
                leaves,
                &mut body,
                &mut arguments,
                &mut oracle_bindings,
            )?;
            continue;
        }
        let (ty, domain, then) = match &parameter.scalar {
            MachineScalar::Option { .. } => return Err(refusal()),
            MachineScalar::Boolean => (
                KaniPrimitiveType::Boolean,
                (0, 1),
                vec![LetBinding {
                    name: oracle_name.clone(),
                    ty: None,
                    mutable: false,
                    init: Init::Expression(expression(&symbolic)?),
                }],
            ),
            MachineScalar::Integer { minimum, maximum } => (
                KaniPrimitiveType::I64,
                (i128::from(*minimum), i128::from(*maximum)),
                vec![LetBinding {
                    name: oracle_name.clone(),
                    ty: None,
                    mutable: false,
                    init: Init::Expression(expression(if lowered.native_integer {
                        format!("nx::NativeInteger::new(i128::from({symbolic}))")
                    } else {
                        format!("rt::Integer::from({symbolic})")
                    })?),
                }],
            ),
        };
        body.push(Statement::Input {
            name: symbolic.clone(),
            ty,
            domain,
            then,
        });
        arguments.push(ScalarObligationArgument {
            identifier: symbolic.clone(),
            minimum: i64::try_from(domain.0).map_err(|_| refusal())?,
            maximum: i64::try_from(domain.1).map_err(|_| refusal())?,
        });
        oracle_bindings.insert(oracle_name);
    }
    let subject_arguments = signature
        .parameters
        .iter()
        .flat_map(|parameter| match &parameter.scalar {
            MachineScalar::Option { leaves } => option_symbolics(&parameter.name, leaves)
                .into_iter()
                .map(|(_, symbolic)| symbolic)
                .collect::<Vec<_>>(),
            _ => vec![format!("parameter_{}_current", parameter.name)],
        })
        .collect::<Vec<_>>();
    let (result_type, result_domain) = match signature.result {
        Some(MachineScalar::Boolean) => (LetType::Boolean, None),
        Some(MachineScalar::Integer { minimum, maximum }) => {
            (LetType::I64, Some((minimum, maximum)))
        }
        Some(MachineScalar::Option { .. }) | None => return Err(refusal()),
    };
    if let (true, Some(expectation)) = (lowered.native_integer, &lowered.native_expectation) {
        body.push(binding(
            "expected",
            Init::Expression(expression(native_expected_expression(expectation)?)?),
            false,
        ));
    }
    body.push(Statement::Let(LetBinding {
        name: "subject_result".to_owned(),
        ty: Some(result_type),
        mutable: false,
        init: Init::Expression(expression(format!(
            "{}({})",
            request.subject_path,
            subject_arguments.join(", ")
        ))?),
    }));
    if lowered
        .oracle
        .descriptor
        .inputs
        .iter()
        .any(|input| input.name == "result_result")
    {
        body.push(binding(
            "result_result",
            Init::Expression(expression(match signature.result {
                Some(MachineScalar::Boolean) => "subject_result".to_owned(),
                Some(MachineScalar::Integer { .. }) if lowered.native_integer => {
                    "nx::NativeInteger::new(i128::from(subject_result))".to_owned()
                }
                Some(MachineScalar::Integer { .. }) => {
                    "rt::Integer::from(subject_result)".to_owned()
                }
                Some(MachineScalar::Option { .. }) | None => return Err(refusal()),
            })?),
            false,
        ));
        oracle_bindings.insert("result_result".to_owned());
    }
    if lowered
        .oracle
        .descriptor
        .inputs
        .iter()
        .any(|input| !oracle_bindings.contains(&input.name))
    {
        return Err(refusal());
    }
    let direct_boolean = lowered.oracle.descriptor.operation_identities.is_empty();
    if !direct_boolean {
        body.push(binding(
            "meter",
            Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
            true,
        ));
    }
    if let Some((minimum, maximum)) = result_domain {
        body.push(Statement::ProducedResultDomainAssert {
            condition: expression(format!(
                "subject_result >= {} && subject_result <= {}",
                crate::kani::abi::i64_literal(minimum),
                crate::kani::abi::i64_literal(maximum)
            ))?,
            message: "the produced V2 result lies in its admitted domain".to_owned(),
        });
    }
    let mut oracle_arguments = lowered
        .oracle
        .descriptor
        .inputs
        .iter()
        .map(|input| {
            if input.boolean {
                Argument::Value(input.name.clone())
            } else {
                Argument::Borrow(input.name.clone())
            }
        })
        .collect::<Vec<_>>();
    if !direct_boolean {
        oracle_arguments.push(Argument::BorrowMut("meter".to_owned()));
    }
    body.push(Statement::Call {
        result: "outcome".to_owned(),
        callee: "oracle_clause".to_owned(),
        args: oracle_arguments,
    });
    if lowered.native_integer && lowered.native_expectation.is_some() {
        let completed_prefix = append_arithmetic_probes(&mut body, &lowered.arithmetic)?;
        body.push(Statement::Assert {
            condition: expression(format!(
                "!({completed_prefix}) || subject_result == expected"
            ))?,
            message: "the produced V2 Boolean result agrees with the independent checked condition"
                .to_owned(),
        });
    }
    body.push(Statement::Assert {
        condition: expression(if direct_boolean {
            "outcome".to_owned()
        } else {
            format!(
                "matches!(&outcome, {})",
                outcome_pattern(lowered.native_integer, OutcomeKind::CompletedTrue)
            )
        })?,
        message: "the V2 postcondition holds for the produced result".to_owned(),
    });
    let path = harness_path(&lowered.module_symbol, &lowered.harness_symbol)?;
    if !direct_boolean {
        body.extend(forget_fat_values(lowered.native_integer));
    }
    let spec = HarnessSpec::new(
        format!(
            "V2 postcondition {}",
            message_safe_node_label(&lowered.node_id)
        ),
        path,
        lowered.oracle.source.clone(),
        ProofKind::Proof,
        body,
        vec![Cover {
            condition: expression(if direct_boolean {
                "outcome".to_owned()
            } else {
                format!(
                    "matches!(&outcome, {})",
                    outcome_pattern(lowered.native_integer, OutcomeKind::Completed)
                )
            })?,
            message: "the V2 result postcondition reaches its admitted domain".to_owned(),
        }],
        request.unwind,
        KaniSolver::Cadical,
    )?;
    let rust = render::render(&spec)?;
    let identity = V2ClauseObligationIdentity {
        dependencies: lowered.dependencies.clone(),
        readiness: dependency_readiness(&lowered.dependencies),
        proof_execution_state: ProofExecutionState::NotRun,
        integer_representation: if lowered.native_integer {
            crate::kani::identity::ScalarIntegerRepresentation::Native
        } else {
            crate::kani::identity::ScalarIntegerRepresentation::General
        },
        native_integer_selection: lowered.oracle.native_integer_selection.clone(),
        ceilings: request.ceilings,
        package: lowered.package.clone(),
        clause: lowered.node_id.clone(),
        collection_precondition: None,
        collection_operation: None,
        kind: ObligationKind::Postcondition,
        requires: Vec::new(),
        oracle_symbol: "oracle_clause".to_owned(),
        subject_path: request.subject_path.to_owned(),
        arguments,
        state_fields: Vec::new(),
        module_symbol: spec.path().module.clone(),
        harness_symbol: spec.path().harness.clone(),
        solver: spec.solver(),
        unwind: spec.unwind(),
        options: spec.options().to_vec(),
        witness_options: spec.witness_options().to_vec(),
    };
    let record = record(&lowered.module_symbol, &identity, &rust)?;
    Ok(KaniV2ClauseObligationHarness::generated(
        identity,
        lowered.obligation.clone(),
        rust,
        record,
    ))
}

/// A postcondition over state: the harness draws the pre-state of each state field and each
/// parameter within their declared domains, assumes every sibling precondition, copies each field
/// into a harness-owned value, calls the subject with `&mut` to that value and then the
/// parameters, asserts that each produced field lies in its domain, and asserts the clause over
/// the pre-state copy and the produced state. The subject returns `()`.
fn render_state_postcondition(
    request: &KaniObligationRequest<'_>,
    lowered: &LoweredV2Clause,
    state: &StatePostcondition,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    let refusal = || UnsupportedObligation::V2ClauseConditionNotRendered {
        node_id: lowered.node_id.clone(),
    };
    let expectation = lowered.native_expectation.as_deref().ok_or_else(refusal)?;
    if !lowered.native_integer {
        return Err(refusal());
    }
    let mut body = Vec::new();
    let mut arguments = Vec::new();
    let mut state_fields = Vec::new();
    for read in &state.reads {
        body.push(Statement::Input {
            name: read.pre.clone(),
            ty: KaniPrimitiveType::I64,
            domain: (i128::from(read.minimum), i128::from(read.maximum)),
            then: Vec::new(),
        });
        state_fields.push(V2ClauseStateField {
            declaring_node_id: read.declaring_node_id.clone(),
            field_name: read.field_name.clone(),
            identifier: read.pre.clone(),
            minimum: read.minimum,
            maximum: read.maximum,
        });
    }
    for parameter in &state.parameters {
        body.push(Statement::Input {
            name: parameter.drawn.clone(),
            ty: KaniPrimitiveType::I64,
            domain: (i128::from(parameter.minimum), i128::from(parameter.maximum)),
            then: Vec::new(),
        });
        arguments.push(ScalarObligationArgument {
            identifier: parameter.drawn.clone(),
            minimum: parameter.minimum,
            maximum: parameter.maximum,
        });
    }
    let mut sibling_meters = Vec::new();
    for (index, require) in state.requires.iter().enumerate() {
        let direct = require.oracle.descriptor.operation_identities.is_empty();
        let mut call_arguments = Vec::new();
        for (name, source) in &require.inputs {
            let local = format!("requires_{index}_{name}");
            body.push(binding(
                &local,
                Init::Expression(expression(format!(
                    "nx::NativeInteger::new(i128::from({source}))"
                ))?),
                false,
            ));
            call_arguments.push(format!("&{local}"));
        }
        if !direct {
            let meter = format!("requires_{index}_meter");
            body.push(binding(
                &meter,
                Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
                true,
            ));
            call_arguments.push(format!("&mut {meter}"));
            sibling_meters.push(meter);
        }
        let outcome = format!("requires_{index}_outcome");
        body.push(binding(
            &outcome,
            Init::Expression(expression(format!(
                "requires_{index}::oracle_clause({})",
                call_arguments.join(", ")
            ))?),
            false,
        ));
        body.push(Statement::Assume {
            condition: expression(if direct {
                outcome
            } else {
                format!(
                    "matches!(&{outcome}, {})",
                    outcome_pattern(true, OutcomeKind::CompletedTrue)
                )
            })?,
        });
    }
    for read in &state.reads {
        body.push(Statement::Let(LetBinding {
            name: read.post.clone(),
            ty: Some(LetType::I64),
            mutable: true,
            init: Init::Expression(expression(&read.pre)?),
        }));
    }
    let subject_arguments = state
        .reads
        .iter()
        .map(|read| format!("&mut {}", read.post))
        .chain(
            state
                .parameters
                .iter()
                .map(|parameter| parameter.drawn.clone()),
        )
        .collect::<Vec<_>>();
    body.push(Statement::Let(LetBinding {
        name: "subject".to_owned(),
        ty: Some(LetType::Unit),
        mutable: false,
        init: Init::Expression(expression(format!(
            "{}({})",
            request.subject_path,
            subject_arguments.join(", ")
        ))?),
    }));
    for read in &state.reads {
        body.push(Statement::ProducedResultDomainAssert {
            condition: expression(format!(
                "{post} >= {} && {post} <= {}",
                crate::kani::abi::i64_literal(read.minimum),
                crate::kani::abi::i64_literal(read.maximum),
                post = read.post
            ))?,
            message: "the produced V2 state field lies in its admitted domain".to_owned(),
        });
    }
    // The expectation reads the produced state, so it follows the call.
    body.push(binding(
        "expected",
        Init::Expression(expression(native_expected_expression(expectation)?)?),
        false,
    ));
    for (name, source) in &state.oracle_inputs {
        body.push(binding(
            name,
            Init::Expression(expression(format!(
                "nx::NativeInteger::new(i128::from({source}))"
            ))?),
            false,
        ));
    }
    body.push(binding(
        "meter",
        Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
        true,
    ));
    let mut oracle_arguments = state
        .oracle_inputs
        .iter()
        .map(|(name, _)| Argument::Borrow(name.clone()))
        .collect::<Vec<_>>();
    oracle_arguments.push(Argument::BorrowMut("meter".to_owned()));
    body.push(Statement::Call {
        result: "outcome".to_owned(),
        callee: "oracle_clause".to_owned(),
        args: oracle_arguments,
    });
    let completed_prefix = append_arithmetic_probes(&mut body, &lowered.arithmetic)?;
    body.extend([
        Statement::Assert {
            condition: expression(format!(
                "(({completed_prefix}) && matches!(&outcome, {completed})) || (!({completed_prefix}) && matches!(&outcome, {refused}))",
                completed = outcome_pattern(true, OutcomeKind::Completed),
                refused = outcome_pattern(true, OutcomeKind::OutOfDomain),
            ))?,
            message: "the clause oracle preserves completed and refused semantic domains".to_owned(),
        },
        Statement::Assert {
            condition: expression(format!(
                "!({completed_prefix}) || (matches!(&outcome, {completed_true}) == expected)",
                completed_true = outcome_pattern(true, OutcomeKind::CompletedTrue),
            ))?,
            message: "the clause oracle agrees with the independent checked condition".to_owned(),
        },
        Statement::Assert {
            condition: expression(format!(
                "matches!(&outcome, {})",
                outcome_pattern(true, OutcomeKind::CompletedTrue)
            ))?,
            message: "the V2 postcondition holds for the produced state".to_owned(),
        },
    ]);
    body.extend(forget_fat_values(true));
    body.extend(sibling_meters.into_iter().map(Statement::Forget));
    // Each sibling oracle sits in its own module: the oracles share item names.
    let mut oracle_source = lowered.oracle.source.clone();
    for (index, require) in state.requires.iter().enumerate() {
        let _ = write!(
            &mut oracle_source,
            "\nmod requires_{index} {{\n{}\n}}\n",
            require.oracle.source
        );
    }
    let spec = HarnessSpec::new(
        format!(
            "V2 postcondition {}",
            message_safe_node_label(&lowered.node_id)
        ),
        harness_path(&lowered.module_symbol, &lowered.harness_symbol)?,
        oracle_source,
        ProofKind::Proof,
        body,
        vec![Cover {
            condition: expression(format!(
                "({completed_prefix}) && matches!(&outcome, {completed})",
                completed = outcome_pattern(true, OutcomeKind::Completed),
            ))?,
            message: "the V2 postcondition is evaluated on an admitted state and parameters"
                .to_owned(),
        }],
        request.unwind,
        KaniSolver::Cadical,
    )?;
    let rust = render::render(&spec)?;
    let identity = V2ClauseObligationIdentity {
        dependencies: lowered.dependencies.clone(),
        readiness: dependency_readiness(&lowered.dependencies),
        proof_execution_state: ProofExecutionState::NotRun,
        integer_representation: crate::kani::identity::ScalarIntegerRepresentation::Native,
        native_integer_selection: lowered.oracle.native_integer_selection.clone(),
        ceilings: request.ceilings,
        package: lowered.package.clone(),
        clause: lowered.node_id.clone(),
        collection_precondition: None,
        collection_operation: None,
        kind: ObligationKind::Postcondition,
        requires: state
            .requires
            .iter()
            .map(|require| require.precondition.clone())
            .collect(),
        oracle_symbol: "oracle_clause".to_owned(),
        subject_path: request.subject_path.to_owned(),
        arguments,
        state_fields,
        module_symbol: spec.path().module.clone(),
        harness_symbol: spec.path().harness.clone(),
        solver: spec.solver(),
        unwind: spec.unwind(),
        options: spec.options().to_vec(),
        witness_options: spec.witness_options().to_vec(),
    };
    let record = record(&lowered.module_symbol, &identity, &rust)?;
    Ok(KaniV2ClauseObligationHarness::generated(
        identity,
        lowered.obligation.clone(),
        rust,
        record,
    ))
}

fn append_arithmetic_probes(
    body: &mut Vec<Statement>,
    arithmetic: &[ArithmeticProbe],
) -> Result<String, UnsupportedObligation> {
    let mut completed_prefix = "true".to_owned();
    for (index, probe) in arithmetic.iter().enumerate() {
        if probe.operation == DEFINED_PROBE {
            completed_prefix = format!(
                "({completed_prefix}) && (!({}) || ({}))",
                probe.guard, probe.expected
            );
            continue;
        }
        let guard = format!("({}) && ({completed_prefix})", probe.guard);
        let mut probe_body = Vec::new();
        let expected = native_expected_expression(&probe.expected)?;
        let in_domain = probe.semantic_domain.map_or_else(
            || "true".to_owned(),
            |(lower, upper)| {
                format!(
                    "({expected} >= {} && {expected} <= {})",
                    i128_literal(lower),
                    i128_literal(upper)
                )
            },
        );

        let mut operands = Vec::new();
        let mut intervals = Vec::new();
        for (position, (operand, (lower, upper))) in
            probe.operands.iter().zip(&probe.operand_bounds).enumerate()
        {
            let value = format!("arithmetic_operand_{index}_{position}");
            probe_body.push(binding(
                &value,
                Init::Expression(expression(format!(
                    "nx::NativeInteger::new(i128::from({operand}))"
                ))?),
                false,
            ));
            operands.push(value);
            let interval = format!("arithmetic_interval_{index}_{position}");
            probe_body.push(binding(
                &interval,
                Init::NativeAdmission {
                    checked: expression(format!(
                        "nx::NativeIntegerInterval::new({}, {})",
                        i128_literal(*lower),
                        i128_literal(*upper)
                    ))?,
                    stage: NativeAdmissionStage::OperandInterval,
                },
                false,
            ));
            intervals.push(interval);
        }
        let (descriptor, payload) = if probe.operation == "Negate" {
            let ([operand], [interval]) = (operands.as_slice(), intervals.as_slice()) else {
                return Err(UnsupportedObligation::RenderFailed);
            };
            (
                format!("nx::NativeIntegerOperation::Negate {{ operand: {interval} }}"),
                format!("nx::NativeIntegerOperands::Unary {{ operand: {operand} }}"),
            )
        } else {
            let ([left, right], [left_interval, right_interval]) =
                (operands.as_slice(), intervals.as_slice())
            else {
                return Err(UnsupportedObligation::RenderFailed);
            };
            (
                format!(
                    "nx::NativeIntegerOperation::{} {{ left: {left_interval}, right: {right_interval} }}",
                    probe.operation
                ),
                format!("nx::NativeIntegerOperands::Binary {{ left: {left}, right: {right} }}"),
            )
        };
        let domain = probe.semantic_domain.map_or_else(
            || "None".to_owned(),
            |(lower, upper)| {
                format!(
                    "Some(nx::NativeIntegerInterval::new({}, {}).expect(\"selected semantic domain\"))",
                    i128_literal(lower),
                    i128_literal(upper)
                )
            },
        );
        let plan = format!("arithmetic_plan_{index}");
        probe_body.push(binding(
            &plan,
            Init::NativeAdmission {
                checked: expression(format!(
                    "nx::NativeIntegerPlan::new({descriptor}, {domain})"
                ))?,
                stage: NativeAdmissionStage::PlanCapacity,
            },
            false,
        ));
        let admitted = format!("arithmetic_admitted_{index}");
        probe_body.push(binding(
            &admitted,
            Init::NativeAdmission {
                checked: expression(format!("{plan}.admit({payload})"))?,
                stage: NativeAdmissionStage::OperandPopulation,
            },
            false,
        ));
        let meter = format!("arithmetic_meter_{index}");
        probe_body.push(binding(
            &meter,
            Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
            true,
        ));
        let outcome = format!("arithmetic_outcome_{index}");
        probe_body.push(binding(
            &outcome,
            Init::Expression(expression(format!(
                "nx::evaluate_native_integer_arithmetic({admitted}, &mut {meter})"
            ))?),
            false,
        ));
        probe_body.push(Statement::Assert {
            condition: expression(format!(
                "match &{outcome} {{ rt::Outcome::Completed(value) => ({in_domain}) && value.value() == {expected}, rt::Outcome::Refused(rt::Refusal::IntegerOutOfDomain {{ .. }}) => !({in_domain}), _ => false }}"
            ))?,
            message: "arithmetic subterm agrees with independent i128 expectation".to_owned(),
        });
        probe_body.push(Statement::Forget(outcome));
        probe_body.push(Statement::Forget(meter));
        body.push(Statement::ConditionalProbe {
            condition: expression(guard)?,
            body: probe_body,
        });
        completed_prefix = format!(
            "({completed_prefix}) && (!({}) || ({in_domain}))",
            probe.guard
        );
    }
    Ok(completed_prefix)
}

fn render_postcondition(
    request: &KaniObligationRequest<'_>,
    lowered: &LoweredV2Clause,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    let [input] = lowered.oracle.descriptor.inputs.as_slice() else {
        return Err(UnsupportedObligation::RenderFailed);
    };
    let symbolic = lowered
        .symbolic_name
        .as_deref()
        .ok_or(UnsupportedObligation::RenderFailed)?;
    let expectation = lowered
        .native_expectation
        .as_deref()
        .ok_or(UnsupportedObligation::RenderFailed)?;
    let minimum = i64::try_from(input.lower).map_err(|_| UnsupportedObligation::RenderFailed)?;
    let maximum = i64::try_from(input.upper).map_err(|_| UnsupportedObligation::RenderFailed)?;
    let mut body = vec![
        Statement::Input {
            name: symbolic.to_owned(),
            ty: KaniPrimitiveType::I64,
            domain: (input.lower, input.upper),
            then: vec![LetBinding {
                name: input.name.clone(),
                ty: None,
                mutable: false,
                init: Init::Expression(expression(format!(
                    "nx::NativeInteger::new(i128::from({symbolic}))"
                ))?),
            }],
        },
        binding(
            "expected",
            Init::Expression(expression(native_expected_expression(expectation)?)?),
            false,
        ),
        binding(
            "meter",
            Init::ScalarMeter(crate::kani::abi::SCALAR_HARNESS_LIMITS),
            true,
        ),
        Statement::Call {
            result: "outcome".to_owned(),
            callee: "oracle_clause".to_owned(),
            args: vec![
                Argument::Borrow(input.name.clone()),
                Argument::BorrowMut("meter".to_owned()),
            ],
        },
        Statement::Let(LetBinding {
            name: "subject".to_owned(),
            ty: Some(LetType::Unit),
            mutable: false,
            init: Init::Expression(expression(format!("{}({symbolic})", request.subject_path))?),
        }),
    ];
    let completed_prefix = append_arithmetic_probes(&mut body, &lowered.arithmetic)?;
    body.extend([
        Statement::Assert {
            condition: expression(format!(
                "(({completed_prefix}) && matches!(&outcome, {completed})) || (!({completed_prefix}) && matches!(&outcome, {refused}))",
                completed = outcome_pattern(lowered.native_integer, OutcomeKind::Completed),
                refused = outcome_pattern(lowered.native_integer, OutcomeKind::OutOfDomain),
            ))?,
            message: "the clause oracle preserves completed and refused semantic domains".to_owned(),
        },
        Statement::Assert {
            condition: expression(format!(
                "!({completed_prefix}) || (matches!(&outcome, {completed_true}) == expected)",
                completed_true = outcome_pattern(lowered.native_integer, OutcomeKind::CompletedTrue),
            ))?,
            message: "the clause oracle agrees with the independent checked condition".to_owned(),
        },
    ]);
    body.extend(forget_fat_values(lowered.native_integer));
    let spec = HarnessSpec::new(
        format!(
            "V2 postcondition {}",
            message_safe_node_label(&lowered.node_id)
        ),
        harness_path(&lowered.module_symbol, &lowered.harness_symbol)?,
        lowered.oracle.source.clone(),
        ProofKind::Proof,
        body,
        vec![Cover {
            condition: expression(format!(
                "({completed_prefix}) && matches!(&outcome, {completed})",
                completed = outcome_pattern(lowered.native_integer, OutcomeKind::Completed),
            ))?,
            message: "the original V2 condition completes on an admitted parameter".to_owned(),
        }],
        request.unwind,
        KaniSolver::Cadical,
    )?;
    let rust = render::render(&spec)?;
    let identity = V2ClauseObligationIdentity {
        dependencies: lowered.dependencies.clone(),
        readiness: dependency_readiness(&lowered.dependencies),
        proof_execution_state: ProofExecutionState::NotRun,
        integer_representation: crate::kani::identity::ScalarIntegerRepresentation::Native,
        native_integer_selection: lowered.oracle.native_integer_selection.clone(),
        ceilings: request.ceilings,
        package: lowered.package.clone(),
        clause: lowered.node_id.clone(),
        collection_precondition: None,
        collection_operation: None,
        kind: ObligationKind::Postcondition,
        requires: Vec::new(),
        oracle_symbol: "oracle_clause".to_owned(),
        subject_path: request.subject_path.to_owned(),
        arguments: vec![ScalarObligationArgument {
            identifier: symbolic.to_owned(),
            minimum,
            maximum,
        }],
        state_fields: Vec::new(),
        module_symbol: spec.path().module.clone(),
        harness_symbol: spec.path().harness.clone(),
        solver: spec.solver(),
        unwind: spec.unwind(),
        options: spec.options().to_vec(),
        witness_options: spec.witness_options().to_vec(),
    };
    let record = record(&lowered.module_symbol, &identity, &rust)?;
    Ok(KaniV2ClauseObligationHarness::generated(
        identity,
        lowered.obligation.clone(),
        rust,
        record,
    ))
}
#[derive(Clone)]
enum NativeTerm {
    Integer {
        expression: String,
        lower: i128,
        upper: i128,
        small_expression: Option<String>,
    },
    Boolean(String),
}

/// The `operation` of a probe that is not arithmetic: its `expected` text is the condition under
/// which an option read is defined (the presence draw), and it runs no kernel operation.
const DEFINED_PROBE: &str = "Defined";

struct ArithmeticProbe {
    operation_identity: String,
    operation: &'static str,
    operands: Vec<String>,
    expected: String,
    operand_bounds: Vec<(i128, i128)>,
    semantic_domain: Option<(i128, i128)>,
    guard: String,
}

fn bind_native_arithmetic_probes(
    probes: &mut [ArithmeticProbe],
    selection: &crate::core::native_integer::NativeIntegerSelection,
) -> Result<(), UnsupportedObligation> {
    let decisions = selection
        .operations
        .iter()
        .filter(|decision| decision.capacity.is_some())
        .collect::<Vec<_>>();
    let mut arithmetic = probes
        .iter_mut()
        .filter(|probe| probe.operation != DEFINED_PROBE)
        .collect::<Vec<_>>();
    if decisions.len() != arithmetic.len() {
        return Err(UnsupportedObligation::RenderFailed);
    }
    for (probe, decision) in arithmetic.iter_mut().zip(decisions) {
        if probe.operation_identity != decision.operation_identity {
            return Err(UnsupportedObligation::RenderFailed);
        }
        probe.operand_bounds = decision
            .operands
            .iter()
            .map(|interval| {
                Ok::<_, UnsupportedObligation>((
                    interval
                        .lower
                        .parse()
                        .map_err(|_| UnsupportedObligation::RenderFailed)?,
                    interval
                        .upper
                        .parse()
                        .map_err(|_| UnsupportedObligation::RenderFailed)?,
                ))
            })
            .collect::<Result<Vec<_>, UnsupportedObligation>>()?;
        probe.semantic_domain = decision
            .semantic_domain
            .as_ref()
            .map(|interval| {
                Ok::<_, UnsupportedObligation>((
                    interval
                        .lower
                        .parse()
                        .map_err(|_| UnsupportedObligation::RenderFailed)?,
                    interval
                        .upper
                        .parse()
                        .map_err(|_| UnsupportedObligation::RenderFailed)?,
                ))
            })
            .transpose()?;
    }
    Ok(())
}

/// One native selection: a `match` over a Boolean condition (`true` and `false`) or over a member
/// index (one pattern per arm, the last being the exhaustive pattern). Both branch values are of
/// one family; an Integer selection holds the hull of its branches.
fn select_native<const N: usize>(
    head: String,
    arms: [(String, NativeTerm); N],
) -> Option<NativeTerm> {
    select_native_vec(head, arms.to_vec())
}

fn select_native_vec(head: String, arms: Vec<(String, NativeTerm)>) -> Option<NativeTerm> {
    let compose = |values: Vec<String>, patterns: &[String]| -> String {
        let body = values
            .iter()
            .zip(patterns)
            .map(|(value, pattern)| format!("{pattern} => {value},"))
            .collect::<Vec<_>>()
            .join(" ");
        format!("({head} {{ {body} }})")
    };
    let patterns = arms
        .iter()
        .map(|(pattern, _)| pattern.clone())
        .collect::<Vec<_>>();
    if arms
        .iter()
        .all(|(_, value)| matches!(value, NativeTerm::Boolean(_)))
    {
        let values = arms
            .iter()
            .map(|(_, value)| value.clone_boolean())
            .collect::<Option<Vec<_>>>()?;
        return Some(NativeTerm::Boolean(compose(values, &patterns)));
    }
    let integers = arms
        .iter()
        .map(|(_, value)| value.clone_integer())
        .collect::<Option<Vec<_>>>()?;
    let lower = integers.iter().map(|(_, lower, _, _)| *lower).min()?;
    let upper = integers.iter().map(|(_, _, upper, _)| *upper).max()?;
    let small_values = integers
        .iter()
        .map(|(_, _, _, small)| small.clone())
        .collect::<Option<Vec<_>>>();
    let small_expression = small_values
        .filter(|_| i64::try_from(lower).is_ok() && i64::try_from(upper).is_ok())
        .map(|values| compose(values, &patterns));
    let values = integers
        .into_iter()
        .map(|(expression, _, _, _)| expression)
        .collect();
    Some(NativeTerm::Integer {
        expression: compose(values, &patterns),
        lower,
        upper,
        small_expression,
    })
}

impl NativeTerm {
    fn boolean(self) -> Option<String> {
        match self {
            Self::Boolean(expression) => Some(expression),
            Self::Integer { .. } => None,
        }
    }
}

/// One clause input the independent native expectation reads, and the variable holding its value.
struct NativeBinding<'a> {
    side: ClauseInputSide,
    name: &'a str,
    leaf: ClauseLeaf,
    symbolic: &'a str,
}

impl<'a> NativeBinding<'a> {
    fn parameter(name: &'a str, symbolic: &'a str) -> Self {
        Self {
            side: ClauseInputSide::Parameter,
            name,
            leaf: ClauseLeaf::Whole,
            symbolic,
        }
    }
}

/// The independent `i128` expectation of a clause term. `scope` holds the value each enclosing `let`
/// binder was bound to: a read of the binder substitutes that value's expression, which was probed
/// once where the `let` initializer evaluated. An `if` or `case` guards the probes of each branch or
/// arm by its selection, so a probe runs only where the oracle evaluates that node.
fn native_term(
    term: &ClauseTerm,
    bindings: &[NativeBinding<'_>],
    scope: &[(usize, NativeTerm)],
    arithmetic: &mut Vec<ArithmeticProbe>,
) -> Option<NativeTerm> {
    match term {
        ClauseTerm::Binder { binder } => scope
            .iter()
            .rev()
            .find_map(|(bound, value)| (bound == binder).then(|| value.clone())),
        ClauseTerm::Let {
            binder,
            value,
            body,
        } => {
            let value = native_term(value, bindings, scope, arithmetic)?;
            let mut inner = scope.to_vec();
            inner.push((*binder, value));
            native_term(body, bindings, &inner, arithmetic)
        }
        ClauseTerm::If {
            condition,
            then,
            other,
        } => {
            let selector = native_term(condition, bindings, scope, arithmetic)?.clone_boolean()?;
            let first_then = arithmetic.len();
            let then = native_term(then, bindings, scope, arithmetic)?;
            let first_other = arithmetic.len();
            let other = native_term(other, bindings, scope, arithmetic)?;
            for probe in &mut arithmetic[first_then..first_other] {
                probe.guard = format!("({}) && ({selector})", probe.guard);
            }
            for probe in &mut arithmetic[first_other..] {
                probe.guard = format!("({}) && !({selector})", probe.guard);
            }
            // The expectation is an expression text that may hold no block, so the selection is a
            // `match` over the condition.
            select_native(
                format!("match {selector}"),
                [("true".to_owned(), then), ("false".to_owned(), other)],
            )
        }
        ClauseTerm::Case { scrutinee, arms } => {
            let selector = bindings
                .iter()
                .find(|binding| {
                    binding.side == ClauseInputSide::Parameter && binding.name == scrutinee.name
                })?
                .symbolic;
            let mut selected = Vec::with_capacity(arms.len());
            for (member, arm) in arms.iter().enumerate() {
                let first = arithmetic.len();
                let value = native_term(arm, bindings, scope, arithmetic)?;
                for probe in &mut arithmetic[first..] {
                    probe.guard = format!("({}) && ({selector} == {member}_i64)", probe.guard);
                }
                selected.push((format!("{member}_i64"), value));
            }
            let (last, leading) = selected.split_last()?;
            let mut arms = leading.to_vec();
            arms.push(("_".to_owned(), last.1.clone()));
            select_native_vec(format!("match {selector}"), arms)
        }
        ClauseTerm::Boolean(value) => Some(NativeTerm::Boolean(value.to_string())),
        ClauseTerm::Integer(value) => {
            let parsed = value.parse::<i128>().ok()?;
            Some(NativeTerm::Integer {
                expression: i128_literal(parsed),
                lower: parsed,
                upper: parsed,
                small_expression: i64::try_from(parsed)
                    .ok()
                    .map(|value| format!("{value}i64")),
            })
        }
        ClauseTerm::Input(input) => {
            let binding = bindings.iter().find(|binding| {
                binding.side == input.side
                    && binding.name == input.name
                    && binding.leaf == input.leaf
            })?;
            if input.boolean && input.leaf != ClauseLeaf::Whole {
                return Some(NativeTerm::Boolean(binding.symbolic.to_owned()));
            }
            if input.boolean
                || input.lower > input.upper
                || i64::try_from(input.lower).is_err()
                || i64::try_from(input.upper).is_err()
            {
                return None;
            }
            Some(NativeTerm::Integer {
                expression: format!("i128::from({})", binding.symbolic),
                lower: input.lower,
                upper: input.upper,
                small_expression: Some(binding.symbolic.to_owned()),
            })
        }
        ClauseTerm::OptionValue { presence, payload } => {
            let symbolic = |input: &ClauseInput| {
                bindings
                    .iter()
                    .find(|binding| {
                        binding.side == input.side
                            && binding.name == input.name
                            && binding.leaf == input.leaf
                    })
                    .map(|binding| binding.symbolic)
            };
            let presence = symbolic(presence)?;
            let payload_symbol = symbolic(payload)?;
            // The read is defined only where the presence draw is true: a probe whose guard the
            // enclosing short-circuit connectives narrow, as for an arithmetic probe.
            arithmetic.push(ArithmeticProbe {
                operation_identity: OPTION_VALUE.to_owned(),
                operation: DEFINED_PROBE,
                operands: Vec::new(),
                expected: presence.to_owned(),
                operand_bounds: Vec::new(),
                semantic_domain: None,
                guard: "true".to_owned(),
            });
            if payload.boolean {
                Some(NativeTerm::Boolean(payload_symbol.to_owned()))
            } else {
                Some(NativeTerm::Integer {
                    expression: format!("i128::from({payload_symbol})"),
                    lower: payload.lower,
                    upper: payload.upper,
                    small_expression: Some(payload_symbol.to_owned()),
                })
            }
        }
        ClauseTerm::Operation {
            identity,
            arguments,
            ..
        } => {
            if identity == "filament.op.numeric.narrow" {
                let literal = crate::oracle::clause::admitted_literal_narrow(term)?;
                return native_term(literal, bindings, scope, arithmetic);
            }
            if let [left, right] = arguments.as_slice() {
                let gate = match identity.as_str() {
                    "filament.op.boolean.and" | "filament.op.boolean.implies" => Some(false),
                    "filament.op.boolean.or" => Some(true),
                    _ => None,
                };
                if let Some(invert) = gate {
                    let left = native_term(left, bindings, scope, arithmetic)?.clone_boolean()?;
                    let first_right = arithmetic.len();
                    let right = native_term(right, bindings, scope, arithmetic)?.clone_boolean()?;
                    let condition = if invert {
                        format!("!({left})")
                    } else {
                        left.clone()
                    };
                    for probe in &mut arithmetic[first_right..] {
                        probe.guard = format!("({}) && ({condition})", probe.guard);
                    }
                    let expression = match identity.as_str() {
                        "filament.op.boolean.and" => format!("({left} && {right})"),
                        "filament.op.boolean.or" => format!("({left} || {right})"),
                        "filament.op.boolean.implies" => format!("(!{left} || {right})"),
                        _ => return None,
                    };
                    return Some(NativeTerm::Boolean(expression));
                }
            }
            let values = arguments
                .iter()
                .map(|argument| native_term(argument, bindings, scope, arithmetic))
                .collect::<Option<Vec<_>>>()?;
            match (identity.as_str(), values.as_slice()) {
                ("filament.op.boolean.not", [value]) => Some(NativeTerm::Boolean(format!(
                    "(!{})",
                    value.clone_boolean()?
                ))),
                ("filament.op.boolean.and", [left, right]) => Some(NativeTerm::Boolean(format!(
                    "({} && {})",
                    left.clone_boolean()?,
                    right.clone_boolean()?
                ))),
                ("filament.op.boolean.or", [left, right]) => Some(NativeTerm::Boolean(format!(
                    "({} || {})",
                    left.clone_boolean()?,
                    right.clone_boolean()?
                ))),
                ("filament.op.boolean.implies", [left, right]) => Some(NativeTerm::Boolean(
                    format!("(!{} || {})", left.clone_boolean()?, right.clone_boolean()?),
                )),
                ("filament.op.integer.negate", [value]) => {
                    let (expression, lower, upper, small) = value.clone_integer()?;
                    let operand_bounds = vec![(lower, upper)];
                    let expected = format!("(-{expression})");
                    let (lower, upper) = (upper.checked_neg()?, lower.checked_neg()?);
                    let small_expression =
                        if i64::try_from(lower).is_ok() && i64::try_from(upper).is_ok() {
                            small.as_ref().map(|value| format!("(-{value})"))
                        } else {
                            None
                        };
                    let operand = if small_expression.is_some() {
                        small?
                    } else {
                        expression
                    };
                    arithmetic.push(ArithmeticProbe {
                        operation_identity: identity.clone(),
                        operation: "Negate",
                        operand_bounds,
                        operands: vec![operand],
                        expected: expected.clone(),
                        semantic_domain: None,
                        guard: "true".to_owned(),
                    });
                    Some(NativeTerm::Integer {
                        expression: expected,
                        lower,
                        upper,
                        small_expression,
                    })
                }
                (
                    "filament.op.integer.add"
                    | "filament.op.integer.sub"
                    | "filament.op.integer.mul",
                    [left, right],
                ) => {
                    let (a, al, ah, a_small) = left.clone_integer()?;
                    let (b, bl, bh, b_small) = right.clone_integer()?;
                    let (operator, operation, bounds) = match identity.as_str() {
                        "filament.op.integer.add" => {
                            ("+", "Add", (al.checked_add(bl)?, ah.checked_add(bh)?))
                        }
                        "filament.op.integer.sub" => {
                            ("-", "Subtract", (al.checked_sub(bh)?, ah.checked_sub(bl)?))
                        }
                        "filament.op.integer.mul" => {
                            let corners = [
                                al.checked_mul(bl)?,
                                al.checked_mul(bh)?,
                                ah.checked_mul(bl)?,
                                ah.checked_mul(bh)?,
                            ];
                            (
                                "*",
                                "Multiply",
                                (*corners.iter().min()?, *corners.iter().max()?),
                            )
                        }
                        _ => return None,
                    };
                    let expected = format!("({a} {operator} {b})");
                    let small_expression =
                        if i64::try_from(bounds.0).is_ok() && i64::try_from(bounds.1).is_ok() {
                            a_small
                                .as_ref()
                                .zip(b_small.as_ref())
                                .map(|(a, b)| format!("({a} {operator} {b})"))
                        } else {
                            None
                        };
                    let operands = if small_expression.is_some() {
                        vec![a_small?, b_small?]
                    } else {
                        vec![a, b]
                    };
                    arithmetic.push(ArithmeticProbe {
                        operation_identity: identity.clone(),
                        operation,
                        operand_bounds: vec![(al, ah), (bl, bh)],
                        operands,
                        expected: expected.clone(),
                        semantic_domain: None,
                        guard: "true".to_owned(),
                    });
                    Some(NativeTerm::Integer {
                        expression: expected,
                        lower: bounds.0,
                        upper: bounds.1,
                        small_expression,
                    })
                }
                (
                    "filament.op.integer.lt"
                    | "filament.op.integer.le"
                    | "filament.op.integer.gt"
                    | "filament.op.integer.ge",
                    [left, right],
                ) => {
                    let (a, _, _, _) = left.clone_integer()?;
                    let (b, _, _, _) = right.clone_integer()?;
                    let operator = match identity.as_str() {
                        "filament.op.integer.lt" => "<",
                        "filament.op.integer.le" => "<=",
                        "filament.op.integer.gt" => ">",
                        "filament.op.integer.ge" => ">=",
                        _ => return None,
                    };
                    Some(NativeTerm::Boolean(format!("({a} {operator} {b})")))
                }
                ("filament.op.integer.eq" | "filament.op.integer.ne", [left, right]) => {
                    let (a, _, _, _) = left.clone_integer()?;
                    let (b, _, _, _) = right.clone_integer()?;
                    let operator = if identity == "filament.op.integer.eq" {
                        "=="
                    } else {
                        "!="
                    };
                    Some(NativeTerm::Boolean(format!("({a} {operator} {b})")))
                }
                _ => None,
            }
        }
    }
}

impl NativeTerm {
    fn clone_integer(&self) -> Option<(String, i128, i128, Option<String>)> {
        match self {
            Self::Integer {
                expression,
                lower,
                upper,
                small_expression,
            } => Some((expression.clone(), *lower, *upper, small_expression.clone())),
            Self::Boolean(_) => None,
        }
    }

    fn clone_boolean(&self) -> Option<String> {
        match self {
            Self::Boolean(expression) => Some(expression.clone()),
            Self::Integer { .. } => None,
        }
    }
}

#[cfg(test)]
mod unsupported_operator_tests {
    use super::unsupported_clause_operator;
    use crate::core::ir::clause::ClauseTerm;
    use ix_trace_rs::trace;

    #[trace("FR-015-AC-42", "TC-025")]
    #[test]
    fn nested_absolute_value_names_its_exact_operator() {
        let term = ClauseTerm::Operation {
            identity: "filament.op.integer.ge".to_owned(),
            arguments: vec![
                ClauseTerm::Operation {
                    identity: "filament.op.integer.abs".to_owned(),
                    arguments: vec![ClauseTerm::Integer("-2".to_owned())],
                    result_bound: None,
                },
                ClauseTerm::Integer("0".to_owned()),
            ],
            result_bound: None,
        };
        assert_eq!(
            unsupported_clause_operator(&term),
            Some("filament.op.integer.abs")
        );
    }
}

#[cfg(test)]
mod postcondition_signature_tests {
    use super::*;
    use filament_contract_model::CheckedOperationReadErrorCode;
    use ix_trace_rs::trace;

    #[trace("FR-025-AC-11", "TC-036")]
    #[test]
    fn refused_complete_census_keeps_its_original_typed_work_record() {
        let (package, clause) =
            native_twin::Twin::with_bounded_result_postcondition().emitted_package("DepositResult");
        let view = ClauseGraphView::new(&package);
        let refusal = match lower_with_census(
            &view,
            &clause,
            ObligationKind::Postcondition,
            &V2ClauseObligationClaim {
                obligation: crate::kani::test_support::fixture_obligation(),
                record: crate::kani::identity::V2ClauseObligationRecord {
                    occurrence: crate::core::requirement::RequirementOccurrence {
                        node_id: clause.clone(),
                        role: filament_contract_model::CheckedOccurrenceRole::Expression,
                        ordinal: 0,
                    },
                    kind: ObligationKind::Postcondition,
                    parameters: Vec::new(),
                    state_reads: Vec::new(),
                },
            },
            IntegerRoute::General,
            &crate::kani::tractability::TractabilityTable::empty(),
            |p, c| p.model_operations(c, 0),
        ) {
            Err(refusal) => refusal,
            Ok(_) => panic!("a refused complete read cannot expose a lowerable harness"),
        };
        let UnsupportedObligation::V2ClauseOperationRead { node_id, source } = refusal else {
            panic!("the original typed read refusal is retained: {refusal:?}");
        };
        assert_eq!(node_id, clause);
        assert_eq!(source.code, CheckedOperationReadErrorCode::WorkLimit);
        assert_eq!(source.limit, 0);
        assert_eq!(source.work, 1);
        let context = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.semantic_form.as_ref() == "object_type")
            .expect("selected object context");
        assert_eq!(source.context, context.node_id);
    }

    #[trace("FR-025-AC-11", "TC-036")]
    #[test]
    fn every_unavailable_or_ambiguous_read_keeps_its_typed_record() {
        let (package, clause) =
            native_twin::Twin::with_bounded_result_postcondition().emitted_package("DepositResult");
        let view = ClauseGraphView::new(&package);
        let context = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.semantic_form.as_ref() == "object_type")
            .expect("selected object context")
            .node_id
            .clone();
        for code in [
            CheckedOperationReadErrorCode::ForeignContext,
            CheckedOperationReadErrorCode::UnknownNode,
            CheckedOperationReadErrorCode::WrongContextKind,
            CheckedOperationReadErrorCode::AmbiguousOperation,
            CheckedOperationReadErrorCode::MissingDeclaringNode,
            CheckedOperationReadErrorCode::AmbiguousDeclaringNode,
            CheckedOperationReadErrorCode::UnavailableParameterName,
            CheckedOperationReadErrorCode::WorkLimit,
        ] {
            let read_context = context.clone();
            let refusal = match read_postcondition_signature(&view, &clause, move |_, _| {
                Err(filament_contract_model::CheckedOperationReadError {
                    code,
                    context: read_context,
                    work: 7,
                    limit: 65_536,
                })
            }) {
                Err(refusal) => refusal,
                Ok(_) => panic!("{code:?} read exposed a signature"),
            };
            let UnsupportedObligation::V2ClauseOperationRead { node_id, source } = refusal else {
                panic!("{code:?}: the typed read refusal is retained");
            };
            assert_eq!(node_id, clause);
            assert_eq!(
                (source.code, source.context, source.work, source.limit),
                (code, context.clone(), 7, 65_536)
            );
        }
    }

    #[trace("FR-025-AC-11", "TC-036")]
    #[test]
    fn binder_role_type_and_order_mismatches_refuse_before_harness_lowering() {
        let (package, clause) = native_twin::Twin::with_two_parameter_result_postcondition()
            .emitted_package("ResultWithParameters");
        let view = ClauseGraphView::new(&package);
        let context = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.semantic_form.as_ref() == "object_type")
            .expect("selected object context");
        let operations = package
            .model_operations(&context.node_id, 65_536)
            .expect("authentic complete signature");
        let operation = operations
            .operations()
            .iter()
            .find(|operation| operation.name() == "deposit")
            .expect("selected deposit operation");
        let [CheckedTerm::Aggregate { members }, _, _] = application(
            package.term(&clause).expect("admitted clause body"),
            ApplicationOperator::StateClause,
            "filament.op.state.clause",
        )
        .expect("admitted state clause") else {
            panic!("admitted ordered binders");
        };
        assert_eq!(members.len(), 4);
        let result = machine_scalar(
            operation
                .result()
                .flatten()
                .expect("bounded produced result"),
        )
        .expect("signed-i64 result domain");
        let mut wrong_role = members.clone();
        wrong_role.swap(0, 1);
        let mut wrong_order = members.clone();
        wrong_order.swap(2, 3);
        for (label, changed, expected_result) in [
            ("role", wrong_role, Some(result.clone())),
            ("order", wrong_order, Some(result)),
            ("type", members.clone(), Some(MachineScalar::Boolean)),
        ] {
            let refusal = match validate_signature_binders(
                &view,
                &clause,
                &context.node_id,
                &changed,
                operation.parameters(),
                expected_result,
            ) {
                Err(refusal) => refusal,
                Ok(_) => panic!("{label} mismatch would expose a lowerable harness"),
            };
            assert!(matches!(
                refusal,
                UnsupportedObligation::V2ClauseSignatureMismatch { node_id }
                    if node_id == clause
            ));
        }
    }
}

#[cfg(test)]
mod native_probe_tests {
    use super::{
        bind_native_arithmetic_probes, escape_message_label, native_term, ArithmeticProbe,
        NativeBinding, NativeTerm,
    };
    use crate::core::ir::clause::ClauseTerm;
    use crate::core::native_integer::{
        NativeIntegerIntervalDecision as Interval, NativeIntegerOperationDecision as Decision,
        NativeIntegerSelection,
    };
    use ix_trace_rs::trace;

    /// The independent native expectation sees a checked literal narrow only as its integer, and
    /// only when the target admits that integer.
    #[trace("FR-014-AC-54", "FR-015-AC-41", "TC-025")]
    #[test]
    fn native_clause_expectation_accepts_only_in_range_literal_narrows() {
        let narrow = |value: &str| ClauseTerm::Operation {
            identity: "filament.op.numeric.narrow".to_owned(),
            arguments: vec![ClauseTerm::Integer(value.to_owned())],
            result_bound: Some((0, 1000)),
        };
        let expectation = |value: &str| {
            native_term(
                &narrow(value),
                &[NativeBinding::parameter("amount", "amount_current")],
                &[],
                &mut Vec::new(),
            )
        };
        for (value, expected) in [("0", 0_i128), ("1000", 1000)] {
            assert!(
                matches!(
                    expectation(value),
                    Some(NativeTerm::Integer { lower, upper, .. }) if lower == expected && upper == expected
                ),
                "{value}"
            );
        }
        assert!(expectation("-1").is_none());
        assert!(expectation("1001").is_none());

        let wide = |value: &str, lower: i128, upper: i128| {
            native_term(
                &ClauseTerm::Operation {
                    identity: "filament.op.numeric.narrow".to_owned(),
                    arguments: vec![ClauseTerm::Integer(value.to_owned())],
                    result_bound: Some((lower, upper)),
                },
                &[NativeBinding::parameter("amount", "amount_current")],
                &[],
                &mut Vec::new(),
            )
        };
        for (value, lower, upper) in [
            ("-5", -10_i128, 10_i128),
            ("-9223372036854775808", i128::from(i64::MIN), 0),
            ("9223372036854775807", 0, i128::from(i64::MAX)),
            ("9223372036854775808", 0, i128::from(i64::MAX) + 10),
            ("-9223372036854775809", i128::from(i64::MIN) - 10, 0),
        ] {
            let expected: i128 = value.parse().expect("literal");
            assert!(
                matches!(
                    wide(value, lower, upper),
                    Some(NativeTerm::Integer { lower: low, upper: high, .. }) if low == expected && high == expected
                ),
                "{value}"
            );
        }
        assert!(wide("9223372036854775808", 0, i128::from(i64::MAX)).is_none());
        assert!(wide("-11", -10, 10).is_none());
    }

    #[test]
    fn structural_message_label_escapes_delimiters_and_percent_injectively() {
        let label = escape_message_label("p%7B{\"q\"}\\\n\r");
        assert_eq!(label, "p%257B%7B%22q%22%7D%5C%0A%0D");
        assert_ne!(escape_message_label("%7B"), escape_message_label("{"));
        assert!(!label.contains(['"', '\\', '{', '}', '\n', '\r']));
    }

    #[trace("FR-015-AC-125", "TC-025")]
    #[test]
    fn bounded_add_feeding_multiply_uses_the_typed_completed_interval() {
        let node_id: filament_contract_model::CheckedNodeId =
            serde_json::from_value(serde_json::json!({
                "package": "kani-clause-v2-test",
                "path": ["bounded-add"],
                "ordinal": 0,
            }))
            .expect("checked clause key");
        let interval = |lower: i128, upper: i128| Interval {
            lower: lower.to_string(),
            upper: upper.to_string(),
        };
        let mut selection = NativeIntegerSelection::new();
        selection.operations = vec![
            Decision {
                node_id: node_id.clone(),
                step: 0,
                operation_identity: "filament.op.integer.add".to_owned(),
                operands: vec![interval(0, 1000), interval(1, 1)],
                capacity: Some(interval(1, 1001)),
                semantic_domain: Some(interval(0, 1000)),
            },
            Decision {
                node_id,
                step: 1,
                operation_identity: "filament.op.integer.mul".to_owned(),
                operands: vec![interval(1, 1000), interval(2, 2)],
                capacity: Some(interval(2, 2000)),
                semantic_domain: None,
            },
        ];
        let mut probes = vec![
            ArithmeticProbe {
                operation_identity: "filament.op.integer.add".to_owned(),
                operation: "Add",
                operands: vec!["amount".to_owned(), "1".to_owned()],
                expected: "amount + 1".to_owned(),
                operand_bounds: vec![(0, 1000), (1, 1)],
                semantic_domain: None,
                guard: "true".to_owned(),
            },
            ArithmeticProbe {
                operation_identity: "filament.op.integer.mul".to_owned(),
                operation: "Multiply",
                operands: vec!["amount + 1".to_owned(), "2".to_owned()],
                expected: "(amount + 1) * 2".to_owned(),
                operand_bounds: vec![(1, 1001), (2, 2)],
                semantic_domain: None,
                guard: "true".to_owned(),
            },
        ];
        bind_native_arithmetic_probes(&mut probes, &selection).expect("admitted typed plan");
        assert_eq!(probes[0].semantic_domain, Some((0, 1000)));
        assert_eq!(probes[1].operand_bounds, [(1, 1000), (2, 2)]);
    }
}

#[cfg(test)]
mod state_postcondition_shape_tests {
    use super::{
        record_state_input, state_parameters, state_reads, MachineScalar, ResultParameter,
        ResultPostcondition, StateFieldDomains, StateParameter,
    };
    use crate::core::ir::clause::{ClauseInput, ClauseInputSide, ClauseLeaf};
    use filament_contract_model::CheckedNodeId;
    use ix_trace_rs::trace;

    fn node(name: &str) -> CheckedNodeId {
        serde_json::from_value(serde_json::json!({
            "package": "state-shape-test",
            "path": [name],
            "ordinal": 0,
        }))
        .expect("checked node key")
    }

    fn input(
        side: ClauseInputSide,
        declaring: Option<CheckedNodeId>,
        bounds: (i128, i128),
    ) -> ClauseInput {
        ClauseInput {
            leaf: ClauseLeaf::Whole,
            name: "balance".to_owned(),
            side,
            declaring_node_id: declaring,
            boolean: false,
            lower: bounds.0,
            upper: bounds.1,
        }
    }

    fn amount() -> Vec<StateParameter> {
        vec![StateParameter {
            name: "balance".to_owned(),
            minimum: 0,
            maximum: 100,
            drawn: "parameter_balance_current".to_owned(),
        }]
    }

    /// A non-integer parameter, a Boolean field, a field without a declaring node, a result
    /// read, a parameter read over another domain, a field read over two domains and two fields
    /// of one name each leave the shape unrendered.
    #[trace("FR-015-AC-43", "TC-025")]
    #[test]
    fn every_unrepresentable_state_shape_is_refused() {
        let boolean_parameter = ResultPostcondition {
            result: None,
            parameters: vec![ResultParameter {
                name: "flag".to_owned(),
                scalar: MachineScalar::Boolean,
            }],
        };
        assert!(state_parameters(&boolean_parameter).is_none());

        let mut fields = StateFieldDomains::new();
        let declaring = node("account");
        let mut boolean = input(ClauseInputSide::Current, Some(declaring.clone()), (0, 1));
        boolean.boolean = true;
        assert!(record_state_input(&mut fields, &[], &boolean).is_none());
        let undeclared = input(ClauseInputSide::Current, None, (0, 100));
        assert!(record_state_input(&mut fields, &[], &undeclared).is_none());
        let result = input(ClauseInputSide::Result, None, (0, 100));
        assert!(record_state_input(&mut fields, &[], &result).is_none());
        let narrowed = input(ClauseInputSide::Parameter, None, (0, 50));
        assert!(record_state_input(&mut fields, &amount(), &narrowed).is_none());
        let declared = input(ClauseInputSide::Parameter, None, (0, 100));
        assert!(record_state_input(&mut fields, &amount(), &declared).is_some());

        let first = input(ClauseInputSide::Current, Some(declaring.clone()), (0, 100));
        let second = input(ClauseInputSide::Pre, Some(declaring.clone()), (0, 99));
        assert!(record_state_input(&mut fields, &[], &first).is_some());
        assert!(record_state_input(&mut fields, &[], &second).is_none());

        let mut twice = StateFieldDomains::new();
        for owner in ["account", "vault"] {
            let read = input(ClauseInputSide::Current, Some(node(owner)), (0, 100));
            assert!(record_state_input(&mut twice, &[], &read).is_some());
        }
        assert!(state_reads(twice).is_none(), "two fields named balance");
        let mut once = StateFieldDomains::new();
        assert!(record_state_input(&mut once, &[], &first).is_some());
        assert_eq!(state_reads(once).expect("one field").len(), 1);
    }
}

#[cfg(test)]
mod control_expectation_tests {
    use super::*;
    use ix_trace_rs::trace;

    fn integer(value: &str) -> ClauseTerm {
        ClauseTerm::Integer(value.to_owned())
    }

    fn input(name: &str, upper: i128) -> ClauseInput {
        ClauseInput {
            leaf: ClauseLeaf::Whole,
            name: name.to_owned(),
            side: ClauseInputSide::Parameter,
            declaring_node_id: None,
            boolean: false,
            lower: 0,
            upper,
        }
    }

    /// FR-015-AC-154: the independent expectation of a `case` is a `match` on the same member index,
    /// one arm per member, the last holding the exhaustive pattern.
    #[trace("FR-015-AC-154", "TC-025")]
    #[test]
    fn a_case_expectation_matches_the_member_index_in_declaration_order() {
        let term = ClauseTerm::Case {
            scrutinee: input("kind", 2),
            arms: vec![integer("1"), integer("2"), integer("0")],
        };
        let value = native_term(
            &term,
            &[NativeBinding::parameter("kind", "parameter_kind_current")],
            &[],
            &mut Vec::new(),
        )
        .expect("the case has an expectation");
        let NativeTerm::Integer { expression, .. } = value else {
            panic!("an integer-valued case");
        };
        assert_eq!(
            expression,
            "(match parameter_kind_current { 0_i64 => 1_i128, 1_i64 => 2_i128, _ => 0_i128, })"
        );
    }

    /// FR-015-AC-170 and FR-015-AC-173: a read of a binder is the value of the innermost enclosing
    /// binder with that number, so a rebinding resolves each read to its own `let`.
    #[trace("FR-015-AC-152", "TC-025")]
    #[test]
    fn a_binder_read_resolves_to_the_innermost_binding() {
        let amount = ClauseTerm::Input(input("amount", 999));
        let add = |left: ClauseTerm, right: &str| ClauseTerm::Operation {
            identity: "filament.op.integer.add".to_owned(),
            arguments: vec![left, integer(right)],
            result_bound: None,
        };
        // let a = amount + 1 in (let a' = a + 1 in a' + 0), with each read by its own binder number.
        let term = ClauseTerm::Let {
            binder: 0,
            value: Box::new(add(amount, "1")),
            body: Box::new(ClauseTerm::Let {
                binder: 1,
                value: Box::new(add(ClauseTerm::Binder { binder: 0 }, "1")),
                body: Box::new(add(ClauseTerm::Binder { binder: 1 }, "0")),
            }),
        };
        let mut arithmetic = Vec::new();
        let value = native_term(
            &term,
            &[NativeBinding::parameter("amount", "amount_current")],
            &[],
            &mut arithmetic,
        )
        .expect("the nested let has an expectation");
        let NativeTerm::Integer { expression, .. } = value else {
            panic!("an integer-valued let");
        };
        assert_eq!(
            arithmetic.len(),
            3,
            "each initializer and the body add is probed once"
        );
        assert_eq!(
            expression,
            "(((i128::from(amount_current) + 1_i128) + 1_i128) + 0_i128)"
        );
    }

    /// FR-015-AC-153: a branch probe runs only where its branch is selected.
    #[trace("FR-015-AC-153", "FR-015-AC-168", "TC-025")]
    #[test]
    fn a_branch_probe_is_guarded_by_its_selection() {
        let amount = || ClauseTerm::Input(input("amount", 1000));
        let operation = |identity: &str, right: &str| ClauseTerm::Operation {
            identity: identity.to_owned(),
            arguments: vec![amount(), integer(right)],
            result_bound: None,
        };
        let term = ClauseTerm::If {
            condition: Box::new(operation("filament.op.integer.lt", "500")),
            then: Box::new(operation("filament.op.integer.add", "1")),
            other: Box::new(operation("filament.op.integer.sub", "1")),
        };
        let mut arithmetic = Vec::new();
        native_term(
            &term,
            &[NativeBinding::parameter("amount", "amount_current")],
            &[],
            &mut arithmetic,
        )
        .expect("the if has an expectation");
        let [then, other] = arithmetic.as_slice() else {
            panic!("one probe per branch");
        };
        assert!(then.guard.ends_with("< 500_i128))") && !then.guard.contains("!("));
        assert!(other.guard.contains("!(") && other.guard.contains("< 500_i128"));
    }
}

#[cfg(test)]
mod control_shape_tests {
    use super::*;
    use crate::core::ir::clause::control_fixture::{applications, package, read_node};
    use ix_trace_rs::trace;

    /// `control_operands` over the `n`th application of `identity` in the fixture package, with the
    /// scrutinee's parameter declared unless `declared` is false.
    fn shape(
        identity: &str,
        position: usize,
        declared: bool,
    ) -> Result<usize, V2ClauseControlReason> {
        let package = package();
        let view = ClauseGraphView::new(&package);
        let arguments = applications(&package, identity)[position];
        let parameters = if declared {
            vec![read_node(&arguments[0]).clone()]
        } else {
            Vec::new()
        };
        control_operands(&view, identity, arguments, &parameters).map(|operands| operands.len())
    }

    #[trace("FR-015-AC-154", "TC-025")]
    #[test]
    fn a_case_over_an_arity_0_union_with_native_arms_contributes_its_arm_bodies() {
        assert_eq!(shape(CONTROL_CASE, 0, true), Ok(3));
    }

    #[trace("FR-015-AC-161", "TC-025")]
    #[test]
    fn an_if_with_a_non_native_branch_is_refused_and_native_branches_contribute_all_operands() {
        // The two `if` functions are `texts` (Text branches) and `plain` (Integer branches).
        let mut outcomes = [shape(CONTROL_IF, 0, true), shape(CONTROL_IF, 1, true)];
        outcomes.sort_by_key(Result::is_ok);
        assert_eq!(
            outcomes,
            [Err(V2ClauseControlReason::IfBranchNotNativeScalar), Ok(3)]
        );
    }

    #[trace("FR-015-AC-162", "TC-025")]
    #[test]
    fn a_case_over_a_union_with_a_payload_member_is_refused_for_the_payload() {
        assert_eq!(
            shape(CONTROL_CASE, 2, true),
            Err(V2ClauseControlReason::CaseUnionPayloadNotSupported)
        );
    }

    #[trace("FR-015-AC-163", "TC-025")]
    #[test]
    fn a_case_with_a_non_native_arm_body_is_refused_for_the_arm() {
        assert_eq!(
            shape(CONTROL_CASE, 1, true),
            Err(V2ClauseControlReason::CaseArmNotNativeScalar)
        );
    }

    #[trace("FR-015-AC-171", "TC-025")]
    #[test]
    fn a_case_whose_scrutinee_is_not_a_declared_operation_parameter_is_refused() {
        assert_eq!(
            shape(CONTROL_CASE, 0, false),
            Err(V2ClauseControlReason::CaseScrutineeNotParameter)
        );
    }
}
