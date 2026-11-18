//! The integer division, remainder and modulo scalar harness (FR-057).
//!
//! The harness proves that the claim's embedded oracle agrees with an independent statement of the
//! operation over every `i64` operand pair in the claim's operand ranges. The independent side
//! shares nothing with the oracle or the kernel and divides nowhere at 128 bits:
//!
//! - `div` is checked relationally. Each law has a remainder window `W`, a set of exactly `|b|`
//!   consecutive integers, and exactly one quotient `q*` has `a - q * b` in `W`. A completed `q`
//!   is checked by that membership, and whether `q*` lies in the result range is decided from the
//!   products of the range's endpoints with the divisor, with no division.
//! - `rem` and `mod` take the `i64` Euclidean remainder (with its `b == -1` arm, the one
//!   overflowing case) and shift it into the window over `i128`.

use filament_exact::DivisionProfile;

use super::{LoweredScalarClaim, ScalarOperation};
use crate::kani::{
    abi::{i64_literal, KaniPrimitiveType, KaniSolver},
    generate::{
        outcome::{KaniObligationRequest, UnsupportedObligation},
        record::{harness_path, record},
        render,
        spec::{
            Argument, Cover, ExprText, HarnessSpec, Init, LetBinding, LetType, ProofKind, Statement,
        },
    },
    identity::{
        DivisionScalarIdentity, KaniScalarObligationHarness, ScalarObligationArgument,
        ScalarObligationIdentity,
    },
};

/// The remainder window of one law as the body of `expected_window(a, b)`, over `i128` operands.
/// `reach` is `|b| - 1`, the window's width less one, and is the largest remainder magnitude.
fn window_arms(profile: DivisionProfile) -> &'static str {
    match profile {
        DivisionProfile::Truncating => "if a >= 0 { (0, reach) } else { (-reach, 0) }",
        DivisionProfile::Floor => "if b > 0 { (0, reach) } else { (-reach, 0) }",
        DivisionProfile::Euclidean => "(0, reach)",
    }
}

/// The profile whose window the expected side uses: the law for `div` and `rem`, Euclidean for
/// `mod`.
fn window_profile(operation: ScalarOperation) -> DivisionProfile {
    match operation {
        ScalarOperation::Divide(profile) | ScalarOperation::Remainder(profile) => profile,
        _ => DivisionProfile::Euclidean,
    }
}

/// The independent expected side of a division claim: plain functions over `i64` and `i128` that
/// name no kernel, oracle or runtime path. `lower` and `upper` are the result range.
pub(super) fn expected_side(operation: ScalarOperation, lower: i64, upper: i64) -> String {
    let window = format!(
        "fn expected_window(a: i128, b: i128) -> (i128, i128) {{\n    let reach = (if b < 0 {{ -b }} else {{ b }}) - 1;\n    {}\n}}\n",
        window_arms(window_profile(operation))
    );
    let zero = "fn expected_zero_divisor(b: i64) -> bool {\n    b == 0\n}\n";
    let body = match operation {
        ScalarOperation::Divide(_) => format!(
            "fn expected_admitted(a: i128, b: i128) -> bool {{\n    let (window_low, window_high) = expected_window(a, b);\n    let at_lower = a - i128::from({lower}) * b;\n    let at_upper = a - i128::from({upper}) * b;\n    if b > 0 {{\n        at_lower >= window_low && at_upper <= window_high\n    }} else {{\n        at_lower <= window_high && at_upper >= window_low\n    }}\n}}\nfn expected_member(a: i128, b: i128, q: i128) -> bool {{\n    let (window_low, window_high) = expected_window(a, b);\n    let r = a - q * b;\n    r >= window_low && r <= window_high\n}}\n",
            lower = i64_literal(lower),
            upper = i64_literal(upper),
        ),
        _ => "fn expected_remainder(a: i64, b: i64) -> i128 {\n    let euclid = if b == -1 { 0_i128 } else { i128::from(i64::rem_euclid(a, b)) };\n    let (window_low, window_high) = expected_window(i128::from(a), i128::from(b));\n    if euclid >= window_low && euclid <= window_high {\n        euclid\n    } else {\n        euclid - (if b < 0 { -i128::from(b) } else { i128::from(b) })\n    }\n}\n".to_owned(),
    };
    format!("// Independent expected side (FR-057): i128 relations, no kernel path.\n{window}{zero}{body}")
}

