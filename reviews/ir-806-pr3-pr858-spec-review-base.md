---
id: SR-17940486709564847048064
title: "IR-806 PR-3 PR #858 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #858 at 124ea590, git diff c2931a1b...124ea590 (stacked on PR #849); IR-806 PR-3; spec/kani/functional/FR-029-run-outcome-terminal-record.md, spec/kani/matrix/tests.md, spec/replay/functional/FR-033-composite-parity-replay-binding.md, spec/replay/matrix/TC-048-composite-parity-replay-binding.md, spec/replay/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-033
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-3). Base spec review of the five spec files this PR touches.
`FSPEC_DIR=/home/peter/dev/filament-specification make spec` exits 0 at 124ea590; none of its
grammar warnings is in a touched file. No tool version, pin or digest is added.

The FR-029 "Design decision (IR-806 PR-3)" matches the lead's decision and the code. One
shadow-aware entry reuses the original proving context. A retained disagreement and verified
runs go through `settle_verified_shadow`, falsified runs through `replay_composite_parity`, and
inconclusive, cover-unsatisfied and cancelled runs through the ordinary rows. There is no second
route or FSL producer, and `run_terminal_value`'s signature is unchanged (AC-21; `src/kani/terminal.rs`
is not in the diff). The decision is sound: per the pinned FSL docs, the verified entry's V-1
settles `Disagreed` right after the common prepare steps, before any operand or native setup.
AC-24 permits that precedence ("after common claim/source/node/bounds/O-09 validation").

## Verdict

**Changes requested (no high findings).** Some text still calls the route planned, and the
design decision does not state how a disagreement with a cancelled run settles.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Stale planned text: the FR-029 subsection heading still reads "IR-806 production route planned", and its body says "only the production invocation of FSL's delivered parity facade is pending". The TC-048 sentence still says "AC-31 (PARTIAL: verified entry only) and AC-19 to AC-28 (... the rest PLANNED under IR-806)" right before saying the route is built. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:213-223, spec/replay/matrix/TC-048-composite-parity-replay-binding.md |
| FND-002 | low | The design decision says "any retained disagreement" settles through `settle_verified_shadow`, but the code sends a cancelled run with a retained disagreement to the ordinary rows (`Incomplete(Cancelled)`). AC-24 lists only verified, falsified, inconclusive and cover-unsatisfied. State the Disagreed-plus-Cancelled row explicitly in the decision and AC-24/AC-30. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:422-434, src/replay/composite_shadow_route.rs:346-350 |

## Coverage

- `make spec` exit 0; matrix rows in both tests.md files agree with the AC Status text, except
  FR-029-AC-27 (see SR-17940486709564847048063 FND-002).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The tests.md FR-029-AC-27 row says the "claim (bounds, work limit)" binding refuses. The FR-029-AC-27 PLANNED (IR-1567) text says "the operand projection's work limit is not part of the claim identity and no artifact tie covers it". The test varies the accounting `work_units` limit (`fixture_with(u64::MAX - 1)`), not the projection work limit, so the row should say "accounting limits". | spec/kani/matrix/tests.md:104, spec/kani/functional/FR-029-run-outcome-terminal-record.md (FR-029-AC-27) |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at baecb8c0: the heading reads "IR-806 PR-3 production route built, remainder IR-1567". The body names `settle_composite_shadow` and IR-1567 instead of "pending". The TC-048 sentence lists AC-31, AC-19, AC-23 to AC-26 and AC-30 as built, and AC-20 and AC-27 as PARTIAL (IR-1567). |
| FND-002 | fixed | Round 1 at baecb8c0: the design decision reads "any retained disagreement of a run that was not cancelled" and states that a cancelled run with a retained disagreement is `Incomplete(Cancelled)`. AC-24 says the same and cites `tc_048_a_cancelled_run_takes_the_ordinary_row_even_with_a_retained_disagreement`. |
