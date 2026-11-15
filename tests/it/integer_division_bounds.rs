// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Genuine FSL operand bounds at the scalar derivation/generation boundary.

use std::{fmt::Write as _, fs, process::Command};

use filament_contract_codegen::{
    derive_exact_scalar_items, generate_exact_scalar_oracles, ClaimDisposition, ExactScalarItem,
    ExactScalarOperation, ExactScalarRefusal, OperationProvenance, EXACT_DEPENDENCY_SOURCE,
};
use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_exact::{DivisionProfile, IntegerDomain, IntegerInterval};
use fsl_replay::{
    admit_integer_division, compile_package, compile_package_with_integer_division, CompileUnit,
    DefinitionRole, DependencyInput, ReplayLimits, SourceIdentity, StageLimits,
};
use ix_trace_rs::trace;

use crate::{exact_function_agreement::body_id, scratch_crate};

fn compiled(parameters: &str, result: &str, expression: &str) -> CheckedPackageV2 {
    let profile = expression
        .contains("div")
        .then_some(DivisionProfile::Truncating);
    compiled_with_profile(parameters, result, expression, profile)
}

fn compiled_with_profile(
    parameters: &str,
    result: &str,
    expression: &str,
    profile: Option<DivisionProfile>,
) -> CheckedPackageV2 {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction subject using v({parameters}): {result} pure {{ {expression} }}\n"
    );
    let unit = CompileUnit {
        source: SourceIdentity::new("test", "integer-division-bounds"),
        package_name: "integer-division-bounds".to_owned(),
        path: "integer-division-bounds.native",
        bytes: source.as_bytes(),
    };
    let compiled = if let Some(profile) = profile {
        let role = match profile {
            DivisionProfile::Truncating => DefinitionRole::IntegerDivisionTruncating,
            DivisionProfile::Floor => DefinitionRole::IntegerDivisionFloor,
            DivisionProfile::Euclidean => DefinitionRole::IntegerDivisionEuclidean,
        };
        let selection = admit_integer_division(&[role.reference()], None)
            .expect("owner admits one actual division definition");
        compile_package_with_integer_division(
            unit,
            [],
            &DependencyInput::default(),
            &selection,
            StageLimits::default(),
            ReplayLimits::default(),
        )
    } else {
        compile_package(
            unit,
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
    }
    .expect("FSL compiles genuine integer source under the supplied owner selection");
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

fn derived(package: &CheckedPackageV2) -> ExactScalarItem {
    let node = body_id(package, "subject");
    derive_exact_scalar_items(package, &[node])
        .remove(0)
        .expect("finite operands give an honest derived result interval")
}

fn interval(lower: &str, upper: &str) -> IntegerInterval {
    IntegerInterval::new(lower.parse().unwrap(), upper.parse().unwrap()).unwrap()
}

fn domain(item: &ExactScalarItem) -> &IntegerDomain {
    match &item.operation {
        ExactScalarOperation::IntegerDivision { domain, .. }
        | ExactScalarOperation::IntegerRemainder { domain, .. }
        | ExactScalarOperation::IntegerModulo { domain } => domain,
        other => panic!("expected an integer division/modulo operation: {other:?}"),
    }
}

#[trace("TC-024", "FR-014-AC-18")]
#[test]
fn bounded_division_and_modulo_derive_sound_domains_and_execute_generated_values() {
    generated_values(false);
}

#[trace("TC-024", "FR-014-AC-18")]
#[test]
fn genuine_modulo_sources_execute_independently_of_the_division_selection_entry() {
    generated_values(true);
    unbounded_and_mismatch(&["x mod y"]);
    explicit_result_bounds();
}

fn generated_values(modulo_only: bool) {
    let mut native = String::from(NATIVE_SUPPORT);
    let cases = [
        (
            "x: Int[-9, 9], y: Int[1, 4]",
            "x div y",
            "-9",
            "9",
            "-7",
            "3",
            "-2",
        ),
        (
            "x: Int[-9, 9], y: Int[-4, -1]",
            "x div y",
            "-9",
            "9",
            "7",
            "-3",
            "-2",
        ),
        (
            "x: Int[-9, 9], y: Int[1, 4]",
            "x mod y",
            "0",
            "3",
            "-7",
            "3",
            "2",
        ),
        (
            "x: Int[-9, 9], y: Int[-4, -1]",
            "x mod y",
            "0",
            "3",
            "-7",
            "-3",
            "2",
        ),
        (
            "x: Int[-9223372036854775808, 9223372036854775807], y: Int[-1, -1]",
            "x div y",
            "-9223372036854775808",
            "9223372036854775808",
            "-9223372036854775808",
            "-1",
            "9223372036854775808",
        ),
        (
            "x: Int[-9, 9], y: Int[-9223372036854775808, -1]",
            "x mod y",
            "0",
            "9223372036854775807",
            "-1",
            "-9223372036854775808",
            "9223372036854775807",
        ),
        (
            "x: Int[0, 0], y: Int[1, 1]",
            "x div y",
            "0",
            "0",
            "0",
            "1",
            "0",
        ),
        (
            "x: Int[-9, 9], y: Int[-1, -1]",
            "x mod y",
            "0",
            "0",
            "-9",
            "-1",
            "0",
        ),
        (
            "x: Int[-9, 9], y: Int[1, 4]",
            "x div 3",
            "-9",
            "9",
            "-7",
            "3",
            "-2",
        ),
        (
            "x: Int[-9, 9], y: Int[1, 4]",
            "x mod 3",
            "0",
            "2",
            "-7",
            "3",
            "2",
        ),
        (
            "x: Int[-9, 9], y: Int[0, 0]",
            "x div y",
            "-9",
            "9",
            "7",
            "0",
            "undefined",
        ),
        (
            "x: Int[-9, 9], y: Int[0, 0]",
            "x mod y",
            "0",
            "0",
            "7",
            "0",
            "undefined",
        ),
    ];
    let selected = cases
        .iter()
        .filter(|case| !modulo_only || case.1.contains("mod"))
        .flat_map(|case| {
            if modulo_only {
                vec![(case, None)]
            } else {
                DivisionProfile::ALL
                    .into_iter()
                    .map(|profile| (case, Some(profile)))
                    .collect()
            }
        })
        .collect::<Vec<_>>();
    for (index, ((parameters, expression, lower, upper, left, right, expected), profile)) in
        selected.iter().copied().enumerate()
    {
        let package = compiled_with_profile(parameters, "Integer", expression, profile);
        let item = derived(&package);
        assert_eq!(
            domain(&item),
            &IntegerDomain::Bounded(interval(lower, upper)),
            "{expression}: {parameters}"
        );
        if let ExactScalarOperation::IntegerDivision {
            profile: actual, ..
        } = &item.operation
        {
            assert_eq!(Some(*actual), profile, "derive the actual selected law");
        }
        let expected = match (profile, *expression, *left, *right) {
            (
                Some(DivisionProfile::Floor | DivisionProfile::Euclidean),
                "x div y" | "x div 3",
                "-7",
                "3",
            ) => "-3",
            (Some(DivisionProfile::Floor), "x div y", "7", "-3") => "-3",
            _ => *expected,
        };
        let generated =
            generate_exact_scalar_oracles(&package, &[item]).expect("generate derived oracle");
        let claim = &generated.claim_map.items[0];
        assert_eq!(claim.operation.provenance, OperationProvenance::IrConfirmed);
        let ClaimDisposition::Generated(claim) = &claim.result else {
            panic!("derived oracle refused: {claim:?}")
        };
        let node = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.node_id == claim.node_id)
            .unwrap();
        let filament_contract_model::CheckedTerm::Application { arguments, .. } = &node.body else {
            panic!("original checked application");
        };
        let mut operand_bounds = Vec::new();
        for argument in arguments {
            let filament_contract_model::CheckedTerm::Reference { target } = argument else {
                continue;
            };
            let operand = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| &node.node_id == target)
                .unwrap();
            let typing = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| node.node_id == operand.semantic_type)
                .unwrap();
            if typing.node_tag.as_ref() == "bounded_domain"
                && !operand_bounds.contains(&typing.node_id)
            {
                operand_bounds.push(typing.node_id.clone());
            }
        }
        assert_eq!(
            claim.checked_bounds, operand_bounds,
            "retain exactly the original operand bound ids"
        );
        let expected_outcome = if expected == "undefined" {
            "rt::Outcome::Undefined(rt::Undefined::DivisionByZero)".to_owned()
        } else {
            format!("rt::Outcome::Completed({expected:?}.parse::<rt::Integer>().unwrap())")
        };
        writeln!(
            native,
            "mod generated_{index} {{ {} }}",
            claim.oracle_source
        )
        .unwrap();
        writeln!(
            native,
            r#"
#[test]
fn case_{index}() {{
    let left = {left:?}.parse().unwrap();
    let right = {right:?}.parse().unwrap();
    let outcome = generated_{index}::{symbol}(&left, &right, &mut meter()).expect("generated ABI");
    assert_eq!(outcome, {expected_outcome});
    // The original runtime zero-divisor outcome is retained, not a derived-range refusal.
    let zero = rt::Integer::zero();
    assert_eq!(generated_{index}::{symbol}(&left, &zero, &mut meter()).unwrap(),
        rt::Outcome::Undefined(rt::Undefined::DivisionByZero));
}}
"#,
            symbol = claim.symbol
        )
        .unwrap();
    }
    run_generated_native(&native, selected.len(), "integer-division-bounds");
}

