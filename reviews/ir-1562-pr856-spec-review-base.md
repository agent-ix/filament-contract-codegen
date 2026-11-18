---
id: SR-17940486709564847048061
title: "spec-review of IR-1562 PR #856 FR-056 Verus verdict rows"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen; PR #856 at bd5ececf; spec/kani/functional/FR-056-kani-tractability-refusal.md, spec/kani/functional/FR-053-float-scalar-harnesses.md, spec/kani/matrix/TC-070-kani-tractability-refusal.md, spec/kani/matrix/tests.md, docs/qualification/ir260-float-scalar-kani-measurements.md"
review_set: subset
---

## Summary

Ticket: IR-1562. Base spec review of the spec edits, checked against filament-exact `origin/main` (be9359f) and against the code of this PR. `make spec` passes with installed quire 0.36.2, and none of its warnings is in a line this PR changed.

## Examined scope

- FR-056 Description, lines 55-105 (examined). This is the new Verus paragraph, the 77-row count and the decimal rows.
- FR-056 Behavior "Measured table" and FR-056-AC-3 (examined). Neither is changed by the PR, which is the defect in FND-001.
- FR-056-AC-15 and Status (examined). The Status names the three new tests.
- TC-070 steps 15 and 16 and its Status (examined). Step 16 matches the tests: 24 rows; record, change and theorem equality; the unproved sets; the loader rejections.
- FR-053 Status paragraph (examined). The ir260 measurements doc line 94 (examined).
- tests.md FR-056 row (examined).
- No tool versions or pins were added (examined). The FR-405, FR-402, FR-403 and FR-409 deferral texts live only in filament-exact; this PR does not edit them (see the gap-analysis SR).

## Verdict

**CHANGES REQUESTED.** The new verdict, the new field and the loader rules are introduced only in Description prose. The normative Behavior and AC-3 still say that a verdict is `tractable` or `not_tractable` and that `wall_seconds` is a number, so the spec now contradicts the code. A Behavior edit and a new AC (FR-056-AC-16) are needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-056 Behavior "Measured table" still says the row fields are `verdict` (`tractable` or `not_tractable`) and `wall_seconds`, with no `verus_proof`. FR-056-AC-3 says that "a row with an unknown verdict ... fails the load" against those Behavior fields. The code now loads `verus_proved` with a null `wall_seconds` and rejects inconsistent rows, and these rules appear only in Description prose (lines 78-92). The spec contradicts the code. The fix is to add the verdict, the nullable `wall_seconds`, `verus_proof` and the consistency rules to the Behavior, plus a new FR-056-AC-16 for the loader and never-refuse rules. | spec/kani/functional/FR-056-kani-tractability-refusal.md:164; spec/kani/functional/FR-056-kani-tractability-refusal.md:189 |
| FND-002 | medium | The decimal paragraph says "The proofs assume `Meter::charge_units` appends an admitted request; admission stays with FR-359 and the CG-domain Kani rows". FR-403 Behavior 41 makes no Meter assumption (the Meter is excluded outright). That assumption belongs to FR-408 Behavior 18a. There are no CG-domain Kani rows for decimal, and the decimal rows' `assumes` does not list it. The sentence attributes the rational assumption to the decimal proofs. | spec/kani/functional/FR-056-kani-tractability-refusal.md:102-104 |
| FND-003 | low | The tests.md FR-056 row says "the Verus verdicts of the kernel's record on 21 of those rows", where "those" means the 44 float and 14 rational rows. Of the 21, 3 are the integer division rows, which are not among them. The correct statement is 18 of those plus the 3 division rows. | spec/kani/matrix/tests.md:14 |
| FND-004 | low | The FR-053 Status sentence is garbled: "(FR-405-AC-17 and AC-18, TC-953; the numeric path only, FR-056) (FR-056; they are a record, not a mechanism: ...". It has two adjacent parentheticals, and FR-056 is cited twice. | spec/kani/functional/FR-053-float-scalar-harnesses.md:484-486 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The Behavior lists `verus_proved`, the nullable `wall_seconds`, `verus_proof` and the SHALL rules. AC-3 names the three verdicts, and AC-16 and AC-17 were added. |
| FND-002 | fixed | The Meter assumption is now attributed to the rational proofs (FR-408 Behavior 18a). |
| FND-003 | fixed | Now reads "18 of those rows plus the 3 native division rows". |
| FND-004 | fixed | The FR-053 sentence was rewritten as one parenthetical. |
