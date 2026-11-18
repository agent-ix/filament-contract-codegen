// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-057 V2 Kani integer div/rem/mod scalar harnesses over genuine FSL sources admitted through
//! Contract IR and routed through `generate_routed`.

use filament_contract_codegen::{
    KaniScalarObligationHarness, ObligationDisposition, RoutedGeneration, UnsupportedObligation,
};
use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_exact::DivisionProfile;
use fsl_replay::{
    admit_integer_division, compile_package_with_integer_division, CompileUnit, DefinitionRole,
    DependencyInput, ReplayLimits, SourceIdentity, StageLimits,
};
use ix_trace_rs::trace;

use super::{
    exact_function_agreement::body_id,
    routed_generation::{admitted_routing_input, generate_routed, kani_context, kani_parts, route},
};

pub(crate) fn role(profile: DivisionProfile) -> DefinitionRole {
    match profile {
        DivisionProfile::Truncating => DefinitionRole::IntegerDivisionTruncating,
        DivisionProfile::Floor => DefinitionRole::IntegerDivisionFloor,
        DivisionProfile::Euclidean => DefinitionRole::IntegerDivisionEuclidean,
    }
}

/// One function of the corpus source: `name using v(parameters): Integer pure { expression }`.
pub(crate) fn source(functions: &[(&str, &str, &str, &str)]) -> String {
    let mut text = String::from(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n",
    );
    for (name, parameters, result, expression) in functions {
        text.push_str(&format!(
            "function {name} using v({parameters}): {result} pure {{ {expression} }}\n"
        ));
    }
    text
}

pub(crate) fn compile(text: &str, profile: DivisionProfile) -> CheckedPackageV2 {
    let unit = CompileUnit {
        source: SourceIdentity::new("test", "integer-division-harness"),
        package_name: "integer-division-harness".to_owned(),
        path: "integer-division-harness.native",
        bytes: text.as_bytes(),
    };
    let selection = admit_integer_division(&[role(profile).reference()], None)
        .expect("owner admits one actual division definition");
    let compiled = compile_package_with_integer_division(
        unit,
        [],
        &DependencyInput::default(),
        &selection,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("FSL compiles the division source");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    match CheckedPackageV2::read(
        compiled.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("IR admits the FSL-emitted bytes: {other:?}"),
    }
}

/// The `identity` application node inside `function`'s source region (the body root is its
/// narrowing conversion when the function declares a bounded result).
pub(crate) fn application_in(
    package: &CheckedPackageV2,
    function: &str,
    identity: &str,
) -> filament_contract_model::CheckedNodeId {
    use filament_contract_model::CheckedTerm;
    let declaration = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            node.declaration
                .as_ref()
                .is_some_and(|decl| decl.qualified_name == [Box::<str>::from(function)])
        })
        .expect("the function declaration");
    let regions = package
        .source_map()
        .iter()
        .filter(|entry| entry.node_id == declaration.node_id)
        .flat_map(|entry| &entry.regions)
        .collect::<Vec<_>>();
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            matches!(&node.body, CheckedTerm::Application { operation, .. }
                if operation.identity() == Some(identity))
                && package
                    .source_map()
                    .iter()
                    .filter(|entry| entry.node_id == node.node_id)
                    .any(|entry| {
                        entry.regions.iter().any(|region| {
                            regions.iter().any(|outer| {
                                outer.source == region.source
                                    && outer.start <= region.start
                                    && region.end <= outer.end
                            })
                        })
                    })
        })
        .unwrap_or_else(|| panic!("{function} holds a {identity} node"))
        .node_id
        .clone()
}

/// Routes `function` of `package` as one request and returns the generation.
pub(crate) fn routed(package: &CheckedPackageV2, function: &str, unwind: u32) -> RoutedGeneration {
    routed_node(package, &body_id(package, function), unwind)
}

pub(crate) fn routed_node(
    package: &CheckedPackageV2,
    node: &filament_contract_model::CheckedNodeId,
    unwind: u32,
) -> RoutedGeneration {
    routed_nodes(package, std::slice::from_ref(node), unwind, false)
}

/// Routes each of `nodes` as one request through the general or the native entry.
pub(crate) fn routed_nodes(
    package: &CheckedPackageV2,
    nodes: &[filament_contract_model::CheckedNodeId],
    unwind: u32,
    native: bool,
) -> RoutedGeneration {
    let items = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| route(index, node))
        .collect::<Vec<_>>();
    let input = admitted_routing_input(package, &items);
    if native {
        let bound = items
            .iter()
            .cloned()
            .map(|item| match &input.positions[item.request_index] {
                filament_contract_codegen::ContractRequestPosition::DriverItem {
                    result_selection,
                    ..
                } => item.with_scalar_selection(&input.package, result_selection.clone()),
                _ => item,
            })
            .collect::<Vec<_>>();
        let requirements = crate::common::requirements::project(input.source, &input.positions);
        let records = crate::common::requirements::records(&requirements);
        let contexts = kani_context("crate::subject", unwind);
        return filament_contract_codegen::generate_routed_native(
            input.source,
            &input.package,
            &input.positions,
            &crate::common::requirements::bind(bound, &requirements),
            &crate::common::requirements::with_records(&contexts, &records),
        )
        .expect("the native routed call generates");
    }
    generate_routed(
        input.source,
        &input.package,
        &input.positions,
        &items,
        &kani_context("crate::subject", unwind),
    )
    .expect("the routed call generates")
}

pub(crate) fn harness_of(
    generation: &RoutedGeneration,
) -> Result<
    (
        &KaniScalarObligationHarness,
        filament_contract_codegen::Artifact,
    ),
    String,
> {
    let manifest = generation
        .oracle_artifacts
        .as_ref()
        .and_then(|artifacts| artifacts.iter().find(|a| a.path == "Cargo.toml"))
        .cloned();
    let (record, harness) = kani_parts(&generation.items[0].output);
    match (harness, manifest) {
        (Some(harness), Some(manifest)) => Ok((harness, manifest)),
        _ => Err(format!("{:?}", record.disposition)),
    }
}

/// The result range of a harness's identity, as `(minimum, maximum)`.
fn result_range(harness: &KaniScalarObligationHarness) -> (i64, i64) {
    let division = harness
        .identity
        .division_scalar
        .as_ref()
        .expect("a division harness carries its division identity");
    (division.result_minimum, division.result_maximum)
}

/// The part of a generated harness a host crate can compile: the oracle, the expected side and
/// the outcome mapping, without the `cfg(kani)` module.
fn host_prefix(harness: &KaniScalarObligationHarness) -> &str {
    let text = harness.rust.contents.as_str();
    &text[..text
        .find("#[cfg(kani)]")
        .expect("the Kani module follows the support")]
}

/// The function with `header` in generated source, to its closing brace in column 0.
fn function_text<'a>(text: &'a str, header: &str) -> &'a str {
    let start = text
        .find(header)
        .unwrap_or_else(|| panic!("{header} in the harness"));
    let end = text[start..].find("\n}\n").expect("a closing brace") + start + 3;
    &text[start..end]
}

/// `text` with `from` replaced by `to`, confirmed to have changed.
fn mutate(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "the mutation site {from:?} is in the source"
    );
    let mutated = text.replace(from, to);
    assert_ne!(mutated, text, "the mutation changed the source");
    mutated
}

/// One division shape. A shape with an inferred result is compiled from FSL source; a shape
/// with an explicit narrowed result is a reader-admitted fixture, because the FSL range prover
/// does not admit that narrowing.
pub(crate) struct Shape {
    pub(crate) profile: DivisionProfile,
    pub(crate) operation: &'static str,
    pub(crate) left: (i64, i64),
    pub(crate) right: (i64, i64),
    pub(crate) narrow: Option<(i64, i64)>,
    pub(crate) unwind: u32,
}

impl Shape {
    pub(crate) fn operation_identity(&self) -> String {
        format!("filament.op.integer.{}", self.operation)
    }