pub(crate) fn run_generated_native(native: &str, count: usize, label: &str) {
    let scratch = scratch_crate::TemporaryDirectory::new(label);
    let manifest = format!("[package]\nname=\"integer-division-derived-native\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[dependencies]\nfilament-exact = {{ {EXACT_DEPENDENCY_SOURCE} }}\n[workspace]\n");
    scratch_crate::write_manifest(&scratch.path, &manifest);
    fs::write(scratch.path.join("src/lib.rs"), native).unwrap();
    scratch_crate::resolve_seeded_lock(&scratch.path);
    let build = scratch_crate::lock_agreement_build();
    let output = Command::new(env!("CARGO"))
        .args(["test", "--locked", "--offline", "-j2", "--lib"])
        .env("CARGO_TARGET_DIR", scratch_crate::agreement_target())
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .current_dir(&scratch.path)
        .output()
        .expect("execute generated native oracles");
    drop(build);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains(&format!("{} passed; 0 failed", count)),
        "all generated cases execute: {stdout}"
    );
}

#[trace("TC-024", "FR-014-AC-21")]
#[test]
fn unbounded_operands_still_refuse_and_caller_domains_cannot_replace_inference() {
    unbounded_and_mismatch(&["x div y", "x mod y"]);
}

fn unbounded_and_mismatch(expressions: &[&str]) {
    for expression in expressions {
        for parameters in ["x: Integer, y: Int[1, 4]", "x: Int[-9, 9], y: Integer"] {
            let package = compiled(parameters, "Integer", expression);
            let node_id = body_id(&package, "subject");
            assert!(
                matches!(
                    derive_exact_scalar_items(&package, std::slice::from_ref(&node_id)).remove(0),
                    Err(ExactScalarRefusal::MissingBound { .. })
                ),
                "{parameters}: {expression}"
            );
            let domain = IntegerDomain::Bounded(interval("-100", "100"));
            let operation = if expression.contains("div") {
                ExactScalarOperation::IntegerDivision {
                    profile: filament_exact::DivisionProfile::Truncating,
                    domain,
                }
            } else {
                ExactScalarOperation::IntegerModulo { domain }
            };
            let generated =
                generate_exact_scalar_oracles(&package, &[ExactScalarItem { node_id, operation }])
                    .unwrap();
            assert!(matches!(
                &generated.claim_map.items[0].result,
                ClaimDisposition::Refused {
                    refusal: ExactScalarRefusal::MissingBound { .. }
                }
            ));
        }

        let package = compiled("x: Int[-9, 9], y: Int[1, 4]", "Integer", expression);
        let mut item = derived(&package);
        match &mut item.operation {
            ExactScalarOperation::IntegerDivision { domain, .. }
            | ExactScalarOperation::IntegerModulo { domain } => {
                *domain = IntegerDomain::Bounded(interval("-1", "1"))
            }
            _ => unreachable!(),
        }
        let node_id = item.node_id.clone();
        let generated = generate_exact_scalar_oracles(&package, &[item]).unwrap();
        assert_eq!(
            generated.claim_map.items[0].result,
            ClaimDisposition::Refused {
                refusal: ExactScalarRefusal::DerivedBoundMismatch { node_id }
            }
        );
    }
}

