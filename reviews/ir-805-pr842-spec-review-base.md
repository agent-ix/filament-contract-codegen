---
id: SR-17940486709564847048013
title: "IR-805 PR #842 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #842 at a1a4c6b6, git diff origin/main...HEAD -- spec/; FR-053, TC-067, FR-056, FR-057, TC-071, FR-015-AC-151, TC-025, spec/kani/matrix/tests.md, plus rename-only edits across spec/**"
review_set: subset
---

## Summary

Ticket: IR-805 (with IR-260, IR-854, IR-1509).

The spec changes fall into four groups:
- The rename (`quire.op.*` to `filament.op.*`, `qsl_replay` to `fsl_replay`, crate names). These
  are mechanical.
- The IR-260 flip of FR-053 and TC-067 to Implemented.
- The IR-854 FR-056 and FR-057 prose for the copied native rows, with AC-7 kept PARTIAL.
- The IR-1509 AC-151 wording.

`make spec` exits 0. Its seven grammar warnings sit on lines this PR only renamed.

The added lines bring in no new tool versions. The `Kani 0.68.0` / `CBMC 6.11.0` mentions are
renamed lines that already existed on main; the diff adds and removes the same count.

FR-057-AC-7 PARTIAL and FR-015-AC-151 PARTIAL are worded honestly.

## Verdict

**PASS with one medium finding (IR-260 only).**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | IR-260 only. FR-053 and TC-067 are flipped to "Implemented", and tests.md to "✅ Implemented", but FR-053 still contradicts that status. Line 57 says every supported-harness criterion is conditional on P1. The AC preamble (line 375) says "All supported-harness criteria are conditional on P1 (IR-1446)", and lines 68-69 say "Until IR-1446 lands them, AC-13 cannot run". IR-1446 is In Progress in Linear. tests.md meanwhile claims AC-13 delivered. The prerequisites sentence was rewritten to "P2 and P3 (IR-1502) are delivered" and dropped P1's status instead of settling it. Scenario: a reader cannot tell whether the comparison row's harnesses rest on a delivered carrier. Fix: state P1 as delivered for the three comparisons (and cite the filament-exact evidence), or keep the FR PARTIAL on P1. | spec/kani/functional/FR-053-float-scalar-harnesses.md:56-69,375-377,419; spec/kani/matrix/TC-067-float-scalar-harnesses.md:140; spec/kani/matrix/tests.md:18 |

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-053 Status | examined | Implemented. Scope ruling (planner): the row is the supported combinations, the three comparisons |
| FR-053-AC-3 | examined | requires-bound is the Contract IR record CG reports |
| FR-053 prerequisites | examined | Three prerequisites are named; P2 and P3 (IR-1502) are delivered |
| FR-056 measured table | examined | the kernel's rows (IR-1541) are in the table: tractable at 8 bits, not_tractable at 16 bits |
| FR-057-AC-7 | examined | stays PARTIAL; native route still refuses NativeDivisionPlanUnavailable |
| FR-015-AC-151 | examined | PARTIAL; PLANNED pipeline subject, unowned, planner to assign |
| TC-025 AC-151 step | examined | PLANNED: the subject compiled by the pipeline |