    pub(crate) fn generation(&self) -> RoutedGeneration {
        if let Some(narrow) = self.narrow {
            let profile = (self.operation != "mod").then_some(self.profile);
            let (package, node) = super::integer_division_bounds::reader_division_operation(
                915_000,
                &self.operation_identity(),
                profile,
                Some(self.left),
                Some(self.right),
                Some(narrow),
            );
            return routed_node(&package, &node, self.unwind);
        }
        let parameters = format!(
            "x: Int[{}, {}], y: Int[{}, {}]",
            self.left.0, self.left.1, self.right.0, self.right.1
        );
        let text = source(&[(
            "subject",
            &parameters,
            "Integer",
            &format!("x {} y", self.operation),
        )]);
        routed(&compile(&text, self.profile), "subject", self.unwind)
    }
}

const HOST_TEST: &str = r#"
#[cfg(test)]
mod host {
    use super::*;
@MUTANTS@
    fn meter(work: u64) -> rt::Meter {
        rt::Meter::new(rt::ScalarLimits {
            integer_bits: u64::MAX, decimal_digits: u64::MAX, scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX, text_scalars: u64::MAX, normalized_scalars: u64::MAX,
            unit_edges: u64::MAX, value_occurrences: u64::MAX, work_units: work, result_units: u64::MAX,
        })
    }
    fn run(a: i64, b: i64, work: u64) -> Result<rt::Outcome<rt::Integer>, OracleStop> {
        @SYMBOL@(&rt::Integer::from(a), &rt::Integer::from(b), &mut meter(work))
    }
    fn domain() -> Box<rt::IntegerInterval> {
        Box::new(rt::IntegerInterval::new(rt::Integer::from(@LOWER@_i64), rt::Integer::from(@UPPER@_i64)).unwrap())
    }
    #[test]
    fn real_outcomes_agree_and_wrong_outcomes_disagree() {
        let mut completed = None;
        let mut refused = None;
        let mut zero = None;
        for a in @AL@_i64..=@AH@_i64 {
            for b in @BL@_i64..=@BH@_i64 {
                let outcome = run(a, b, u64::MAX);
                assert!(outcome_agrees(a, b, &outcome), "{a} {b}: {outcome:?}");
                match &outcome {
                    Ok(rt::Outcome::Completed(_)) => completed = completed.or(Some((a, b))),
                    Ok(rt::Outcome::Refused(_)) => refused = refused.or(Some((a, b))),
                    Ok(rt::Outcome::Undefined(_)) => zero = zero.or(Some((a, b))),
                    other => panic!("a real outcome {other:?}"),
                }
            }
        }
        let (ca, cb) = completed.expect("a pair completes");
        let (ra, rb) = refused.expect("a pair is refused");
        let (za, zb) = zero.expect("the divisor range holds zero");
        let wrong_value = || -> Result<rt::Outcome<rt::Integer>, OracleStop> { Ok(rt::Outcome::Completed(rt::Integer::from(1_000_003_i64))) };
        let by_zero = || -> Result<rt::Outcome<rt::Integer>, OracleStop> { Ok(rt::Outcome::Undefined(rt::Undefined::DivisionByZero)) };
        let same_cause = || -> Result<rt::Outcome<rt::Integer>, OracleStop> { Ok(rt::Outcome::Refused(@REFUSAL@)) };
        let other_member = || -> Result<rt::Outcome<rt::Integer>, OracleStop> { Ok(rt::Outcome::Refused(@OTHER_REFUSAL@)) };
        let other_cause = || -> Result<rt::Outcome<rt::Integer>, OracleStop> { Ok(rt::Outcome::Refused(rt::Refusal::IntegerOutOfDomain { target: domain() })) };
        assert!(!outcome_agrees(ca, cb, &wrong_value()), "a wrong value");
        assert!(!outcome_agrees(ca, cb, &by_zero()), "a zero-divisor outcome at a non-zero divisor");
        assert!(!outcome_agrees(ca, cb, &same_cause()), "a refusal inside the domain");
        let incomplete = run(ca, cb, 0);
        assert!(matches!(incomplete, Ok(rt::Outcome::Incomplete(_))), "a real Incomplete: {incomplete:?}");
        assert!(!outcome_agrees(ca, cb, &incomplete), "an Incomplete");
        assert!(!outcome_agrees::<OracleStop>(za, zb, &Ok(rt::Outcome::Completed(rt::Integer::from(0_i64)))), "Completed at zero");
        assert!(!outcome_agrees(za, zb, &same_cause()), "a refusal at zero");
        assert!(!outcome_agrees(ra, rb, &wrong_value()), "Completed outside the domain");
        assert!(!outcome_agrees(ra, rb, &by_zero()), "a zero-divisor outcome outside the domain");
        assert!(!outcome_agrees(ra, rb, &other_member()), "the other member's refusal");
        assert!(!outcome_agrees(ra, rb, &other_cause()), "another cause");
        @MUTANT_CHECKS@
    }
}
"#;

/// A host crate over `harness`: the generated support plus the grid over `operands`, run against
/// the real `filament-exact`. Each mutant is `(name, mutated mapping, what it wrongly accepts)`.
fn host_crate(
    harness: &KaniScalarObligationHarness,
    operands: ((i64, i64), (i64, i64)),
    mutants: &[(&str, String, &str)],
    member: Option<&str>,
) -> String {
    let (lower, upper) = result_range(harness);
    let ((al, ah), (bl, bh)) = operands;
    let divide = |member: &str| {
        format!("rt::Refusal::DivisionOutOfDomain {{ domain: domain(), member: rt::DivisionMember::{member} }}")
    };
    let (refusal, other) = match member {
        Some("Quotient") => (divide("Quotient"), divide("Remainder")),
        Some(_) => (divide("Remainder"), divide("Quotient")),
        None => (
            "rt::Refusal::ModuloOutOfDomain { domain: domain() }".to_owned(),
            divide("Quotient"),
        ),
    };
    let mut checks = String::new();
    let mut functions = String::new();
    for (name, text, accepts) in mutants {
        let call = match *accepts {
            "zero-at-nonzero" => "ca, cb, &by_zero()",
            "wrong-value" => "ca, cb, &wrong_value()",
            "refusal-in-domain" => "ca, cb, &same_cause()",
            "incomplete" => "ca, cb, &incomplete",
            "other-member" => "ra, rb, &other_member()",
            other => panic!("{other}"),
        };
        checks.push_str(&format!(
            "assert!({name}({call}), \"{name} accepts {accepts}\");\n        "
        ));
        functions.push_str(&text.replacen("fn outcome_agrees<E>", &format!("fn {name}<E>"), 1));
        functions.push('\n');
    }
    let test = HOST_TEST
        .replace("@MUTANTS@", &functions)
        .replace("@SYMBOL@", &harness.identity.oracle_symbol)
        .replace("@LOWER@", &lower.to_string())
        .replace("@UPPER@", &upper.to_string())
        .replace("@AL@", &al.to_string())
        .replace("@AH@", &ah.to_string())
        .replace("@BL@", &bl.to_string())
        .replace("@BH@", &bh.to_string())
        .replace("@REFUSAL@", &refusal)
        .replace("@OTHER_REFUSAL@", &other)
        .replace("@MUTANT_CHECKS@", &checks);
    format!("{}\n{test}", host_prefix(harness))
}

