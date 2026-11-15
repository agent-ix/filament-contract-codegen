//! FR-054: `quire.op.option.present` and `quire.op.option.value` in a V2 clause harness.
//!
//! Each fixture is a clause compiled from FSL source through `Twin`, over an operation parameter
//! `amount` typed by an option type (an `Option<Int[0, 1000]>`, or a record `Pair` of `n` and
//! `flag` for the composite option). The default lane measures the generated source, the draw
//! domain by host enumeration and the typed refusals; the `kani` lane (`#[ignore]`) runs the
//! installed controls through CG's two-phase runner, one at a time.

#![allow(clippy::result_large_err)]

use filament_contract_codegen::{
    KaniObligationOutcome, KaniObligationRequest, KaniRunOutcome, KaniV2ClauseObligationHarness,
    ObligationDisposition, ObligationItem, OptionNotLoweredReason, ProofCeilings,
    UnsupportedObligation,
};
use ix_trace_rs::trace;

use super::kani_obligations_state_frame::native_twin::Twin;
use super::kani_obligations_v2_clause_controls::{
    append_inside_module, assert_two_phase_structure, host_source, remove_appended, run_installed,
    ScratchCrate, RESULT_CHECK,
};
use super::kani_obligations_v2_control::{negotiate_packages, negotiate_twin};

const GUARDED: &str = "result = (present(amount) implies value(amount) <= 1000)";
const GUARDED_COMPOSITE: &str = "result = (present(amount) implies value(amount).n <= 1000)";
const PRESENT: &str = "result = present(amount)";
/// The clause whose oracle reads the negation of the presence draw: the equivalent package of the
/// FR-054-AC-3 oracle mutant, because the runner refuses a hand-edited generated oracle.
const NOT_PRESENT: &str = "result = (not present(amount))";

const GUARDED_SUBJECT: &str = "!present || value <= 1000";
const BELOW_SUBJECT: &str = "!present || value < 1000";
const AND_SUBJECT: &str = "present && value <= 1000";
const COMPOSITE_SUBJECT: &str = "!present || n <= 1000";
const COMPOSITE_BELOW_SUBJECT: &str = "!present || n < 1000";

fn option_twin(condition: &str) -> Twin {
    Twin::with_bounded_amount_implication_condition(condition).with_option_amount()
}

fn composite_twin(condition: &str) -> Twin {
    Twin::with_bounded_amount_implication_condition(condition).with_record_option_amount()
}

fn harness_of(twin: &Twin) -> KaniV2ClauseObligationHarness {
    negotiate_twin(twin).expect("the option clause lowers")
}

fn subject_of(parameters: &str, body: &str) -> String {
    format!(
        "\npub mod subject {{\n    #[allow(unused_variables)]\n    pub fn check({parameters}) -> bool {{ {body} }}\n}}\n"
    )
}

fn scalar_subject(body: &str) -> String {
    subject_of("present: bool, value: i64", body)
}

fn composite_subject(body: &str) -> String {
    subject_of("present: bool, flag: bool, n: i64", body)
}

// ---- the draw: a presence bool and native payload leaves (FR-054-AC-1, AC-2, AC-9) -----------

/// Whether the generated source declares an enum with a payload-carrying variant: a heap enum arm
/// that would carry the option's value (FR-054-AC-1).
fn carries_a_value_in_an_enum_arm(source: &str) -> bool {
    let mut rest = source;
    while let Some(start) = rest.find("enum ") {
        let body = &rest[start..];
        let Some(open) = body.find('{') else { break };
        let Some(close) = body[open..].find("\n}") else {
            break;
        };
        if body[open..open + close]
            .lines()
            .any(|line| line.trim().ends_with("),") && !line.trim().starts_with("//"))
        {
            return true;
        }
        rest = &body[open + close..];
    }
    false
}