#[trace("TC-024", "FR-014-AC-21")]
#[test]
fn explicit_result_narrowing_keeps_its_own_bound() {
    explicit_result_bounds();
}

fn explicit_result_bounds() {
    for (code, profile) in [
        (914_000, Some(filament_exact::DivisionProfile::Truncating)),
        (914_010, None),
    ] {
        let (package, node_id) =
            reader_integer_operation(code, profile, Some((-9, 9)), Some((1, 4)), true);
        let item = derive_exact_scalar_items(&package, std::slice::from_ref(&node_id))
            .remove(0)
            .unwrap();
        assert_eq!(
            domain(&item),
            &IntegerDomain::Bounded(interval("-100", "100"))
        );
        let generated = generate_exact_scalar_oracles(&package, &[item]).unwrap();
        assert!(matches!(
            &generated.claim_map.items[0].result,
            ClaimDisposition::Generated(_)
        ));
    }
}

const NATIVE_SUPPORT: &str = r#"
use filament_exact as rt;
fn meter() -> rt::Meter {
    let ceiling = u64::from(u32::MAX);
    rt::Meter::new(rt::ScalarLimits {
        integer_bits: ceiling, decimal_digits: ceiling, scale_expansion: ceiling,
        text_input_bytes: ceiling, text_scalars: ceiling, normalized_scalars: ceiling,
        unit_edges: ceiling, value_occurrences: ceiling, work_units: ceiling, result_units: ceiling,
    })
}
"#;

