//! FR-015-AC-152 to FR-015-AC-173: `let`, `if` and `case` in a V2 clause harness.
//!
//! Each fixture is a clause compiled from FSL source through `Twin`, or a constructed package where
//! the criterion needs a shape FSL refuses. The default lane measures the generated source and the
//! typed refusals; the `kani` lane (`#[ignore]`) runs the installed controls through CG's
//! two-phase runner.

#![allow(clippy::result_large_err)]

use filament_contract_codegen::{
    negotiate_kani_native_obligations, KaniObligationOutcome, KaniObligationRequest,
    KaniV2ClauseObligationHarness, ObligationDisposition, ObligationItem, ProofCeilings,
    UnsupportedObligation, V2ClauseControlReason,
};
use ix_trace_rs::trace;

use super::kani_obligations_state_frame::native_twin::Twin;

#[path = "../checked_package_support/rekey.rs"]
#[allow(dead_code, clippy::duplicate_mod)]
mod rekey;

#[allow(clippy::duplicate_mod)]
#[path = "../checked_package_support/draft_rows.rs"]
mod draft_rows;

use filament_contract_model::{
    CheckedNodeId, CheckedNodeOwner, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult, CheckedTerm,
};
use serde_json::json;

pub(crate) const SUBJECT: &str = "crate::subject::check";

fn ceilings() -> ProofCeilings {
    ProofCeilings {
        memory_bytes: std::num::NonZeroU64::new(24 * 1024 * 1024 * 1024).unwrap(),
        wall_clock: std::time::Duration::from_secs(30 * 60),
    }
}

/// What the generator makes of the clause `result = (condition)` over the bounded amount fixture.
pub(crate) fn negotiate_twin(
    twin: &Twin,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    let (package, clause) = twin.emitted_package("BalanceNeverDrops");
    negotiate_package(&package, &clause)
}

pub(crate) fn negotiate_package(
    package: &CheckedPackageV2,
    clause: &CheckedNodeId,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    negotiate_packages(&[(package, clause)])
        .remove(0)
        .map(|harness| harness.expect("a single item emits its harness"))
}

/// One request holding every `(package, clause)` item: per item, its refusal or its harness
/// (`None` when the item is neither refused nor emitted as a V2 clause harness).
pub(crate) fn negotiate_packages(
    items: &[(&CheckedPackageV2, &CheckedNodeId)],
) -> Vec<Result<Option<KaniV2ClauseObligationHarness>, UnsupportedObligation>> {
    let claims: Vec<_> = items
        .iter()
        .map(|(package, clause)| crate::common::v2_claim::claim_for(package, clause))
        .collect();
    let requests: Vec<_> = items
        .iter()
        .zip(&claims)
        .map(|((package, clause), claim)| ObligationItem::V2Clause {
            package,
            clause,
            claim,
            dependencies: None,
        })
        .collect();
    let outcome = negotiate_kani_native_obligations(&KaniObligationRequest {
        ceilings: ceilings(),
        items: &requests,
        subject_path: SUBJECT,
        unwind: 3,
    })
    .expect("the clauses negotiate");
    match outcome {
        KaniObligationOutcome::Emitted {
            v2_clause_harnesses,
            records,
            ..
        } => {
            let mut harnesses = v2_clause_harnesses.into_iter();
            records
                .iter()
                .map(|record| match &record.disposition {
                    ObligationDisposition::Unsupported { reason } => Err(reason.clone()),
                    _ => Ok(harnesses.next()),
                })
                .collect()
        }
        KaniObligationOutcome::Rejected { records } => panic!("rejected: {records:?}"),
    }
}

pub(crate) fn negotiate_condition(
    condition: &str,
) -> Result<KaniV2ClauseObligationHarness, UnsupportedObligation> {
    negotiate_twin(&Twin::with_bounded_amount_implication_condition(condition))
}

// ---- constructed fixtures: an arithmetic node bounded by its own result type ----------------