#[trace("FR-054-AC-1", "TC-068")]
#[test]
fn tc_068_an_option_is_drawn_as_a_presence_bool_and_a_native_i64() {
    let harness = harness_of(&option_twin(GUARDED));
    let source = &harness.rust.contents;
    assert_eq!(
        source
            .matches("let parameter_amount_present_current: bool = kani::any();")
            .count(),
        1,
        "{source}"
    );
    assert_eq!(
        source
            .matches("let parameter_amount_value_current: i64 = kani::any();")
            .count(),
        1
    );
    let harness_body = &source[source.find("fn prove_clause()").expect("the proof")..];
    for allocating in ["Box::", "Vec::", "vec!", "String::", "Rc::", "Arc::", "Option<"] {
        assert!(
            !harness_body.contains(allocating),
            "no allocation in the draw: {allocating}"
        );
    }
    assert!(!carries_a_value_in_an_enum_arm(source));
    // The mutant renders the value in an enum arm that carries it.
    let mutant = format!("{source}\nenum Slot {{\n    Absent,\n    Present(i64),\n}}\n");
    assert!(carries_a_value_in_an_enum_arm(&mutant));
}

/// The host stand-in for the `kani` crate: `any` yields the bytes `concrete_playback_run` queued
/// and a failed `assume` panics with a marker the enumeration tells from an assertion failure.
const HOST_KANI: &str = r#"
mod kani {
    use std::cell::RefCell;
    thread_local! { static QUEUE: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) }; }
    pub trait Arbitrary: Sized { fn from_bytes(bytes: &[u8]) -> Self; }
    impl Arbitrary for i64 {
        fn from_bytes(bytes: &[u8]) -> Self { i64::from_le_bytes(bytes.try_into().expect("eight bytes")) }
    }
    impl Arbitrary for bool {
        fn from_bytes(bytes: &[u8]) -> Self { bytes == [1] }
    }
    pub fn any<T: Arbitrary>() -> T {
        QUEUE.with(|queue| T::from_bytes(&queue.borrow_mut().remove(0)))
    }
    pub fn assume(condition: bool) { assert!(condition, "kani::assume failed"); }
    pub fn concrete_playback_run(values: Vec<Vec<u8>>, proof: fn()) {
        QUEUE.with(|queue| *queue.borrow_mut() = values);
        proof();
    }
    macro_rules! cover { ($($tokens:tt)*) => {}; }
    pub(crate) use cover;
}
"#;

/// Every draw of the scalar option over `-2..=1003`, once, each run through the unchanged harness
/// body: whether the harness's own assumptions admit it and whether the whole body then passes.
const SCALAR_ENUMERATION: &str = r#"
    #[test]
    fn enumerate_option_domain() {
        std::panic::set_hook(Box::new(|_| {}));
        for present in [false, true] {
            for value in -2..=1003_i64 {
                let outcome = std::panic::catch_unwind(|| {
                    crate::kani::concrete_playback_run(
                        vec![vec![u8::from(present)], value.to_le_bytes().to_vec()],
                        prove_clause,
                    )
                });
                let admitted = match &outcome {
                    Ok(()) => true,
                    Err(payload) => {
                        let text = payload.downcast_ref::<String>().cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_owned()))
                            .unwrap_or_default();
                        !text.contains("kani::assume failed")
                    }
                };
                println!("\nDRAW {present} - {value} admitted={admitted} passed={}", outcome.is_ok());
            }
        }
    }
"#;

/// The composite option: presence, then `flag`, then `n`.
const COMPOSITE_ENUMERATION: &str = r#"
    #[test]
    fn enumerate_option_domain() {
        std::panic::set_hook(Box::new(|_| {}));
        for present in [false, true] {
            for flag in [false, true] {
                for value in -2..=1003_i64 {
                    let outcome = std::panic::catch_unwind(|| {
                        crate::kani::concrete_playback_run(
                            vec![vec![u8::from(present)], vec![u8::from(flag)], value.to_le_bytes().to_vec()],
                            prove_clause,
                        )
                    });
                    let admitted = match &outcome {
                        Ok(()) => true,
                        Err(payload) => {
                            let text = payload.downcast_ref::<String>().cloned()
                                .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_owned()))
                                .unwrap_or_default();
                            !text.contains("kani::assume failed")
                        }
                    };
                    println!("\nDRAW {present} {flag} {value} admitted={admitted} passed={}", outcome.is_ok());
                }
            }
        }
    }
