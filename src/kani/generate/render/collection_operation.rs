// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emission of whole exact-value model states and typed collection operation calls.

use super::super::{
    outcome::UnsupportedObligation,
    spec::{CollectionOperationSpec, CollectionSpec, HarnessSpec},
};
use crate::{
    core::artifact::{Artifact, MAX_GENERATED_SOURCE_BYTES},
    kani::{
        abi::i64_literal,
        identity::{
            CollectionElementIdentity, CollectionIfBranch, CollectionKindIdentity,
            CollectionObjectFieldIdentity, CollectionOperationConditionIdentity,
            CollectionPredicateIdentity, SymbolicBounds,
        },
    },
};
use std::fmt::Write;

fn primitive_type(domain: &CollectionElementIdentity) -> &'static str {
    match domain {
        CollectionElementIdentity::Boolean => "bool",
        CollectionElementIdentity::Integer { .. } => "i64",
    }
}

fn bounds_type(bounds: &SymbolicBounds) -> Option<String> {
    match bounds {
        SymbolicBounds::Boolean => Some("filament_exact::ValueType::Boolean".into()),
        SymbolicBounds::Integer { minimum, maximum } => Some(format!("filament_exact::ValueType::Int(filament_exact::IntegerInterval::new(filament_exact::Integer::from({}), filament_exact::Integer::from({})).expect(\"checked interval\"))", i64_literal(*minimum), i64_literal(*maximum))),
        SymbolicBounds::Sequence { .. } => None,
    }
}

fn domain_check(value: &str, bounds: &SymbolicBounds) -> Option<String> {
    match bounds {
        SymbolicBounds::Boolean => Some(format!("matches!({value}, filament_exact::Value::Boolean(_))")),
        SymbolicBounds::Integer { minimum, maximum } => Some(format!("matches!({value}, filament_exact::Value::Integer(value) if value >= &filament_exact::Integer::from({}) && value <= &filament_exact::Integer::from({}))", i64_literal(*minimum), i64_literal(*maximum))),
        SymbolicBounds::Sequence { .. } => None,
    }
}

fn scalar_bounds(domain: &CollectionElementIdentity) -> SymbolicBounds {
    match domain {
        CollectionElementIdentity::Boolean => SymbolicBounds::Boolean,
        CollectionElementIdentity::Integer { minimum, maximum } => SymbolicBounds::Integer {
            minimum: *minimum,
            maximum: *maximum,
        },
    }
}

fn draw_scalar(source: &mut String, name: &str, domain: &SymbolicBounds) -> Option<String> {
    match domain {
        SymbolicBounds::Boolean => {
            writeln!(source, "let {name}: bool = kani::any();").ok()?;
            Some(format!("filament_exact::Value::Boolean({name})"))
        }
        SymbolicBounds::Integer { minimum, maximum } => {
            writeln!(
                source,
                "let {name}: i64 = kani::any();\nkani::assume({name} >= {} && {name} <= {});",
                i64_literal(*minimum),
                i64_literal(*maximum)
            )
            .ok()?;
            Some(format!(
                "filament_exact::Value::Integer(filament_exact::Integer::from({name}))"
            ))
        }
        SymbolicBounds::Sequence { .. } => None,
    }
}

fn collection_schema(source: &mut String, field: &CollectionObjectFieldIdentity) -> Option<()> {
    let SymbolicBounds::Sequence {
        minimum_length,
        maximum_length,
        element,
    } = &field.bounds
    else {
        return Some(());
    };
    let ty = bounds_type(element)?;
    let check = domain_check("value", element)?;
    let kind = match field.collection_kind? {
        CollectionKindIdentity::Sequence => "Sequence",
        CollectionKindIdentity::OrderedSet => "OrderedSet",
    };
    writeln!(source, "fn type_{}() -> filament_exact::CollectionType {{ filament_exact::CollectionType::new(filament_exact::CollectionKind::{kind}, {ty}, Some(filament_exact::CardinalityBound::new({minimum_length}, {maximum_length}).expect(\"checked cardinality\"))) }}", field.slot).ok()?;
    writeln!(source, "fn admit_{}(ty: &filament_exact::ValueType, value: &filament_exact::Value) -> Result<(), filament_exact::ConstructionCause> {{\nif ty == &{ty} && {check} {{ Ok(()) }} else {{ Err(filament_exact::ConstructionCause::TypeMismatch) }}\n}}", field.slot).ok()?;
    Some(())
}