/// The clause of `twin` with every integer addition node typed by the `amount` range, the shape of
/// FR-015-AC-168, FR-015-AC-169 and FR-015-AC-172. FSL types its arithmetic as mathematical
/// integers, so a bounded node is a constructed package, admitted through Contract IR.
fn bounded_addition(twin: &Twin) -> (CheckedPackageV2, CheckedNodeId) {
    let mut changed = 0;
    let package = rewrite_nodes(twin, "BalanceNeverDrops", |frame, nodes, _, node| {
        if node["body"]["operation"]["identity"] != "filament.op.integer.add" {
            return false;
        }
        let bound_position = nodes
            .iter()
            .position(|node| {
                node["node_tag"] == "bounded_domain" && node["semantic_form"] == "integer_range"
            })
            .expect("the amount range");
        let bound = frame
            .document_value_in_row(bound_position, &nodes[bound_position]["node_id"])
            .unwrap();
        node["semantic_type"] = bound.clone();
        node["body"]["result_type"] = bound;
        changed += 1;
        true
    });
    assert_eq!(changed, 1, "one addition node is bounded");
    package
}

/// The clause `clause_name` of `twin`'s emitted package with every node `edit` changes rewritten
/// on the wire (its operand row re-authored), admitted again through Contract IR. `edit` gets the
/// original holding rows, the original nodes, the node position and the node's document value.
pub(crate) fn rewrite_nodes(
    twin: &Twin,
    clause_name: &str,
    mut edit: impl FnMut(
        &fsl_replay::WireGraph,
        &[serde_json::Value],
        usize,
        &mut serde_json::Value,
    ) -> bool,
) -> (CheckedPackageV2, CheckedNodeId) {
    let (package, clause) = twin.emitted_package(clause_name);
    let mut wire = json!({
        "contract_version": "filament.checked-package/v2",
        "name": package.name(),
        "lock": package.lock(),
        "semantic_graph": package.graph(),
        "source_map": package.source_map(),
        "capability_report": package.capability_report(),
        "diagnostics": package.diagnostics(),
    });
    let frame = fsl_replay::WireGraph::of_package_value(&wire).expect("original holding rows");
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap().clone();
    let package_name = wire["name"].as_str().unwrap().to_owned();
    for (position, node) in nodes.iter().enumerate() {
        let mut node = frame.document_value_in_row(position, node).unwrap();
        if !edit(&frame, &nodes, position, &mut node) {
            continue;
        }
        let mut row = Vec::new();
        draft_rows::author_row(&package_name, &mut node, &mut row);
        wire["semantic_graph"]["nodes"][position] = node;
        wire["semantic_graph"]["operand_rows"][position] = json!(row);
    }
    let inputs = twin.state_clause_inputs(&package, &clause, clause_name, (0, 0), (0, 0));
    let [document] = inputs.packages.as_slice() else {
        panic!("one original model document")
    };
    let selected_model = package
        .resolved_graph()
        .nodes
        .iter()
        .find_map(|node| match &node.owner {
            Some(CheckedNodeOwner::ModelRoot { identity }) => Some(identity.to_string()),
            _ => None,
        })
        .expect("the twin selects one model document");
    let mut evidence = CheckedPackageEvidence::new();
    evidence
        .insert_domain_package_document(selected_model, document.bytes.clone())
        .unwrap();
    evidence.support_feature("filament.value.complete/v1");
    let resolved = rekey::resolve_wire(&wire, &evidence, CheckedPackageReadLimits::bounded());
    let CheckedPackageV2ReadResult::Admitted(constructed) = resolved.outcome else {
        panic!("the rewritten package is admitted: {:?}", resolved.outcome)
    };
    (*constructed, resolved.ids.resolve(&clause))
}

