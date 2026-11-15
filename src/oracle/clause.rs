//! V2 state-clause condition oracles over admitted checked packages.

mod native;

use std::collections::BTreeMap;

use quire_contract_model::{CheckedNodeId, CheckedPackageV2};
use serde::{Deserialize, Serialize};

use crate::core::artifact::MAX_GENERATED_SOURCE_BYTES;
use crate::core::ir::clause::{
    read_clause, ClauseGraphView, ClauseInput, ClauseInputSide, ClauseLeaf, ClauseTerm,
};
use crate::core::source_map::SourceProbe;

use super::claim::{ClaimDisposition, ClaimMap};
use super::operator_catalog::IntegerOperator;
use super::scalar::render::{clause_scalar_operation, render_clause_operation_helpers};
use super::scalar::{
    BooleanOperator, EqualityOperandKind, ExactScalarOperation, OperationProvenance,
    OrderingOperandKind,
};

/// A state field or bounded operation value supplied to one generated clause oracle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClauseOracleInput {
    /// The generated input name.
    pub name: String,
    /// The name this input is declared under: the operation parameter or the model field.
    #[serde(skip)]
    pub declared_name: String,
    /// Admitted declaring field and name when this input reads model state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_field: Option<ClauseOracleStateField>,
    /// Whether this parameter is a Boolean rather than an integer.
    pub boolean: bool,
    /// Inclusive lower bound from the admitted model member or parameter type.
    pub lower: i128,
    /// Inclusive upper bound from the admitted model member or parameter type.
    pub upper: i128,
}

/// Source identity of a model state field read by a clause oracle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClauseOracleStateField {
    /// Model object declaration that owns the selected field.
    pub declaring_node_id: CheckedNodeId,
    /// Field name in that declaration.
    pub field_name: String,
}

/// Checked inputs and operations of one clause-identified Boolean condition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct V2ClauseDescriptor {
    /// The clause node that owns the oracle claim.
    pub node_id: CheckedNodeId,
    /// Referenced condition expression, if the third argument is a reference.
    pub condition_node_id: Option<CheckedNodeId>,
    /// Ordered operand values and their admitted inclusive bounds.
    pub inputs: Vec<ClauseOracleInput>,
    /// Checked scalar operation identities in evaluation-plan order.
    /// An empty plan emits a direct Boolean read; otherwise oracle_clause accepts a meter
    /// and returns `Result<Outcome<bool>, ClauseStop>`.
    pub operation_identities: Vec<String>,
    /// Number of implication consequents derived from the condition tree.
    pub expected_consequents: u32,
}

/// One generated oracle of a V2 clause's Boolean condition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct V2ClauseOracle {
    /// Selected native integer capacity decisions, when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_integer_selection: Option<crate::core::native_integer::NativeIntegerSelection>,
    /// The operation's checked provenance.
    pub provenance: OperationProvenance,
    /// Self-contained generated Rust source. Retained in memory for harness embedding.
    #[serde(skip)]
    pub source: String,
    /// Descriptor derived from the admitted condition.
    pub descriptor: V2ClauseDescriptor,
    /// Probe regions derived from the generated source and the checked condition.
    pub source_map: Vec<V2ClauseCoverageRegion>,
}

/// The one requested clause's generated-or-refused disposition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct V2ClauseClaim {
    /// Exact requested node.
    pub node_id: CheckedNodeId,
    /// Generated oracle or typed refusal.
    pub result: ClaimDisposition<V2ClauseOracle, V2ClauseRefusal>,
}

/// One source span for a V2 clause condition oracle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct V2ClauseCoverageRegion {
    /// Generated Rust artifact containing the probe.
    pub artifact_path: String,
    /// Checked clause node that owns the condition.
    pub node_id: CheckedNodeId,
    /// Evaluation entry or implication consequent.
    pub role: String,
    /// One-based first source line.
    pub start_line: u32,
    /// One-based last source line.
    pub end_line: u32,
    /// Exact executable token to measure.
    pub probe: SourceProbe,
    /// Consequent census, present on the evaluation entry only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_consequents: Option<u32>,
}

/// Why a selected clause condition could not be generated.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum V2ClauseRefusal {
    /// The checked clause's body is outside this generator's supported shape.
    UnsupportedCondition,
    /// An admitted node is absent or is not a state clause.
    NotClause,
    /// Source rendering exceeded a finite ceiling or failed internally.
    Render,
    /// A source operation cannot retain its complete population in the native kernel.
    NativeCapability {
        /// Authentic checked clause.
        node_id: CheckedNodeId,
        /// Refused operator identity, absent for a literal or input.
        operation_identity: Option<String>,
        /// Evaluation-plan index.
        step: usize,
        /// Typed reason distinct from semantic-domain refusal.
        reason: NativeClauseCapabilityReason,
        /// Complete operand intervals given to the kernel constructor.
        operands: Vec<crate::core::native_integer::NativeIntegerIntervalDecision>,
    },
}

/// Why an admitted clause cannot use the native integer route.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeClauseCapabilityReason {
    /// An authored literal or endpoint does not fit signed i128.
    EndpointNotRepresentable,
    /// The operation's complete interval image exceeds kernel capacity.
    CapacityExceeded,
    /// The operation has no native evaluator.
    UnsupportedOperation,
}