/// This is a reader-admitted CG fixture, separate from the genuine FSL producer
/// cases above. It also admits explicit narrowing shapes that FSL's source
/// range prover may reject before emission.
fn reader_integer_operation(
    code: u32,
    profile: Option<filament_exact::DivisionProfile>,
    left: Option<(i64, i64)>,
    right: Option<(i64, i64)>,
    narrowed: bool,
) -> (CheckedPackageV2, filament_contract_model::CheckedNodeId) {
    let identity = if profile.is_some() {
        "filament.op.integer.div"
    } else {
        "filament.op.integer.mod"
    };
    reader_division_operation(
        code,
        identity,
        profile,
        left,
        right,
        narrowed.then_some((-100, 100)),
    )
}

/// A reader-admitted `div`, `rem` or `mod` node over parameters of the given ranges, with an
/// explicit narrowed result range when `narrow` is given.
pub(crate) fn reader_division_operation(
    code: u32,
    identity: &str,
    profile: Option<filament_exact::DivisionProfile>,
    left: Option<(i64, i64)>,
    right: Option<(i64, i64)>,
    narrow: Option<(i64, i64)>,
) -> (CheckedPackageV2, filament_contract_model::CheckedNodeId) {
    use crate::exact_scalar_generation::package as fixture;
    let mut builder = fixture::corpus_package_for_codes(&[fixture::T_INTEGER, fixture::T_TEXT]);
    let integer = fixture::key(fixture::T_INTEGER);
    let mut arguments = Vec::new();
    for (offset, name, bound) in [(0, "x", left), (1, "y", right)] {
        let typing = bound.map_or_else(
            || integer.clone(),
            |(lower, upper)| builder.bound(&fixture::Bound::Integer(lower, upper)),
        );
        builder.code(
            code + offset,
            "value",
            "parameter",
            &typing,
            fixture::parameter_body(name, offset),
        );
        arguments.push(fixture::reference(&fixture::key(code + offset)));
    }
    let (identity, laws) = if let Some(profile) = profile {
        let definition = fixture::integer_division_definition(profile);
        builder.select_definition(definition.clone());
        (identity, vec![fixture::law("integer_division", definition)])
    } else {
        (identity, Vec::new())
    };
    builder.application_bounded(
        code + 2,
        "expression",
        "binary",
        &integer,
        fixture::application(
            "binary",
            fixture::op_full(identity, laws, None, None),
            &integer,
            arguments,
        ),
        &[],
    );
    if let Some((lower, upper)) = narrow {
        let bound = builder.bound(&fixture::Bound::Integer(lower, upper));
        builder.narrow(code + 3, &fixture::key(code + 2), &bound);
    }
    builder.expression_occurrences();
    let (read, _, ids) = builder.read_resolved();
    let CheckedPackageV2ReadResult::Admitted(package) = read else {
        panic!("IR admits canonical division fixture: {read:?}");
    };
    (*package, ids.resolve(&fixture::code_id(code + 2)))
}