fn constructed_harness(condition: &str, maximum: i64) -> KaniV2ClauseObligationHarness {
    let twin =
        Twin::with_bounded_amount_implication_condition(condition).with_amount_maximum(maximum);
    let (package, clause) = bounded_addition(&twin);
    negotiate_package(&package, &clause).expect("the constructed clause lowers")
}

// ---- default lane: the generated source and the typed refusals -----------------------------

fn oracle_of(condition: &str) -> String {
    let harness = negotiate_condition(condition).expect("the control clause lowers");
    harness.rust.contents.clone()
}

fn expectation_of(source: &str) -> String {
    source
        .lines()
        .find(|line| line.starts_with("let expected = "))
        .expect("the harness carries the independent expectation")
        .to_owned()
}

#[trace("FR-015-AC-152", "TC-025")]
#[test]
fn tc_025_a_let_renders_a_native_binding_and_a_substituted_expectation() {
    let source = oracle_of(LET);
    assert!(source.contains("let bound_0 = value_3("), "{source}");
    assert!(source.contains("value_6(parameter_amount, result_result, &bound_0, meter)"));
    let expected = expectation_of(&source);
    assert!(expected.contains("(i128 :: from (parameter_amount_current) + 1_i128) <= 1000_i128"));
    assert!(
        !expected.contains("bound_"),
        "the expectation substitutes the bound value"
    );
}

#[trace("FR-015-AC-153", "TC-025")]
#[test]
fn tc_025_an_if_renders_a_native_if_and_a_selecting_expectation() {
    let source = oracle_of(IF);
    assert!(source.contains("if value_"), "{source}");
    assert!(source.contains("} else {"));
    let expected = expectation_of(&source);
    assert!(expected.contains("match (i128 :: from (parameter_amount_current) < 500_i128)"));
    assert!(expected.contains("true =>") && expected.contains("false =>"));
}

#[trace("FR-015-AC-164", "TC-025")]
#[test]
fn tc_025_a_let_over_an_if_and_an_if_over_a_let_each_yield_one_harness() {
    for condition in [
        NESTED,
        "result = ((if amount < 500 then (let y = amount + 1 in y) else amount - 1) >= 500)",
        "result = ((let y = amount + 1 in y <= 1000) and (if amount < 500 then amount >= 0 else amount < 900))",
    ] {
        let harness = negotiate_condition(condition)
            .unwrap_or_else(|reason| panic!("{condition}: {reason:?}"));
        assert_eq!(harness.rust.contents.matches("#[kani::proof]").count(), 1);
    }
}

/// The reason, node and operation identity of a refused control clause.
fn control_refusal(condition: &str) -> (V2ClauseControlReason, String) {
    match negotiate_condition(condition) {
        Err(UnsupportedObligation::V2ClauseControlNotSupported {
            reason,
            operation_identity,
            ..
        }) => (reason, operation_identity),
        other => panic!("{condition}: expected a control refusal, got {other:?}"),
    }
}

fn operator_refusal(condition: &str) -> String {
    match negotiate_condition(condition) {
        Err(UnsupportedObligation::V2ClauseOperatorNotSupported {
            operation_identity, ..
        }) => operation_identity,
        other => panic!("{condition}: expected an operator refusal, got {other:?}"),
    }
}

/// A twin whose clause `result = (let q = amount in true)` binds the nullable `amount`.
fn optional_binding() -> Twin {
    Twin::with_bounded_amount_implication_condition("result = (let q = amount in true)")
        .with_optional_amount()
}

