---
id: SR-17940486709564847048037
title: "IR-806 PR-2 PR #849 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #849 at 29227cbd, git diff origin/main...HEAD (base e9918842); IR-806 PR-2; spec/kani/functional/FR-025-generated-subject-abi.md, spec/kani/matrix/TC-036-generated-subject-abi.md, spec/kani/matrix/tests.md, spec/replay/functional/FR-033-composite-parity-replay-binding.md, spec/replay/matrix/TC-048-composite-parity-replay-binding.md, spec/replay/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-033
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-2). Base spec review of the six spec files this PR touches. `make spec` exits
0 at 29227cbd (the grammar warnings are in files this PR does not touch).

FR-025-AC-9 and FR-033-AC-2 to AC-5 now read PARTIAL with named tests and explicit not-covered
lists. The FR-033 description, TC-036 step 9, TC-048 steps 2 and 4 and both tests.md rows agree
with them. The tests.md row that grouped AC-2 to AC-5 with AC-8, AC-10 and AC-19 is split into a
Partial row and a Planned (IR-806 PR-3) row. The TC-048 run record gives wall time, memory peak
and verdict with no tool versions. No PLANNED/GATED tag is left on the five criteria this PR
moves.

## Verdict

**Approve with one low finding.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-033-AC-8 and FR-033-AC-10 in the touched FR-033 table still read "PLANNED/GATED". The gate they carried (QSL-640) is Done, and the split tests.md row of this PR now calls them "Planned (IR-806 PR-3)". Change both to PLANNED (IR-806 PR-3) so FR and matrix agree. | spec/replay/functional/FR-033-composite-parity-replay-binding.md:367, spec/replay/functional/FR-033-composite-parity-replay-binding.md:369, spec/replay/matrix/tests.md:42 |

## Coverage

Examined FR-025-AC-9, FR-033-AC-2 to AC-5, the FR-033 description paragraph at lines 59-64,
TC-036 step 9, TC-048 steps 2 and 4, and the changed rows of both tests.md files. AC-8, AC-10 and
AC-19 were read for status consistency only.
