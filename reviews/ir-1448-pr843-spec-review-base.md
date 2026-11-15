---
id: SR-17940486709564847048016
title: "IR-1448 PR #843 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #843 at ce2221e9, git diff origin/main...HEAD -- spec/ docs/; FR-054 (Refusal, AC table, Status), TC-068, spec/kani/matrix/tests.md, spec/spec.md, docs/qualification/ir1448-kani-option-clause-runs.md; FR-015-AC-42 (context)"
review_set: subset
---

## Summary

Ticket: IR-1448.

The spec diff does three things:
- It flips FR-054, TC-068, tests.md and spec.md from PLANNED to PARTIAL.
- It lists Implemented, PARTIAL and not-run criteria.
- It rewrites FR-054-AC-16 and its Refusal paragraph.

`make spec` exits 0, and no grammar warning is on an FR-054 or TC-068 line. The new run doc names
no tool version. The `Kani 0.68.0` in FR-054's probe paragraph (line 54) predates this PR and is
not in the diff.

FR-015-AC-42 already carries the IR-1448 amendment on main. It agrees with FR-054-AC-23 and with
the code: `option.present` and `option.value` are added to `supported_clause_operator`, and every
other `filament.op.option.*` goes to `OptionOperationNotLowered` ahead of the generic operator
check.

The PARTIAL and not-run wording ("unowned, planner to assign") is consistent across FR-054
Status, TC-068 Status, tests.md and the run doc. The Implemented overclaims are recorded in the
gap-analysis SR.

## Verification of the FR-054-AC-16 rewrite

The old criterion: "An option of an unbounded integer keeps `requires_bound` with the type node
and has no harness. Mutant: remapping it to `PayloadNotScalar` fails."

The new criterion: "refused at package admission with `OperatorIneligible`, so no package reaches
the lowering".

The test fixture is `with_scalar_option_amount("integer")`, a DECLARED value type with
`scalar: integer` and no constraints. For that spelling, Contract IR's checked-package admission
does refuse (`IllTyped`/`OperatorIneligible` at `/semantic_graph/nodes/11/body/arguments/0`).

I ran a scratch probe on a copy of the PR head. The worktree was untouched and the copy has since
been deleted. The probe typed `amount` by an option over the native unbounded Integer with
`Twin::with_option_amount_over("ix://filament/native/Integer")`.

Result for `result = present(amount)` and for the guarded `value` claim: FSL emits the package,
Contract IR ADMITS it, and CG's lowering refuses it as
`V2ClauseConditionNotRendered { node_id }`. That is neither `requires_bound` (the old AC-16) nor
the admission refusal the new AC-16 claims.

So the rewrite is not a correction of a wrong criterion. It generalizes one fixture-specific
refusal into a rule that does not hold. The old obligation (`requires_bound` for an unbounded
option payload, FR-015-AC-61) is still owed, and the code does not meet it.

## Verdict

**FAIL. One high finding (AC-16) and two low ones.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-054-AC-16 rewrite moves the goalposts, and its new claim is false. The FR-054 Refusal paragraph and AC-16 now say Contract IR package admission refuses "an option of an unbounded integer" (`OperatorIneligible`), "so such a clause never reaches this lowering". Measured at ce2221e9: `Option<ix://filament/native/Integer>` is admitted, reaches the lowering, and is refused as the generic `V2ClauseConditionNotRendered`, with no `requires_bound` and no FR-054 reason. Only the declared unconstrained integer value type the test uses is refused at admission. The AC was moved to Implemented on that one fixture, and its mutant leg was dropped. TC-068 step 9 and tests.md repeat the claim. Fix: restore the `requires_bound` criterion (FR-015-AC-61) and implement it for the native-Integer option. Or, if another typed reason is chosen, state it, test it on the native-Integer fixture, and keep AC-16 PARTIAL until then. | spec/kani/functional/FR-054-option-clause-harnesses.md:126-127,187; spec/kani/matrix/TC-068-option-clause-harnesses.md:52-53; spec/kani/matrix/tests.md:16; docs/qualification/ir1448-kani-option-clause-runs.md:50-51; tests/it/kani_obligations_v2_option.rs:454-469 |
| FND-002 | low | Stale PLANNED prose survives the status flip. FR-054's probe paragraph still says "no criterion below is implemented" (line 57), and its Dependencies line says TC-068 "defines the planned verification" (line 203). Both contradict the new Status. | spec/kani/functional/FR-054-option-clause-harnesses.md:57,203 |
| FND-003 | low | The tests.md FR-054 row reads `PARTIAL (IR-1448)` with no status emoji. Every other non-implemented row in the table uses `🚧`, for example `🚧 PARTIAL (IR-854 ...)`. | spec/kani/matrix/tests.md:16 |

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-054 Refusal (unbounded) | examined | Contract IR package admission refuses an option of an unbounded integer (OperatorIneligible), so such a clause never reaches this lowering |
| FR-054-AC-16 | examined | An option of an unbounded integer is refused at package admission with OperatorIneligible, so no package reaches the lowering and no harness exists. |
| FR-054 Status | examined | PARTIAL (IR-1448; executed runs in docs/qualification/ir1448-kani-option-clause-runs.md) |
| FR-054-AC-23 / FR-015-AC-42 | examined | Amended by IR-1448: filament.op.option.present and filament.op.option.value are lowered or refused by FR-054 |
| TC-068 step 9 and Status | examined | Build an option of an unbounded integer and assert package admission refuses it with OperatorIneligible |
| tests.md FR-054 row | examined | PARTIAL (IR-1448): AC-1 to AC-5, AC-7, AC-15, AC-16, AC-22 and AC-23 implemented |
| spec.md FR-054 row | examined | lowering of option presence and value ... (partial) |
| docs/qualification/ir1448-kani-option-clause-runs.md | examined | Not reachable or not run (PARTIAL, unowned, planner to assign) |

## Dispositions
