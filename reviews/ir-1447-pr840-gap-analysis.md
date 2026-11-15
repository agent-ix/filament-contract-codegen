---
id: SR-17940486709564847048006
title: "IR-1447 PR #840 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #840 at 63e870c3, git diff origin/main...HEAD; IR-1447; FR-015-AC-152..173, FR-025-AC-19, TC-025 step 57, TC-036 step 19, spec/kani/matrix/tests.md, docs/qualification/ir1447-kani-control-clause-runs.md; tests/it/kani_obligations_v2_control.rs, src/kani/generate/clause/v2.rs, src/core/ir/clause.rs, src/oracle/clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-1447. Planless gap analysis of the status the PR gives each control-lowering
criterion against the tagged tests, `quoin matrix --json` at 63e870c3 and the run record
`docs/qualification/ir1447-kani-control-clause-runs.md`. Plan completion: not assessed.

The executed Kani rows are real: each `#[ignore]` kani-lane test in
`tests/it/kani_obligations_v2_control.rs` negotiates the clause through
`negotiate_kani_native_obligations`, appends a test-written `crate::subject` and runs it through
CG's `execute_kani_obligation` (`run_installed`), and the falsified rows assert the two-phase
shape, the failing check message and the one in-domain playback value
(`assert_two_phase_falsified`). The let/if/nested subject mutants fail at the produced-result
comparison for the stated reason (999, 500, 500). The AC-169 and AC-172 healthy fixtures fail at
the postcondition step at 1000 because the bounded addition node refuses, which is the claimed
reason.

Per-criterion verdict:

- AC-152, AC-153, AC-156, AC-157, AC-165: Implemented is supported (structural default-lane
  tests plus executed healthy Verified and mutant Falsified runs).
- AC-168, AC-172: the healthy half is executed under Kani; the mutant half of each AC states a
  Kani verdict (`Falsified ... amount = 0`, `Verified`) that was never run. Only a host test of
  the generated oracle's return value was run. Implemented overclaims (FND-001, FND-002).
- AC-169: healthy executed under Kani and its stop recorded by the host probe; the mutant's
  "also Falsified at the same step" was not run (FND-003).
- AC-160, AC-166: Implemented, but sub-clauses of each have no test (FND-004, FND-005).
- PARTIAL groups (AC-154/158/162/163/171 + FR-025-AC-19; AC-155/159; AC-170/173; AC-161/164/167):
  each is PARTIAL with a reason, and FR-015, FR-025, TC-025 step 57, TC-036 step 19 and
  tests.md agree. The wording of AC-154/162/163/171 overclaims existing unit tests (FND-006) and
  AC-154/155 name an owner (FND-007).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | FR-015-AC-168 is marked Implemented, but its mutant clause ("an oracle that evaluates every branch eagerly ... the harness is `Falsified` at the step `the V2 postcondition holds for the produced result` with any in-domain input, for example `amount = 0`") has no executed Kani verdict. The run record says the runner refuses a mutated generated source, so the eager mutant was run only as a host test (`tc_025_the_oracle_evaluates_only_the_selected_branch`) that records `Err(IntegerOutOfDomain)` from `oracle_clause` at amount 0. That shows the oracle stops; it does not show the harness is Falsified at that step. Either mark AC-168 PARTIAL (mutant Kani verdict not run, with the runner reason), or have the AC amended so the mutant control is the host stop probe, and keep the status in FR-015, TC-025 step 57 and tests.md in step. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1014, tests/it/kani_obligations_v2_control.rs:445-458, docs/qualification/ir1447-kani-control-clause-runs.md:33-41 |
| FND-002 | high | FR-015-AC-172 is marked Implemented, but its only mutant claim ("an oracle that skips an unread initializer is `Verified` and fails this control") was never run under Kani; the host test `tc_025_an_unread_let_initializer_is_still_evaluated_once` records `Ok(true)` from the skip-initializer oracle at 1000. The healthy Falsified run is executed. Same remedy as FND-001: PARTIAL, or an AC amendment that makes the host stop probe the control. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1018, tests/it/kani_obligations_v2_control.rs:477-490 |
| FND-003 | medium | FR-015-AC-169 Implemented: the healthy run is Falsified at the postcondition step at 1000 (executed) and the stop `IntegerOutOfDomain` is recorded by the host probe, but the AC also says the `false`-mapping mutant "is also `Falsified` at the same step with the same input", which was not run. The distinguishing part (the recorded stop) is executed, so this is weaker than FND-001/002, but the status still claims an unexecuted verdict. Say in the AC status that the mutant's Kani verdict is not run (only its stop), or mark PARTIAL. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1015, tests/it/kani_obligations_v2_control.rs:460-475 |
| FND-004 | medium | FR-015-AC-160 is marked Implemented, but its one test (`tc_025_a_let_binding_a_text_is_a_typed_control_refusal_not_an_operator_refusal`) covers one family (a string literal), checks only the reason and operation identity (not the node), and nothing tests "a supported sibling item of the same request is unaffected" (`negotiate_package` sends one item). The status also cites "executed CG-generated runs in `ir1447-kani-control-clause-runs.md`", which has no AC-160 row. Add a two-item request (refused `let` plus a supported sibling) and the node assertion, or mark the sibling clause PARTIAL, and point the status at the test. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1006, tests/it/kani_obligations_v2_control.rs:240-246 |
| FND-005 | medium | FR-015-AC-166 is marked Implemented, but `tc_025_the_first_refusal_the_walk_meets_decides` covers only the operator-first, control-first and two-controls cases. The AC's option clauses ("a `let` whose bound value is an option-typed parameter is refused as `LetValueNotNativeScalar` (not `OptionNotLowered`)" and "a `let` whose bound value is ... `quire.op.option.value` passes the control check and is then decided by FR-054") have no test. The status also cites the run record, which has no AC-166 row. Add the two option cases (constructed package if FSL cannot express them) or mark AC-166 PARTIAL for those clauses. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1012, tests/it/kani_obligations_v2_control.rs:248-264 |
| FND-006 | medium | The PARTIAL notes overclaim existing tests: AC-162, AC-163 and AC-171 say "unit-level only", and AC-154 says union `case` "lowers in the reader, oracle and expectation (unit tests)". `quoin matrix` shows AC-161, AC-162, AC-163 and AC-171 untagged, and no test anywhere exercises `CaseUnionPayloadNotSupported`, `CaseArmNotNativeScalar`, `CaseScrutineeNotParameter` or `IfBranchNotNativeScalar` (v2.rs `control_operands`), or `Reader::case_term` (clause.rs). Only the oracle render and the expectation of a hand-built `ClauseTerm::Case` are unit tested. Reword to "no test" (or add the unit tests the notes describe). | spec/kani/functional/FR-015-bounded-kani-obligations.md:1000, spec/kani/functional/FR-015-bounded-kani-obligations.md:1008-1009, spec/kani/functional/FR-015-bounded-kani-obligations.md:1017, src/kani/generate/clause/v2.rs:945-997, src/core/ir/clause.rs:346-394 |
| FND-007 | low | AC-154 and AC-155 PARTIAL notes name an owner ("owner IR-1447 follow-up on the union parameter ABI", "owner IR-1447 follow-up (collection predicate arm)"). No such ticket exists and the planner assigns owners; the note also reads as if IR-1447 stays open for it. State only what is missing. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1000-1001 |