"#;

#[derive(Debug, PartialEq)]
struct Draw {
    present: bool,
    flag: Option<bool>,
    value: i64,
    admitted: bool,
    passed: bool,
}

fn parse_draws(output: &str) -> Vec<Draw> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("DRAW "))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (present, flag, value, admitted, passed) = match fields[..] {
                [present, "-", value, admitted, passed] => (present, None, value, admitted, passed),
                [present, flag, value, admitted, passed] => {
                    (present, Some(flag), value, admitted, passed)
                }
                _ => panic!("malformed draw line `{line}`"),
            };
            Draw {
                present: present == "true",
                flag: flag.map(|flag| flag == "true"),
                value: value.parse().expect("a payload"),
                admitted: admitted == "admitted=true",
                passed: passed == "passed=true",
            }
        })
        .collect()
}

/// Enumerate a harness's draws on the host, with `mutation` applied to the host source.
fn enumerate(
    label: &str,
    harness: &KaniV2ClauseObligationHarness,
    composite: bool,
    subject: &str,
    mutation: impl Fn(&str) -> String,
) -> Vec<Draw> {
    let sealed = &harness.rust.contents;
    let test = if composite {
        COMPOSITE_ENUMERATION
    } else {
        SCALAR_ENUMERATION
    };
    let with_test = append_inside_module(sealed, test);
    assert_eq!(remove_appended(&with_test, test), *sealed);
    let lib = format!("{}{HOST_KANI}{subject}", mutation(&host_source(&with_test)));
    let scratch = ScratchCrate::new(label, &lib, None);
    let (success, output) = scratch.host_test("enumerate_option_domain");
    assert!(success, "the enumeration runs to completion: {output}");
    let draws = parse_draws(&output);
    assert!(!draws.is_empty(), "the enumeration visited no draw: {output}");
    draws
}

fn admitted_draws(draws: &[Draw]) -> Vec<&Draw> {
    draws.iter().filter(|draw| draw.admitted).collect()
}

const ABSENT_RULE: &str =
    "kani::assume(parameter_amount_present_current || (parameter_amount_value_current == 0));";

#[trace("FR-054-AC-2", "TC-068")]
#[test]
fn tc_068_the_abstract_domain_of_an_int_option_is_exactly_1002_draws() {
    let harness = harness_of(&option_twin(GUARDED));
    let subject = scalar_subject(GUARDED_SUBJECT);
    let draws = enumerate("ir1448-ac2-healthy", &harness, false, &subject, str::to_owned);
    let admitted = admitted_draws(&draws);
    assert_eq!(admitted.len(), 1002, "1 absent draw and 1001 present draws");
    assert_eq!(
        admitted
            .iter()
            .filter(|draw| !draw.present)
            .map(|draw| draw.value)
            .collect::<Vec<_>>(),
        [0],
        "the absent draw is the canonical payload"
    );
    assert!(admitted
        .iter()
        .filter(|draw| draw.present)
        .map(|draw| draw.value)
        .eq(0..=1000), "each present payload once");
    assert!(
        admitted.iter().all(|draw| draw.passed),
        "the healthy subject passes every admitted draw"
    );

    // Mutant: an unconstrained absent payload admits the absent draw at payload 1 as well.
    let free_absent = enumerate("ir1448-ac2-free-absent", &harness, false, &subject, |source| {
        assert_eq!(source.matches(ABSENT_RULE).count(), 1);
        source.replace(ABSENT_RULE, "")
    });
    let absent_payloads = admitted_draws(&free_absent)
        .iter()
        .filter(|draw| !draw.present)
        .map(|draw| draw.value)
        .collect::<Vec<_>>();
    assert!(absent_payloads.contains(&1), "{absent_payloads:?}");
    assert!(absent_payloads.len() > 1);

    // Mutant: a widened present bound admits payload 1001.
    let widened = enumerate("ir1448-ac2-widened", &harness, false, &subject, |source| {
        source.replace("1000_i64", "1001_i64")
    });
    assert!(admitted_draws(&widened)
        .iter()
        .any(|draw| draw.present && draw.value == 1001));
}

