---
id: SR-17940486709564847048030
title: "IR-806 PR-1 PR #848 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #848 at 653b0170, git diff origin/main...HEAD (base 149c98d2); IR-806 PR-1; FR-029-AC-17, AC-21, AC-22, AC-25, AC-30, AC-31; src/kani/terminal.rs, src/kani/classify.rs, tests/it/composite_parity_converter.rs, tests/it/shadow_refinement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-1). Planless gap analysis of the criteria this PR claims (FR-029-AC-17, AC-21,
AC-22, AC-31) against code and tagged tests, plus the two it leaves PLANNED (AC-25, AC-30).
Plan completion: not assessed.

Bindings checked: `tc_048_every_strength_projects_onto_its_refinement_state` and
`tc_048_a_non_verified_shadow_outcome_acquires_no_strength` -> FR-029-AC-17;
`tc_048_the_enumerated_strengths_equal_the_closed_set_of_the_spec` -> FR-029-AC-22;
`tc_048_each_strength_settles_to_the_terminal_value_fsl_reports` -> FR-029-AC-17/22/31;
`tc_040_the_ordinary_map_has_no_shadow_input_or_variant` -> FR-029-AC-21. All are correct
bindings. Each claim was mutated in the review worktree (see the code-review SR for the runs):
changing a projection row, a settled expected value or the spec's strength list each fails a test;
a new `TerminalPairError` variant or strength fails to compile.

AC-25 and AC-30 stay PLANNED with no test, which is accurate: no code path exists for either.

## Verdict

**Changes requested (no high findings).** AC-17, AC-21 and AC-22 are backed by tests that can
fail. AC-31 is claimed fully covered on verified-entry evidence only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-029-AC-31 is marked done (no status tag; tests.md "Covered") but its criterion covers the parity route for any bounded-shadow outcome "under AC-17 and AC-19 to AC-27", and AC-19 to AC-27 are PLANNED. The test settles only `VerifiedShadow` entries; the added sentence "for the verified entry" narrows the evidence, not the criterion. Mark AC-31 PARTIAL (verified entry) or narrow its statement. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:408, spec/kani/matrix/tests.md:101, tests/it/composite_parity_converter.rs:1109 |
| FND-002 | low | FR-029-AC-17 states that `proof_strength` gives a falsified outcome no strength unless the refinement disagreed, but the FR-029-tagged test covers only the inconclusive reasons and cover-unsatisfied. Falsified is checked in `shadow_refinement.rs` under FR-028-AC-17, and only with `None`, `Disagreed` and `ReachedCeiling`, not `Agreed(..)`. Add `Falsified` to the outcome list in `tc_048_a_non_verified_shadow_outcome_acquires_no_strength`. | tests/it/composite_parity_converter.rs:1019-1058, tests/it/shadow_refinement.rs:500-512 |

## Coverage

Examined FR-029-AC-17, AC-21, AC-22, AC-25, AC-30 and AC-31 and their tests. Gate run at 653b0170:
whole `--lib` 617 passed / 36 ignored, whole `--test it` 1155 passed / 127 ignored, no failures.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at 621d0668 (7ffa085c, 621d0668): FR-029-AC-31 now reads "PARTIAL (verified entry; the falsified-entry rows await AC-19 to AC-27)"; tests.md has its own 🚧 PARTIAL row for AC-31; TC-048 and interface-001 say "AC-31 (verified entry)". |
| FND-002 | fixed | Round 1 at 7ffa085c: `tc_048_a_non_verified_shadow_outcome_acquires_no_strength` adds `KaniRunOutcome::Falsified`, which runs against every `Agreed(..)`, `ReachedCeiling` and `None` summary. Mutation (letting `proof_strength` accept Falsified) fails it at composite_parity_converter.rs:1047. |