/// The mapping mutants of AC-2 for `harness`, each a wrong mapping that must be caught.
fn mapping_mutants(
    harness: &KaniScalarObligationHarness,
    member: Option<&str>,
) -> Vec<(&'static str, String, &'static str)> {
    let mapping = function_text(&harness.rust.contents, "fn outcome_agrees<E>").to_owned();
    let is_division = harness
        .identity
        .operation_identity
        .contains("filament.op.integer.div")
        || harness.identity.operation_identity.contains("division");
    let _ = is_division;
    let mut mutants = vec![
        (
            "m_zero_any",
            mutate(
                &mapping,
                "DivisionByZero)) => zero_divisor,",
                "DivisionByZero)) => true,",
            ),
            "zero-at-nonzero",
        ),
        (
            "m_refusal_any",
            mutate(&mapping, "=> !zero_divisor && !admitted,", "=> true,"),
            "refusal-in-domain",
        ),
        (
            "m_otherwise_true",
            mutate(&mapping, "_ => false,", "_ => true,"),
            "incomplete",
        ),
    ];
    let value_check = if mapping.contains("expected_member(") {
        mutate(
            &mapping,
            "admitted && expected_member(",
            "true || expected_member(",
        )
    } else {
        mutate(
            &mapping,
            "admitted && *value == rt::Integer::from(expected)",
            "true",
        )
    };
    mutants.push(("m_value_any", value_check, "wrong-value"));
    if let Some(member) = member {
        mutants.push((
            "m_member_any",
            mutate(
                &mapping,
                &format!("member: rt::DivisionMember::{member}, .."),
                "member: _, ..",
            ),
            "other-member",
        ));
    }
    mutants
}

fn run_host(shape: &Shape, operands: ((i64, i64), (i64, i64)), member: Option<&str>, label: &str) {
    let generation = shape.generation();
    let (harness, _) = harness_of(&generation).expect("a division harness");
    let mutants = mapping_mutants(harness, member);
    let crate_text = host_crate(harness, operands, &mutants, member);
    super::integer_division_bounds::run_generated_native(&crate_text, 1, label);
}

#[trace("TC-071", "FR-057-AC-2", "FR-057-AC-4")]
#[test]
fn host_controls_each_law_and_member_against_filament_exact() {
    for (index, (profile, operation, narrow, member)) in [
        (
            DivisionProfile::Truncating,
            "div",
            (-3, 3),
            Some("Quotient"),
        ),
        (DivisionProfile::Floor, "div", (-3, 3), Some("Quotient")),
        (DivisionProfile::Euclidean, "div", (-3, 3), Some("Quotient")),
        (
            DivisionProfile::Truncating,
            "rem",
            (-1, 1),
            Some("Remainder"),
        ),
        (DivisionProfile::Floor, "rem", (-1, 1), Some("Remainder")),
        (DivisionProfile::Euclidean, "rem", (0, 1), Some("Remainder")),
        (DivisionProfile::Euclidean, "mod", (0, 1), None),
    ]
    .into_iter()
    .enumerate()
    {
        let shape = Shape {
            profile,
            operation,
            left: (-9, 9),
            right: (-4, 4),
            narrow: Some(narrow),
            unwind: 12,
        };
        run_host(
            &shape,
            ((-9, 9), (-4, 4)),
            member,
            &format!("ir854-host-{index}"),
        );
    }
}

const DIFF_TEST: &str = r#"
#[cfg(test)]
mod diff {
    use super::*;
    fn meter() -> rt::Meter {
        rt::Meter::new(rt::ScalarLimits {
            integer_bits: u64::MAX, decimal_digits: u64::MAX, scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX, text_scalars: u64::MAX, normalized_scalars: u64::MAX,
            unit_edges: u64::MAX, value_occurrences: u64::MAX, work_units: u64::MAX, result_units: u64::MAX,
        })
    }
    fn domain() -> rt::IntegerDomain {
        rt::IntegerDomain::Bounded(rt::IntegerInterval::new(rt::Integer::from(@LOWER@_i64), rt::Integer::from(@UPPER@_i64)).unwrap())
    }
    fn real(a: i64, b: i64) -> rt::Outcome<rt::Integer> {
        @REAL@
    }
    fn quotient(value: &rt::Integer) -> i128 {
        let magnitude = i128::from(value.abs().to_u64().expect("a quotient within i64 range"));
        if value.is_negative() { -magnitude } else { magnitude }
    }
    fn vectors_hold() -> bool {
        let mut holds = true;
        for (a, b, expected) in [@VECTORS@] {
            let outcome = real(a, b);
            let inside = i128::from(expected) >= i128::from(@LOWER@_i64) && i128::from(expected) <= i128::from(@UPPER@_i64);
            holds &= if inside { outcome == rt::Outcome::Completed(rt::Integer::from(expected)) } else { matches!(outcome, rt::Outcome::Refused(_)) };
            @VECTOR_CHECK@
        }
        holds
    }
    #[test]
    fn differential() {
        if @MUTATED@ {
            assert!(!vectors_hold(), "a swapped window table must fail the named vectors");
            return;
        }
        assert!(vectors_hold(), "the named vectors");
        let values = [0_i64, 1, -1, 2, -2, 7, -7, i64::MIN, i64::MIN + 1, i64::MAX, 3, -3];
        for a in values {
            for b in values {
                let outcome = real(a, b);
                assert!(outcome_agrees::<OracleStop>(a, b, &Ok(outcome.clone())), "{a} {b}: {outcome:?}");
                @UNIQUE@
                let oracle = @ORACLE@(&rt::Integer::from(a), &rt::Integer::from(b), &mut meter());
                assert_eq!(oracle.unwrap(), outcome, "the generated oracle equals the owner at {a} {b}");
            }
        }
    }
}
"#;

/// The expected side (from its marker to the outcome mapping) of a generated harness.
fn expected_side(harness: &KaniScalarObligationHarness) -> &str {
    let text = harness.rust.contents.as_str();
    let start = text
        .find("// Independent expected side")
        .expect("the expected side marker");
    let end = text.find("fn outcome_agrees").expect("the outcome mapping");
    &text[start..end]
}

/// The window arms of each law, restated here independently of the generator.
const WINDOW_ARMS: [(DivisionProfile, &str); 3] = [
    (
        DivisionProfile::Truncating,
        "if a >= 0 { (0, reach) } else { (-reach, 0) }",
    ),
    (
        DivisionProfile::Floor,
        "if b > 0 { (0, reach) } else { (-reach, 0) }",
    ),
    (DivisionProfile::Euclidean, "(0, reach)"),
];

fn window_arms(profile: DivisionProfile) -> &'static str {
    WINDOW_ARMS
        .iter()
        .find(|(candidate, _)| *candidate == profile)
        .map(|(_, arms)| *arms)
        .unwrap()
}

/// The `(dividend, divisor, expected)` vectors of FR-057-AC-4 for one operation and law.
fn named_vectors(operation: &str, profile: DivisionProfile) -> [(i64, i64, i64); 2] {
    let pick = |truncating: [i64; 2], floor: [i64; 2], euclidean: [i64; 2]| {
        let row = match profile {
            DivisionProfile::Truncating => truncating,
            DivisionProfile::Floor => floor,
            DivisionProfile::Euclidean => euclidean,
        };
        [(-7, 2, row[0]), (-7, -2, row[1])]
    };
    match operation {
        "div" => pick([-3, 3], [-4, 3], [-4, 4]),
        "rem" => pick([-1, -1], [1, -1], [1, 1]),
        _ => [(-7, 2, 1), (-7, -2, 1)],
    }
}