#[trace("FR-054-AC-9", "TC-068")]
#[test]
fn tc_068_a_composite_option_is_1_plus_1001_times_2_draws_with_canonical_absent_leaves() {
    let harness = harness_of(&composite_twin(GUARDED_COMPOSITE));
    let source = &harness.rust.contents;
    let order = [
        "let parameter_amount_present_current: bool = kani::any();",
        "let parameter_amount_value_flag_current: bool = kani::any();",
        "let parameter_amount_value_n_current: i64 = kani::any();",
    ]
    .map(|draw| source.find(draw).unwrap_or_else(|| panic!("{draw}\n{source}")));
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "presence, then leaves in path order");
    let subject = composite_subject(COMPOSITE_SUBJECT);
    let draws = enumerate("ir1448-ac9-healthy", &harness, true, &subject, str::to_owned);
    let admitted = admitted_draws(&draws);
    assert_eq!(admitted.len(), 1 + 1001 * 2, "2003 draws");
    let absent = admitted
        .iter()
        .filter(|draw| !draw.present)
        .collect::<Vec<_>>();
    assert_eq!(absent.len(), 1);
    assert_eq!((absent[0].flag, absent[0].value), (Some(false), 0), "every absent leaf is canonical");
    assert!(admitted.iter().all(|draw| draw.passed));

    // Mutant: the absent Boolean left free admits the absent value twice, at flag true.
    let free_flag = enumerate("ir1448-ac9-free-flag", &harness, true, &subject, |source| {
        let rule = source
            .lines()
            .find(|line| line.starts_with("kani::assume(parameter_amount_present_current || ("))
            .expect("the canonical-absent rule")
            .to_owned();
        assert!(rule.contains("!parameter_amount_value_flag_current"), "{rule}");
        source.replace(&rule, "kani::assume(parameter_amount_present_current || (parameter_amount_value_n_current == 0));")
    });
    let absent_free = admitted_draws(&free_flag)
        .into_iter()
        .filter(|draw| !draw.present && draw.value == 0)
        .map(|draw| draw.flag)
        .collect::<Vec<_>>();
    assert_eq!(absent_free, [Some(false), Some(true)]);
}

// ---- the operations (FR-054-AC-3 to AC-7) ----------------------------------------------------

#[trace("FR-054-AC-3", "TC-068")]
#[test]
fn tc_068_the_presence_claim_lowers_to_the_presence_draw() {
    let source = harness_of(&option_twin(PRESENT)).rust.contents;
    assert!(
        source.contains("    Ok(parameter_amount_present.clone())"),
        "option.present reads the presence leaf:\n{source}"
    );
    assert!(
        !source.contains("return Err(ClauseStop::Undefined)"),
        "a presence read never stops"
    );
}

#[trace("FR-054-AC-4", "FR-054-AC-6", "TC-068")]
#[test]
fn tc_068_a_value_read_returns_the_payload_and_stops_only_when_absent() {
    let source = harness_of(&option_twin(GUARDED)).rust.contents;
    assert!(source.contains("    if !parameter_amount_present {\n        return Err(ClauseStop::Undefined);\n    }\n    Ok(parameter_amount_value.clone())"));
    // The implication evaluates its consequent only where the presence test needs it.
    assert!(source.contains("let right = if !left { false } else { value_"));
}