fn draw_field(source: &mut String, field: &CollectionObjectFieldIdentity) -> Option<()> {
    let name = &field.slot;
    match &field.bounds {
        SymbolicBounds::Sequence {
            minimum_length,
            maximum_length,
            element,
        } => {
            let (ty, conversion, assumption) = match element.as_ref() {
                SymbolicBounds::Boolean => (
                    "bool",
                    "filament_exact::Value::Boolean(item)".to_owned(),
                    String::new(),
                ),
                SymbolicBounds::Integer { minimum, maximum } => (
                    "i64",
                    "filament_exact::Value::Integer(filament_exact::Integer::from(item))"
                        .to_owned(),
                    format!(
                        "kani::assume(item >= {} && item <= {});",
                        i64_literal(*minimum),
                        i64_literal(*maximum)
                    ),
                ),
                SymbolicBounds::Sequence { .. } => return None,
            };
            writeln!(source, "let {name}_len: i64 = kani::any();\nkani::assume({name}_len >= {minimum_length} && {name}_len <= {maximum_length});\nlet {name}_items: [{ty}; {maximum_length}] = kani::any();\nlet mut {name}_occurrences = Vec::new();\nfor index in 0..{name}_len {{\nlet item = {name}_items[index as usize];\n{assumption}").ok()?;
            match field.collection_kind? {
                CollectionKindIdentity::Sequence => {}
                CollectionKindIdentity::OrderedSet => {
                    writeln!(source, "for prior in 0..index {{ kani::assume(item != {name}_items[prior as usize]); }}").ok()?;
                }
            }
            writeln!(source, "{name}_occurrences.push({conversion});\n}}\nlet {name} = match filament_exact::build_typed_prestate(&type_{name}(), {name}_occurrences, &mut meter(), Some(&mut admit_{name})) {{\nOk(filament_exact::Outcome::Completed(value)) => value, _ => panic!(\"the complete admitted prestate must construct\"),\n}};").ok()?;
        }
        bounds => {
            let value = draw_scalar(source, &format!("{name}_input"), bounds)?;
            writeln!(source, "let {name} = {value};").ok()?;
        }
    }
    Some(())
}

fn predicate(predicate: &CollectionPredicateIdentity) -> String {
    match predicate {
        CollectionPredicateIdentity::IntegerGreaterOrEqual { threshold } => format!(
            "integer(item) >= &filament_exact::Integer::from({})",
            i64_literal(*threshold)
        ),
        CollectionPredicateIdentity::IntegerEqual { value } => format!(
            "integer(item) == &filament_exact::Integer::from({})",
            i64_literal(*value)
        ),
        CollectionPredicateIdentity::IntegerIfAtMost {
            split,
            then_branch,
            else_branch,
            bound,
        } => {
            let branch = |branch: &CollectionIfBranch| match branch {
                CollectionIfBranch::Element => "integer(item).clone()".to_owned(),
                CollectionIfBranch::Literal { value } => {
                    format!("filament_exact::Integer::from({})", i64_literal(*value))
                }
            };
            format!(
                "(if integer(item) < &filament_exact::Integer::from({}) {{ {} }} else {{ {} }}) <= filament_exact::Integer::from({})",
                i64_literal(*split),
                branch(then_branch),
                branch(else_branch),
                i64_literal(*bound)
            )
        }
        CollectionPredicateIdentity::BooleanNotEqual { value } => {
            format!("boolean(item) != {value}")
        }
    }
}

fn quantifier(
    field: usize,
    operation: &str,
    claim: &CollectionPredicateIdentity,
    state: &str,
) -> Option<String> {
    let method = match operation {
        "filament.op.collection.forall" => "all",
        "filament.op.collection.exists" => "any",
        _ => return None,
    };
    Some(format!(
        "collection(&{state}.field_{field}).elements().iter().{method}(|item| {})",
        predicate(claim)
    ))
}

