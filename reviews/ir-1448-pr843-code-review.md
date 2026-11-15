---
id: SR-17940486709564847048014
title: "IR-1448 PR #843 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #843 at ce2221e9, git diff origin/main...HEAD (15 files); src/core/ir/clause.rs, src/kani/generate/clause/v2.rs, src/kani/generate/outcome.rs, src/oracle/clause.rs, src/oracle/clause/native.rs, src/lib.rs, tests/it/kani_obligations_v2_option.rs, tests/it/kani_obligations_v2_clause_controls.rs, tests/state_frame_support/native_twin.rs, docs/qualification/ir1448-kani-option-clause-runs.md"
review_set: subset
---

## Summary

Ticket: IR-1448. This is code-review with the rust-review lane folded in.

What the change does:
- It adds `ClauseLeaf` (whole, presence, payload path) to `ClauseInput`.
- The clause reader turns `option.present(x)` into the presence input and `option.value(x)` into
  `ClauseTerm::OptionValue`.
- The generated oracle renders an absent read as `return Err(ClauseStop::Undefined)`, or as the
  runtime `NoneValue` stop.
- The result-postcondition harness draws a presence `bool` and native payload leaves. It assumes
  canonical payloads when absent and in-bound payloads when present.
- The independent native expectation adds a `Defined` pseudo-probe, so later arithmetic probes are
  checked only where the read is defined.
- `UnsupportedObligation::OptionNotLowered` with four reasons is added beside the existing arms.
  It is not a parallel family.

Rust-lane checks that passed:
- No `unwrap`, `expect` or `panic!` was added on a generation path.
- No `unsafe`. Integer narrowing goes through `i64::try_from` with typed refusals.
- No pins, `[patch]`, compat re-exports or environment workarounds. Cargo.toml and Cargo.lock are
  untouched.
- Each scratch crate label is unique per test (`ir1448-*`), and the package name comes from the
  label.
- Host runs fail on `running 0 tests`. The enumeration asserts it visited at least one draw, and
  the stop probe asserts its `RESULT` line exists.
- The Kani-lane mutants go through `assert_two_phase_structure(.., RESULT_CHECK)`, which asserts
  the witness-phase counterexample names the produced-result check. The playback vectors are
  asserted exactly, in draw order (presence, then payload).
- Every new test carries `#[trace(..)]`.
- `clause.rs` adds no `.resolved_graph()` call (role parity).
- The evidence doc names no tool version.

The equivalent-package mutant of AC-3 (`not present(amount)` against subject `present`) is not
tautological. The healthy oracle and the mutant oracle differ at every draw.

## Verdict

**FAIL. Medium findings in the code and tests; see also the gap-analysis SR.**

Gate at ce2221e9:
- `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo deny check` and
  `make spec`: all exit 0.
- whole `--lib`: 604 passed, 0 failed, 36 ignored.
- whole `--test it`: 1127 passed, 0 failed, 120 ignored.
- `layout::l_2_*`, `cover_last::tc_025_*` and the role-parity scans pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `NullablePresence` misclassifies some reads. `option_refusal` returns it for ANY `option.*` application whose operand is a `filament.op.record.project`, without reading the projected field's `presence` / `nullable` flags. A required (`presence: "required"`) record field typed `Option<T>` is refused as `NullablePresence`, but FR-054 reserves that reason for a `field?` slot of optional presence. No test reaches this branch. | src/kani/generate/clause/v2.rs:1320-1334 |
| FND-002 | medium | `OptionNotLowered.option_node_id` is documented as "The option type node, or the clause node when the read has none" (and FR-054 Outputs say "retaining the node"). It is filled from the walk's `node`, which is the node of the `option.*` application, not the option type node. No test reads the field: every match uses `..`. | src/kani/generate/outcome.rs:364-375; src/kani/generate/clause/v2.rs:324-335,1244-1255 |
| FND-003 | medium | Three of the four new `OptionNotLoweredReason` arms have no test: `NestedOption`, `NullablePresence` and `OptionOperationNotLowered`. Only `PayloadNotScalar` is reached (string payload and record payload). IR-1448's AC "Remaining shapes refuse with a specific typed code" rests on one reason. `option_refusal` and `option_shape` are pure functions over a `ClauseGraphView`, so a constructed-package unit test can reach each branch without FSL. | src/kani/generate/clause/v2.rs:1304-1342; src/core/ir/clause.rs:117-202 |
| FND-004 | medium | Option reads outside the one lowered shape get no FR-054 reason. Such reads include a precondition, an invariant, a state postcondition, a non-Boolean-result postcondition, an option state field, or an option over an unbounded integer payload (measured: `Option<native Integer>` is admitted and refused this way). They fall to the generic `V2ClauseConditionNotRendered`: `option_operand` returns `None` for non-parameter or non-`Leaves` operands, and the post-read check at v2.rs:351-363 returns `refusal()`. FR-054 does not limit lowering to Boolean-result postclauses over parameters, and no test covers these paths, so the ticket's "specific typed code" is not met for them. | src/kani/generate/clause/v2.rs:351-363; src/core/ir/clause.rs:497-505 |
| FND-005 | medium | The AC-10 test cannot fail on a wrong family. `tc_068_a_non_scalar_payload_is_refused_naming_its_family` asserts only `!detail.is_empty()` and prints the family. The run doc claims family `text_bounds`, but no assertion pins it, and the node is not checked either. FR-054-AC-10 requires "naming the family and node". | tests/it/kani_obligations_v2_option.rs:427-439 |
| FND-006 | low | The AC-15 test checks `sibling.is_some()` only. FR-054-AC-15 says "with its harness unchanged". Compare the sibling's harness bytes with the harness the same package yields alone (`negotiate_twin(&option_twin(GUARDED))`). | tests/it/kani_obligations_v2_option.rs:472-494 |
| FND-007 | low | Dead code in the test module keeps unused imports alive: `let _ = (ObligationDisposition::Unsupported { .. },);` in the AC-15 test, and `#[allow(dead_code)] fn _types(..)`. Drop the unused imports instead. | tests/it/kani_obligations_v2_option.rs:490-492,599-605 |
| FND-008 | low | The `option_refusal` doc says "an unbounded integer payload keeps its bound refusal". No bound refusal exists on this path. `OptionShape::Unbounded` makes `option_operand` return `None`, which yields `V2ClauseConditionNotRendered`. The comment is stale against the rewritten FR-054-AC-16. This path is reachable: `Option<native Integer>` reaches it (spec-review SR FND-001). | src/kani/generate/clause/v2.rs:1305-1307; src/core/ir/clause.rs:112-114,138 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The new evidence claims a non-Boolean `result = value(amount)` is refused by the FSL checker (`ill_typed`). The fixture is `option_twin`, i.e. `with_bounded_amount_implication_condition`, which declares the `deposit` result `ix://filament/native/Boolean`. The refusal is therefore a plain type mismatch of an integer against a Boolean result. It is not evidence about an option read in a postcondition with a non-Boolean (integer) result. Either build an integer-result fixture and record what it yields, or reword the run doc and test comment to say only that `result = value(amount)` is ill-typed against a Boolean result. | docs/qualification/ir1448-kani-option-clause-runs.md:62-65; tests/it/kani_obligations_v2_option.rs:709-736 |