#[trace("FR-015-AC-160", "FR-015-AC-167", "TC-025")]
#[test]
fn tc_025_a_let_binding_a_non_native_value_is_a_typed_control_refusal_and_spares_a_sibling() {
    let (sibling_package, sibling_clause) =
        Twin::with_bounded_amount_implication_condition(LET).emitted_package("BalanceNeverDrops");
    for (family, twin) in [
        (
            "text",
            Twin::with_bounded_amount_implication_condition(r#"result = (let q = "a" in true)"#),
        ),
        ("option", optional_binding()),
    ] {
        let (package, clause) = twin.emitted_package("BalanceNeverDrops");
        let mut outcomes =
            negotiate_packages(&[(&package, &clause), (&sibling_package, &sibling_clause)]);
        let sibling = outcomes
            .remove(1)
            .expect("the supported sibling is unaffected");
        assert!(sibling.is_some(), "{family}: the sibling keeps its harness");
        match outcomes.remove(0) {
            Err(UnsupportedObligation::V2ClauseControlNotSupported {
                node_id,
                control_node_id,
                operation_identity,
                reason,
            }) => {
                assert_eq!(
                    reason,
                    V2ClauseControlReason::LetValueNotNativeScalar,
                    "{family}"
                );
                assert_eq!(operation_identity, "filament.op.control.let", "{family}");
                assert_eq!(node_id, clause, "{family}: the clause node");
                let Some(CheckedTerm::Application { operation, .. }) =
                    package.term(&control_node_id)
                else {
                    panic!("{family}: the control node holds an application")
                };
                assert_eq!(
                    operation.identity(),
                    Some("filament.op.control.let"),
                    "{family}"
                );
            }
            other => panic!("{family}: expected a control refusal, got {other:?}"),
        }
    }
}

#[trace("FR-015-AC-166", "TC-025")]
#[test]
fn tc_025_the_first_refusal_the_walk_meets_decides() {
    // An unsupported operator met before a failing control.
    assert_eq!(
        operator_refusal(r#"result = (("a" = "a") and (let q = "a" in true))"#),
        "filament.op.string.eq"
    );
    // A failing control met before an unsupported operator: the control's shape check runs on
    // entering its node, before its operands are walked.
    let (reason, _) = control_refusal(r#"result = ((let q = "a" in true) and ("a" = "a"))"#);
    assert_eq!(reason, V2ClauseControlReason::LetValueNotNativeScalar);
    // Two failing controls report the first met.
    let (reason, _) =
        control_refusal(r#"result = ((let p = "a" in true) and (let q = "b" in true))"#);
    assert_eq!(reason, V2ClauseControlReason::LetValueNotNativeScalar);
}

#[trace("FR-015-AC-166", "TC-025")]
#[test]
fn tc_025_a_let_of_an_option_parameter_is_a_control_refusal_not_an_option_refusal() {
    match negotiate_twin(&optional_binding()) {
        Err(UnsupportedObligation::V2ClauseControlNotSupported { reason, .. }) => {
            assert_eq!(reason, V2ClauseControlReason::LetValueNotNativeScalar)
        }
        other => panic!("expected the control refusal, got {other:?}"),
    }
}

// ---- kani lane ---------------------------------------------------------------------------

use filament_contract_codegen::KaniRunOutcome;

use super::kani_obligations_v2_clause_controls::{
    assert_two_phase_falsified, run_installed, subject_module, ScratchCrate, RESULT_CHECK,
};

const LET: &str = "result = (let y = amount + 1 in y <= 1000)";
const IF: &str = "result = ((if amount < 500 then amount + 1 else amount - 1) >= 500)";
const NESTED: &str =
    "result = (let y = (if amount < 500 then amount + 1 else amount - 1) in y >= 500)";

const LET_HEALTHY: &str = "amount + 1 <= 1000";
const LET_MUTANT: &str = "amount + 2 <= 1000";
const IF_HEALTHY: &str = "(if amount < 500 { amount + 1 } else { amount - 1 }) >= 500";
const IF_MUTANT: &str = "(if amount <= 500 { amount + 1 } else { amount - 1 }) >= 500";

fn kani_crate(
    label: &str,
    condition: &str,
    subject_body: &str,
) -> (ScratchCrate, KaniV2ClauseObligationHarness) {
    let harness = negotiate_condition(condition).expect("the control clause lowers");
    let lib = format!("{}{}", harness.rust.contents, subject_module(subject_body));
    (ScratchCrate::new(label, &lib, None), harness)
}

fn assert_verified(label: &str, condition: &str, subject_body: &str) {
    let (scratch, harness) = kani_crate(label, condition, subject_body);
    let run = run_installed(&scratch, &harness);
    assert_eq!(run.evidence.outcome, KaniRunOutcome::Verified);
    assert!(
        run.evidence.proof_phase.is_none(),
        "a verified proof makes one launch"
    );
    assert!(
        run.evidence
            .checks
            .iter()
            .any(|check| format!("{:?}", check.status) == "Satisfied"),
        "the final cover is satisfied"
    );
}

fn assert_falsified_at(label: &str, condition: &str, subject_body: &str, input: i64) {
    let (scratch, harness) = kani_crate(label, condition, subject_body);
    let run = run_installed(&scratch, &harness);
    let values = assert_two_phase_falsified(&run, &harness, RESULT_CHECK);
    assert_eq!(values, vec![input]);
}

#[trace("FR-015-AC-156", "FR-015-AC-152", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_let_healthy_subject_verifies() {
    assert_verified("ir1447-let-healthy", LET, LET_HEALTHY);
}

#[trace("FR-015-AC-156", "FR-015-AC-152", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_let_mutant_is_falsified_at_999() {
    assert_falsified_at("ir1447-let-mutant", LET, LET_MUTANT, 999);
}

#[trace("FR-015-AC-157", "FR-015-AC-153", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_if_healthy_subject_verifies() {
    assert_verified("ir1447-if-healthy", IF, IF_HEALTHY);
}

#[trace("FR-015-AC-157", "FR-015-AC-153", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_if_mutant_is_falsified_at_500() {
    assert_falsified_at("ir1447-if-mutant", IF, IF_MUTANT, 500);
}