fn condition_expression(
    condition: &CollectionOperationConditionIdentity,
    state: &str,
) -> Option<String> {
    match condition {
        CollectionOperationConditionIdentity::Scalar { expression } => {
            Some(expression.as_str().to_owned())
        }
        CollectionOperationConditionIdentity::Quantifier {
            field,
            operation,
            predicate,
        } => quantifier(*field, operation, predicate, state),
        CollectionOperationConditionIdentity::And(left, right) => Some(format!(
            "({}) && ({})",
            condition_expression(left, state)?,
            condition_expression(right, state)?
        )),
    }
}

fn emit(
    spec: &HarnessSpec,
    _selected: &CollectionSpec,
    operation: &CollectionOperationSpec,
) -> Option<String> {
    let path = spec.path();
    let mut source = format!("// SPDX-License-Identifier: MIT OR Apache-2.0\n#[cfg(kani)]\nmod {} {{\nuse super::*;\n#[derive(Clone)]\nstruct CgObjectState {{\n", path.module);
    for field in &operation.identity.fields {
        writeln!(source, "{}: filament_exact::Value,", field.slot).ok()?;
    }
    source.push_str("}\nfn integer(value: &filament_exact::Value) -> &filament_exact::Integer { match value { filament_exact::Value::Integer(value) => value, _ => panic!(\"checked integer field\") } }\nfn boolean(value: &filament_exact::Value) -> bool { match value { filament_exact::Value::Boolean(value) => *value, _ => panic!(\"checked Boolean field\") } }\nfn collection(value: &filament_exact::Value) -> &filament_exact::CollectionValue { match value { filament_exact::Value::Collection(value) => value, _ => panic!(\"checked collection field\") } }\nfn meter() -> filament_exact::Meter { filament_exact::Meter::new(filament_exact::ScalarLimits { integer_bits:u64::MAX, decimal_digits:u64::MAX, scale_expansion:u64::MAX, text_input_bytes:u64::MAX, text_scalars:u64::MAX, normalized_scalars:u64::MAX, unit_edges:u64::MAX, value_occurrences:u64::MAX, work_units:u64::MAX, result_units:u64::MAX }) }\n");
    for field in &operation.identity.fields {
        collection_schema(&mut source, field)?;
    }
    source.push_str("fn structural_equal(left: &filament_exact::Value, right: &filament_exact::Value) -> bool { match filament_exact::planned_equality(left, right, &mut meter()) { filament_exact::Outcome::Completed(value) => value, _ => panic!(\"the admitted collection equality must complete\") } }\n");
    let mut oracle_parameters = vec![
        "state: &CgObjectState".to_owned(),
        "pre: &CgObjectState".to_owned(),
    ];
    let mut oracle_arguments = operation
        .identity
        .parameters
        .iter()
        .map(|value| value.binding.clone())
        .collect::<Vec<_>>();
    for value in &operation.identity.parameters {
        oracle_parameters.push(format!(
            "{}: {}",
            value.binding,
            primitive_type(&value.domain)
        ));
    }
    if let Some(value) = &operation.identity.result {
        oracle_parameters.push(format!("subject_result: {}", primitive_type(&value.domain)));
        oracle_arguments.push("subject_result".into());
    }
    let oracle_arguments = if oracle_arguments.is_empty() {
        String::new()
    } else {
        format!(", {}", oracle_arguments.join(", "))
    };
    writeln!(source, "#[allow(dead_code)] enum ClauseEvaluation {{ Completed(bool), Undefined, Refused, Incomplete }}\nfn oracle_clause({}) -> ClauseEvaluation {{", oracle_parameters.join(", ")).ok()?;
    let claim = condition_expression(&operation.identity.condition, "state")?;
    writeln!(
        source,
        "let claim = {claim};\nClauseEvaluation::Completed(claim)\n}}\n#[kani::proof]\nfn {}() {{",
        path.harness
    )
    .ok()?;
    for field in &operation.identity.fields {
        draw_field(&mut source, field)?;
    }
    for parameter in &operation.identity.parameters {
        draw_scalar(
            &mut source,
            &parameter.binding,
            &scalar_bounds(&parameter.domain),
        )?;
    }
    source.push_str("let pre = CgObjectState {\n");
    for field in &operation.identity.fields {
        writeln!(source, "{},", field.slot).ok()?;
    }
    source.push_str("};\n");
    if operation.postcondition {
        for condition in &operation.identity.precondition_bodies {
            let condition = condition_expression(condition, "pre")?;
            writeln!(source, "kani::assume({condition});").ok()?;
        }
        source.push_str("let mut post = pre.clone();\n");
        let mut arguments = operation
            .identity
            .fields
            .iter()
            .map(|field| format!("&mut post.{}", field.slot))
            .collect::<Vec<_>>();
        arguments.extend(
            operation
                .identity
                .parameters
                .iter()
                .map(|parameter| parameter.binding.clone()),
        );
        // The subject is a typed function item, so Rust checks every field, parameter and result.
        let result_type = operation
            .identity
            .result
            .as_ref()
            .map_or("()", |result| primitive_type(&result.domain));
        writeln!(
            source,
            "let subject_result: {result_type} = {}({});",
            selected_subject(spec)?,
            arguments.join(", ")
        )
        .ok()?;
        for field in &operation.identity.fields {
            match &field.bounds {
                SymbolicBounds::Sequence { .. } => {
                    writeln!(source, "assert!(matches!(filament_exact::inspect_typed_poststate(&type_{}(), &post.{}, &mut meter(), Some(&mut admit_{})), Ok(filament_exact::Outcome::Completed(_))), \"every produced collection retains its admitted type and domain\");", field.slot, field.slot, field.slot).ok()?;
                }
                bounds => {
                    writeln!(source, "assert!({}, \"every produced field retains its admitted type and domain\");", domain_check(&format!("&post.{}", field.slot), bounds)?).ok()?;
                }
            }
        }
        if let Some(result) = &operation.identity.result {
            if let CollectionElementIdentity::Integer { minimum, maximum } = result.domain {
                writeln!(source, "assert!(subject_result >= {} && subject_result <= {}, \"the produced result retains its admitted domain\");", i64_literal(minimum), i64_literal(maximum)).ok()?;
            }
        }
        writeln!(source, "let outcome = oracle_clause(&post, &pre{oracle_arguments});\nassert!(matches!(outcome, ClauseEvaluation::Completed(true)), \"the produced state satisfies the authored collection postcondition\");").ok()?;
        writeln!(
            source,
            "kani::cover!({}, \"{}\");",
            spec.cover().condition.as_str(),
            spec.cover().message
        )
        .ok()?;
    } else {
        writeln!(
            source,
            "let outcome = oracle_clause(&pre, &pre{oracle_arguments});\n{}\nkani::cover!({}, \"{}\");",
            crate::kani::identity::collection_precondition_totality_assertion(),
            spec.cover().condition.as_str(),
            spec.cover().message
        )
        .ok()?;
    }
    source.push_str("}\n}\n");
    Some(source)
}

// The validated subject path is retained with the operation description, not parsed from text.
fn selected_subject(spec: &HarnessSpec) -> Option<&str> {
    let super::super::spec::HarnessShape::CollectionOperation { operation, .. } = spec.shape()
    else {
        return None;
    };
    Some(&operation.subject_path)
}

pub(super) fn render(
    spec: &HarnessSpec,
    selected: &CollectionSpec,
    operation: &CollectionOperationSpec,
) -> Result<Artifact, UnsupportedObligation> {
    let source = emit(spec, selected, operation).ok_or(UnsupportedObligation::RenderFailed)?;
    if source.len() > MAX_GENERATED_SOURCE_BYTES {
        return Err(UnsupportedObligation::ResourceLimitExceeded {
            bytes: source.len(),
        });
    }
    syn::parse_file(&source).map_err(|error| UnsupportedObligation::InvalidGeneratedSyntax {
        error: error.to_string(),
    })?;
    Ok(Artifact::new(
        format!("src/generated/{}.rs", spec.path().module),
        source,
    ))
}