/// One differential module: the harness's support compiled as host Rust and compared with the
/// real `filament-exact` evaluation over the grid and the named vectors. With `swapped`, the
/// expected side's window table is the one of another law and the vectors must then fail.
fn diff_module(name: &str, shape: &Shape, swapped: bool) -> String {
    let generation = shape.generation();
    let (harness, _) = harness_of(&generation).expect("a division harness");
    let (lower, upper) = result_range(harness);
    let mut prefix = host_prefix(harness).to_owned();
    let window_profile = if shape.operation == "mod" {
        DivisionProfile::Euclidean
    } else {
        shape.profile
    };
    if swapped {
        let other = match window_profile {
            DivisionProfile::Truncating => DivisionProfile::Floor,
            _ => DivisionProfile::Truncating,
        };
        prefix = mutate(&prefix, window_arms(window_profile), window_arms(other));
    }
    let profile = format!("{:?}", shape.profile);
    let vectors = named_vectors(shape.operation, shape.profile)
        .iter()
        .map(|(a, b, expected)| format!("({a}_i64, {b}_i64, {expected}_i64)"))
        .collect::<Vec<_>>()
        .join(", ");
    let (real, vector_check, unique) = match shape.operation {
        "div" => (
            format!("rt::divide(rt::DivisionProfile::{profile}, rt::DivisionMember::Quotient, &rt::Integer::from(a), &rt::Integer::from(b), &domain(), &mut meter())"),
            "holds &= expected_member(i128::from(a), i128::from(b), i128::from(expected)) && !expected_member(i128::from(a), i128::from(b), i128::from(expected) - 1) && !expected_member(i128::from(a), i128::from(b), i128::from(expected) + 1);".to_owned(),
            "if let rt::Outcome::Completed(value) = &outcome { let q = quotient(value); assert!(expected_member(i128::from(a), i128::from(b), q) && !expected_member(i128::from(a), i128::from(b), q - 1) && !expected_member(i128::from(a), i128::from(b), q + 1), \"{a} {b}: exactly one member of the window\"); }".to_owned(),
        ),
        "rem" => (
            format!("rt::divide(rt::DivisionProfile::{profile}, rt::DivisionMember::Remainder, &rt::Integer::from(a), &rt::Integer::from(b), &domain(), &mut meter())"),
            "holds &= expected_remainder(a, b) == i128::from(expected);".to_owned(),
            String::new(),
        ),
        _ => (
            "rt::modulo(&rt::Integer::from(a), &rt::Integer::from(b), &domain(), &mut meter())".to_owned(),
            "holds &= expected_remainder(a, b) == i128::from(expected);".to_owned(),
            String::new(),
        ),
    };
    let test = DIFF_TEST
        .replace("@LOWER@", &lower.to_string())
        .replace("@UPPER@", &upper.to_string())
        .replace("@REAL@", &real)
        .replace("@VECTORS@", &vectors)
        .replace("@VECTOR_CHECK@", &vector_check)
        .replace("@UNIQUE@", &unique)
        .replace("@MUTATED@", if swapped { "true" } else { "false" })
        .replace("@ORACLE@", &harness.identity.oracle_symbol);
    format!("mod {name} {{\n{prefix}\n{test}\n}}\n")
}

/// The AC-4 grid result ranges: one holding every member and one excluding most of them.
fn diff_ranges(operation: &str, profile: DivisionProfile) -> [(i64, i64); 2] {
    let non_negative =
        operation == "mod" || profile == DivisionProfile::Euclidean && operation == "rem";
    if non_negative {
        [(0, i64::MAX), (0, 1)]
    } else if operation == "div" {
        [(i64::MIN, i64::MAX), (-3, 3)]
    } else {
        [(i64::MIN, i64::MAX), (-1, 1)]
    }
}

#[trace("TC-071", "FR-057-AC-4", "FR-057-AC-5")]
#[test]
fn host_differential_of_the_expected_side_against_filament_exact() {
    for (operation, profile) in [
        ("div", DivisionProfile::Truncating),
        ("div", DivisionProfile::Floor),
        ("div", DivisionProfile::Euclidean),
        ("rem", DivisionProfile::Truncating),
        ("rem", DivisionProfile::Floor),
        ("rem", DivisionProfile::Euclidean),
        ("mod", DivisionProfile::Euclidean),
    ] {
        let mut modules = String::new();
        let mut count = 0;
        for (index, narrow) in diff_ranges(operation, profile).into_iter().enumerate() {
            let shape = Shape {
                profile,
                operation,
                left: (i64::MIN, i64::MAX),
                right: (i64::MIN, i64::MAX),
                narrow: Some(narrow),
                unwind: 12,
            };
            for swapped in [false, true] {
                modules.push_str(&diff_module(
                    &format!("m{index}{}", u8::from(swapped)),
                    &shape,
                    swapped,
                ));
                count += 1;
            }
        }
        super::integer_division_bounds::run_generated_native(
            &modules,
            count,
            &format!("ir854-diff-{operation}-{profile:?}"),
        );
    }
}

const PARAMETERS: &str = "x: Int[-8, 7], y: Int[-3, 3]";

/// The seven claims of FR-057-AC-1 as `(function, operation, profile)`.
const CLAIMS: [(&str, &str, DivisionProfile); 7] = [
    ("dt", "div", DivisionProfile::Truncating),
    ("df", "div", DivisionProfile::Floor),
    ("de", "div", DivisionProfile::Euclidean),
    ("rt", "rem", DivisionProfile::Truncating),
    ("rf", "rem", DivisionProfile::Floor),
    ("re", "rem", DivisionProfile::Euclidean),
    ("md", "mod", DivisionProfile::Euclidean),
];

/// A package with the `div`/`rem`/`mod` claims of one law and an `add` sibling, over operands with
/// a negative dividend and both divisor signs.
fn catalog(profile: DivisionProfile) -> CheckedPackageV2 {
    let text = source(&[
        ("dv", PARAMETERS, "Integer", "x div y"),
        ("rm", PARAMETERS, "Integer", "x rem y"),
        ("md", PARAMETERS, "Integer", "x mod y"),
        ("ad", PARAMETERS, "Int[-11, 10]", "x + y"),
    ]);
    compile(&text, profile)
}

fn parts(
    generation: &RoutedGeneration,
    index: usize,
) -> (
    &filament_contract_codegen::ObligationRecord,
    Option<&KaniScalarObligationHarness>,
) {
    kani_parts(&generation.items[index].output)
}

fn unsupported(generation: &RoutedGeneration, index: usize) -> &UnsupportedObligation {
    match &parts(generation, index).0.disposition {
        ObligationDisposition::Unsupported { reason } => reason,
        other => panic!("expected an unsupported claim, got {other:?}"),
    }
}

#[trace("TC-071", "FR-057-AC-1")]
#[test]
fn every_division_claim_of_each_law_generates_one_harness_from_its_own_node() {
    for profile in DivisionProfile::ALL {
        let package = catalog(profile);
        for (function, operation) in [("dv", "div"), ("rm", "rem"), ("md", "mod")] {
            let node = body_id(&package, function);
            let generation = routed_node(&package, &node, 12);
            let (record, harness) = parts(&generation, 0);
            assert!(
                matches!(record.disposition, ObligationDisposition::Supported { .. }),
                "{operation} {profile:?}: {:?}",
                record.disposition
            );
            let harness = harness.expect("a supported claim has its harness");
            assert_eq!(harness.identity.node_id, node, "the harness keeps its node");
            assert_eq!(
                harness.identity.operation_identity,
                format!("filament.op.integer.{operation}")
            );
            let division = harness.identity.division_scalar.as_ref().unwrap();
            assert_eq!(
                division.profile.as_deref(),
                (operation != "mod").then(|| profile.definition_identity()),
                "a div or rem harness renders the claim's own profile, a mod harness none"
            );
            assert_eq!(
                harness
                    .identity
                    .arguments
                    .iter()
                    .map(|argument| (argument.minimum, argument.maximum))
                    .collect::<Vec<_>>(),
                [(-8, 7), (-3, 3)],
                "a negative dividend and both divisor signs"
            );
            assert_eq!(
                harness.identity.integer_representation,
                filament_contract_codegen::ScalarIntegerRepresentation::General
            );
        }
    }
}

#[trace("TC-071", "FR-057-AC-2", "FR-057-AC-10")]
#[test]
fn the_harness_draws_each_operand_over_its_range_with_one_assertion_and_one_last_cover() {
    for (function, operation, profile) in CLAIMS {
        let package = catalog(profile);
        let name = match operation {
            "div" => "dv",
            "rem" => "rm",
            _ => "md",
        };
        let _ = function;
        let generation = routed_node(&package, &body_id(&package, name), 12);
        let (_, harness) = parts(&generation, 0);
        let text = &harness.unwrap().rust.contents;
        let kani_module = &text[text.find("#[cfg(kani)]").unwrap()..];
        assert_eq!(kani_module.matches("kani::any()").count(), 2, "{operation}");
        assert_eq!(
            kani_module.matches("kani::assume(").count(),
            2,
            "only the two operand-range assumptions: none on the divisor, dividend or outcome"
        );
        assert!(kani_module.contains("kani::assume(left >= -8_i64 && left <= 7_i64)"));
        assert!(kani_module.contains("kani::assume(right >= -3_i64 && right <= 3_i64)"));
        assert_eq!(kani_module.matches("assert!(").count(), 1);
        assert_eq!(kani_module.matches("kani::cover!(").count(), 1);
        assert!(
            kani_module.find("assert!(").unwrap() < kani_module.find("kani::cover!(").unwrap(),
            "the completion cover follows the assertion"
        );
        assert!(
            kani_module.trim_end().ends_with("}\n}") || kani_module.trim_end().ends_with("}\n}\n}"),
            "{kani_module}"
        );
        let after_cover = &kani_module[kani_module.find("kani::cover!(").unwrap()..];
        assert_eq!(
            after_cover
                .lines()
                .filter(|line| line.contains(';'))
                .count(),
            1,
            "the cover is the last statement"
        );
    }
}