## Dispositions

Disposition pass round 1: fix commit 651e88f5 (was ce2221e9). Gate at 651e88f5: fmt, clippy `-D warnings`, `cargo deny check` and `make spec` exit 0; whole `--lib` 604 passed, 0 failed, 36 ignored; whole `--test it` 1131 passed, 0 failed, 120 ignored (the option module alone: 16 passed, 5 ignored). The committed `reviews/` copies of the three SR files cmp-match the review-pass files.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | Measured at 651e88f5 with a scratch probe (a copy of the head; deleted): the admitted checked packages for `f?: Int[0, 1000]` and for a required `f: Option<Int[0, 1000]>` are byte-identical over every node's tag, form, semantic type and term (`Debug`). `f?` over an option type is refused at intake. The checked package carries no presence flag, so `option_refusal` cannot tell the two apart; the defect as stated does not exist. FR-054 (Description, Refusal), the `NullablePresence` doc and AC-12 PARTIAL now say both are refused `NullablePresence`, and `tc_068_a_record_field_read_as_an_option_is_refused_nullable_presence` covers both. |
| FND-002 | fixed | The `option_node_id` doc now reads "The checked node of the option operation that was refused, or the clause node when the walk reached the operation without a node", and `tc_068_the_refusal_names_the_option_operation_node` asserts `node_id == clause` and `option_node_id` is the `presence_read` node. |
| FND-003 | deferred | `NullablePresence` is now tested (both field shapes). `NestedOption` is unreachable from an admitted package: intake refuses `Option<Option<T>>` (`tc_068_fsl_has_no_spelling...`) and the checked record carries no fields. `OptionOperationNotLowered` is unreached: FSL elides `Some` (`tc_068_a_some_constructor_leaves_no_option_operation_node...`). AC-11 and AC-13 are PARTIAL in FR-054 Status, "unowned, planner to assign"; a constructed-view unit test of the two dead arms is that remainder and is out of scope for this PR's admitted-input controls. |
| FND-004 | fixed | FR-054 Refusal now states that an option read outside a Boolean-result postcondition over an operation parameter is refused by the existing `V2ClauseConditionNotRendered` and that FR-054 adds no reason for it. The native-Integer option (AC-16 test) and the `Some` clause (AC-13 test) pin that refusal. The precondition, invariant and state-field paths are documented but not pinned by a test (residual, low). |
| FND-005 | fixed | The AC-10 test now asserts `detail == "text_bounds"`; the option node is asserted by the new `tc_068_the_refusal_names_the_option_operation_node`. |
| FND-006 | fixed | The AC-15 test now asserts `sibling.rust.contents == harness_of(&option_twin(GUARDED)).rust.contents`. |
| FND-007 | fixed | The `let _ = (ObligationDisposition::Unsupported .., );` and `fn _types` are removed, with their imports. |
| FND-008 | fixed | The `option_refusal` doc now says the unbounded integer payload and option reads outside a Boolean-result postcondition over a parameter are refused later by the generic `V2ClauseConditionNotRendered`. |

Disposition pass round 2: fix commit 50713836 (was 651e88f5). Gate at 50713836: fmt, clippy `-D warnings`, `cargo deny check` and `make spec` exit 0; whole `--lib` 606 passed, 0 failed, 36 ignored (both `option_refusal_tests` ran); whole `--test it` 1133 passed, 0 failed, 120 ignored (option module 18 passed, 5 ignored). The committed `reviews/` copies cmp-match the round-1 files.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | - |
| FND-009 | still-open | New this round: the non-Boolean-result claim rests on a Boolean-result fixture. |

`option_refusal` is now split into `operation_refusal(identity)` and `shape_refusal(shape)`. The split is a pure refactor: the same predicate (`!matches!(identity, OPTION_PRESENT | OPTION_VALUE)`) and the same match arms, in the same order. `option_refusal_tests` (labelled constructed input) asserts `NestedOption` with detail `option` and `OptionOperationNotLowered` with detail `filament.op.option.some`, plus `None` for present, value, `Leaves` and `Unbounded`. AC-11 and AC-13 stay PARTIAL in FR-054 Status, so the tags claim nothing beyond that.
