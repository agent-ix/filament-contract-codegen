---
id: SR-17940486709564847048031
title: "IR-806 PR-1 PR #848 spec-review (base)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #848 at 653b0170, git diff origin/main...HEAD (base 149c98d2); IR-806 PR-1; spec/kani/functional/FR-028-bounded-proof-ceilings.md, spec/kani/functional/FR-029-run-outcome-terminal-record.md, spec/kani/matrix/TC-040-run-outcome-terminal-record.md, spec/kani/matrix/tests.md, spec/replay/matrix/TC-048-composite-parity-replay-binding.md; context spec/core/functional/interface-001-codegen-api.md, spec/replay/functional/FR-033-composite-parity-replay-binding.md, fsl-replay 14c0fd8 composite.rs and execute/composite_parity.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-040
    type: reviews
---

## Summary

Ticket: IR-806 (PR-1). Spec delta review: FR-029-AC-17/21/22/31 moved to implemented, the
`GATED (IR-806/QSL-640)` tags dropped from FR-029-AC-19/23/24/26/27 (now `PLANNED (IR-806)`), the
Implementation Gate rewritten ("None remains upstream"), three QSL-640-gated wordings in FR-028
replaced, the `TerminalPairError` variant list corrected, and AC-25's memory row rewritten after
ADR-007.

Verified clean: QSL-640 is delivered (fsl-replay at the locked 14c0fd8 exports
`settle_verified_shadow` and the composite parity facade), so dropping the QSL-640 tags is correct;
`TerminalPairError` has exactly the five variants listed (`MissingSettlement`,
`UnexpectedSettlement`, `MissingProvedCount`, `ProvedCountOutOfRange`, `InvalidCancellation`,
src/kani/terminal.rs:80-92); `MemoryExhausted` no longer exists in `src/` and AC-25's memory clause
now points at the AC-34 `out_of_memory` cause row as ADR-007 requires; the AC-17 projection text
matches `ProofStrength::parity_refinement` row for row; FR-028-AC-17 states exactly six strength names.

AC-25 / AC-30 diagnosis: the coder is right that the two criteria cannot both hold with the
delivered API. AC-25 requires every `bounded_shadow` inconclusive reason (vacuous, cover-unsatisfied,
timeout, unwind, no-verdict, missing-summary, witness-disagreed, failure-without-counterexample) to
"map through the ordinary rows" (Proved0, Incomplete(TimedOut), ...), which are the rows of
`run_terminal_value`; AC-30 forbids any `bounded_shadow` outcome, inconclusive and cover-unsatisfied
included, from reaching `run_terminal_value`; and FSL's composite route takes only
`CompositeEvidence::{Falsified, Verified}` (`settle_verified_shadow` takes a `VerifiedShadow`), so
there is no parity entry for an inconclusive or cover-unsatisfied shadow run. Leaving both PLANNED
and not inventing a route is right; but the conflict itself is stated nowhere in FR-029 (FND-004).

## Verdict

**Changes requested (no high findings).** The status moves are supported by code and tests, but the
PR leaves a dead test-file pointer in FR-029's own Status, leaves interface-001 and TC-048 asserting
the pre-PR state for the same criteria it marks implemented, and does not record the AC-25/AC-30
contradiction in the spec it edits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-029's Status cites the evidence as `tests/it/proof_strength_projection.rs`, a file that does not exist; the tests are in `tests/it/composite_parity_converter.rs` (as tests.md and TC-048 say). The same sentence pair also lists AC-25 twice ("AC-19, AC-23 to AC-27 and AC-30 ... and AC-25 remain planned"). | spec/kani/functional/FR-029-run-outcome-terminal-record.md:460-461 |
| FND-002 | medium | interface-001 still describes the same contract as before this PR: "Planned IR-806 composite parity ... FR-029 AC-17 and AC-19 to AC-27 own closed strength projection ... UNVERIFIED SOURCE and gated on actual QSL-640 API delivery". This PR marks FR-029-AC-17 implemented and says no upstream gate remains, so the interface spec now contradicts FR-029. It is the last QSL-640 gate in the repo. | spec/core/functional/interface-001-codegen-api.md:160-166 |
| FND-003 | low | TC-048 still lumps implemented and planned rows: its intro says "FR-029 AC-17 and AC-19 to AC-28. All completed-route checks remain GATED." and the scenario table row is "FR-029-AC-17 and AC-19 to AC-27; FR-028-AC-17/24". AC-17 is now covered and AC-22/AC-31 have no row there. This PR edits TC-048 step 8 for exactly these criteria, so the stale rows sit in a file it touches. | spec/replay/matrix/TC-048-composite-parity-replay-binding.md:21-23, spec/replay/matrix/TC-048-composite-parity-replay-binding.md:227 |
| FND-004 | medium | FR-029-AC-25 and FR-029-AC-30 contradict each other given FSL's delivered API, and FR-029 does not say so. AC-25 sends shadow-inconclusive and cover-unsatisfied outcomes through the ordinary rows of `run_terminal_value`; AC-30 says no shadow outcome, inconclusive and cover-unsatisfied included, ever reaches `run_terminal_value`; FSL's `CompositeEvidence` has only `Falsified` and `Verified`. The PR rewrites AC-25's text but the only trace of the conflict is a tests.md note ("AC-25 needs a shadow-aware route"). Record it as an open question in FR-029 (on AC-25 or in Status) with an owning ticket, without choosing an answer. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:402, spec/kani/functional/FR-029-run-outcome-terminal-record.md:407 |

## Coverage

Examined FR-029-AC-17, AC-19, AC-21, AC-22, AC-23, AC-24, AC-25, AC-26, AC-27, AC-30, AC-31, the FR-029
Implementation Gate and Status, FR-028-AC-17 and the two FR-028 QSL-640 rewordings, the TC-040 and
TC-048 step edits and the tests.md rows. FR-033's `PLANNED/GATED` tags (AC-2..5, 8, 10) name no
QSL-640 gate and lie outside this PR's status claims; they are not counted as leftovers here.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new FR-029 open question says "No function therefore maps a non-verified shadow outcome to a terminal value". That is too broad: a falsified shadow outcome has FSL's `replay_composite_parity` (it takes a `FalsifiedParity`; FR-029-AC-20/24 state its rows). The gap is only the inconclusive and cover-unsatisfied outcomes AC-25 names. Say "an inconclusive or cover-unsatisfied shadow outcome" and note that the falsified route exists. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:427-428 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at 7ffa085c: FR-029 Status cites `tests/it/composite_parity_converter.rs` and lists AC-25 once (inside AC-23 to AC-27). |
| FND-002 | fixed | Round 1 at 7ffa085c: interface-001:161-166 lists AC-17, AC-21, AC-22 and AC-31 (verified entry) as implemented and AC-19, AC-23 to AC-27 and AC-30 as PLANNED under IR-806 with no upstream gate. No QSL-640 gate remains in spec/ or src/; the only QSL-640 mention left is FR-029's "QSL-640 delivered" note. |
| FND-003 | fixed | Round 1 at 7ffa085c and 621d0668: the TC-048 intro separates implemented, partial and planned criteria; the scenario table has separate rows for AC-17/AC-22, AC-21, AC-31 (verified entry) and the planned AC-19, AC-23 to AC-27. |
| FND-004 | fixed | Round 1 at 7ffa085c: FR-029 has a new "Open question (IR-806 PR-3 design decision)" section. It states the AC-25/AC-30 conflict, both options, IR-806 PR-3 as owner, and gives no answer. Its wording is too broad; see FND-005. |