#[trace("FR-015-AC-165", "FR-015-AC-164", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_nested_healthy_subject_verifies() {
    assert_verified("ir1447-nested-healthy", NESTED, IF_HEALTHY);
}

#[trace("FR-015-AC-165", "FR-015-AC-164", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_nested_mutant_is_falsified_at_500() {
    assert_falsified_at("ir1447-nested-mutant", NESTED, IF_MUTANT, 500);
}

// ---- lazy branches and strict initializers (FR-015-AC-168, AC-169, AC-172) -----------------

const LAZY: &str = "result = ((if amount < 1000 then amount else amount + 1000) >= 0)";
const LAZY_SUBJECT: &str = "(if amount < 1000 { amount } else { amount + 1000 }) >= 0";
const SELECTED: &str = "result = ((if amount < 1000 then amount else amount + 1) >= 0)";
const SELECTED_SUBJECT: &str = "(if amount < 1000 { amount } else { amount + 1 }) >= 0";
const UNREAD: &str = "result = (let y = amount + 1 in amount >= 0)";
const UNREAD_SUBJECT: &str = "amount >= 0";
const POSTCONDITION_CHECK: &str = "the V2 postcondition holds for the produced result";

/// The generated source with the one `if` step evaluating both branches before it selects.
fn eager_branches(source: &str) -> String {
    let start = source
        .find("    if value_")
        .expect("the generated `if` step");
    let end = start
        + source[start..]
            .find("\n}\n")
            .expect("the end of the `if` step");
    let step = &source[start..end];
    let [condition, then, other] = step
        .split(['\n'])
        .map(str::trim)
        .filter(|line| line.starts_with("if value_") || line.starts_with("value_"))
        .collect::<Vec<_>>()[..]
    else {
        panic!("the `if` step has a condition and two branches: {step}");
    };
    let condition = condition.trim_start_matches("if ").trim_end_matches(" {");
    let eager = format!(
        "    let selected = {condition};\n    let then_value = {then}?;\n    let other_value = {other}?;\n    if selected {{ Ok(then_value) }} else {{ Ok(other_value) }}"
    );
    format!("{}{eager}{}", &source[..start], &source[end..])
}

