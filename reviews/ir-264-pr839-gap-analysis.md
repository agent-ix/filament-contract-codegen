---
id: SR-17940486709564847048003
title: "IR-264 PR #839 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #839 at f0d42093, git diff origin/main...HEAD; IR-264 PR1; FR-018-AC-21..23, TC-029 steps 13-15, spec/oracle/matrix/tests.md; src/oracle/equality/closure.rs, tests/it/composite_equality_closure.rs, tests/composite_equality_support/closure_cases.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-029
    type: reviews
---

## Summary

Ticket: IR-264. Planless gap analysis of FR-018-AC-21, AC-22 and AC-23 against the tagged tests
and the code. `quoin matrix` at f0d42093: AC-21, AC-22 and AC-23 are each `tagged` (static) with
`no run evidence`. Binders: `tc_029_ac21_ac22_native_closure_and_totality_over_the_refinement_domain`
and `tc_029_ac22_native_run_refuses_an_oracle_of_the_other_operator` (tests/it, which stage the
native `closure_cases.rs` tests `tc_029_ac21_closure_equals_the_constructed_environment_declarations`
and `tc_029_ac22_oracle_completes_every_domain_pair_with_the_fr149_verdict_and_schedule`), and
`tc_029_ac23_closure_equals_an_independent_read_of_the_checked_package`.

Per-criterion status as evidenced:

- AC-21: covered natively over the bounded corpus (record with optional field under both
  operators, tuple, option, nested option), operand types included, four closure mutants.
  The AC was explicitly rescoped to "the TC-029 bounded corpus". Honest. The union path has a
  recording defect (code-review FND-001) outside the corpus.
- AC-22: whole domain of every bounded-corpus item (30, 6, 4, 4 values; 1,868 pairs), verdict and
  node count test-computed, unlimited meter; boundary and seeded selection above the case cap
  left to FR-028-AC-16 (IR-241). Marked PARTIAL in FR-018, TC-029 step 14 and tests.md. Honest.
- AC-23: covered over the bounded corpus only (FND-001), with claim mutants; the option-payload
  clause has no real control (code-review FND-002).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-018-AC-23 still quantifies over "every item of the TC-029 corpus whose leaves are Boolean or bounded integers" (AC-21 was narrowed to "the TC-029 bounded corpus" in this PR, AC-23 was not), and tests.md marks it Covered, but the AC-23 test checks only the five `bounded` items. The retained TC-029 `option` case (`Option<Int[-100, 100]>` on both sides) has only bounded-integer leaves and is never read against the package. Either add the `option` case (and any other qualifying retained case) to the AC-23 check, or narrow AC-23 and TC-029 step 15 to the bounded corpus as AC-21 was. | spec/oracle/functional/FR-018-composite-equality-oracles.md:729, spec/oracle/matrix/tests.md:46, tests/composite_equality_support/authentic_native.rs:36, tests/it/composite_equality_closure.rs:363-382 |

## Verdict

AC-21 and AC-22 statuses are honest (AC-22 PARTIAL with a named owner). AC-23's Covered status overclaims by scope (FND-001), and its option-payload clause and the reconstruction-bound clause lack working controls (code-review FND-002, FND-004). Fix FND-001 in this PR, by widening the test or narrowing the AC.

## Dispositions