/// The outcome mapping of a division claim, as one function over the oracle's result: exactly the
/// outcomes FR-057 allows are accepted and every other outcome, including `Incomplete` and a
/// refusal with another cause or member, is rejected.
pub(super) fn outcome_mapping(
    operation: ScalarOperation,
    lower: i64,
    upper: i64,
    native: bool,
) -> String {
    let carrier = if native {
        "nx::NativeInteger"
    } else {
        "rt::Integer"
    };
    let signature = format!("fn outcome_agrees<E>(left: i64, right: i64, outcome: &Result<rt::Outcome<{carrier}>, E>) -> bool {{\n    let zero_divisor = expected_zero_divisor(right);\n");
    let (setup, completed, refused) = match operation {
        ScalarOperation::Divide(_) => (
            "    let admitted = !zero_divisor && expected_admitted(i128::from(left), i128::from(right));\n".to_owned(),
            if native {
                format!("        Ok(rt::Outcome::Completed(value)) => admitted && value.value() >= i128::from({}) && value.value() <= i128::from({}) && expected_member(i128::from(left), i128::from(right), value.value()),\n", i64_literal(lower), i64_literal(upper))
            } else {
                "        Ok(rt::Outcome::Completed(value)) => match (value.is_negative(), value.abs().to_u64()) {\n            (false, Some(magnitude)) => admitted && expected_member(i128::from(left), i128::from(right), i128::from(magnitude)),\n            (true, Some(magnitude)) => admitted && expected_member(i128::from(left), i128::from(right), -i128::from(magnitude)),\n            _ => false,\n        },\n".to_owned()
            },
            "        Ok(rt::Outcome::Refused(rt::Refusal::DivisionOutOfDomain { member: rt::DivisionMember::Quotient, .. })) => !zero_divisor && !admitted,\n",
        ),
        ScalarOperation::Remainder(_) | ScalarOperation::Modulo => {
            let refusal = if operation == ScalarOperation::Modulo {
                "        Ok(rt::Outcome::Refused(rt::Refusal::ModuloOutOfDomain { .. })) => !zero_divisor && !admitted,\n"
            } else {
                "        Ok(rt::Outcome::Refused(rt::Refusal::DivisionOutOfDomain { member: rt::DivisionMember::Remainder, .. })) => !zero_divisor && !admitted,\n"
            };
            (
                format!(
                    "    let expected = if zero_divisor {{ 0 }} else {{ expected_remainder(left, right) }};\n    let admitted = !zero_divisor && expected >= i128::from({}) && expected <= i128::from({});\n",
                    i64_literal(lower),
                    i64_literal(upper)
                ),
                if native {
                    "        Ok(rt::Outcome::Completed(value)) => admitted && value.value() == expected,\n".to_owned()
                } else {
                    "        Ok(rt::Outcome::Completed(value)) => admitted && *value == rt::Integer::from(expected),\n".to_owned()
                },
                refusal,
            )
        }
        _ => (String::new(), String::new(), ""),
    };
    format!(
        "{signature}{setup}    match outcome {{\n        Ok(rt::Outcome::Undefined(rt::Undefined::DivisionByZero)) => zero_divisor,\n{completed}{refused}        _ => false,\n    }}\n}}\n"
    )
}

