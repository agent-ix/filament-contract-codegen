---
id: SR-17940486709564847048012
title: "IR-805 PR #842 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #842 at a1a4c6b6, git diff origin/main...HEAD; FR-053-AC-3 (IR-260), FR-057-AC-7 (IR-854), FR-015-AC-151 (IR-1509), FR-037-AC-9 cohort corpus, IR-1547 scratch-crate helpers; tests/it/float_scalar_harness.rs, tests/it/integer_division_harness.rs, tests/it/kani_obligations_v2_clause_controls.rs, tests/exact_scalar_support/package.rs, tests/it/scratch_crate.rs, docs/qualification/ir1530-general-route-scalar-table.json"
review_set: subset
---

## Summary

Ticket: IR-805 (with IR-260, IR-854, IR-1547 and IR-1509). Plan completion: not assessed.

**Gate at the head**, with `--locked` under `flock /tmp/cargo-slot.lock` and FSPEC_DIR set as
the brief gives:

| Check | Result |
| --- | --- |
| whole `cargo test --lib` | 604 passed, 0 failed, 36 ignored |
| whole `cargo test --test it` | 1115 passed, 0 failed, 115 ignored |
| `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo deny check`, `make spec` | all exit 0 |
| structural tests | `layout::` 8, `cover_last::` 7, `role_parity` 5, all ok and none ignored |

The ignored counts are unchanged from main: there are 157 `#[ignore]` under tests/ on both sides,
and the diff touches none of them.

**IR-260 (FR-053-AC-3).** `an_unpinned_float_operand_requires_a_bound_naming_its_type` runs in
the whole `it` pass. Mutant (local only, reverted): I cfg'd out the Contract IR `RequiresBound`
read in `settle()`. The result is recorded in the Verdict.

**Cohort 6 to 4.** This is legitimate:
- `corpus_cohorts` only partitions the same golden items into packages under the unchanged 1 MiB
  ceiling.
- `bounded_corpus_cohorts_admit_every_requested_scalar_once` still asserts that the flattened
  cohorts equal `golden_items()`.
- No item, AC or ceiling is reduced, and no spec mentions cohort size.
- The float comparison operands gained `float_rounding` bounds, which matches the Contract IR
  `requires_bound` ruling that IR-260 consumes.

**IR-854 rows.** The six native rows match filament-exact TC-957 at the locked f1ef496 exactly:
- div 8 bits: 52.3 s and 1.71 GiB (euclid, the slowest profile).
- rem 8 bits: 88.2 s and 2.28 GiB.
- mod 8 bits: 64.7 s and 2.26 GiB.
- 16 bits: each row is the timed-out profile's peak.
- `kernel_version` 1 equals `filament_exact::KERNEL_VERSION`.

The "least tractable" disclosure in FR-057 is honest, and FR-057-AC-7 stays PARTIAL.

**IR-1547.**
- `per_label_package` renames the scratch package.
- `tests_ran` rejects `running 0 tests`.
- The guard-only helpers (`kani_obligations_state_frame`, `v2_clause_integer_literal*`) hold
  `lock_agreement_build()` across build and run.
- The two shared-target helpers that take no lock (`composite_equality_generation`,
  `exact_scalar_generation`) only `cargo build`, so there is no stale-executable run.
- `exact_scalar_agreement` and `kani_obligations_v2_clause_controls` already had their own
  ran-tests checks.

**IR-1509.** AC-151 stays PARTIAL. The new test reads the authored `deposit` body row and passes
in the whole run.

## Verdict

**PASS with low findings.** No test was skipped, ignored or deleted to get green. The four red
items on main are fixed by renames and data changes, not by loosening:
- the native meter crate name;
- the tc_029 / oracle_generation ids;
- cargo deny, which fails on main with 16 license and 10 source errors and passes at the head;
- the scalar corpus ceiling.

IR-260 mutant result: with `#[cfg(any())]` on the `if let Some(CompleteLoweringRecordV2::RequiresBound ..)` read in `settle()` (src/kani/generate/float_scalar.rs:345), the test FAILS (`panicked at tests/it/float_scalar_harness.rs:863: an unpinned operand requires a bound: Supported(...)`; rc 101). The unmutated head passes it in the whole `it` run. Because the stopgap is gone, an unpinned operand now reaches `Supported` with no refusal from CG, which shows the refusal really comes from the IR record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | IR-1509 only. `the_fsl_source_of_the_clause_fixture_carries_the_deposit_body_as_a_function_node` asserts only that the `deposit` row is `OperationBody::Function { node }` and that `node` is some node of the checked graph. It does not tie the node to the authored body (`amount < 1000 implies amount + 1 <= 1000`) by kind, operator or source span. Scenario: a producer that points the row at the wrong graph node (for example the clause node) still passes. Fix: also assert the node's operator/kind, or its source association with the `body DepositBody` span. | tests/it/kani_obligations_v2_clause_controls.rs:113-147 |
| FND-002 | low | IR-854 only. The six native rows were measured by filament-exact's own harnesses (TC-957), but they carry `generator_version` "0.1.0", which FR-056 defines as the CG crate version the probe ran against. The decider therefore treats them as current CG measurements. The profile-less 16-bit `not_tractable` rows would also refuse combinations TC-957 Verified: truncating div and rem at 16 bits (374.7 s and 437.8 s), and constant divisors 7, -7 and -1 at 64 bits. This is latent today, because the native route still refuses `NativeDivisionPlanUnavailable`, and it is disclosed in FR-057. Fix: when the native route opens, add a profile and divisor-shape feature or re-measure with CG harnesses, and record row provenance. | docs/qualification/ir1530-general-route-scalar-table.json:145-222; spec/kani/functional/FR-057-integer-division-scalar-harnesses.md |
| FND-003 | low | IR-1509 only. FR-015-AC-151, TC-025 and tests.md park the pipeline-compiled subject as "unowned, planner to assign" with no ticket. The remaining work (specify and build the CG step that renders the `operation_bodies` function node into `crate::subject`) has no tracker home, so it can silently stall. Fix: file the ticket and cite it in place of "unowned". | spec/kani/functional/FR-015-bounded-kani-obligations.md:997; spec/kani/matrix/TC-025-bounded-kani-obligations.md:1075-1077; spec/kani/matrix/tests.md:63 |

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-053-AC-3 | examined | unpinned comparison operand yields `requires-bound` naming the operand type node; Contract IR raises it, CG reports it |
| FR-057-AC-7 | examined | PARTIAL; native rows copied from TC-957; native route still refuses |
| FR-015-AC-151 | examined | PARTIAL; carrier read; pipeline subject PLANNED (unowned, planner to assign) |
| FR-037-AC-9 | examined | bounded corpus cohorts admit every requested scalar once |
| FR-056 measured table | context_only | rows carry generator and kernel version the probe ran against |