#[trace("TC-071", "FR-057-AC-3")]
#[test]
fn the_expected_side_is_relational_for_div_and_euclidean_for_rem_and_mod_and_names_no_kernel() {
    for (_, operation, profile) in CLAIMS {
        let package = catalog(profile);
        let name = match operation {
            "div" => "dv",
            "rem" => "rm",
            _ => "md",
        };
        let generation = routed_node(&package, &body_id(&package, name), 12);
        let (_, harness) = parts(&generation, 0);
        let harness = harness.unwrap();
        let expected = expected_side(harness)
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in [
            "filament_exact",
            "rt::",
            "nx::",
            "oracle_",
            "checked_div",
            "checked_rem",
            "wrapping_div",
            "wrapping_rem",
            "div_euclid",
            "/",
            "%",
        ] {
            assert!(
                !expected.contains(forbidden),
                "{operation} {profile:?} expected side names {forbidden}: {expected}"
            );
        }
        assert!(!expected.contains(&harness.identity.oracle_symbol));
        let euclid = expected.matches("rem_euclid").count();
        if operation == "div" {
            assert_eq!(euclid, 0, "a div expected side divides nowhere");
            assert!(
                expected.contains("a - i128::from("),
                "r(L) and r(U) from products"
            );
            assert!(
                expected.contains("let r = a - q * b;"),
                "quotient, then divisor"
            );
            assert!(!expected.contains("let r = a - b * q"));
        } else {
            assert_eq!(euclid, 1);
            assert_eq!(expected.matches("i64::rem_euclid(a, b)").count(), 1);
            assert!(expected.contains("b == -1"));
            assert!(!expected.contains("i128::rem_euclid"));
        }
    }
}

#[trace("TC-071", "FR-057-AC-5")]
#[test]
fn lowering_refuses_the_inferred_quotient_range_that_is_not_i64_and_a_zero_only_divisor() {
    let wide = "x: Int[-9223372036854775808, 9223372036854775807], y: Int[-3, 3]";
    for (profile, operation) in [
        (DivisionProfile::Truncating, "div"),
        (DivisionProfile::Floor, "div"),
        (DivisionProfile::Euclidean, "div"),
    ] {
        let package = compile(
            &source(&[("subject", wide, "Integer", &format!("x {operation} y"))]),
            profile,
        );
        let generation = routed(&package, "subject", 12);
        assert!(parts(&generation, 0).1.is_none(), "no harness");
        let UnsupportedObligation::DomainNotRepresentableInI64 { lower, upper, .. } =
            unsupported(&generation, 0)
        else {
            panic!("{:?}", unsupported(&generation, 0));
        };
        assert_eq!(lower, "-9223372036854775808");
        assert_eq!(upper, "9223372036854775808");
    }
    // `rem` and `mod` over the same full-range dividend infer a small interval and generate.
    for (profile, operation) in [
        (DivisionProfile::Truncating, "rem"),
        (DivisionProfile::Floor, "rem"),
        (DivisionProfile::Euclidean, "rem"),
        (DivisionProfile::Euclidean, "mod"),
    ] {
        let package = compile(
            &source(&[("subject", wide, "Integer", &format!("x {operation} y"))]),
            profile,
        );
        let generation = routed(&package, "subject", 12);
        assert!(parts(&generation, 0).1.is_some(), "{operation} {profile:?}");
    }
    // A divisor range that is only zero has no completed member: the result bound is unreachable.
    let package = compile(
        &source(&[(
            "subject",
            "x: Int[-8, 7], y: Int[0, 0]",
            "Integer",
            "x div y",
        )]),
        DivisionProfile::Floor,
    );
    let generation = routed(&package, "subject", 12);
    assert!(matches!(
        unsupported(&generation, 0),
        UnsupportedObligation::ResultBoundUnreachable { .. }
    ));
    // An explicit result range inside `i64` generates, including the `i64::MIN / -1` operands.
    let shape = Shape {
        profile: DivisionProfile::Truncating,
        operation: "div",
        left: (i64::MIN, i64::MAX),
        right: (-1, 1),
        narrow: Some((i64::MIN, i64::MAX)),
        unwind: 12,
    };
    let generation = shape.generation();
    assert!(parts(&generation, 0).1.is_some());
}

#[trace("TC-071", "FR-057-AC-6")]
#[test]
fn the_native_route_refuses_each_division_claim_and_leaves_its_sibling_byte_identical() {
    for profile in DivisionProfile::ALL {
        let package = catalog(profile);
        let nodes = [
            application_in(&package, "dv", "filament.op.integer.div"),
            application_in(&package, "rm", "filament.op.integer.rem"),
            application_in(&package, "md", "filament.op.integer.mod"),
            application_in(&package, "ad", "filament.op.integer.add"),
        ];
        let together = routed_nodes(&package, &nodes, 12, true);
        for (index, identity) in [
            (0, "filament.op.integer.div"),
            (1, "filament.op.integer.rem"),
            (2, "filament.op.integer.mod"),
        ] {
            assert!(parts(&together, index).1.is_none(), "no harness");
            let UnsupportedObligation::NativeDivisionPlanUnavailable {
                node_id,
                operation_identity,
            } = unsupported(&together, index)
            else {
                panic!("{:?}", unsupported(&together, index));
            };
            assert_eq!(node_id, &nodes[index]);
            assert_eq!(operation_identity, identity);
        }
        let alone = routed_nodes(&package, &nodes[3..], 12, true);
        let sibling = parts(&together, 3);
        assert!(
            matches!(
                sibling.0.disposition,
                ObligationDisposition::Supported { .. }
            ),
            "{:?}",
            sibling.0.disposition
        );
        assert_eq!(
            sibling.1,
            parts(&alone, 0).1,
            "the sibling harness is byte-identical"
        );
        assert_eq!(
            sibling.0.subject,
            parts(&alone, 0).0.subject,
            "and its subject"
        );
        assert_eq!(
            sibling.0.disposition,
            parts(&alone, 0).0.disposition,
            "and its record"
        );
    }
}

#[trace("TC-071", "FR-057-AC-11")]
#[test]
fn the_integer_siblings_of_a_division_claim_generate_as_they_do_alone() {
    let package = compile(
        &source(&[
            ("dv", PARAMETERS, "Integer", "x div y"),
            ("ad", PARAMETERS, "Int[-11, 10]", "x + y"),
            ("su", PARAMETERS, "Int[-11, 10]", "x - y"),
            ("mu", PARAMETERS, "Int[-24, 24]", "x * y"),
            ("ng", PARAMETERS, "Int[-7, 8]", "-x"),
        ]),
        DivisionProfile::Floor,
    );
    let names = ["dv", "ad", "su", "mu", "ng"];
    let identities = [
        "filament.op.integer.div",
        "filament.op.integer.add",
        "filament.op.integer.sub",
        "filament.op.integer.mul",
        "filament.op.integer.negate",
    ];
    let nodes: Vec<_> = names
        .iter()
        .zip(identities)
        .map(|(name, identity)| application_in(&package, name, identity))
        .collect();
    let together = routed_nodes(&package, &nodes, 12, false);
    for index in 1..names.len() {
        let alone = routed_nodes(&package, &nodes[index..=index], 12, false);
        assert!(
            parts(&alone, 0).1.is_some(),
            "{} generates: {:?}",
            names[index],
            parts(&alone, 0).0.disposition
        );
        assert_eq!(
            parts(&together, index).1,
            parts(&alone, 0).1,
            "{} is unchanged by a division sibling",
            names[index]
        );
    }
}