/// The scratch library with a test that runs `call` on the compiled oracle and prints its result.
fn oracle_result(label: &str, source: &str, call: &str) -> String {
    let test = format!(
        "\n#[cfg(test)]\nmod stop_probe {{\n    use super::*;\n    #[test]\n    fn stop_at() {{\n        let mut meter = rt::Meter::new(rt::ScalarLimits {{ integer_bits: u64::MAX, decimal_digits: u64::MAX, scale_expansion: u64::MAX, text_input_bytes: u64::MAX, text_scalars: u64::MAX, normalized_scalars: u64::MAX, unit_edges: u64::MAX, value_occurrences: u64::MAX, work_units: u64::MAX, result_units: u64::MAX }});\n        let outcome = {call};\n        println!(\"RESULT {{outcome:?}}\");\n    }}\n}}\n"
    );
    let scratch = ScratchCrate::new(label, &format!("{source}{test}"), None);
    let (success, output) = scratch.host_test("stop_probe::stop_at");
    assert!(success, "{output}");
    output
        .lines()
        .find_map(|line| line.split_once("RESULT ").map(|(_, rest)| rest.trim()))
        .unwrap_or_else(|| panic!("no result line: {output}"))
        .to_owned()
}

/// The step function of the `option.value` read in the generated oracle.
fn value_step(source: &str) -> String {
    let stop = source
        .find("return Err(ClauseStop::Undefined);")
        .expect("the absent stop");
    let head = source[..stop].rfind("fn value_").expect("the step function");
    let end = source[head + 3..].find('(').expect("its parameters") + head + 3;
    source[head + 3..end].trim().to_owned()
}

#[trace("FR-054-AC-5", "FR-054-AC-22", "TC-068")]
#[test]
fn tc_068_the_oracle_value_read_of_an_absent_option_stops_undefined() {
    let source = harness_of(&option_twin(GUARDED)).rust.contents;
    let step = value_step(&source);
    let absent = format!("{step}(false, &nx::NativeInteger::new(0_i128), true, &mut meter)");
    let present = format!("{step}(true, &nx::NativeInteger::new(7_i128), true, &mut meter)");
    assert_eq!(
        oracle_result("ir1448-ac5-absent", &source, &absent),
        "Err(Undefined)"
    );
    assert_eq!(
        oracle_result("ir1448-ac5-present", &source, &present),
        "Ok(NativeInteger(7))"
    );
    // Mutant: a default payload 0 in place of the stop makes the absent read complete.
    let defaulted = source.replace(
        "return Err(ClauseStop::Undefined);",
        "return Ok(nx::NativeInteger::new(0_i128));",
    );
    assert_ne!(defaulted, source);
    assert_eq!(
        oracle_result("ir1448-ac5-default", &defaulted, &absent),
        "Ok(NativeInteger(0))"
    );
}

#[trace("FR-054-AC-6", "FR-054-AC-22", "TC-068")]
#[test]
fn tc_068_the_guarded_oracle_completes_at_the_absent_draw_and_the_unguarded_one_stops() {
    let source = harness_of(&option_twin(GUARDED)).rust.contents;
    let absent = "oracle_clause(false, &nx::NativeInteger::new(0_i128), true, &mut meter)";
    assert_eq!(
        oracle_result("ir1448-ac6-guarded", &source, absent),
        "Ok(true)",
        "the guarded claim completes at the absent draw with no stop"
    );
    // Mutant: the consequent evaluated before the presence test.
    let before = "let right = if !left { false } else { ";
    let start = source.find(before).expect("the guard site");
    let end = start + source[start..].find(" };").expect("the end of the guard");
    let inner = &source[start + before.len()..end];
    let eager = format!(
        "{}let right = {inner};{}",
        &source[..start],
        &source[end + 3..]
    );
    assert_eq!(
        oracle_result("ir1448-ac6-eager", &eager, absent),
        "Err(Undefined)",
        "the eager read stops at presence 0, payload 0"
    );
}