#[trace("TC-024", "FR-014-AC-18")]
#[test]
fn reader_admitted_division_laws_derive_from_both_operand_domains() {
    use filament_exact::DivisionProfile;
    let mut native = String::from(NATIVE_SUPPORT);
    for (index, profile) in DivisionProfile::ALL.into_iter().enumerate() {
        let (package, node) = reader_integer_operation(
            913_910 + 10 * u32::try_from(index).unwrap(),
            Some(profile),
            Some((i64::MIN, i64::MAX)),
            Some((-3, 3)),
            false,
        );
        let item = derive_exact_scalar_items(&package, &[node])
            .remove(0)
            .unwrap();
        let expected_domain =
            IntegerDomain::Bounded(interval("-9223372036854775808", "9223372036854775808"));
        assert_eq!(
            item.operation,
            ExactScalarOperation::IntegerDivision {
                profile,
                domain: expected_domain
            }
        );
        let generated = generate_exact_scalar_oracles(&package, &[item]).unwrap();
        let ClaimDisposition::Generated(claim) = &generated.claim_map.items[0].result else {
            panic!("reader-admitted division did not generate");
        };
        writeln!(
            native,
            "mod generated_{index} {{ {} }}",
            claim.oracle_source
        )
        .unwrap();
        let (negative_by_positive, positive_by_negative, negative_by_negative) = match profile {
            DivisionProfile::Truncating => ("-2", "-2", "2"),
            DivisionProfile::Floor => ("-3", "-3", "2"),
            DivisionProfile::Euclidean => ("-3", "-2", "3"),
        };
        writeln!(native, r#"
#[test]
fn case_{index}() {{
    for (left, right, expected) in [
        ("-9223372036854775808", "-1", "9223372036854775808"),
        ("-7", "3", {negative_by_positive:?}),
        ("7", "-3", {positive_by_negative:?}),
        ("-7", "-3", {negative_by_negative:?}),
    ] {{
        assert_eq!(generated_{index}::{symbol}(&left.parse().unwrap(), &right.parse().unwrap(), &mut meter()).unwrap(),
            rt::Outcome::Completed(expected.parse::<rt::Integer>().unwrap()));
    }}
    assert_eq!(generated_{index}::{symbol}(&rt::Integer::one(), &rt::Integer::zero(), &mut meter()).unwrap(),
        rt::Outcome::Undefined(rt::Undefined::DivisionByZero));
}}
"#, symbol = claim.symbol).unwrap();
    }
    run_generated_native(&native, 3, "reader-division-derived-bounds");
}

/// The generated remainder oracle's source for one genuine FSL `x rem y`, after the derived
/// descriptor is asserted: its profile, its inferred interval and its original operand bounds.
fn remainder_oracle(
    parameters: &str,
    profile: DivisionProfile,
    lower: &str,
    upper: &str,
) -> (String, String) {
    let package = compiled_with_profile(parameters, "Integer", "x rem y", Some(profile));
    let item = derived(&package);
    assert_eq!(
        item.operation,
        ExactScalarOperation::IntegerRemainder {
            profile,
            domain: IntegerDomain::Bounded(interval(lower, upper)),
        },
        "{parameters} under {profile:?}"
    );
    let generated = generate_exact_scalar_oracles(&package, &[item]).expect("generate rem oracle");
    let claim = &generated.claim_map.items[0];
    assert_eq!(claim.operation.provenance, OperationProvenance::IrConfirmed);
    assert_eq!(claim.operation.identity, "filament.op.integer.rem");
    let ClaimDisposition::Generated(claim) = &claim.result else {
        panic!("a derived remainder generates: {claim:?}")
    };
    let operand_bounds = claim.checked_bounds.clone();
    assert!(
        !operand_bounds.is_empty() && operand_bounds.len() <= 2,
        "only the operand bound ids are checked for an inferred interval: {operand_bounds:?}"
    );
    for bound in &operand_bounds {
        let node = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == bound)
            .expect("a checked bound is a real graph node");
        assert_eq!(node.node_tag.as_ref(), "bounded_domain");
    }
    (claim.oracle_source.clone(), claim.symbol.clone())
}