## Verdict

The let, if and nested lowering and its Kani controls are real and executed through CG's own
runner; the mutants fail for the stated reason and are not tautological. AC-168 and AC-172 must
not be Implemented while their mutant Kani verdicts are unexecuted (FND-001, FND-002): mark them
PARTIAL or get the AC text amended to accept a host stop probe as the oracle-side control. AC-169
needs its status wording narrowed (FND-003). AC-160 and AC-166 have untested clauses (FND-004,
FND-005). The PARTIAL groups are honest in substance and consistent across FR-015, FR-025,
TC-025, TC-036 and tests.md, apart from the "unit-level only" wording (FND-006) and owner naming
(FND-007). Not mergeable until FND-001 to FND-006 are fixed in this PR.

## Dispositions

Round 1, checked against the fix commits 5ff50540 and 1733eb3e (head 1733eb3e).

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | AC-168 now says the executed Kani verdict is for the equivalent clause `let e = amount + 1000 in (if amount < 1000 then amount else e) >= 0` (strict `let` evaluates the never-selected branch value), with the hand-edited oracle as a host stop probe only. `tc_025_installed_eager_branch_equivalent_is_falsified_at_the_postcondition` runs it through `run_installed`, asserting Falsified at the postcondition step with an in-domain witness (record: 109.3 s, amount = 31). Accepted as satisfying the control: in the healthy harness the else-branch probe is guarded by its selection and never fires, so the eager-oracle mutant's only failing check is the postcondition on `Err(IntegerOutOfDomain)`, the same check and outcome the equivalent package produces on every draw. |
| FND-002 | fixed | AC-172 now names the equivalent clause `result = (amount >= 0)` as the executed mutant verdict (Verified, 6.1 s) with the hand-edited oracle as host stop probe only; `tc_025_installed_clause_without_the_unread_initializer_verifies`. Accepted: in the healthy harness with a skip-initializer oracle, the arithmetic probe of `amount + 1` checks the kernel against the i128 domain independently of the oracle (Refused and out of domain agree at 1000), the result comparison is gated off by `completed_prefix`, and the postcondition sees `Ok(true)`, so that mutant harness is Verified exactly as the equivalent package is. |
| FND-003 | fixed | AC-169 now reads "Only the stop was run, as a host probe of the hand-edited generated oracle (`Ok(false)` at `amount = 1000`); the mutant's Kani verdict is not run"; the "also Falsified" claim is gone. Stays Implemented: the healthy verdict and both stops are executed and the AC says the stop is the only discriminator. |
| FND-004 | fixed | `tc_025_a_let_binding_a_non_native_value_is_a_typed_control_refusal_and_spares_a_sibling` covers text and option families, asserts reason, identity, `node_id == clause` and that `control_node_id` holds a `quire.op.control.let` application, and sends a two-item request whose supported sibling keeps its harness. Status cites the test file, not the run record. |
| FND-005 | fixed | AC-166 is now PARTIAL: operator-first, control-first, two-control and option-parameter `let` (`tc_025_a_let_of_an_option_parameter_is_a_control_refusal_not_an_option_refusal`) are tested; the `option.value` clause is named as untested because the FSL checker refuses `value(...)` over the nullable fixture parameter. TC-025 step 57 says the same. |
| FND-006 | fixed | New tests: `control_shape_tests` in v2.rs (one per reason, all four refusal reasons plus the accepted case/if shapes) over an FSL-compiled function package, and `clause_tests.rs` (`union_members`, `case_term` order/payload/duplicate/missing/side/foreign, `Reader::binder`). The PARTIAL notes now describe exactly those tests and say no end-to-end clause fixture exists. |
| FND-007 | fixed | AC-154 and AC-155 now read "unowned, planner to assign". |
