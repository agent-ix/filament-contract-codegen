---
id: SR-17940486709564847048063
title: "IR-806 PR-3 PR #858 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #858 at 124ea590, git diff c2931a1b...124ea590 (stacked on PR #849); IR-806 PR-3; FR-029-AC-19 to AC-27, AC-30, AC-31, FR-033-AC-7, AC-8, AC-10, AC-19; src/replay/composite_shadow_route.rs, tests/it/composite_shadow_route.rs, tests/it/composite_shadow_kani.rs"
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

Ticket: IR-806 (PR-3). Planless gap analysis of the criteria this PR marks Implemented or PARTIAL,
against `quoin matrix` tags, the route tests and the code. Plan completion: not assessed.

Claimed Implemented: FR-029-AC-19, AC-23, AC-24, AC-25, AC-26, AC-27, AC-30, AC-31. Claimed
PARTIAL: FR-029-AC-20, FR-033-AC-8, AC-10, AC-19. FR-033-AC-18 and FR-028-AC-23 are untouched.
`quoin matrix` reports every one of these as `tagged` (FR-033-AC-10 by an ignored Kani-lane test).
AC-19, AC-23, AC-24, AC-25, AC-26 and AC-30 are backed by tests that fail under mutation (see
SR-17940486709564847048062). AC-20 PARTIAL is honest: F-3 `RefusedInput` and a native `Refused`
row are not reached from the entry, and the AC text says so. FR-033-AC-8 PARTIAL lists the
untested key cases; the reviewer ran omitted, empty, duplicate, unknown and wider cases by probe
and FSL and CG handle each correctly, so the remainder is test coverage only.

No Linear ticket named "IR-806 follow-up" or similar exists: an IR-team issue scan (all states,
updated since 2026-11-10) finds only IR-806 for this work, and IR-806's comments name no
follow-up id.

## Verdict

**Changes requested.** FR-029-AC-27 is claimed Implemented and Covered, but its run, content and
domain bindings are not checked. FR-033-AC-19's remainder leaves out parts the AC requires that
nothing tests. The PARTIAL remainders point at an owner that is not a ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Every PARTIAL remainder (FR-029-AC-20 F-3/native Refused, FR-033-AC-8 key/refusal matrix, FR-033-AC-10 other families, FR-033-AC-19 measured pair count) and the tests.md rows name "IR-806 follow-up" as owner. No such ticket exists in Linear. File an owned ticket and cite its id before merge. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:397, spec/replay/matrix/tests.md, spec/kani/matrix/tests.md |
| FND-002 | medium | FR-029-AC-27 reads "Implemented (IR-806 PR-3) for the node, operation, operand-source, harness-bound and limits binding", and tests.md marks it Covered. The AC also names run, domain and content bindings: the entry checks no run binding (verified evidence of another harness settles `Proved`, code-review FND-001), and the bounds and limits ties are untested (code-review FND-004). Mark AC-27 PARTIAL with an owner, or implement and test the missing bindings. | spec/kani/functional/FR-029-run-outcome-terminal-record.md:404, spec/kani/matrix/tests.md |
| FND-003 | medium | The FR-033-AC-19 remainder lists only the measured native pair count. The AC also requires refusing changed retained bytes or context and altered limits, and a body-only mutant from the same context exercising both sides. None is exercised through the route; only an operand swap is. The native outcome the tie compares against is test-supplied (`authenticated(..., Outcome::Completed(false))`), so the consistency check compares two synthetic values. It is not vacuous for the operand tie, but it gives no native-authenticity evidence. List these in the PLANNED remainder. | spec/replay/functional/FR-033-composite-parity-replay-binding.md (FR-033-AC-19), tests/it/composite_shadow_route.rs:481-514, 730-783 |
| FND-004 | low | Spec evidence cites tests not tagged to the criterion: FR-029-AC-31 cites `tc_048_a_falsified_shadow_replays_through_fsl_from_a_cg_built_request` (not tagged AC-31); AC-23 cites the inconclusive-rows and accounting-limit tests (tagged AC-25/26/30 and AC-20 only); AC-27 cites `tc_048_the_native_observation_is_tied_to_the_authenticated_artifact` (tagged FR-033-AC-19, AC-24). | tests/it/composite_shadow_route.rs:323-327, 557-561, 685-687, 727-730 |

## Coverage

- Matrix: `quoin matrix --repo . --json` at 124ea590, criteria above all `tagged`.
- Route tests 9/9 pass; mutation results in SR-17940486709564847048062.
- Linear: IR-team issues scanned for a follow-up owner; none found.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at baecb8c0: Linear IR-1567 ("IR-806 follow-up: remaining composite-shadow production-route ACs", Backlog, project Kani Lowering MVP) exists. Every PARTIAL remainder and tests.md row cites IR-1567, and no "IR-806 follow-up" text is left in spec/. |
| FND-002 | fixed | Round 1 at baecb8c0: FR-029-AC-27 reads "PARTIAL (IR-806 PR-3; owner IR-1567 for the remainder)". It lists the implemented and tested bindings and, as PLANNED, the work-limit, ceiling, declared-domain/content and retained-bytes gaps. tests.md has a separate Partial row for AC-27, and the Covered row now spans AC-23 to AC-26. |
| FND-003 | fixed | Round 1 at baecb8c0: the FR-033-AC-19 PLANNED (IR-1567) remainder lists the driver-supplied native observation without a pair count, the shadow trust assumption, the synthetic capture, changed bytes/source refused by IR-1184 rather than the route, and the unbuilt body-only mutant. Other bounds and limits are now tested through the route. |
| FND-004 | fixed | Round 1 at baecb8c0: the falsified test is tagged FR-029-AC-31; the inconclusive-rows and accounting-limit tests are tagged FR-029-AC-23; the native-tie test is tagged FR-029-AC-27. |