const MIN: &str = "-9223372036854775808";
const MAX: &str = "9223372036854775807";

/// FR-014-AC-81: `rem` derives under each law with the interval FR-014-AC-81 states, the
/// generated oracle equals direct owner `divide(.., Remainder, ..)` over the grid, and the two
/// mutants (`modulo`, and the quotient member) each disagree with the grid.
#[trace("TC-024", "FR-014-AC-81")]
#[test]
fn remainder_derives_each_law_and_the_generated_oracle_equals_the_owner_over_the_grid() {
    let mut native = String::from(NATIVE_SUPPORT);
    native.push_str(
        r#"
fn domain(lower: &str, upper: &str) -> rt::IntegerDomain {
    rt::IntegerDomain::Bounded(rt::IntegerInterval::new(lower.parse().unwrap(), upper.parse().unwrap()).unwrap())
}
const GRID: [&str; 11] = ["0", "1", "-1", "2", "-2", "7", "-7", "-9223372036854775808", "-9223372036854775807", "9223372036854775807", "3"];
fn owner(profile: rt::DivisionProfile, a: &str, b: &str, domain: &rt::IntegerDomain) -> rt::Outcome<rt::Integer> {
    rt::divide(profile, rt::DivisionMember::Remainder, &a.parse().unwrap(), &b.parse().unwrap(), domain, &mut meter())
}
"#,
    );
    let mut count = 0;
    for profile in DivisionProfile::ALL {
        let (lower, upper) = match profile {
            DivisionProfile::Euclidean => ("0", MAX),
            _ => ("-9223372036854775807", MAX),
        };
        let parameters = format!("x: Int[{MIN}, {MAX}], y: Int[{MIN}, {MAX}]");
        let (source, symbol) = remainder_oracle(&parameters, profile, lower, upper);
        let rt_profile = format!("rt::DivisionProfile::{profile:?}");
        let healthy_call = format!(
            "rt::divide({rt_profile}, rt::DivisionMember::Remainder, left, right, &domain, meter)"
        );
        assert!(source.contains(&healthy_call), "{source}");
        let quotient = source.replace(
            "rt::DivisionMember::Remainder",
            "rt::DivisionMember::Quotient",
        );
        let modulo = source.replace(&healthy_call, "rt::modulo(left, right, &domain, meter)");
        assert_ne!(quotient, source);
        assert_ne!(modulo, source);
        let case = count;
        count += 1;
        writeln!(native, "mod healthy_{case} {{ {source} }}\nmod quotient_{case} {{ {quotient} }}\nmod modulo_{case} {{ {modulo} }}").unwrap();
        // The named vectors, truncating / floor / Euclidean.
        let vectors: &[(&str, &str, &str, &str, &str)] = &[
            ("-7", "2", "-1", "1", "1"),
            ("-7", "-2", "-1", "-1", "1"),
            (MIN, "-1", "0", "0", "0"),
        ];
        let column = match profile {
            DivisionProfile::Truncating => 2,
            DivisionProfile::Floor => 3,
            DivisionProfile::Euclidean => 4,
        };
        let named = vectors
            .iter()
            .map(|vector| {
                let expected = [vector.2, vector.3, vector.4][column - 2];
                format!("({:?}, {:?}, {:?})", vector.0, vector.1, expected)
            })
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            native,
            r#"
#[test]
fn rem_case_{case}() {{
    let domain = domain({lower:?}, {upper:?});
    let profile = {rt_profile};
    for (a, b, expected) in [{named}] {{
        let outcome = healthy_{case}::{symbol}(&a.parse().unwrap(), &b.parse().unwrap(), &mut meter()).unwrap();
        assert_eq!(outcome, rt::Outcome::Completed(expected.parse::<rt::Integer>().unwrap()), "{{a}} rem {{b}}");
    }}
    let (mut quotient_differs, mut modulo_differs) = (false, false);
    for a in GRID {{
        for b in GRID {{
            let expected = owner(profile, a, b, &domain);
            let (left, right) = (a.parse().unwrap(), b.parse().unwrap());
            assert_eq!(healthy_{case}::{symbol}(&left, &right, &mut meter()).unwrap(), expected, "{{a}} rem {{b}}");
            quotient_differs |= quotient_{case}::{symbol}(&left, &right, &mut meter()).unwrap() != expected;
            modulo_differs |= modulo_{case}::{symbol}(&left, &right, &mut meter()).unwrap() != expected;
        }}
    }}
    assert!(quotient_differs, "the quotient-member mutant must fail the grid");
    assert!(modulo_differs || profile == rt::DivisionProfile::Euclidean, "the modulo mutant must fail the grid except under the law it equals");
    let zero = healthy_{case}::{symbol}(&rt::Integer::one(), &rt::Integer::zero(), &mut meter()).unwrap();
    assert_eq!(zero, rt::Outcome::Undefined(rt::Undefined::DivisionByZero));
}}
"#
        )
        .unwrap();
    }
    run_generated_native(&native, count, "remainder-derived-oracle");
}

