---
id: SR-17940486709564847048007
title: "IR-1447 PR #840 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #840 at 63e870c3, git diff origin/main...HEAD; IR-1447; spec/kani/functional/FR-015-bounded-kani-obligations.md (AC-152..173), spec/kani/functional/FR-025-generated-subject-abi.md (AC-19), spec/kani/matrix/TC-025-bounded-kani-obligations.md (step 57), spec/kani/matrix/TC-036-generated-subject-abi.md (step 19), spec/kani/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-1447. Spec-review of the status and sentence changes. The PR replaces the stated
arithmetic-node bounds of AC-152, AC-153, AC-154 and AC-165 (for example `amount + 1` has bound
`Int[0, 1000]`) with "FSL types the arithmetic nodes of this fixture as mathematical integers, so
none carries a result bound and the kernel refuses none of them". That correction is right: the
FSL spec states that integer arithmetic over bounded operands has the unbounded `Int` result type
(filament-spec-language TC-1070, `Composed.arith`), and the PR's constructed fixtures
(`bounded_addition`) exist because the FSL-emitted addition node carries no bound. The kernel
therefore checks no result bound on those fixtures, and the old bounds could not have been
observed. The TC-025 step 57 rewording of the `case` arms (last arm exhaustive, none outside the
members) matches the rendered `match`. FR-025-AC-19, TC-036 step 19 and both tests.md rows agree
on PARTIAL.

`make spec` (quire validate) exits 0 at the head with warnings only on files this PR does not
touch. Sub-analyses not run: the diff adds no requirement statement, relationship edge or domain
object; it edits status cells and one fixture sentence per AC.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | The bound correction stops at AC-165: AC-170 and AC-173 still state FSL-style interval bounds ("the `amount + 2` node has bound `Int[0, 1001]`", "`amount + 1` ... `Int[0, 1000]` and the `a + 1` node ... `Int[0, 1001]`") for packages built by renaming an FSL-emitted binder. Unless the constructed package also retypes those nodes (as `bounded_addition` does for AC-168/169/172), they carry no bound, and the sentence repeats the error the PR fixes elsewhere. Either say the constructed package sets these bounds, or replace them with the same "no result bound" sentence. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1016, spec/kani/functional/FR-015-bounded-kani-obligations.md:1019 |

## Verdict

The FSL arithmetic-bound correction is right and consistently applied to the four fixtures it
touches; AC-170/173 were left with the old style (FND-001, low). Statuses agree across FR-015,
FR-025, TC-025, TC-036 and tests.md; the Implemented overclaims are recorded in the gap-analysis
SpecReview. AC-156 keeps a Kani version in its criterion text from before this PR (not
introduced here); the new run record carries none.

## Dispositions

Round 1, checked against the fix commits 5ff50540 and 1733eb3e (head 1733eb3e).

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | AC-170 and AC-173 now read "the constructed package ... keeps FSL's mathematical-integer arithmetic, so no node carries a result bound and the kernel refuses none of them", matching AC-152/153/154/165. |
