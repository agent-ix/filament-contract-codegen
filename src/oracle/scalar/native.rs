// SPDX-License-Identifier: AGPL-3.0-or-later
//! Production rendering of kernel-admitted native scalar arithmetic.

use filament_contract_model::CheckedNodeId;
use filament_exact::{
    DivisionMember, DivisionProfile, NativeIntegerInterval, NativeIntegerOperation,
    NativeIntegerPlan,
};

use crate::core::native_integer::{
    NativeIntegerIntervalDecision, NativeIntegerOperationDecision, NativeIntegerSelection,
};
use crate::oracle::operator_catalog::{IntegerOperator, Operator as CatalogOperator};

pub(crate) fn select(
    node_id: &CheckedNodeId,
    identity: &str,
    symbol: &str,
    operator: IntegerOperator,
    operands: &[(i64, i64)],
    result: (i64, i64),
) -> Option<(String, NativeIntegerSelection)> {
    let interval = |position: usize| {
        let &(lower, upper) = operands.get(position)?;
        NativeIntegerInterval::new(i128::from(lower), i128::from(upper)).ok()
    };
    let left = interval(0)?;
    let operation = match operator {
        IntegerOperator::Add => NativeIntegerOperation::Add {
            left,
            right: interval(1)?,
        },
        IntegerOperator::Subtract => NativeIntegerOperation::Subtract {
            left,
            right: interval(1)?,
        },
        IntegerOperator::Multiply => NativeIntegerOperation::Multiply {
            left,
            right: interval(1)?,
        },
        IntegerOperator::Negate => NativeIntegerOperation::Negate { operand: left },
    };
    let domain = NativeIntegerInterval::new(i128::from(result.0), i128::from(result.1)).ok()?;
    let plan = NativeIntegerPlan::new(operation, Some(domain)).ok()?;
    let mut selection = NativeIntegerSelection::new();
    selection.operations.push(NativeIntegerOperationDecision {
        node_id: node_id.clone(),
        step: 0,
        operation_identity: identity.to_owned(),
        operands: operands
            .iter()
            .map(|&(lower, upper)| NativeIntegerIntervalDecision {
                lower: lower.to_string(),
                upper: upper.to_string(),
            })
            .collect(),
        capacity: Some(plan.capacity().into()),
        semantic_domain: Some(domain.into()),
        division_profile: None,
    });
    Some((render(symbol, operator, &selection)?, selection))
}

fn literal(endpoint: i128) -> String {
    if endpoint == i128::MIN {
        "i128::MIN".to_owned()
    } else {
        format!("{endpoint}_i128")
    }
}

fn interval_source(interval: NativeIntegerInterval) -> String {
    format!(
        "nx::NativeIntegerInterval::new({}, {}).map_err(OracleStop::NativeAdmission)?",
        literal(interval.lower()),
        literal(interval.upper())
    )
}

fn decision_interval(interval: &NativeIntegerIntervalDecision) -> Option<NativeIntegerInterval> {
    NativeIntegerInterval::new(interval.lower.parse().ok()?, interval.upper.parse().ok()?).ok()
}

