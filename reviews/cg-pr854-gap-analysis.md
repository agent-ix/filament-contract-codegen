---
id: SR-17940486709564847048048
title: "gap-analysis of filament-contract-codegen PR #854 FSL unproved-nonzero fixture fix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; PR #854 at 7667c4b9; FR-048 (TC-062), FR-014 (TC-024), FR-056 (TC-070) criteria traced by the edited tests"
review_set: subset
---

## Summary

Gap analysis scoped to the criteria traced by the tests this PR edits. The PR has no ticket of its own. Its title refs QSL-2286, and findings are posted to IR-854 as the dispatch directs. Plan completion: not assessed. The PR adds no AC, test or trace tag. Production code is unchanged.

## Examined scope

- FR-014-AC-18 and FR-014-AC-21 (examined). All 5 `integer_division_bounds` tests pass again, and their assertions are unchanged. Zero-holding and unbounded divisors now come from the reader-admitted package rather than FSL. Mutants show that the derivation, generation and native-execution lines are still guarded, including the zero-magnitude window (`lower.rs:1207`), which only the reader path reaches. No gap.
- FR-056-AC-4 (examined). The operator-refused clause still holds the multiply that the row refuses, and it is still `V2ClauseOperatorNotSupported`. No gap.
- FR-048-AC-2 and FR-048-AC-8 (examined, `same_named...`). The method-identity binding is still guarded: the constant-digest mutant is killed. No gap.
- FR-048-AC-6 (examined). The PR changes how this criterion is covered; see FND-001.
- FR-048-AC-1 and FR-048-AC-11 (context_only). They are traced by `boolean_and_partial...`, and their parts that do not involve division are unchanged.

## Verdict

Coverage for FR-014 and FR-056 is intact. FR-048-AC-6 lost its only authentic owner-produced `Undefined`, and the `=`-over-`mod` workaround has no owner to revert it. Both have to be recorded or restored before the gaps are closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | After the PR, no test makes the owner evaluator produce `Undefined` for a constraint at a reached state (FR-048-AC-6). `boolean_and_partial...` dropped it, and the lib conversion test hand-builds the value. The classifier is still guarded, but the authentic path from a partial clause to `Undefined` to `EvaluationViolation` is unexercised. A zero `aux` cannot restore it, because a typed snapshot refuses it. No ticket records the loss: QSL-2361's ACs name only tc_025. | tests/it/constrained_method.rs:400-432; src/kani/terminal/constrained/tests.rs:179-183 |
| FND-002 | low | `same_named...` and `boolean_and_partial...` use `<=` instead of `=` to avoid QSL-2361 defect A (equality over `mod` fails emission; reproduced at head). QSL-2361's AC requires "no CG test edits" only for tc_025, so nothing owns restoring `=` here once A is fixed. | tests/it/constrained_method.rs:403,653-663; QSL-2361 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-2361 now records, in comment bf74557d, the restoration of an owner-produced `Undefined` test for FR-048-AC-6, or an explicit decision that the hand-built value is the accepted evidence. The comment names the IR code lead as owner. At cb7b27a3, the code points to QSL-2361 at `src/kani/terminal/constrained/tests.rs:183` and `tests/it/constrained_method.rs:420-423`. |
| FND-002 | deferred | Comment bf74557d on QSL-2361 records reverting the `<=` stand-in to an equality over `mod` in both fixtures. The comment names the IR code lead as owner. At cb7b27a3, pointers are at `tests/it/constrained_method.rs:420-423` and `:656`. |
