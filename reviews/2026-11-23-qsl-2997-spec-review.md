---
id: SR-17940486709564847048110
title: "QSL-2997 PR #898 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #898 at f1ffeae1; QSL-2997; FR-016-AC-31, FR-024-AC-78, interface-001, TC-026, TC-035, spec/replay/matrix/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-2997. FR-016-AC-31 states exact behaviour (two namespaces, one helper, refused under empty selection,
malformed row to `DependencyLockError::Clock`, `CgDefect` per FR-029-AC-16) and is tagged by three tests of
`tests/it/replay_clocks.rs`. FR-024-AC-78 states that four further inputs types build their requests from
`ReplayInputs.clocks`; its only tagged evidence is the malformed-row test, which runs through `ReplayPackage::new`.
interface-001 and TC-026/TC-035 text match the code. The matrix row for FR-016-AC-31 and FR-024-AC-78 reads
"Covered", while TC-026/TC-035 summary rows stay Partial, which is consistent.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-024-AC-78 is marked Covered but no test reaches FrameReplay or StateClauseReplay paths with clocks; the AC's frame/state-clause claim has no evidence | spec/replay/functional/FR-024-counterexample-envelope-intake.md:621 |

## Verdict

Mergeable once FR-024-AC-78 has a test per path or its Covered claim is narrowed.