/// Render a selected plan for both the routed artifact and its proof harness.
pub(crate) fn render(
    symbol: &str,
    operator: IntegerOperator,
    selection: &NativeIntegerSelection,
) -> Option<String> {
    let [decision] = selection.operations.as_slice() else {
        return None;
    };
    let identity = decision.operation_identity.as_str();
    if CatalogOperator::from_identity(identity)?
        .entry()
        .scalar_operator
        != Some(operator)
    {
        return None;
    }
    let (parameters, descriptor, payloads) = match (operator, decision.operands.as_slice()) {
        (IntegerOperator::Negate, [operand]) => (
            "operand: &nx::NativeInteger",
            format!(
                "nx::NativeIntegerOperation::Negate {{ operand: {} }}",
                interval_source(decision_interval(operand)?)
            ),
            "nx::NativeIntegerOperands::Unary { operand: *operand }",
        ),
        (operator, [left, right]) => {
            let variant = match operator {
                IntegerOperator::Add => "Add",
                IntegerOperator::Subtract => "Subtract",
                IntegerOperator::Multiply => "Multiply",
                IntegerOperator::Negate => return None,
            };
            (
                "left: &nx::NativeInteger, right: &nx::NativeInteger",
                format!(
                    "nx::NativeIntegerOperation::{variant} {{ left: {}, right: {} }}",
                    interval_source(decision_interval(left)?),
                    interval_source(decision_interval(right)?)
                ),
                "nx::NativeIntegerOperands::Binary { left: *left, right: *right }",
            )
        }
        _ => return None,
    };
    let domain = decision_interval(decision.semantic_domain.as_ref()?)?;
    let capacity = decision_interval(decision.capacity.as_ref()?)?;
    let operation = match (operator, decision.operands.as_slice()) {
        (IntegerOperator::Negate, [operand]) => NativeIntegerOperation::Negate {
            operand: decision_interval(operand)?,
        },
        (IntegerOperator::Add, [left, right]) => NativeIntegerOperation::Add {
            left: decision_interval(left)?,
            right: decision_interval(right)?,
        },
        (IntegerOperator::Subtract, [left, right]) => NativeIntegerOperation::Subtract {
            left: decision_interval(left)?,
            right: decision_interval(right)?,
        },
        (IntegerOperator::Multiply, [left, right]) => NativeIntegerOperation::Multiply {
            left: decision_interval(left)?,
            right: decision_interval(right)?,
        },
        _ => return None,
    };
    if NativeIntegerPlan::new(operation, Some(domain))
        .ok()?
        .capacity()
        != capacity
    {
        return None;
    }
    let mut source = format!(
        "// @generated by filament-contract-codegen. Do not edit.\nuse filament_exact as rt;\nuse filament_exact as nx;\n\n/// Why native admission stopped.\n#[derive(Debug)]\npub enum OracleStop {{\n    /// The exact kernel rejected the plan or operand membership.\n    NativeAdmission(nx::NativeIntegerAdmissionError),\n}}\n\n/// Admitted checked {identity} operation.\npub fn {symbol}({parameters}, meter: &mut rt::Meter) -> Result<rt::Outcome<nx::NativeInteger>, OracleStop> {{\n    let plan = nx::NativeIntegerPlan::new({descriptor}, Some({})).map_err(OracleStop::NativeAdmission)?;\n    let admitted = plan.admit({payloads}).map_err(OracleStop::NativeAdmission)?;\n    Ok(nx::evaluate_native_integer_arithmetic(admitted, meter))\n}}\n",
        interval_source(domain)
    );
    source.push_str(&selection.annotation()?);
    let subject = crate::core::provenance::node_subject(&decision.node_id);
    Some(crate::core::provenance::with_rust_header(
        source,
        [subject.as_str()],
    ))
}

/// The native division family of one claim: `div` and `rem` under their law, and `mod`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeDivision {
    /// The quotient under the claim's law.
    Divide(DivisionProfile),
    /// The remainder under the claim's law.
    Remainder(DivisionProfile),
    /// The Euclidean remainder.
    Modulo,
}

impl NativeDivision {
    fn profile(self) -> Option<DivisionProfile> {
        match self {
            Self::Divide(profile) | Self::Remainder(profile) => Some(profile),
            Self::Modulo => None,
        }
    }

    /// The division family of a catalogued identity and the law its decision recorded.
    fn of(identity: &str, profile: Option<&str>) -> Option<Self> {
        let profile = || {
            DivisionProfile::ALL
                .into_iter()
                .find(|law| Some(law.definition_identity()) == profile)
        };
        match identity {
            "filament.op.integer.div" => profile().map(Self::Divide),
            "filament.op.integer.rem" => profile().map(Self::Remainder),
            "filament.op.integer.mod" => Some(Self::Modulo),
            _ => None,
        }
    }

    fn operation(
        self,
        dividend: NativeIntegerInterval,
        divisor: NativeIntegerInterval,
    ) -> NativeIntegerOperation {
        match self {
            Self::Divide(profile) => NativeIntegerOperation::Divide {
                profile,
                member: DivisionMember::Quotient,
                dividend,
                divisor,
            },
            Self::Remainder(profile) => NativeIntegerOperation::Divide {
                profile,
                member: DivisionMember::Remainder,
                dividend,
                divisor,
            },
            Self::Modulo => NativeIntegerOperation::Modulo { dividend, divisor },
        }
    }

    fn source(self, dividend: NativeIntegerInterval, divisor: NativeIntegerInterval) -> String {
        let (dividend, divisor) = (interval_source(dividend), interval_source(divisor));
        match self {
            Self::Divide(profile) | Self::Remainder(profile) => {
                let member = if matches!(self, Self::Divide(_)) {
                    "Quotient"
                } else {
                    "Remainder"
                };
                format!(
                    "nx::NativeIntegerOperation::Divide {{ profile: rt::DivisionProfile::{profile:?}, member: rt::DivisionMember::{member}, dividend: {dividend}, divisor: {divisor} }}"
                )
            }
            Self::Modulo => format!(
                "nx::NativeIntegerOperation::Modulo {{ dividend: {dividend}, divisor: {divisor} }}"
            ),
        }
    }
}

