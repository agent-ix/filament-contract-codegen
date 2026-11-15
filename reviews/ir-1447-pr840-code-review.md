---
id: SR-17940486709564847048005
title: "IR-1447 PR #840 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #840 at 63e870c3, git diff origin/main...HEAD; IR-1447; src/core/ir/clause.rs, src/kani/generate/clause/v2.rs, src/kani/generate/outcome.rs, src/oracle/clause.rs, src/oracle/clause/native.rs, src/lib.rs, tests/it/kani_obligations_v2_control.rs, tests/it/kani_obligations_v2_clause_controls.rs, tests/it/main.rs, tests/state_frame_support/native_twin.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-1447. The PR adds `ClauseTerm::{Binder, Let, If, Case}` to the clause reader, a
scoped step plan with lazy `if`/`match` rendering in the native oracle, guarded arithmetic
probes and `match` selections in the independent `i128` expectation, a first-met control shape
check (`control_operands`) with the closed `V2ClauseControlReason`, and the
`V2ClauseControlNotSupported` refusal.

Checked by reading:

- Laziness. `render_step` emits `if value_c? { value_t } else { value_o }` and a `match` with one
  call per arm, so only the selected child runs; the `let` initializer is called once before the
  body and passed by reference. The expectation guards each branch probe by its selection
  (`probe.guard && selector` / `!selector`) and probes the initializer once, unguarded.
- Scoping. `let` lowers its value in the outer scope and its body in the inner one, in the reader,
  the plan and the expectation. Binder parameters are threaded through every child call.
- First-met walk. Control shape checks run on entering the node, before its operands are pushed,
  and operands are pushed in reverse so the walk stays left to right.
- Union `case` index: the native `match` puts the last member on `_`, which is sound only because
  the index is drawn in `0..member_count` (no end-to-end path yet; see gap-analysis).
- Trace attributes use `#[trace("FR-...", "TC-025")]` via `ix_trace_rs`; no `/// Trace:` lines,
  no pins, no compatibility layers, no tool versions in the run record.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | Four of the five refusal branches of `control_operands` (`IfBranchNotNativeScalar`, `CaseScrutineeNotParameter`, `CaseUnionPayloadNotSupported`, `CaseArmNotNativeScalar`) and the whole of `Reader::case_term` and `union_members` have no test at any level: deleting any of those branches, or swapping two of the case reasons, leaves every test green. A constructed-package or view-level unit test of `control_operands` per reason, and one `case_term` test (order by member name, duplicate arm, arity above 0, wrong side), would make the closed reason set load-bearing. | src/kani/generate/clause/v2.rs:945-997, src/core/ir/clause.rs:346-394, src/core/ir/clause.rs:452-476 |
| FND-002 | low | `control_expectation_tests::a_binder_read_resolves_to_the_innermost_binding` is traced to FR-015-AC-170 and FR-015-AC-173, but it builds `ClauseTerm::Binder { binder: 0/1 }` by hand, so it bypasses `Reader::binder`, the name-based resolution that decides shadowing. A reader that resolved a shadowed name to the parameter or to the outer binder would still pass it. Either retag it to what it covers (expectation substitution of binder numbers) or add a reader-level test of name resolution. | src/kani/generate/clause/v2.rs:3213-3256, src/core/ir/clause.rs:315-328 |

## Verdict

The lowering is correct as read: laziness, strict `let` initializers, scoping and the first-met
walk match FR-015-AC-164/166/168/172, and the kani-lane mutants fail for the stated reason.

Gates at 63e870c3, reviewer-run in its own worktree: `cargo fmt --check` ok; `cargo clippy
--locked --all-targets -- -D warnings` ok; `make spec` rc 0; whole `--lib` 589 passed, 1 failed
(known `native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`);
whole `--test it` 1097 passed, 2 failed (known `oracle_generation::tc_001_...` and
`composite_equality_generation::tc_029_authoritative_fspec_recursive_items_generate_through_reader`,
`UnknownContractVersion` on the `filament.checked-package/v2` FSpec fixture, outside this diff and
not re-run on main). All `layout::` and `cover_last::` tests pass, as do the six new default-lane
control tests. The Kani lane was not re-run by the reviewer. FND-001 should be
fixed in this PR together with the gap-analysis status findings; FND-002 can ride the same round.

## Dispositions

Round 1, checked against the fix commits 5ff50540 and 1733eb3e (head 1733eb3e).

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | `control_shape_tests` (src/kani/generate/clause/v2.rs) tests `control_operands` for `IfBranchNotNativeScalar`, `CaseUnionPayloadNotSupported`, `CaseArmNotNativeScalar`, `CaseScrutineeNotParameter` and the accepted case/if shapes over an FSL-compiled package (`src/core/ir/clause_control_fixture.rs`); `src/core/ir/clause_tests.rs` tests `union_members`, `case_term` (declaration order from source order C, A, B; payload, duplicate, missing arm, Pre side, foreign scrutinee) and `Reader::binder`. Each reason now has a test that fails if its branch is deleted or swapped. |
| FND-002 | fixed | `a_binder_read_resolves_to_the_innermost_binding` retagged to FR-015-AC-152 only; the new `a_parameter_read_resolves_to_the_innermost_binder_of_its_declared_name` exercises `Reader::binder` (none, innermost of two, other name does not hide) and carries the AC-170/AC-173 tags. |

Round 1 gates at 1733eb3e, run by the reviewer in its own worktree: `cargo fmt --check` ok; clippy `--locked --all-targets -- -D warnings` ok; `make spec` rc 0; whole `--lib` 598 passed, 1 failed (known `native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`), and `role_parity_tests::clause_planning_and_reading_share_one_graph_walk` plus the new `control_shape_tests` and `clause::tests` pass; whole `--test it` 1098 passed, 2 failed (known `oracle_generation::tc_001_...` and `tc_029_authoritative_fspec_recursive_items_generate_through_reader`); all 15 `layout::` and `cover_last::` tests pass. The reviewer did not re-run the Kani lane; the two new installed runs are as recorded in `docs/qualification/ir1447-kani-control-clause-runs.md`.