/// The generated source with the unread `let` initializer replaced by a constant.
fn skip_initializer(source: &str) -> String {
    let start = source
        .find("    let bound_0 = value_")
        .expect("the generated `let` step");
    let end = start + source[start..].find('\n').unwrap();
    format!(
        "{}    let bound_0 = nx::NativeInteger::new(0_i128);{}",
        &source[..start],
        &source[end..]
    )
}

/// The generated source with an out-of-domain refusal of the whole oracle mapped to a completed `false`.
fn refusal_as_false(source: &str) -> String {
    let call = "    value_";
    let at = source.rfind("pub fn oracle_clause(").unwrap();
    let start = at + source[at..].find(call).unwrap();
    let end = start + source[start..].find('\n').unwrap();
    let inner = &source[start..end];
    format!(
        "{}    match {} {{ Err(ClauseStop::IntegerOutOfDomain) => Ok(false), other => other }}{}",
        &source[..start],
        inner.trim(),
        &source[end..]
    )
}

fn kani_crate_from(label: &str, rust: &str, subject_body: &str) -> ScratchCrate {
    ScratchCrate::new(
        label,
        &format!("{rust}{}", subject_module(subject_body)),
        None,
    )
}

fn stop_of_oracle_at(label: &str, rust: &str, amount: i64) -> String {
    let test = format!(
        "\n#[cfg(test)]\nmod stop_probe {{\n    use super::*;\n    #[test]\n    fn stop_at() {{\n        let mut meter = rt::Meter::new(rt::ScalarLimits {{ integer_bits: u64::MAX, decimal_digits: u64::MAX, scale_expansion: u64::MAX, text_input_bytes: u64::MAX, text_scalars: u64::MAX, normalized_scalars: u64::MAX, unit_edges: u64::MAX, value_occurrences: u64::MAX, work_units: u64::MAX, result_units: u64::MAX }});\n        let outcome = oracle_clause(&nx::NativeInteger::new({amount}_i128), true, &mut meter);\n        println!(\"STOP {{outcome:?}}\");\n    }}\n}}\n"
    );
    let scratch = ScratchCrate::new(label, &format!("{rust}{test}"), None);
    let (success, output) = scratch.host_test("stop_probe::stop_at");
    assert!(success, "{output}");
    output
        .lines()
        .find_map(|line| line.split_once("STOP ").map(|(_, rest)| rest.trim()))
        .unwrap_or_else(|| panic!("no stop line: {output}"))
        .to_owned()
}

#[trace("FR-015-AC-168", "TC-025")]
#[test]
fn tc_025_the_oracle_evaluates_only_the_selected_branch() {
    let harness = constructed_harness(LAZY, 999);
    let source = &harness.rust.contents;
    // At amount 0 the else branch `amount + 1000` is never selected: the oracle returns `Ok(true)`.
    assert_eq!(stop_of_oracle_at("ir1447-lazy-host", source, 0), "Ok(true)");
    // The eager mutant evaluates `amount + 1000` outside Int[0, 999] on every draw.
    let eager = eager_branches(source);
    assert_eq!(
        stop_of_oracle_at("ir1447-eager-host", &eager, 0),
        "Err(IntegerOutOfDomain)"
    );
}

#[trace("FR-015-AC-169", "TC-025")]
#[test]
fn tc_025_a_selected_refusing_branch_stops_the_oracle_and_is_not_assumed_away() {
    let harness = constructed_harness(SELECTED, 1000);
    let source = &harness.rust.contents;
    assert_eq!(
        stop_of_oracle_at("ir1447-selected-host", source, 1000),
        "Err(IntegerOutOfDomain)"
    );
    // The `false`-mapping mutant is told apart only by this recorded stop.
    assert_eq!(
        stop_of_oracle_at(
            "ir1447-selected-false-host",
            &refusal_as_false(source),
            1000
        ),
        "Ok(false)"
    );
}