#[trace("TC-071", "FR-057-AC-12")]
#[test]
fn the_harness_identity_records_node_operation_profile_route_ranges_and_backend_options() {
    let package = catalog(DivisionProfile::Floor);
    let node = body_id(&package, "dv");
    let generation = routed_node(&package, &node, 12);
    let harness = parts(&generation, 0).1.unwrap();
    let identity = &harness.identity;
    assert_eq!(identity.node_id, node);
    assert_eq!(identity.operation_identity, "filament.op.integer.div");
    assert_eq!(identity.unwind, 12);
    assert_eq!(
        identity.solver,
        filament_contract_codegen::KaniSolver::Cadical
    );
    let division = identity.division_scalar.as_ref().unwrap();
    assert_eq!(
        division.profile.as_deref(),
        Some(DivisionProfile::Floor.definition_identity())
    );
    assert_eq!((division.result_minimum, division.result_maximum), (-8, 8));
    assert!(identity.options.iter().any(|option| option == "--unwind"));
    assert!(identity.options.iter().any(|option| option == "cadical"));
    let json = serde_json::to_value(identity).unwrap();
    assert_eq!(json["divisionScalar"]["resultMinimum"], -8);
    assert_eq!(
        identity.integer_representation,
        filament_contract_codegen::ScalarIntegerRepresentation::General
    );
}

/// The division harness and the manifest its crate builds under, for the run controls.
fn first_harness() -> (
    KaniScalarObligationHarness,
    filament_contract_codegen::Artifact,
) {
    let package = catalog(DivisionProfile::Floor);
    let generation = routed_node(&package, &body_id(&package, "dv"), 12);
    let (harness, manifest) = harness_of(&generation).expect("a division harness");
    (harness.clone(), manifest)
}

