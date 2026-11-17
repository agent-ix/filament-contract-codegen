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
    if decision.division_profile.is_some() {
        return None;
    }
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
        let profile_of = || {
            DivisionProfile::ALL
                .into_iter()
                .find(|law| Some(law.definition_identity()) == profile)
        };
        match identity {
            "filament.op.integer.div" => profile_of().map(Self::Divide),
            "filament.op.integer.rem" => profile_of().map(Self::Remainder),
            // A `mod` carries no law: a recorded one is a selection this generator did not build.
            "filament.op.integer.mod" => profile.is_none().then_some(Self::Modulo),
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
        // Only `div` and `rem` record a law; an arithmetic decision carrying one was not built
        // by `select`.
        Some(_) if decision.division_profile.is_some() => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn node() -> CheckedNodeId {
        serde_json::from_value(serde_json::json!({
            "package": "native-division-render-test",
            "path": ["operation"],
            "ordinal": 0,
        }))
        .expect("checked node identity")
    }

    fn division(identity: &str, operation: NativeDivision) -> NativeIntegerSelection {
        select_division(
            &node(),
            identity,
            "oracle",
            operation,
            &[(-8, 7), (1, 3)],
            (-8, 8),
        )
        .expect("admitted native division plan")
        .1
    }

    /// A recorded law belongs to `div` and `rem` only: a `mod` or an arithmetic decision carrying
    /// one, and a `div` or `rem` without one, render nothing.
    #[trace("TC-071", "FR-057-AC-6")]
    #[test]
    fn a_division_profile_is_rendered_only_for_div_and_rem() {
        let law = DivisionProfile::Floor.definition_identity().to_owned();
        for (identity, operation) in [
            (
                "filament.op.integer.div",
                NativeDivision::Divide(DivisionProfile::Floor),
            ),
            (
                "filament.op.integer.rem",
                NativeDivision::Remainder(DivisionProfile::Floor),
            ),
            ("filament.op.integer.mod", NativeDivision::Modulo),
        ] {
            let healthy = division(identity, operation);
            assert!(render_selected("oracle", &healthy).is_some(), "{identity}");
            let mut flipped = healthy.clone();
            flipped.operations[0].division_profile = match operation {
                NativeDivision::Modulo => Some(law.clone()),
                _ => None,
            };
            assert!(render_selected("oracle", &flipped).is_none(), "{identity}");
            assert!(render_division("oracle", &flipped).is_none(), "{identity}");
        }
        let (_, mut add) = select(
            &node(),
            "filament.op.integer.add",
            "oracle",
            IntegerOperator::Add,
            &[(0, 9), (1, 1)],
            (0, 10),
        )
        .expect("admitted native add plan");
        assert!(render_selected("oracle", &add).is_some());
        add.operations[0].division_profile = Some(law);
        assert!(render_selected("oracle", &add).is_none());
        assert!(render("oracle", IntegerOperator::Add, &add).is_none());
    }
}