#[trace("FR-015-AC-172", "TC-025")]
#[test]
fn tc_025_an_unread_let_initializer_is_still_evaluated_once() {
    let harness = constructed_harness(UNREAD, 1000);
    let source = &harness.rust.contents;
    assert_eq!(
        stop_of_oracle_at("ir1447-unread-host", source, 1000),
        "Err(IntegerOutOfDomain)"
    );
    assert_eq!(
        stop_of_oracle_at("ir1447-unread-skip-host", &skip_initializer(source), 1000),
        "Ok(true)"
    );
}

#[trace("FR-015-AC-168", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_lazy_branch_healthy_verifies() {
    let harness = constructed_harness(LAZY, 999);
    let scratch = kani_crate_from("ir1447-lazy-healthy", &harness.rust.contents, LAZY_SUBJECT);
    let run = run_installed(&scratch, &harness);
    assert_eq!(run.evidence.outcome, KaniRunOutcome::Verified);
    assert!(run
        .evidence
        .checks
        .iter()
        .any(|check| format!("{:?}", check.status) == "Satisfied"));
}

#[trace("FR-015-AC-169", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_selected_refusing_branch_is_falsified_at_1000() {
    let harness = constructed_harness(SELECTED, 1000);
    let scratch = kani_crate_from("ir1447-selected", &harness.rust.contents, SELECTED_SUBJECT);
    let run = run_installed(&scratch, &harness);
    assert_eq!(
        assert_two_phase_falsified(&run, &harness, POSTCONDITION_CHECK),
        vec![1000]
    );
}

#[trace("FR-015-AC-172", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_unread_initializer_is_falsified_at_1000() {
    let harness = constructed_harness(UNREAD, 1000);
    let scratch = kani_crate_from("ir1447-unread", &harness.rust.contents, UNREAD_SUBJECT);
    let run = run_installed(&scratch, &harness);
    assert_eq!(
        assert_two_phase_falsified(&run, &harness, POSTCONDITION_CHECK),
        vec![1000]
    );
}

/// The eager evaluation FR-015-AC-168 describes, as a package: each branch value is bound by a
/// strict `let` before the selection, so the never-selected `amount + 1000` is evaluated on every
/// draw. The runner refuses a mutated generated source, so the mutant is the equivalent clause.
const EAGER: &str =
    "result = (let e = amount + 1000 in (if amount < 1000 then amount else e) >= 0)";
/// The unread-initializer skip FR-015-AC-172 describes, as a package: the same clause without the
/// unread `let`.
const SKIPPED: &str = "result = (amount >= 0)";

#[trace("FR-015-AC-168", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_eager_branch_equivalent_is_falsified_at_the_postcondition() {
    let harness = constructed_harness(EAGER, 999);
    let scratch = kani_crate_from("ir1447-eager", &harness.rust.contents, LAZY_SUBJECT);
    let run = run_installed(&scratch, &harness);
    let witness = assert_two_phase_falsified(&run, &harness, POSTCONDITION_CHECK);
    assert!(
        witness.iter().all(|amount| (0..=999).contains(amount)),
        "an in-domain input: {witness:?}"
    );
}

#[trace("FR-015-AC-172", "FR-017-AC-63", "TC-025")]
#[test]
#[ignore = "kani lane: run serially through `make kani`"]
fn tc_025_installed_clause_without_the_unread_initializer_verifies() {
    let harness = negotiate_twin(
        &Twin::with_bounded_amount_implication_condition(SKIPPED).with_amount_maximum(1000),
    )
    .expect("the clause lowers");
    let scratch = kani_crate_from("ir1447-skipped", &harness.rust.contents, UNREAD_SUBJECT);
    let run = run_installed(&scratch, &harness);
    assert_eq!(run.evidence.outcome, KaniRunOutcome::Verified);
}