#[cfg(unix)]
fn run_with_launcher(
    name: &str,
    body: &str,
    ceilings: filament_contract_codegen::ProofCeilings,
) -> filament_contract_codegen::KaniExecutionEvidence {
    use std::os::unix::fs::PermissionsExt;
    let (mut harness, manifest) = first_harness();
    harness.identity.ceilings = ceilings;
    let scratch =
        crate::kani_obligations::write_scalar_crate(name, &manifest, &harness.rust.contents);
    let directory = &scratch.path;
    std::fs::create_dir_all(directory.join("target")).unwrap();
    let launcher = directory.join("cargo-kani");
    std::fs::write(
        &launcher,
        format!("#!/bin/sh\n[ \"$1\" = --probe ] && exit 0\n{body}"),
    )
    .unwrap();
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cancel = filament_exact::Cancel::new();
    filament_contract_codegen::execute_kani_obligation(
        &filament_contract_codegen::KaniExecutionRequest {
            cancel: &cancel,
            installation: &filament_contract_codegen::KaniInstallation::new(launcher),
            harness: (&harness).into(),
            crate_directory: directory,
            target_directory: &directory.join("target"),
            home_directory: None,
        },
    )
    .unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// An unsatisfied cover, an exhausted unwind, a timeout, a missing verdict and a missing cover
/// summary each settle inconclusive or cover-unsatisfied for a division harness, never `Verified`.
#[trace("TC-071", "FR-057-AC-10")]
#[test]
fn an_unsatisfied_cover_an_exhausted_unwind_a_timeout_and_no_verdict_never_verify() {
    use filament_contract_codegen::{classify_kani_run, KaniInconclusiveReason, KaniRunOutcome};
    let (harness, _) = first_harness();
    let total = harness.rust.contents.matches("kani::cover!(").count();
    assert_eq!(total, 1, "one completion cover");
    let check = |status: &str, category: &str| {
        serde_json::json!({
            "id": 1, "status": status, "category": category,
            "location": { "file": "src/lib.rs", "line": "10", "column": "5" },
        })
    };
    let classify = |status: &str, checks: Vec<serde_json::Value>| {
        let report = serde_json::to_vec(&serde_json::json!({
            "metadata": { "version": "1.0" },
            "verification_results": { "results": [{
                "harness_id": "h", "status": status, "checks": checks
            }] }
        }))
        .unwrap();
        classify_kani_run(status == "Success", Some(&report), None, None)
            .unwrap()
            .outcome
    };
    let with_cover = |cover: &str| vec![check("Success", "assertion"), check(cover, "cover")];
    assert_eq!(
        classify("Success", with_cover("Satisfied")),
        KaniRunOutcome::Verified
    );
    assert_eq!(
        classify("Success", with_cover("Unsatisfiable")),
        KaniRunOutcome::CoverUnsatisfied {
            satisfied: 0,
            total: 1
        }
    );
    assert_eq!(
        classify("Success", vec![check("Success", "assertion")]),
        KaniRunOutcome::Inconclusive {
            reason: KaniInconclusiveReason::MissingCoverSummary
        }
    );
    assert_eq!(
        classify(
            "Failure",
            vec![
                check("Failure", "unwind"),
                check("Success", "assertion"),
                check("Satisfied", "cover")
            ]
        ),
        KaniRunOutcome::Inconclusive {
            reason: KaniInconclusiveReason::UnwindBoundExhausted
        }
    );
    #[cfg(unix)]
    {
        let timed = run_with_launcher(
            "ir854-timeout",
            "exec sleep 30\n",
            crate::common::proof_ceilings::proof_ceilings_with_wall_clock(
                std::time::Duration::from_millis(300),
            ),
        );
        assert_eq!(
            timed.outcome,
            KaniRunOutcome::Inconclusive {
                reason: KaniInconclusiveReason::TimedOut
            }
        );
        let silent = run_with_launcher(
            "ir854-no-verdict",
            "printf 'Out of memory\\n' >&2\nexit 1\n",
            crate::common::proof_ceilings::proof_ceilings(),
        );
        assert_eq!(
            silent.outcome,
            KaniRunOutcome::Inconclusive {
                reason: KaniInconclusiveReason::NoVerdict
            }
        );
        // The evidence of a run records the full ordered backend arguments and the target.
        assert_eq!(timed.unwind, 12);
        assert!(timed
            .arguments
            .iter()
            .any(|argument| argument == "--unwind"));
        assert!(timed.arguments.iter().any(|argument| argument == "cadical"));
    }
}

/// The AC-8 mutants of one harness as `(name, mutated source)`: a wrong embedded profile, the
/// wrong member or operator, a completed value at a zero divisor, a wrong expected-side window
/// and a removed expected-side zero arm.
pub(crate) fn kani_mutants(
    harness: &KaniScalarObligationHarness,
    operation: &str,
    profile: DivisionProfile,
) -> Vec<(String, String)> {
    let text = harness.rust.contents.as_str();
    let symbol = &harness.identity.oracle_symbol;
    let oracle = function_text(text, &format!("pub fn {symbol}")).to_owned();
    let splice = |mutated: String| text.replacen(&oracle, &mutated, 1);
    let mut mutants = Vec::new();
    let profile_name = format!("{profile:?}");
    if operation != "mod" {
        for other in DivisionProfile::ALL
            .into_iter()
            .filter(|other| *other != profile)
        {
            mutants.push((
                format!("profile-{profile_name}-as-{other:?}"),
                splice(mutate(
                    &oracle,
                    &format!("rt::DivisionProfile::{profile_name}"),
                    &format!("rt::DivisionProfile::{other:?}"),
                )),
            ));
        }
    }
    match operation {
        "rem" => {
            mutants.push((
                "rem-as-modulo".to_owned(),
                splice(
                    mutate(
                        &oracle,
                        &format!("rt::divide(rt::DivisionProfile::{profile_name}, rt::DivisionMember::Remainder, left, right,"),
                        "rt::modulo(left, right,",
                    ),
                ),
            ));
            mutants.push((
                "rem-with-quotient-member".to_owned(),
                splice(mutate(
                    &oracle,
                    "rt::DivisionMember::Remainder",
                    "rt::DivisionMember::Quotient",
                )),
            ));
        }
        "mod" => {
            for other in [DivisionProfile::Truncating, DivisionProfile::Floor] {
                mutants.push((
                    format!("mod-as-{other:?}-remainder"),
                    splice(mutate(
                        &oracle,
                        "rt::modulo(left, right,",
                        &format!("rt::divide(rt::DivisionProfile::{other:?}, rt::DivisionMember::Remainder, left, right,"),
                    )),
                ));
            }
        }
        _ => {}
    }
    mutants.push((
        "completes-at-zero".to_owned(),
        splice(mutate(
            &oracle,
            "    let domain =",
            "    if *right == rt::Integer::zero() {\n        return Ok(rt::Outcome::Completed(rt::Integer::zero()));\n    }\n    let domain =",
        )),
    ));
    let window_profile = if operation == "mod" {
        DivisionProfile::Euclidean
    } else {
        profile
    };
    let other = match window_profile {
        DivisionProfile::Truncating => DivisionProfile::Floor,
        _ => DivisionProfile::Truncating,
    };
    mutants.push((
        "expected-window-table-of-another-law".to_owned(),
        mutate(text, window_arms(window_profile), window_arms(other)),
    ));
    mutants.push((
        "expected-zero-arm-removed".to_owned(),
        mutate(
            text,
            "fn expected_zero_divisor(b: i64) -> bool {\n    b == 0\n}",
            "fn expected_zero_divisor(b: i64) -> bool {\n    false\n}",
        ),
    ));
    mutants
}

// ---- installed Kani lane ----

/// Runs one routed division harness (or a mutated source of it) through the execution module in
/// the crate the driver assembles, under the real Kani backend, and prints its measured outcome.
fn run_under_real_kani(
    name: &str,
    harness: &KaniScalarObligationHarness,
    manifest: &filament_contract_codegen::Artifact,
    source: Option<&str>,
) -> filament_contract_codegen::KaniExecutionEvidence {
    let mut run = harness.clone();
    if let Some(source) = source {
        assert_ne!(run.rust.contents, source, "the mutation changed the source");
        run.rust.contents = source.to_owned();
    }
    let started = std::time::Instant::now();
    let evidence = crate::kani_obligations::run_scalar_under_real_kani(name, &run, manifest);
    eprintln!(
        "IR854 {name} outcome={:?} wall_s={:.1} unwind={} solver={:?} argv={:?}",
        evidence.outcome,
        started.elapsed().as_secs_f64(),
        evidence.unwind,
        evidence.solver,
        evidence.arguments
    );
    evidence
}

fn expect_timed_out(shape: &Shape, name: &str) {
    let generation = shape.generation();
    let (harness, manifest) = harness_of(&generation).expect("a division harness");
    let evidence = run_under_real_kani(name, harness, &manifest, None);
    assert_eq!(
        evidence.outcome,
        filament_contract_codegen::KaniRunOutcome::Inconclusive {
            reason: filament_contract_codegen::KaniInconclusiveReason::TimedOut
        },
        "{name}: a TimedOut shape is a not_tractable row, never a verdict"
    );
}

/// FR-057-AC-7 and AC-9, general route, constant divisor: the smallest listed shape (a singleton
/// divisor range, dividend at 8 bits) under the FR-028 wall-clock ceiling does not finish, so no
/// verdict is claimed for it and it is a `not_tractable` row of the shipped scalar table.
#[trace("TC-071", "FR-057-AC-9")]
#[test]
#[ignore = "measurement reproducer outside the kani gate (cargo test --test it -- --ignored <name>, under the 24 GiB scope); expects TimedOut at 600 s"]
fn general_route_constant_divisor_8_bit_floor_div_times_out() {
    expect_timed_out(
        &Shape {
            profile: DivisionProfile::Floor,
            operation: "div",
            left: (-128, 127),
            right: (8, 8),
            narrow: None,
            unwind: 10,
        },
        "ir854-const8-div-floor",
    );
}

/// FR-057-AC-7 and AC-9, general route, symbolic divisor: the smallest listed shape (both
/// operands at 4 bits, the divisor range holding zero and both signs) does not finish.
#[trace("TC-071", "FR-057-AC-9")]
#[test]
#[ignore = "measurement reproducer outside the kani gate (cargo test --test it -- --ignored <name>, under the 24 GiB scope); expects TimedOut at 600 s"]
fn general_route_symbolic_divisor_4_bit_floor_div_times_out() {
    expect_timed_out(
        &Shape {
            profile: DivisionProfile::Floor,
            operation: "div",
            left: (-8, 7),
            right: (-8, 7),
            narrow: None,
            unwind: 10,
        },
        "ir854-sym4-div-floor",
    );
}

const SHIPPED_TABLE: &str = "docs/qualification/ir1530-general-route-scalar-table.json";

/// FR-057-AC-9: the measured general-route division shapes are `not_tractable` rows of the single
/// shipped scalar table at their smallest failing width, each with its `route`, no tool version
/// and no `divisor_constant` extension (the rows show no split: no constant-divisor shape was
/// measured `Verified`).
#[trace("TC-071", "FR-057-AC-9")]
#[test]
fn the_shipped_table_holds_the_measured_division_rows_with_their_route() {
    let table: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SHIPPED_TABLE))
            .expect("the shipped table is readable"),
    )
    .expect("the shipped table is JSON");
    for identity in ["div", "rem", "mod"] {
        let id = format!("filament.op.integer.{identity}");
        let rows = table
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["operation_ids"] == serde_json::json!([id]) && row["route"] == "general"
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1, "one general-route row for {id}");
        let row = rows[0];
        assert_eq!(row["route"], "general");
        assert_eq!(row["verdict"], "not_tractable");
        assert_eq!(
            row["max_operand_bits"], 4,
            "the smallest listed symbolic width"
        );
        assert_eq!(row["wall_seconds"], 600);
        assert!(row["peak_bytes"].as_u64().is_some_and(|peak| peak > 0));
        for field in [
            "kani_version",
            "cbmc_version",
            "solver_version",
            "divisor_constant",
        ] {
            assert!(row.get(field).is_none(), "{field} in {row}");
        }
    }
    filament_contract_codegen::TractabilityTable::shipped_general_route_scalar()
        .expect("the embedded table loads with the division rows");
}

/// FR-057-AC-7: the native-route rows copied from the kernel's measured record (FR-409, TC-957)
/// are in the shipped table with their required `route`: per operation one `tractable` row at the
/// 8-bit symbolic width every profile verified, and one `not_tractable` row at the smallest
/// symbolic width that timed out under any profile. They refuse only native-route clauses.
#[trace("TC-071", "FR-057-AC-7")]
#[test]
fn the_shipped_table_holds_the_kernel_native_division_rows_on_the_native_route() {
    use filament_contract_codegen::{
        ClauseFeatures, Decision, InputBound, IntegerRoute, OperationWidth, RowVerdict,
        TractabilityDecider, TractabilityTable,
    };
    let table = TractabilityTable::shipped_general_route_scalar().expect("the shipped table");
    for operation in ["div", "rem", "mod"] {
        let id = format!("filament.op.integer.{operation}");
        let native = table
            .rows()
            .iter()
            .filter(|row| row.route == IntegerRoute::Native && row.operation_ids == [id.clone()])
            .collect::<Vec<_>>();
        assert_eq!(native.len(), 2, "two native rows for {id}");
        let verified = native
            .iter()
            .find(|row| row.verdict == RowVerdict::Tractable)
            .expect("a tractable native row");
        let timed_out = native
            .iter()
            .find(|row| row.verdict == RowVerdict::NotTractable)
            .expect("a not-tractable native row");
        assert_eq!(verified.max_operand_bits, 8);
        assert_eq!(timed_out.max_operand_bits, 16);
        for row in [verified, timed_out] {
            assert!(
                row.provenance
                    .as_deref()
                    .is_some_and(|text| text.contains("TC-957") && text.contains("copied")),
                "a native row names the kernel record it was copied from: {row:?}"
            );
        }
        assert_eq!(timed_out.wall_seconds.unwrap().millis(), 600_000);
        for features_route in [IntegerRoute::Native, IntegerRoute::General] {
            let decide = |bits| {
                table.decide(&ClauseFeatures::new(
                    features_route,
                    vec![OperationWidth {
                        operation_id: id.clone(),
                        bits: Some(bits),
                    }],
                    vec![InputBound {
                        lower: 0,
                        upper: 1,
                        boolean: false,
                    }],
                    None,
                ))
            };
            // At 8 bits the native tractable row refuses nothing; the general row (4 bits)
            // refuses on its own route, and native 16 bits is refused by the native row.
            assert_eq!(
                matches!(decide(8), Decision::Refuse { .. }),
                features_route == IntegerRoute::General
            );
            assert!(matches!(decide(16), Decision::Refuse { .. }));
            if features_route == IntegerRoute::Native {
                let Decision::Refuse { evidence_row, .. } = decide(16) else {
                    unreachable!()
                };
                assert_eq!(evidence_row.route, IntegerRoute::Native);
                assert_eq!(evidence_row.max_operand_bits, 16);
            }
        }
    }
}