/// FR-014-AC-81: the inferred interval over small operand bounds, a zero divisor range, and an
/// explicit narrowed result that excludes members: the remainder outside it is
/// `Refused(DivisionOutOfDomain { member: Remainder })`.
#[trace("TC-024", "FR-014-AC-81")]
#[test]
fn remainder_inferred_interval_and_explicit_result_refusal() {
    for profile in DivisionProfile::ALL {
        let signed = match profile {
            DivisionProfile::Euclidean => "0",
            _ => "-3",
        };
        for (parameters, lower, upper) in [
            ("x: Int[-9, 9], y: Int[1, 4]", signed, "3"),
            ("x: Int[-9, 9], y: Int[-4, -1]", signed, "3"),
            ("x: Int[-9, 9], y: Int[0, 0]", "0", "0"),
            (
                "x: Int[-9, 9], y: Int[-3, 2]",
                if signed == "0" { "0" } else { "-2" },
                "2",
            ),
        ] {
            remainder_oracle(parameters, profile, lower, upper);
        }
    }
    let mut native = String::from(NATIVE_SUPPORT);
    let mut count = 0;
    for (index, profile) in DivisionProfile::ALL.into_iter().enumerate() {
        let (package, node) = reader_division_operation(
            915_000 + 10 * u32::try_from(index).unwrap(),
            "filament.op.integer.rem",
            Some(profile),
            Some((-9, 9)),
            Some((1, 4)),
            Some((0, 1)),
        );
        let item = derive_exact_scalar_items(&package, &[node])
            .remove(0)
            .unwrap();
        assert_eq!(
            item.operation,
            ExactScalarOperation::IntegerRemainder {
                profile,
                domain: IntegerDomain::Bounded(interval("0", "1")),
            },
            "an explicit result bound stays authoritative"
        );
        let generated = generate_exact_scalar_oracles(&package, &[item]).unwrap();
        let ClaimDisposition::Generated(claim) = &generated.claim_map.items[0].result else {
            panic!("reader-admitted remainder did not generate");
        };
        writeln!(
            native,
            "mod generated_{index} {{ {} }}",
            claim.oracle_source
        )
        .unwrap();
        writeln!(
            native,
            r#"
#[test]
fn refusal_case_{index}() {{
    let outcome = generated_{index}::{symbol}(&"7".parse().unwrap(), &"4".parse().unwrap(), &mut meter()).unwrap();
    match outcome {{
        rt::Outcome::Refused(rt::Refusal::DivisionOutOfDomain {{ member, .. }}) => assert_eq!(member, rt::DivisionMember::Remainder),
        other => panic!("{{other:?}}"),
    }}
    assert_eq!(
        generated_{index}::{symbol}(&"8".parse().unwrap(), &"4".parse().unwrap(), &mut meter()).unwrap(),
        rt::Outcome::Completed(rt::Integer::zero())
    );
}}
"#,
            symbol = claim.symbol
        )
        .unwrap();
        count += 1;
    }
    run_generated_native(&native, count, "remainder-explicit-result");
}