#[trace("FR-054-AC-5", "FR-054-AC-6", "TC-068")]
#[test]
fn tc_068_the_fsl_checker_refuses_an_unguarded_value_read_so_no_admitted_package_holds_one() {
    let twin = Twin::with_bounded_amount_implication_condition("result = (value(amount) <= 1000)")
        .with_option_amount();
    std::panic::set_hook(Box::new(|_| {}));
    let panic = std::panic::catch_unwind(|| twin.emitted_package("BalanceNeverDrops"))
        .expect_err("FSL refuses the unguarded read");
    let _ = std::panic::take_hook();
    let text = panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
        .unwrap_or_default();
    assert!(text.contains("unproved-presence"), "{text}");
}

// ---- refusals (FR-054-AC-10 to AC-17) --------------------------------------------------------

fn option_refusal(twin: &Twin) -> (OptionNotLoweredReason, String) {
    match negotiate_twin(twin) {
        Err(UnsupportedObligation::OptionNotLowered {
            reason, detail, ..
        }) => (reason, detail),
        other => panic!("expected an option refusal, got {other:?}"),
    }
}

fn payload_twin(inner: &str) -> Twin {
    Twin::with_bounded_amount_implication_condition(GUARDED)
        .with_option_amount_over(inner.to_owned())
}

#[trace("FR-054-AC-10", "FR-054-AC-14", "TC-068")]
#[test]
fn tc_068_a_non_scalar_payload_is_refused_naming_its_family() {
    for (family, type_ref) in [
        ("string", "ix://quire/native/String"),
        ("uuid", "ix://quire/native/Uuid"),
        ("timestamp", "ix://quire/native/Timestamp"),
        ("float32", "ix://quire/native/Float32"),
        ("float64", "ix://quire/native/Float64"),
    ] {
        let (reason, detail) = option_refusal(&payload_twin(type_ref));
        assert_eq!(reason, OptionNotLoweredReason::PayloadNotScalar, "{family}");
        assert_eq!(detail, family);
    }
}

#[trace("FR-054-AC-11", "FR-054-AC-14", "TC-068")]
#[test]
fn tc_068_an_option_of_an_option_and_a_record_holding_an_option_are_nested() {
    let (reason, _) = option_refusal(&Twin::with_bounded_amount_implication_condition(GUARDED).with_nested_option_amount());
    assert_eq!(reason, OptionNotLoweredReason::NestedOption);
}

#[trace("FR-054-AC-16", "TC-068")]
#[test]
fn tc_068_an_option_of_an_unbounded_integer_is_not_a_payload_family_refusal() {
    let outcome = negotiate_twin(&payload_twin("ix://quire/native/Integer"));
    match outcome {
        Err(UnsupportedObligation::OptionNotLowered { reason, .. }) => {
            panic!("an unbounded integer payload is not an option shape refusal: {reason:?}")
        }
        other => println!("IR1448 unbounded integer payload disposition: {other:?}"),
    }
}

#[trace("FR-054-AC-15", "TC-068")]
#[test]
fn tc_068_a_refused_option_clause_spares_a_supported_sibling() {
    let (supported_package, supported_clause) =
        option_twin(GUARDED).emitted_package("BalanceNeverDrops");
    let (refused_package, refused_clause) =
        payload_twin("ix://quire/native/String").emitted_package("BalanceNeverDrops");
    let mut outcomes = negotiate_packages(&[
        (&refused_package, &refused_clause),
        (&supported_package, &supported_clause),
    ]);
    let sibling = outcomes.remove(1).expect("the sibling is supported");
    assert!(sibling.is_some(), "the sibling keeps its harness");
    assert!(matches!(
        outcomes.remove(0),
        Err(UnsupportedObligation::OptionNotLowered { .. })
    ));
    let _ = (ObligationDisposition::Unsupported {
        reason: UnsupportedObligation::RenderFailed,
    },);
}