/// Derive the typed descriptor of an admitted V2 clause condition without rendering source.
pub fn derive_v2_clause_descriptor(
    package: &CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Result<V2ClauseDescriptor, V2ClauseRefusal> {
    let nodes = ClauseGraphView::new(package);
    derive_v2_clause_descriptor_with_view(node_id, &nodes)
}

pub(crate) fn derive_v2_clause_descriptor_with_view(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Result<V2ClauseDescriptor, V2ClauseRefusal> {
    Ok(plan_clause(node_id, nodes)?.descriptor)
}

/// The width of each operation of a clause after range propagation (FR-056-AC-2): the largest
/// bit width, over the operation's operands, of the inclusive interval propagated to it. Entries
/// follow [`V2ClauseDescriptor::operation_identities`].
///
/// Intervals are propagated in `i128`, with the same image and semantic-domain clipping the
/// native rendering uses, but without the native kernel's capacity limit. The width is `None`
/// for an operation with no integer operand and for one whose operand interval leaves `i128`.
///
/// # Errors
///
/// The refusal of the clause plan.
pub fn derive_v2_clause_operation_widths(
    package: &CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Result<Vec<(String, Option<u32>)>, V2ClauseRefusal> {
    clause_operation_widths(node_id, &ClauseGraphView::new(package))
}

/// The smallest bit count `n` such that every value of `[lower, upper]` is representable in `n`
/// bits: unsigned `bits(upper)` when `lower >= 0`, else two's complement signed.
///
/// The native carrier is signed, so `[0, 2^k]` counts as `k + 1` bits.
#[must_use]
pub fn interval_bits(lower: i128, upper: i128) -> u32 {
    let unsigned_bits = |value: u128| u128::BITS - value.leading_zeros();
    if lower >= 0 {
        // A non-negative lower bound is at most `upper`; `upper` is non-negative.
        unsigned_bits(upper.unsigned_abs())
    } else {
        // `!lower` is `-lower - 1`, the magnitude two's complement needs for a negative bound.
        let negative = unsigned_bits((!lower).unsigned_abs());
        let positive = if upper >= 0 {
            unsigned_bits(upper.unsigned_abs())
        } else {
            0
        };
        negative.max(positive) + 1
    }
}

type Interval = (i128, i128);

pub(crate) fn clause_operation_widths(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Result<Vec<(String, Option<u32>)>, V2ClauseRefusal> {
    let prepared = plan_clause(node_id, nodes)?;
    let mut ranges: Vec<Option<Interval>> = Vec::with_capacity(prepared.plan.len());
    let mut widths = Vec::new();
    for step in &prepared.plan {
        let range = match &step.term {
            StepTerm::Boolean(_) => None,
            StepTerm::Integer(value) => value.parse::<i128>().ok().map(|value| (value, value)),
            StepTerm::Input(input) if input.boolean => None,
            StepTerm::Input(input) => Some((input.lower, input.upper)),
            StepTerm::OptionValue { payload, .. } if payload.boolean => None,
            StepTerm::OptionValue { payload, .. } => Some((payload.lower, payload.upper)),
            StepTerm::Binder { source, .. } => ranges.get(*source).copied().flatten(),
            StepTerm::Let { body, .. } => ranges.get(*body).copied().flatten(),
            StepTerm::If { then, other, .. } => hull(&[
                ranges.get(*then).copied().flatten(),
                ranges.get(*other).copied().flatten(),
            ]),
            StepTerm::Case { arms, .. } => hull(
                &arms
                    .iter()
                    .map(|arm| ranges.get(*arm).copied().flatten())
                    .collect::<Vec<_>>(),
            ),
            StepTerm::Operation {
                identity,
                operation,
                arguments,
            } => {
                let operands: Vec<Option<Interval>> = arguments
                    .iter()
                    .map(|child| ranges.get(*child).copied().flatten())
                    .collect();
                let width = match operation {
                    ExactScalarOperation::Ordering { .. }
                    | ExactScalarOperation::Equality {
                        operands: EqualityOperandKind::Integer,
                        ..
                    }
                    | ExactScalarOperation::IntegerArithmetic { .. } => operands
                        .iter()
                        .map(|operand| operand.map(|(lower, upper)| interval_bits(lower, upper)))
                        .try_fold(0, |widest: u32, bits| bits.map(|bits| widest.max(bits)))
                        .filter(|_| !operands.is_empty()),
                    _ => None,
                };
                widths.push((identity.clone(), width));
                match operation {
                    ExactScalarOperation::IntegerArithmetic { operator, domain } => {
                        image(*operator, &operands).map(|capacity| clip(capacity, domain))
                    }
                    _ => None,
                }
            }
        };
        ranges.push(range);
    }
    Ok(widths)
}

/// The smallest interval holding every branch interval, or `None` when a branch has none.
fn hull(branches: &[Option<Interval>]) -> Option<Interval> {
    let mut hull: Option<Interval> = None;
    for branch in branches {
        let (lower, upper) = (*branch)?;
        hull = Some(match hull {
            Some((low, high)) => (low.min(lower), high.max(upper)),
            None => (lower, upper),
        });
    }
    hull
}

/// The exact image of an integer operation over operand intervals, or `None` past `i128`.
fn image(operator: IntegerOperator, operands: &[Option<Interval>]) -> Option<Interval> {
    match (operator, operands) {
        (IntegerOperator::Negate, [Some((lower, upper))]) => {
            Some((upper.checked_neg()?, lower.checked_neg()?))
        }
        (IntegerOperator::Add, [Some((a, b)), Some((c, d))]) => {
            Some((a.checked_add(*c)?, b.checked_add(*d)?))
        }
        (IntegerOperator::Subtract, [Some((a, b)), Some((c, d))]) => {
            Some((a.checked_sub(*d)?, b.checked_sub(*c)?))
        }
        (IntegerOperator::Multiply, [Some((a, b)), Some((c, d))]) => {
            let products = [
                a.checked_mul(*c)?,
                a.checked_mul(*d)?,
                b.checked_mul(*c)?,
                b.checked_mul(*d)?,
            ];
            Some((*products.iter().min()?, *products.iter().max()?))
        }
        _ => None,
    }
}

/// The values that complete the semantic-domain check: the image clipped to a bounded domain,
/// the whole image when the clip is empty (as the native rendering retains it).
fn clip(capacity: Interval, domain: &quire_exact::IntegerDomain) -> Interval {
    let quire_exact::IntegerDomain::Bounded(bound) = domain else {
        return capacity;
    };
    let Ok(bound) = quire_exact::NativeIntegerInterval::try_from(bound) else {
        return capacity;
    };
    let (lower, upper) = (capacity.0.max(bound.lower()), capacity.1.min(bound.upper()));
    if lower <= upper {
        (lower, upper)
    } else {
        capacity
    }
}

/// Settle one selected clause as one typed claim-map entry.
pub fn generate_v2_clause_claim_map(
    package: &CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> ClaimMap<V2ClauseClaim> {
    let result = match generate_v2_clause_oracle(package, node_id) {
        Ok(generated) => ClaimDisposition::Generated(Box::new(generated)),
        Err(refusal) => ClaimDisposition::Refused { refusal },
    };
    ClaimMap {
        source: quire_contract_model::ContractSourceV1 {
            name: package.name().into(),
            lock: package.lock().clone(),
        },
        blocked: Vec::new(),
        items: vec![V2ClauseClaim {
            node_id: node_id.clone(),
            result,
        }],
    }
}

struct PreparedClause {
    descriptor: V2ClauseDescriptor,
    names: BTreeMap<ClauseInput, String>,
    plan: Vec<Step>,
    root: usize,
}

fn plan_clause(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Result<PreparedClause, V2ClauseRefusal> {
    let clause = nodes.node(node_id).ok_or(V2ClauseRefusal::NotClause)?;
    if &*clause.node_tag != "state" || &*clause.semantic_form != "state_clause" {
        return Err(V2ClauseRefusal::NotClause);
    }
    let condition = read_clause(node_id, nodes).ok_or(V2ClauseRefusal::UnsupportedCondition)?;
    let mut taken = std::collections::BTreeSet::new();
    let names = condition
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let side = match input.side {
                ClauseInputSide::Current => "current",
                ClauseInputSide::Pre => "pre",
                ClauseInputSide::Parameter => "parameter",
                ClauseInputSide::Result => "result",
            };
            let leaf = match &input.leaf {
                ClauseLeaf::Whole => String::new(),
                ClauseLeaf::Presence => "_present".to_owned(),
                ClauseLeaf::Payload(path) if path.is_empty() => "_value".to_owned(),
                ClauseLeaf::Payload(path) => format!("_value_{path}"),
            };
            let candidate = format!("{side}_{}{leaf}", input.name);
            let name = if syn::parse_str::<syn::Ident>(&candidate).is_ok()
                && !taken.contains(&candidate)
            {
                candidate
            } else {
                format!("input_{index}")
            };
            taken.insert(name.clone());
            (input.clone(), name)
        })
        .collect::<BTreeMap<_, _>>();
    let mut plan = Vec::new();
    let root = lower(&condition.term, &names, &mut plan)?;
    if plan.get(root).ok_or(V2ClauseRefusal::Render)?.kind != ValueKind::Boolean {
        return Err(V2ClauseRefusal::UnsupportedCondition);
    }
    let operation_identities = plan
        .iter()
        .filter_map(|step| match &step.term {
            StepTerm::Operation { identity, .. } => Some(identity.clone()),
            _ => None,
        })
        .collect();
    let inputs = condition
        .inputs
        .iter()
        .map(|input| {
            Ok(ClauseOracleInput {
                name: names.get(input).ok_or(V2ClauseRefusal::Render)?.clone(),
                declared_name: input.name.clone(),
                state_field: input.declaring_node_id.as_ref().map(|declaring_node_id| {
                    ClauseOracleStateField {
                        declaring_node_id: declaring_node_id.clone(),
                        field_name: input.name.clone(),
                    }
                }),
                boolean: input.boolean,
                lower: input.lower,
                upper: input.upper,
            })
        })
        .collect::<Result<Vec<_>, V2ClauseRefusal>>()?;
    let descriptor = V2ClauseDescriptor {
        node_id: node_id.clone(),
        condition_node_id: condition.condition_node_id,
        inputs,
        operation_identities,
        expected_consequents: consequent_count(&plan)?,
    };
    Ok(PreparedClause {
        descriptor,
        names,
        plan,
        root,
    })
}

/// Generate a checked V2 clause's Boolean condition oracle.
pub fn generate_v2_clause_oracle(
    package: &CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Result<V2ClauseOracle, V2ClauseRefusal> {
    let nodes = ClauseGraphView::new(package);
    generate_v2_clause_oracle_with_view(node_id, &nodes)
}

/// Select kernel-admitted native integers for every supported intermediate.
///
/// The authentic checked clause and its original parameter domains are retained;
/// an ineligible operation returns a source-associated capability refusal.
pub fn generate_v2_clause_native_oracle(
    package: &CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Result<V2ClauseOracle, V2ClauseRefusal> {
    let nodes = ClauseGraphView::new(package);
    generate_v2_clause_native_oracle_with_view(node_id, &nodes)
}

pub(crate) fn generate_v2_clause_native_oracle_with_view(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Result<V2ClauseOracle, V2ClauseRefusal> {
    generate_clause_oracle_with_view(node_id, nodes, IntegerRenderPolicy::Native)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum IntegerRenderPolicy {
    General,
    /// Native integers; the stop is a payload-free Copy kind (the proof path).
    Native,
}

impl IntegerRenderPolicy {
    fn is_native(self) -> bool {
        self == Self::Native
    }
}

pub(crate) fn generate_v2_clause_oracle_with_view(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Result<V2ClauseOracle, V2ClauseRefusal> {
    generate_clause_oracle_with_view(node_id, nodes, IntegerRenderPolicy::General)
}

fn generate_clause_oracle_with_view(
    node_id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
    policy: IntegerRenderPolicy,
) -> Result<V2ClauseOracle, V2ClauseRefusal> {
    let PreparedClause {
        descriptor,
        names,
        plan,
        root,
    } = plan_clause(node_id, nodes)?;
    let operations = plan
        .iter()
        .enumerate()
        .filter_map(|(index, step)| match &step.term {
            StepTerm::Operation {
                identity,
                operation,
                ..
            } => Some((
                format!("scalar_{index}"),
                identity.clone(),
                operation.clone(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    let (mut source, native_integer_selection) = if policy.is_native() {
        let (source, selected) = native::helpers(&plan, node_id)?;
        (source, Some(selected))
    } else {
        (
            render_clause_operation_helpers(&operations).map_err(|_| V2ClauseRefusal::Render)?,
            None,
        )
    };
    source.push_str(if policy.is_native() {
        THIN_STOP_TYPE
    } else {
        STOP_TYPE
    });
    if !operations.is_empty() {
        source.push_str(if policy.is_native() {
            THIN_MEASURED_HELPER
        } else {
            MEASURED_HELPER
        });
    }
    let integer_type = if policy.is_native() {
        "nx::NativeInteger"
    } else {
        "rt::Integer"
    };
    let parameters = names
        .iter()
        .map(|(input, name)| {
            if input.boolean {
                format!("{name}: bool, ")
            } else {
                format!("{name}: &{integer_type}, ")
            }
        })
        .collect::<String>();
    let arguments = names
        .values()
        .map(|name| format!("{name}, "))
        .collect::<String>();
    if operations.is_empty() {
        // A literal or Boolean read has no accounted operation. Keep this one entry
        // free of a meter allocation; no owner operation or charge is skipped.
        let value = match &plan.get(root).ok_or(V2ClauseRefusal::Render)?.term {
            StepTerm::Boolean(value) => value.to_string(),
            StepTerm::Input(input) if input.boolean => {
                names.get(input).ok_or(V2ClauseRefusal::Render)?.clone()
            }
            _ => return Err(V2ClauseRefusal::UnsupportedCondition),
        };
        source.push_str(&format!("\n/// The admitted V2 clause condition, without accounted operations.\npub fn oracle_clause({parameters}) -> bool {{\n    {value}\n}}\n"));
    } else {
        for (index, step) in plan.iter().enumerate() {
            render_step(
                &mut source,
                &plan,
                index,
                step,
                &names,
                &parameters,
                &arguments,
                policy,
            )?;
        }
        if policy.is_native() {
            source.push_str(&format!("\n/// The admitted V2 clause condition; a stop is a payload-free kind.\npub fn oracle_clause({parameters}meter: &mut rt::Meter) -> Result<bool, ClauseStop> {{\n    value_{root}({arguments}meter)\n}}\n"));
        } else {
            source.push_str(&format!("\n/// The admitted V2 clause condition.\npub fn oracle_clause({parameters}meter: &mut rt::Meter) -> Result<rt::Outcome<bool>, ClauseStop> {{\n    match value_{root}({arguments}meter) {{\n        Ok(value) => Ok(rt::Outcome::Completed(value)),\n        Err(ClauseStop::Runtime(outcome)) => Ok(outcome),\n        Err(other) => Err(other),\n    }}\n}}\n"));
        }
    }
    if source.len() > MAX_GENERATED_SOURCE_BYTES {
        return Err(V2ClauseRefusal::Render);
    }
    let source_map = coverage_regions(&source, node_id, &plan)?;
    Ok(V2ClauseOracle {
        native_integer_selection,
        provenance: OperationProvenance::IrConfirmed,
        source,
        descriptor,
        source_map,
    })
}

fn coverage_regions(
    source: &str,
    node_id: &CheckedNodeId,
    plan: &[Step],
) -> Result<Vec<V2ClauseCoverageRegion>, V2ClauseRefusal> {
    let lines = source.lines().collect::<Vec<_>>();
    let entry = lines
        .iter()
        .position(|line| line.starts_with("pub fn oracle_clause("))
        .ok_or(V2ClauseRefusal::Render)?;
    let census = consequent_count(plan)?;
    let mut regions = vec![coverage_region(
        node_id,
        "oracle_evaluation",
        entry,
        0,
        3,
        Some(census),
    )?];
    for (index, step) in plan.iter().enumerate() {
        let StepTerm::Operation {
            operation:
                ExactScalarOperation::Boolean {
                    operator: BooleanOperator::Implies,
                    ..
                },
            arguments,
            ..
        } = &step.term
        else {
            continue;
        };
        let consequent = arguments.get(1).ok_or(V2ClauseRefusal::Render)?;
        let declaration = format!("fn value_{index}(");
        let start = lines
            .iter()
            .position(|line| line.starts_with(&declaration))
            .ok_or(V2ClauseRefusal::Render)?;
        let token = format!("value_{consequent}(");
        let (line, column) = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .take_while(|(_, line)| !line.starts_with('}'))
            .find_map(|(line, text)| text.find(&token).map(|column| (line, column)))
            .ok_or(V2ClauseRefusal::Render)?;
        regions.push(coverage_region(
            node_id,
            "implication_consequent",
            line,
            column,
            // LLVM can split the call after its `value_` prefix when the right side is
            // skipped. Keep the probe inside that first count-bearing span.
            "value_".len(),
            None,
        )?);
    }
    Ok(regions)
}

fn consequent_count(plan: &[Step]) -> Result<u32, V2ClauseRefusal> {
    u32::try_from(
        plan.iter()
            .filter(|step| {
                matches!(
                    &step.term,
                    StepTerm::Operation {
                        operation: ExactScalarOperation::Boolean {
                            operator: BooleanOperator::Implies,
                            ..
                        },
                        ..
                    }
                )
            })
            .count(),
    )
    .map_err(|_| V2ClauseRefusal::Render)
}

fn coverage_region(
    node_id: &CheckedNodeId,
    role: &str,
    line: usize,
    column: usize,
    width: usize,
    expected_consequents: Option<u32>,
) -> Result<V2ClauseCoverageRegion, V2ClauseRefusal> {
    let line = u32::try_from(line + 1).map_err(|_| V2ClauseRefusal::Render)?;
    let start_column = u32::try_from(column + 1).map_err(|_| V2ClauseRefusal::Render)?;
    let end_column = start_column
        .checked_add(u32::try_from(width).map_err(|_| V2ClauseRefusal::Render)?)
        .ok_or(V2ClauseRefusal::Render)?;
    Ok(V2ClauseCoverageRegion {
        artifact_path: "src/lib.rs".to_owned(),
        node_id: node_id.clone(),
        role: role.to_owned(),
        start_line: line,
        end_line: line,
        probe: SourceProbe {
            line,
            start_column,
            end_column,
        },
        expected_consequents,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueKind {
    Boolean,
    Integer,
}

enum StepTerm {
    Boolean(bool),
    Integer(String),
    Input(ClauseInput),
    Operation {
        identity: String,
        operation: ExactScalarOperation,
        arguments: Vec<usize>,
    },
    /// `quire.op.option.value`: the payload leaf where the presence input is true, else the
    /// undefined stop.
    OptionValue {
        presence: ClauseInput,
        payload: ClauseInput,
    },
    /// A read of an enclosing `let` binder; `source` is the step that computed its value.
    Binder {
        binder: usize,
        source: usize,
    },
    Let {
        binder: usize,
        value: usize,
        body: usize,
    },
    If {
        condition: usize,
        then: usize,
        other: usize,
    },
    /// The arms are in declaration order of the union's members; `scrutinee` reads the member index.
    Case {
        scrutinee: ClauseInput,
        arms: Vec<usize>,
    },
}

/// One `let` binder in force at a step: its number, its value kind and the step that computed it.
#[derive(Clone, Copy)]
struct ScopedBinder {
    binder: usize,
    kind: ValueKind,
    source: usize,
}

/// A checked literal narrowing keeps its integer value when the target admits it.
/// This is shared by the oracle plan and the independent harness expectation.
pub(crate) fn admitted_literal_narrow(term: &ClauseTerm) -> Option<&ClauseTerm> {
    let ClauseTerm::Operation {
        identity,
        arguments,
        result_bound: Some((minimum, maximum)),
    } = term
    else {
        return None;
    };
    if identity != "quire.op.numeric.narrow" {
        return None;
    }
    let [literal @ ClauseTerm::Integer(value)] = arguments.as_slice() else {
        return None;
    };
    let integer = value.parse::<i128>().ok()?;
    (*minimum..=*maximum).contains(&integer).then_some(literal)
}

struct Step {
    kind: ValueKind,
    term: StepTerm,
    /// The binders in force where this step evaluates, outermost first.
    scope: Vec<ScopedBinder>,
}

fn lower(
    term: &ClauseTerm,
    names: &BTreeMap<ClauseInput, String>,
    plan: &mut Vec<Step>,
) -> Result<usize, V2ClauseRefusal> {
    lower_in(term, names, plan, &[])
}

fn lower_in(
    term: &ClauseTerm,
    names: &BTreeMap<ClauseInput, String>,
    plan: &mut Vec<Step>,
    scope: &[ScopedBinder],
) -> Result<usize, V2ClauseRefusal> {
    if plan.len() >= 4096 {
        return Err(V2ClauseRefusal::UnsupportedCondition);
    }
    let kind_of = |plan: &[Step], index: usize| {
        plan.get(index)
            .map(|step| step.kind)
            .ok_or(V2ClauseRefusal::Render)
    };
    let step = match term {
        ClauseTerm::Boolean(value) => Step {
            kind: ValueKind::Boolean,
            term: StepTerm::Boolean(*value),
            scope: scope.to_vec(),
        },
        ClauseTerm::Integer(value) => Step {
            kind: ValueKind::Integer,
            term: StepTerm::Integer(value.clone()),
            scope: scope.to_vec(),
        },
        ClauseTerm::Binder { binder } => {
            let bound = scope
                .iter()
                .find(|bound| bound.binder == *binder)
                .ok_or(V2ClauseRefusal::UnsupportedCondition)?;
            Step {
                kind: bound.kind,
                term: StepTerm::Binder {
                    binder: *binder,
                    source: bound.source,
                },
                scope: scope.to_vec(),
            }
        }
        ClauseTerm::Let {
            binder,
            value,
            body,
        } => {
            let value = lower_in(value, names, plan, scope)?;
            let mut inner = scope.to_vec();
            inner.push(ScopedBinder {
                binder: *binder,
                kind: kind_of(plan, value)?,
                source: value,
            });
            let body = lower_in(body, names, plan, &inner)?;
            Step {
                kind: kind_of(plan, body)?,
                term: StepTerm::Let {
                    binder: *binder,
                    value,
                    body,
                },
                scope: scope.to_vec(),
            }
        }
        ClauseTerm::If {
            condition,
            then,
            other,
        } => {
            let condition = lower_in(condition, names, plan, scope)?;
            let then = lower_in(then, names, plan, scope)?;
            let other = lower_in(other, names, plan, scope)?;
            let kind = kind_of(plan, then)?;
            if kind_of(plan, condition)? != ValueKind::Boolean || kind_of(plan, other)? != kind {
                return Err(V2ClauseRefusal::UnsupportedCondition);
            }
            Step {
                kind,
                term: StepTerm::If {
                    condition,
                    then,
                    other,
                },
                scope: scope.to_vec(),
            }
        }
        ClauseTerm::Case { scrutinee, arms } => {
            if !names.contains_key(scrutinee) || arms.is_empty() {
                return Err(V2ClauseRefusal::UnsupportedCondition);
            }
            let arms = arms
                .iter()
                .map(|arm| lower_in(arm, names, plan, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let kind = kind_of(plan, arms[0])?;
            for arm in &arms {
                if kind_of(plan, *arm)? != kind {
                    return Err(V2ClauseRefusal::UnsupportedCondition);
                }
            }
            Step {
                kind,
                term: StepTerm::Case {
                    scrutinee: scrutinee.clone(),
                    arms,
                },
                scope: scope.to_vec(),
            }
        }
        ClauseTerm::OptionValue { presence, payload }
            if names.contains_key(presence) && names.contains_key(payload) =>
        {
            Step {
                kind: if payload.boolean {
                    ValueKind::Boolean
                } else {
                    ValueKind::Integer
                },
                term: StepTerm::OptionValue {
                    presence: presence.clone(),
                    payload: payload.clone(),
                },
                scope: scope.to_vec(),
            }
        }
        ClauseTerm::OptionValue { .. } => return Err(V2ClauseRefusal::UnsupportedCondition),
        ClauseTerm::Input(input) if names.contains_key(input) => Step {
            kind: if input.boolean {
                ValueKind::Boolean
            } else {
                ValueKind::Integer
            },
            term: StepTerm::Input(input.clone()),
            scope: scope.to_vec(),
        },
        ClauseTerm::Input(_) => return Err(V2ClauseRefusal::UnsupportedCondition),
        ClauseTerm::Operation {
            identity,
            arguments,
            result_bound,
        } => {
            if identity == "quire.op.numeric.narrow" {
                let literal =
                    admitted_literal_narrow(term).ok_or(V2ClauseRefusal::UnsupportedCondition)?;
                return lower_in(literal, names, plan, scope);
            }
            let operation = clause_scalar_operation(identity, *result_bound)
                .ok_or(V2ClauseRefusal::UnsupportedCondition)?;
            let children = arguments
                .iter()
                .map(|child| lower_in(child, names, plan, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let actual = children
                .iter()
                .map(|index| {
                    plan.get(*index)
                        .map(|step| step.kind)
                        .ok_or(V2ClauseRefusal::Render)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (kind, expected) = match &operation {
                ExactScalarOperation::Boolean {
                    operator: BooleanOperator::Not,
                    ..
                } => (ValueKind::Boolean, vec![ValueKind::Boolean]),
                ExactScalarOperation::Boolean { .. } => {
                    (ValueKind::Boolean, vec![ValueKind::Boolean; 2])
                }
                ExactScalarOperation::Equality {
                    operands: EqualityOperandKind::Boolean,
                    ..
                } => (ValueKind::Boolean, vec![ValueKind::Boolean; 2]),
                ExactScalarOperation::Equality {
                    operands: EqualityOperandKind::Integer,
                    ..
                }
                | ExactScalarOperation::Ordering {
                    operands: OrderingOperandKind::Integer,
                    ..
                } => (ValueKind::Boolean, vec![ValueKind::Integer; 2]),
                ExactScalarOperation::IntegerArithmetic {
                    operator: crate::oracle::operator_catalog::IntegerOperator::Negate,
                    ..
                } => (ValueKind::Integer, vec![ValueKind::Integer]),
                ExactScalarOperation::IntegerArithmetic { .. }
                | ExactScalarOperation::IntegerModulo { .. } => {
                    (ValueKind::Integer, vec![ValueKind::Integer; 2])
                }
                _ => return Err(V2ClauseRefusal::UnsupportedCondition),
            };
            if actual != expected {
                return Err(V2ClauseRefusal::UnsupportedCondition);
            }
            Step {
                kind,
                term: StepTerm::Operation {
                    identity: identity.clone(),
                    operation,
                    arguments: children,
                },
                scope: scope.to_vec(),
            }
        }
    };
    let index = plan.len();
    plan.push(step);
    Ok(index)
}

const STOP_TYPE: &str = r#"
/// A generated condition stopped.
#[derive(Debug)]
pub enum ClauseStop {
    /// A typed runtime evaluation stop.
    Runtime(rt::Outcome<bool>),
    /// A scalar oracle construction stop.
    Oracle(OracleStop),
    /// An admitted integer literal could not be represented.
    InvalidConstant,
}
"#;

const THIN_STOP_TYPE: &str = r#"
/// A generated condition stopped. The kind carries no runtime payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseStop {
    /// A typed runtime evaluation was undefined.
    Undefined,
    /// A typed runtime evaluation refused with an integer out of its semantic domain.
    IntegerOutOfDomain,
    /// A typed runtime evaluation refused for another reason.
    Refused,
    /// A typed runtime evaluation did not complete within its meter.
    Incomplete,
    /// A scalar oracle construction stopped.
    Oracle,
    /// An admitted integer literal could not be represented.
    InvalidConstant,
}
"#;

const THIN_MEASURED_HELPER: &str = r#"
fn measured<T>(outcome: rt::Outcome<T>) -> Result<T, ClauseStop> {
    match outcome {
        rt::Outcome::Completed(value) => Ok(value),
        rt::Outcome::Undefined(reason) => {
            core::mem::forget(reason);
            Err(ClauseStop::Undefined)
        }
        rt::Outcome::Refused(reason) => {
            let kind = if matches!(&reason, rt::Refusal::IntegerOutOfDomain { .. }) {
                ClauseStop::IntegerOutOfDomain
            } else {
                ClauseStop::Refused
            };
            core::mem::forget(reason);
            Err(kind)
        }
        rt::Outcome::Incomplete(record) => {
            core::mem::forget(record);
            Err(ClauseStop::Incomplete)
        }
    }
}
"#;

const MEASURED_HELPER: &str = r#"
fn measured<T>(outcome: rt::Outcome<T>) -> Result<T, ClauseStop> {
    match outcome {
        rt::Outcome::Completed(value) => Ok(value),
        rt::Outcome::Undefined(reason) => Err(ClauseStop::Runtime(rt::Outcome::Undefined(reason))),
        rt::Outcome::Refused(reason) => Err(ClauseStop::Runtime(rt::Outcome::Refused(reason))),
        rt::Outcome::Incomplete(record) => Err(ClauseStop::Runtime(rt::Outcome::Incomplete(record))),
    }
}
"#;

/// The call of step `child` from a step that evaluates in the child's own scope, or, for the body of
/// a `let`, in that scope less the binder the `let` just bound (`local`, held as an owned value).
fn child_call(
    plan: &[Step],
    child: usize,
    arguments: &str,
    local: Option<usize>,
) -> Result<String, V2ClauseRefusal> {
    let step = plan.get(child).ok_or(V2ClauseRefusal::Render)?;
    let binders = step
        .scope
        .iter()
        .map(|bound| {
            if Some(bound.binder) == local && bound.kind == ValueKind::Integer {
                format!("&bound_{}, ", bound.binder)
            } else {
                format!("bound_{}, ", bound.binder)
            }
        })
        .collect::<String>();
    Ok(format!("value_{child}({arguments}{binders}meter)"))
}

fn render_step(
    source: &mut String,
    plan: &[Step],
    index: usize,
    step: &Step,
    names: &BTreeMap<ClauseInput, String>,
    parameters: &str,
    arguments: &str,
    policy: IntegerRenderPolicy,
) -> Result<(), V2ClauseRefusal> {
    let result = match step.kind {
        ValueKind::Boolean => "bool",
        ValueKind::Integer if policy.is_native() => "nx::NativeInteger",
        ValueKind::Integer => "rt::Integer",
    };
    let integer_type = if policy.is_native() {
        "nx::NativeInteger"
    } else {
        "rt::Integer"
    };
    let binder_parameters = step
        .scope
        .iter()
        .map(|bound| match bound.kind {
            ValueKind::Boolean => format!("bound_{}: bool, ", bound.binder),
            ValueKind::Integer => format!("bound_{}: &{integer_type}, ", bound.binder),
        })
        .collect::<String>();
    source.push_str(&format!("\nfn value_{index}({parameters}{binder_parameters}meter: &mut rt::Meter) -> Result<{result}, ClauseStop> {{\n"));
    source.push_str("    let _ = (&meter, ");
    for name in names.values() {
        source.push_str(name);
        source.push_str(", ");
    }
    for bound in &step.scope {
        source.push_str(&format!("bound_{}, ", bound.binder));
    }
    source.push_str(");\n");
    match &step.term {
        StepTerm::Boolean(value) => source.push_str(&format!("    Ok({value})\n")),
        StepTerm::Integer(value) if policy.is_native() => {
            let value = value.parse::<i128>().map_err(|_| V2ClauseRefusal::Render)?;
            source.push_str(&format!(
                "    Ok(nx::NativeInteger::new({}))\n",
                native_literal(value)
            ));
        }
        StepTerm::Integer(value) => {
            if let Some(integer) = value
                .parse::<i64>()
                .ok()
                .filter(|integer| integer.to_string() == *value)
            {
                let expression = if integer == i64::MIN {
                    "i64::MIN".to_owned()
                } else {
                    format!("{integer}_i64")
                };
                source.push_str(&format!("    Ok(rt::Integer::from({expression}))\n"));
            } else {
                source.push_str(&format!(
                    "    \"{value}\".parse().map_err(|_| ClauseStop::InvalidConstant)\n"
                ));
            }
        }
        StepTerm::Input(input) => source.push_str(&format!(
            "    Ok({}.clone())\n",
            names.get(input).ok_or(V2ClauseRefusal::Render)?
        )),
        StepTerm::OptionValue { presence, payload } => {
            let presence = names.get(presence).ok_or(V2ClauseRefusal::Render)?;
            let payload = names.get(payload).ok_or(V2ClauseRefusal::Render)?;
            let stop = if policy.is_native() {
                "ClauseStop::Undefined"
            } else {
                "ClauseStop::Runtime(rt::Outcome::Undefined(rt::Undefined::NoneValue))"
            };
            source.push_str(&format!(
                "    if !{presence} {{\n        return Err({stop});\n    }}\n    Ok({payload}.clone())\n"
            ));
        }
        StepTerm::Binder { binder, .. } => {
            source.push_str(&format!("    Ok(bound_{binder}.clone())\n"));
        }
        StepTerm::Let {
            binder,
            value,
            body,
        } => {
            source.push_str(&format!(
                "    let bound_{binder} = {}?;\n    {}\n",
                child_call(plan, *value, arguments, None)?,
                child_call(plan, *body, arguments, Some(*binder))?
            ));
        }
        StepTerm::If {
            condition,
            then,
            other,
        } => {
            source.push_str(&format!(
                "    if {}? {{\n        {}\n    }} else {{\n        {}\n    }}\n",
                child_call(plan, *condition, arguments, None)?,
                child_call(plan, *then, arguments, None)?,
                child_call(plan, *other, arguments, None)?
            ));
        }
        StepTerm::Case { scrutinee, arms } => {
            let selector = names.get(scrutinee).ok_or(V2ClauseRefusal::Render)?;
            let (last, leading) = arms.split_last().ok_or(V2ClauseRefusal::Render)?;
            if policy.is_native() {
                source.push_str(&format!("    match {selector}.value() {{\n"));
                for (member, arm) in leading.iter().enumerate() {
                    source.push_str(&format!(
                        "        {member} => {},\n",
                        child_call(plan, *arm, arguments, None)?
                    ));
                }
                // The index is an `i128` value: the last member's arm is the exhaustive pattern, and
                // no arm exists for a value outside the declared members.
                source.push_str(&format!(
                    "        _ => {},\n    }}\n",
                    child_call(plan, *last, arguments, None)?
                ));
            } else {
                for (member, arm) in leading.iter().enumerate() {
                    source.push_str(&format!(
                        "    if *{selector} == rt::Integer::from({member}_i64) {{\n        return {};\n    }}\n",
                        child_call(plan, *arm, arguments, None)?
                    ));
                }
                source.push_str(&format!(
                    "    {}\n",
                    child_call(plan, *last, arguments, None)?
                ));
            }
        }
        StepTerm::Operation {
            operation,
            arguments: children,
            ..
        } => {
            let oracle_stop = if policy.is_native() {
                "|_| ClauseStop::Oracle"
            } else {
                "ClauseStop::Oracle"
            };
            let arg = |position| -> Result<String, V2ClauseRefusal> {
                let child = children.get(position).ok_or(V2ClauseRefusal::Render)?;
                Ok(format!("{}?", child_call(plan, *child, arguments, None)?))
            };
            let call = match operation {
                ExactScalarOperation::Boolean {
                    operator: BooleanOperator::Not,
                    ..
                } => {
                    format!("let operand = {};\n    let outcome = scalar_{index}(operand, meter).map_err({oracle_stop})?;", arg(0)?)
                }
                ExactScalarOperation::Boolean { operator, .. } => {
                    let decided = match operator {
                        BooleanOperator::And => "!left",
                        BooleanOperator::Or => "left",
                        BooleanOperator::Implies => "!left",
                        BooleanOperator::Not => return Err(V2ClauseRefusal::Render),
                    };
                    format!("let left = {};\n    let right = if {decided} {{ false }} else {{ {} }};\n    let outcome = scalar_{index}(left, |_| rt::Outcome::Completed(right), meter).map_err({oracle_stop})?;", arg(0)?, arg(1)?)
                }
                ExactScalarOperation::IntegerArithmetic {
                    operator: crate::oracle::operator_catalog::IntegerOperator::Negate,
                    ..
                } => {
                    format!("let operand = {};\n    let outcome = scalar_{index}(&operand, meter).map_err({oracle_stop})?;", arg(0)?)
                }
                ExactScalarOperation::IntegerArithmetic { .. }
                | ExactScalarOperation::IntegerModulo { .. }
                | ExactScalarOperation::Ordering { .. }
                | ExactScalarOperation::Equality {
                    operands: EqualityOperandKind::Integer,
                    ..
                } => {
                    format!("let left = {};\n    let right = {};\n    let outcome = scalar_{index}(&left, &right, meter).map_err({oracle_stop})?;", arg(0)?, arg(1)?)
                }
                ExactScalarOperation::Equality {
                    operands: EqualityOperandKind::Boolean,
                    ..
                } => {
                    format!("let left = {};\n    let right = {};\n    let outcome = scalar_{index}(left, right, meter).map_err({oracle_stop})?;", arg(0)?, arg(1)?)
                }
                _ => return Err(V2ClauseRefusal::Render),
            };
            source.push_str("    ");
            source.push_str(&call);
            source.push_str("\n    measured(outcome)\n");
        }
    }
    source.push_str("}\n");
    Ok(())
}

fn native_literal(value: i128) -> String {
    if value == i128::MIN {
        "i128::MIN".to_owned()
    } else {
        format!("{value}_i128")
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[trace("FR-014-AC-54", "TC-024")]
    #[test]
    fn a_literal_narrow_is_elided_only_when_its_value_is_inside_the_target() {
        let narrow = |value: &str, bound| ClauseTerm::Operation {
            identity: "quire.op.numeric.narrow".to_owned(),
            arguments: vec![ClauseTerm::Integer(value.to_owned())],
            result_bound: bound,
        };
        let mut plan = Vec::new();
        let index = lower(&narrow("0", Some((0, 1000))), &BTreeMap::new(), &mut plan)
            .expect("admitted literal keeps its exact value");
        assert_eq!(index, 0);
        assert_eq!(plan.len(), 1);
        assert!(matches!(&plan[0].term, StepTerm::Integer(value) if value == "0"));
        let sibling = lower(&ClauseTerm::Boolean(true), &BTreeMap::new(), &mut plan)
            .expect("an independent sibling remains lowerable");
        assert_eq!(sibling, 1);
        assert_eq!(plan.len(), 2);

        for (value, low, high) in [
            ("-5", -10_i128, 10_i128),
            ("-9223372036854775808", i128::from(i64::MIN), 0),
            ("9223372036854775807", 0, i128::from(i64::MAX)),
            ("9223372036854775808", 0, i128::from(i64::MAX) + 10),
            ("-9223372036854775809", i128::from(i64::MIN) - 10, 0),
        ] {
            let mut edge = Vec::new();
            lower(
                &narrow(value, Some((low, high))),
                &BTreeMap::new(),
                &mut edge,
            )
            .unwrap_or_else(|_| panic!("{value} is inside [{low}, {high}]"));
            assert!(matches!(&edge[0].term, StepTerm::Integer(kept) if kept == value));
        }
        assert!(lower(
            &narrow("9223372036854775808", Some((0, i128::from(i64::MAX)))),
            &BTreeMap::new(),
            &mut Vec::new()
        )
        .is_err());

        for refused in [
            narrow("-1", Some((0, 1000))),
            narrow("1001", Some((0, 1000))),
            narrow("0", None),
            narrow("outside-i128", Some((0, 1000))),
            ClauseTerm::Operation {
                identity: "quire.op.numeric.narrow".to_owned(),
                arguments: Vec::new(),
                result_bound: Some((0, 1000)),
            },
            ClauseTerm::Operation {
                identity: "quire.op.numeric.narrow".to_owned(),
                arguments: vec![ClauseTerm::Input(ClauseInput {
                    leaf: ClauseLeaf::Whole,
                    name: "amount".to_owned(),
                    side: ClauseInputSide::Parameter,
                    declaring_node_id: None,
                    boolean: false,
                    lower: 0,
                    upper: 1000,
                })],
                result_bound: Some((0, 1000)),
            },
            ClauseTerm::Operation {
                identity: "quire.op.numeric.narrow".to_owned(),
                arguments: vec![ClauseTerm::Boolean(true)],
                result_bound: Some((0, 1000)),
            },
            ClauseTerm::Operation {
                identity: "quire.op.quantity.convert".to_owned(),
                arguments: vec![ClauseTerm::Integer("0".to_owned())],
                result_bound: Some((0, 1000)),
            },
        ] {
            assert!(matches!(
                lower(&refused, &BTreeMap::new(), &mut plan),
                Err(V2ClauseRefusal::UnsupportedCondition)
            ));
            assert_eq!(plan.len(), 2, "a refused term must not disturb a sibling");
            assert!(matches!(&plan[0].term, StepTerm::Integer(value) if value == "0"));
            assert!(matches!(&plan[1].term, StepTerm::Boolean(true)));
        }
    }

    fn kind_input() -> ClauseInput {
        ClauseInput {
            leaf: ClauseLeaf::Whole,
            name: "kind".to_owned(),
            side: ClauseInputSide::Parameter,
            declaring_node_id: None,
            boolean: false,
            lower: 0,
            upper: 2,
        }
    }

    /// FR-015-AC-154: a `case` over an arity-0 union index renders one arm per member in declaration
    /// order, the last arm holding the exhaustive pattern, and calls only the selected arm.
    #[trace("FR-015-AC-154", "TC-025")]
    #[test]
    fn a_case_over_a_union_index_renders_a_match_with_one_arm_per_member() {
        let input = kind_input();
        let names = BTreeMap::from([(input.clone(), "parameter_kind".to_owned())]);
        let term = ClauseTerm::Case {
            scrutinee: input,
            arms: vec![
                ClauseTerm::Integer("1".to_owned()),
                ClauseTerm::Integer("2".to_owned()),
                ClauseTerm::Integer("0".to_owned()),
            ],
        };
        let mut plan = Vec::new();
        let root = lower(&term, &names, &mut plan).expect("the case lowers");
        assert_eq!(plan.len(), 4, "three arm values and the case");
        let mut source = String::new();
        render_step(
            &mut source,
            &plan,
            root,
            &plan[root],
            &names,
            "parameter_kind: &nx::NativeInteger, ",
            "parameter_kind, ",
            IntegerRenderPolicy::Native,
        )
        .expect("render the case step");
        syn::parse_file(&source).expect("valid Rust");
        assert!(
            source.contains("match parameter_kind.value() {"),
            "{source}"
        );
        assert!(source.contains("0 => value_0(") && source.contains("1 => value_1("));
        assert!(
            source.contains("_ => value_2("),
            "the last member is the exhaustive arm"
        );
        assert_eq!(source.matches(" => ").count(), 3, "one arm per member");
    }

    /// FR-015-AC-168 and FR-015-AC-172: the `if` calls only the selected branch; the `let` calls
    /// its initializer once, before its body, and the body receives the binder.
    #[trace("FR-015-AC-168", "FR-015-AC-172", "TC-025")]
    #[test]
    fn an_if_calls_one_branch_and_a_let_calls_its_value_once_before_its_body() {
        let term = ClauseTerm::Let {
            binder: 0,
            value: Box::new(ClauseTerm::Integer("7".to_owned())),
            body: Box::new(ClauseTerm::If {
                condition: Box::new(ClauseTerm::Boolean(true)),
                then: Box::new(ClauseTerm::Binder { binder: 0 }),
                other: Box::new(ClauseTerm::Integer("9".to_owned())),
            }),
        };
        let mut plan = Vec::new();
        let root = lower(&term, &BTreeMap::new(), &mut plan).expect("the let lowers");
        let render = |index: usize| {
            let mut source = String::new();
            render_step(
                &mut source,
                &plan,
                index,
                &plan[index],
                &BTreeMap::new(),
                "",
                "",
                IntegerRenderPolicy::Native,
            )
            .expect("render");
            syn::parse_file(&source).expect("valid Rust");
            source
        };
        let rendered_let = render(root);
        assert_eq!(rendered_let.matches("let bound_0 = value_").count(), 1);
        assert!(rendered_let.contains("&bound_0, meter)"), "{rendered_let}");
        let rendered_if = render(root - 1);
        assert_eq!(
            rendered_if.matches("value_").count(),
            4,
            "one condition and two branch calls"
        );
        assert!(rendered_if.contains("if value_") && rendered_if.contains("} else {"));
    }

    fn integer_leaf(spelling: &str) -> String {
        let step = Step {
            kind: ValueKind::Integer,
            term: StepTerm::Integer(spelling.to_owned()),
            scope: Vec::new(),
        };
        let mut source = String::new();
        render_step(
            &mut source,
            &[],
            0,
            &step,
            &BTreeMap::new(),
            "",
            "",
            IntegerRenderPolicy::General,
        )
        .expect("render a standalone integer step");
        syn::parse_file(&source).expect("valid Rust leaf");
        source
    }

    #[trace("TC-024", "FR-014-AC-53")]
    #[test]
    fn canonical_i64_leaves_use_the_public_primitive_constructor() {
        let mut failures = Vec::new();
        for (spelling, value) in [
            ("-9223372036854775808", "i64::MIN"),
            ("-9223372036854775807", "-9223372036854775807_i64"),
            ("-17", "-17_i64"),
            ("-1", "-1_i64"),
            ("0", "0_i64"),
            ("1", "1_i64"),
            ("42", "42_i64"),
            ("1000", "1000_i64"),
            ("9223372036854775807", "9223372036854775807_i64"),
        ] {
            let source = integer_leaf(spelling);
            if !source.contains(&format!("Ok(rt::Integer::from({value}))")) {
                failures.push(spelling);
            }
            assert!(!source.contains("meter.charge"));
        }
        assert!(
            failures.is_empty(),
            "primitive constructor missing: {failures:?}"
        );
    }

    #[trace("TC-024", "FR-014-AC-53")]
    #[test]
    fn every_ineligible_leaf_keeps_the_original_parse_and_typed_failure() {
        for spelling in [
            "-9223372036854775809",
            "9223372036854775808",
            "-170141183460469231731687303715884105728",
            "170141183460469231731687303715884105727",
            "-170141183460469231731687303715884105729",
            "170141183460469231731687303715884105728",
            "+1",
            "01",
            "-0",
            " 1",
            "1 ",
            "1a",
            "",
            "--1",
        ] {
            let source = integer_leaf(spelling);
            assert!(
                source.contains(&format!(
                    "\"{spelling}\".parse().map_err(|_| ClauseStop::InvalidConstant)"
                )),
                "original fallback changed for {spelling:?}: {source}"
            );
            assert!(!source.contains("Integer::from("));
            assert!(!source.contains("meter.charge"));
        }
    }
}