/// Renders the harness of one lowered division claim.
pub(super) fn render(
    request: &KaniObligationRequest<'_>,
    lowered: &LoweredScalarClaim,
) -> Result<KaniScalarObligationHarness, UnsupportedObligation> {
    let expr = |text: &str| ExprText::new(text.to_owned());
    let binding = |name: &str, ty, init| LetBinding {
        name: name.to_owned(),
        ty,
        mutable: false,
        init,
    };
    let scalar_limits = crate::kani::abi::SCALAR_HARNESS_LIMITS;
    let operation = lowered.operation;
    let native = lowered.native_integer_selection.is_some();
    let mut body = Vec::new();
    for (name, (minimum, maximum)) in operation.operand_names().iter().zip(&lowered.operands) {
        body.push(Statement::Input {
            name: (*name).to_owned(),
            ty: KaniPrimitiveType::I64,
            domain: (i128::from(*minimum), i128::from(*maximum)),
            then: vec![
                binding(
                    &format!("{name}_native"),
                    None,
                    Init::Expression(expr(name)?),
                ),
                binding(
                    name,
                    None,
                    Init::Expression(expr(&if native {
                        format!("nx::NativeInteger::new(i128::from({name}))")
                    } else {
                        format!("rt::Integer::from({name})")
                    })?),
                ),
            ],
        });
    }
    let rendered_guards = lowered.rendered_guards();
    body.extend(super::guard_statements(&rendered_guards)?);
    body.push(Statement::Let(LetBinding {
        name: "meter".to_owned(),
        ty: None,
        mutable: true,
        init: Init::ScalarMeter(scalar_limits),
    }));
    let mut args = operation
        .operand_names()
        .iter()
        .map(|name| Argument::Borrow((*name).to_owned()))
        .collect::<Vec<_>>();
    args.push(Argument::BorrowMut("meter".to_owned()));
    body.push(Statement::Call {
        result: "outcome".to_owned(),
        callee: lowered.oracle_symbol.clone(),
        args,
    });
    body.push(Statement::Let(binding(
        "agrees",
        Some(LetType::Boolean),
        Init::Expression(expr("outcome_agrees(left_native, right_native, &outcome)")?),
    )));
    body.push(Statement::Assert {
        condition: expr("agrees")?,
        message: format!(
            "the oracle must agree with the independent {} expectation: DivisionByZero at a zero divisor, the member when it lies in the checked domain, and the typed out-of-domain refusal otherwise",
            lowered.operation_identity
        ),
    });
    body.push(Statement::Let(binding(
        "completed",
        None,
        Init::Expression(expr("matches!(outcome, Ok(rt::Outcome::Completed(_)))")?),
    )));
    if native {
        // The meter and the outcome hold symbolic-discriminant sum types, one with a heap arm. The
        // native proof never drops them; the cover reads the plain `bool` above and stays last.
        body.push(Statement::Forget("meter".to_owned()));
        body.push(Statement::Forget("outcome".to_owned()));
    }
    let support = format!(
        "\n{}\n{}",
        expected_side(operation, lowered.lower, lowered.upper),
        outcome_mapping(operation, lowered.lower, lowered.upper, native)
    );
    let spec = HarnessSpec::new(
        format!(
            "exact-scalar {} `{}`",
            super::super::clause::v2::message_safe_node_label(&lowered.node_id),
            lowered.operation_identity
        ),
        harness_path(&lowered.module_symbol, &lowered.harness_symbol)?,
        format!("{}{support}", lowered.oracle_source),
        ProofKind::Proof,
        body,
        vec![Cover {
            condition: expr("completed")?,
            message: "the generated oracle's Ok(Outcome::Completed(_)) branch is reachable within its checked domain".to_owned(),
        }],
        request.unwind,
        KaniSolver::Cadical,
    )?;
    let rust = render::render(&spec)?;
    let profile = match operation {
        ScalarOperation::Divide(profile) | ScalarOperation::Remainder(profile) => {
            Some(profile.definition_identity().to_owned())
        }
        _ => None,
    };
    let identity = ScalarObligationIdentity {
        integer_representation: if native {
            crate::kani::identity::ScalarIntegerRepresentation::Native
        } else {
            crate::kani::identity::ScalarIntegerRepresentation::General
        },
        native_integer_selection: lowered.native_integer_selection.clone(),
        ceilings: request.ceilings,
        node_id: lowered.node_id.clone(),
        operation_identity: lowered.operation_identity.clone(),
        oracle_symbol: lowered.oracle_symbol.clone(),
        module_symbol: spec.path().module.clone(),
        harness_symbol: spec.path().harness.clone(),
        arguments: operation
            .operand_names()
            .iter()
            .zip(&lowered.operands)
            .map(|(name, (minimum, maximum))| ScalarObligationArgument {
                identifier: (*name).to_owned(),
                minimum: *minimum,
                maximum: *maximum,
            })
            .collect(),
        inferred_operands: lowered.inferred_operands.clone(),
        requirement_occurrence: lowered.requirement_occurrence.clone(),
        path_condition: lowered.path_condition(),
        guard_inputs: rendered_guards
            .inputs
            .iter()
            .map(|input| input.identity.clone())
            .collect(),
        enum_scalar: None,
        float_scalar: None,
        division_scalar: Some(DivisionScalarIdentity {
            profile,
            result_minimum: lowered.lower,
            result_maximum: lowered.upper,
        }),
        solver: spec.solver(),
        unwind: spec.unwind(),
        options: spec.options().to_vec(),
        witness_options: spec.witness_options().to_vec(),
    };
    let record = record(&lowered.module_symbol, &identity, &rust)?;
    Ok(KaniScalarObligationHarness::generated_with_scalar_limits(
        identity,
        rust,
        record,
        scalar_limits,
    ))
}