// ---- kani lane (FR-054-AC-21) ----------------------------------------------------------------

fn assert_verified(label: &str, twin: &Twin, subject: &str) {
    let harness = harness_of(twin);
    let scratch = ScratchCrate::new(label, &format!("{}{subject}", harness.rust.contents), None);
    let run = run_installed(&scratch, &harness);
    assert_eq!(run.evidence.outcome, KaniRunOutcome::Verified);
    assert!(
        run.evidence
            .checks
            .iter()
            .any(|check| format!("{:?}", check.status) == "Satisfied"),
        "the cover is satisfied"
    );
}

/// The playback vectors of a falsified run, in draw order.
fn assert_falsified(label: &str, twin: &Twin, subject: &str, expected: &[Vec<u8>]) {
    let harness = harness_of(twin);
    let scratch = ScratchCrate::new(label, &format!("{}{subject}", harness.rust.contents), None);
    let run = run_installed(&scratch, &harness);
    let counterexample = assert_two_phase_structure(&run, &harness, RESULT_CHECK);
    let vectors = counterexample
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("vec![") && !line.contains("concrete_vals"))
        .map(|line| {
            line.trim_start_matches("vec![")
                .split(']')
                .next()
                .unwrap()
                .split(',')
                .map(|byte| byte.trim().parse::<u8>().expect("a playback byte"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    println!("IR1448 playback {vectors:?}");
    assert_eq!(vectors, expected, "{counterexample}");
}

fn payload(value: i64) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

#[trace("FR-054-AC-3", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_presence_claim_verifies() {
    assert_verified(
        "ir1448-ac3-healthy",
        &option_twin(PRESENT),
        &scalar_subject("present"),
    );
}

#[trace("FR-054-AC-3", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_negated_presence_read_is_falsified() {
    // The equivalent package of the oracle that returns the negation of the presence leaf: the
    // subject `present` disagrees with `not present` at every draw.
    assert_falsified(
        "ir1448-ac3-mutant",
        &option_twin(NOT_PRESENT),
        &scalar_subject("present"),
        &[vec![0], payload(0)],
    );
}

#[trace("FR-054-AC-4", "FR-054-AC-6", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_guarded_value_claim_verifies() {
    assert_verified(
        "ir1448-ac4-healthy",
        &option_twin(GUARDED),
        &scalar_subject(GUARDED_SUBJECT),
    );
}

#[trace("FR-054-AC-4", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_value_below_the_bound_is_falsified_at_1000() {
    assert_falsified(
        "ir1448-ac4-mutant",
        &option_twin(GUARDED),
        &scalar_subject(BELOW_SUBJECT),
        &[vec![1], payload(1000)],
    );
}

#[trace("FR-054-AC-7", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_present_and_value_subject_is_falsified_when_absent() {
    assert_falsified(
        "ir1448-ac7-mutant",
        &option_twin(GUARDED),
        &scalar_subject(AND_SUBJECT),
        &[vec![0], payload(0)],
    );
}

#[trace("FR-054-AC-8", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_composite_option_verifies() {
    assert_verified(
        "ir1448-ac8-healthy",
        &composite_twin(GUARDED_COMPOSITE),
        &composite_subject(COMPOSITE_SUBJECT),
    );
}

#[trace("FR-054-AC-8", "FR-054-AC-21", "TC-068")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_068_installed_composite_n_below_the_bound_is_falsified_at_1000() {
    assert_falsified(
        "ir1448-ac8-mutant",
        &composite_twin(GUARDED_COMPOSITE),
        &composite_subject(COMPOSITE_BELOW_SUBJECT),
        &[vec![1], vec![0], payload(1000)],
    );
}

#[allow(dead_code)]
fn _types(_: ProofCeilings, _: KaniObligationOutcome, _: KaniObligationRequest<'_>, _: ObligationItem<'_>) {}