/// FR-057-AC-9: a division harness at or above the row's width is refused before any Kani
/// launch, and one below it is not refused by the decider.
#[trace("TC-071", "FR-057-AC-9")]
#[test]
fn a_division_harness_at_or_above_its_row_width_is_refused_before_kani_and_below_it_runs() {
    use filament_contract_codegen::{
        execute_kani_obligation_decided, DecidedRunRefusal, IntegerRoute, KaniExecutionRefusal,
        TractabilityTable,
    };
    let table = TractabilityTable::shipped_general_route_scalar().expect("the shipped table");
    let cancel = filament_exact::Cancel::new();
    let installation = filament_contract_codegen::KaniInstallation::new("/nonexistent/cargo-kani");
    let missing = std::path::Path::new("/nonexistent/crate");
    for (function, operation) in [("dv", "div"), ("rm", "rem"), ("md", "mod")] {
        let package = catalog(DivisionProfile::Floor);
        let generation = routed_node(&package, &body_id(&package, function), 12);
        let (harness, _) = harness_of(&generation).expect("a division harness");
        fn request<'a>(
            harness: &'a KaniScalarObligationHarness,
            cancel: &'a filament_exact::Cancel,
            installation: &'a filament_contract_codegen::KaniInstallation,
            missing: &'a std::path::Path,
        ) -> filament_contract_codegen::KaniExecutionRequest<'a> {
            filament_contract_codegen::KaniExecutionRequest {
                cancel,
                installation,
                harness: harness.into(),
                crate_directory: missing,
                target_directory: missing,
                home_directory: None,
            }
        }
        // `Int[-8, 7]` is 4 bits: at the row.
        let refusal = execute_kani_obligation_decided(
            &request(harness, &cancel, &installation, missing),
            &table,
        )
        .expect_err("a 4-bit division harness is a measured not-tractable row");
        let DecidedRunRefusal::NotTractable { refusal, .. } = refusal else {
            panic!("expected NotTractable, got {refusal:?}");
        };
        let UnsupportedObligation::V2ClauseNotTractable {
            operation_identity,
            width_bits,
            route,
            row,
            ..
        } = *refusal
        else {
            panic!("the one refusal arm is V2ClauseNotTractable");
        };
        assert_eq!(
            operation_identity,
            format!("filament.op.integer.{operation}")
        );
        assert_eq!(width_bits, 4);
        assert_eq!(route, IntegerRoute::General);
        assert_eq!(row.max_operand_bits, 4);
        // Below the row, at 3 bits, the decider lets it through to the (missing) launcher.
        let text = source(&[(
            "subject",
            "x: Int[-4, 3], y: Int[-2, 2]",
            "Integer",
            &format!("x {operation} y"),
        )]);
        let small = routed(&compile(&text, DivisionProfile::Floor), "subject", 12);
        let (small_harness, _) = harness_of(&small).expect("a 3-bit division harness");
        assert!(
            matches!(
                execute_kani_obligation_decided(
                    &request(small_harness, &cancel, &installation, missing),
                    &table
                ),
                Err(DecidedRunRefusal::Execution(
                    KaniExecutionRefusal::HarnessNotInCrate { .. } | KaniExecutionRefusal::Tool(_)
                ))
            ),
            "{operation} below its row width is not refused by the decider"
        );
    }
}

/// FR-057-AC-8: the seeded mutants of each operation and law are rendered, each changes the routed
/// source, and the set holds both groups (wrong profile or operator or member, and a missing zero
/// guard on the oracle or the expected side).
#[trace("TC-071")]
#[test]
fn the_seeded_mutants_of_each_claim_change_the_routed_source_and_cover_both_groups() {
    let mut swaps = [0_usize; 2];
    for (_, operation, profile) in CLAIMS {
        let package = catalog(profile);
        let function = match operation {
            "div" => "dv",
            "rem" => "rm",
            _ => "md",
        };
        let generation = routed_node(&package, &body_id(&package, function), 12);
        let (harness, _) = harness_of(&generation).expect("a division harness");
        let mutants = kani_mutants(harness, operation, profile);
        for (name, mutated) in &mutants {
            assert_ne!(mutated, &harness.rust.contents, "{operation} {name}");
        }
        let names = mutants
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        for required in [
            "completes-at-zero",
            "expected-window-table-of-another-law",
            "expected-zero-arm-removed",
        ] {
            assert!(
                names.contains(&required),
                "{operation} {profile:?}: {required}"
            );
        }
        let profile_swaps = names
            .iter()
            .filter(|name| name.starts_with("profile-"))
            .count();
        match operation {
            "div" => swaps[0] += profile_swaps,
            "rem" => swaps[1] += profile_swaps,
            _ => assert_eq!(profile_swaps, 0),
        }
        assert_eq!(names.contains(&"rem-as-modulo"), operation == "rem");
        assert_eq!(
            names.contains(&"rem-with-quotient-member"),
            operation == "rem"
        );
        assert_eq!(
            names
                .iter()
                .filter(|name| name.starts_with("mod-as-"))
                .count(),
            if operation == "mod" { 2 } else { 0 }
        );
    }
    assert_eq!(swaps, [6, 6], "six profile swaps for div and six for rem");
}

/// The explicit-result fixture of FR-057-AC-8 (a negative dividend, both divisor signs and zero,
/// a result range that excludes some members), for `div` under the floor law.
fn mutant_fixture() -> Shape {
    Shape {
        profile: DivisionProfile::Floor,
        operation: "div",
        left: (-9, 9),
        right: (-4, 4),
        narrow: Some((-3, 3)),
        unwind: 10,
    }
}

/// FR-057-AC-8 on the general route: the mutant whose oracle completes at a zero divisor, over the
/// 5-bit fixture, does not finish inside the 600 s ceiling, so it is not `Falsified` and carries no
/// playback. Measured 2026-11-14; see `docs/qualification/ir854-general-route-division-measurements.md`.
#[trace("TC-071")]
#[test]
#[ignore = "measurement reproducer outside the kani gate (cargo test --test it -- --ignored <name>, under the 24 GiB scope); expects TimedOut at 600 s"]
fn general_route_zero_completing_mutant_times_out() {
    let generation = mutant_fixture().generation();
    let (harness, manifest) = harness_of(&generation).expect("a division harness");
    let mutants = kani_mutants(harness, "div", DivisionProfile::Floor);
    let (_, mutated) = mutants
        .iter()
        .find(|(name, _)| name == "completes-at-zero")
        .expect("the zero-completing mutant");
    let evidence = run_under_real_kani("ir854-mutant-zero", harness, &manifest, Some(mutated));
    assert_eq!(
        evidence.outcome,
        filament_contract_codegen::KaniRunOutcome::Inconclusive {
            reason: filament_contract_codegen::KaniInconclusiveReason::TimedOut
        }
    );
}