/// Select the kernel's native division or modulo plan from the full operand intervals and the
/// claim's result range (FR-057). A zero divisor is an evaluation outcome, so a divisor interval
/// holding zero is admitted.
pub(crate) fn select_division(
    node_id: &CheckedNodeId,
    identity: &str,
    symbol: &str,
    operation: NativeDivision,
    operands: &[(i64, i64)],
    result: (i64, i64),
) -> Option<(String, NativeIntegerSelection)> {
    let interval = |position: usize| {
        let &(lower, upper) = operands.get(position)?;
        NativeIntegerInterval::new(i128::from(lower), i128::from(upper)).ok()
    };
    let [_, _] = operands else { return None };
    let plan_operation = operation.operation(interval(0)?, interval(1)?);
    let domain = NativeIntegerInterval::new(i128::from(result.0), i128::from(result.1)).ok()?;
    let plan = NativeIntegerPlan::new(plan_operation, Some(domain)).ok()?;
    let mut selection = NativeIntegerSelection::new();
    selection.operations.push(NativeIntegerOperationDecision {
        node_id: node_id.clone(),
        step: 0,
        operation_identity: identity.to_owned(),
        operands: operands
            .iter()
            .map(|&(lower, upper)| NativeIntegerIntervalDecision {
                lower: lower.to_string(),
                upper: upper.to_string(),
            })
            .collect(),
        capacity: Some(plan.capacity().into()),
        semantic_domain: Some(domain.into()),
        division_profile: operation
            .profile()
            .map(|law| law.definition_identity().to_owned()),
    });
    Some((render_division(symbol, &selection)?, selection))
}

/// Render the selected native source of an integer claim: arithmetic through `render`, division
/// through `render_division`. The selection alone decides, so the routed artifact and the proof
/// harness embed the same bytes.
pub(crate) fn render_selected(symbol: &str, selection: &NativeIntegerSelection) -> Option<String> {
    let [decision] = selection.operations.as_slice() else {
        return None;
    };
    match CatalogOperator::from_identity(&decision.operation_identity)?
        .entry()
        .scalar_operator
    {
        Some(operator) => render(symbol, operator, selection),
        None => render_division(symbol, selection),
    }
}

/// Render a selected division or modulo plan.
pub(crate) fn render_division(symbol: &str, selection: &NativeIntegerSelection) -> Option<String> {
    let [decision] = selection.operations.as_slice() else {
        return None;
    };
    let identity = decision.operation_identity.as_str();
    let operation = NativeDivision::of(identity, decision.division_profile.as_deref())?;
    let [dividend, divisor] = decision.operands.as_slice() else {
        return None;
    };
    let (dividend, divisor) = (decision_interval(dividend)?, decision_interval(divisor)?);
    let domain = decision_interval(decision.semantic_domain.as_ref()?)?;
    let capacity = decision_interval(decision.capacity.as_ref()?)?;
    if NativeIntegerPlan::new(operation.operation(dividend, divisor), Some(domain))
        .ok()?
        .capacity()
        != capacity
    {
        return None;
    }
    let descriptor = operation.source(dividend, divisor);
    let mut source = format!(
        "// @generated by filament-contract-codegen. Do not edit.\nuse filament_exact as rt;\nuse filament_exact as nx;\n\n/// Why native admission stopped.\n#[derive(Debug)]\npub enum OracleStop {{\n    /// The exact kernel rejected the plan or operand membership.\n    NativeAdmission(nx::NativeIntegerAdmissionError),\n}}\n\n/// Admitted checked {identity} operation.\npub fn {symbol}(left: &nx::NativeInteger, right: &nx::NativeInteger, meter: &mut rt::Meter) -> Result<rt::Outcome<nx::NativeInteger>, OracleStop> {{\n    let plan = nx::NativeIntegerPlan::new({descriptor}, Some({})).map_err(OracleStop::NativeAdmission)?;\n    let admitted = plan.admit(nx::NativeIntegerOperands::Binary {{ left: *left, right: *right }}).map_err(OracleStop::NativeAdmission)?;\n    Ok(nx::evaluate_native_integer_arithmetic(admitted, meter))\n}}\n",
        interval_source(domain)
    );
    source.push_str(&selection.annotation()?);
    let subject = crate::core::provenance::node_subject(&decision.node_id);
    Some(crate::core::provenance::with_rust_header(
        source,
        [subject.as_str()],
    ))
}
