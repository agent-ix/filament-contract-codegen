---
id: SR-17940486709564847048106
title: "spec-review of QSL-2612 PR #873 FR-024-AC-76/77, TC-035 step 50, FR-029-AC-16"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen; PR #873 at 7350a284 (git diff origin/main...HEAD); spec/replay/functional/FR-024-counterexample-envelope-intake.md, spec/replay/matrix/TC-035-counterexample-envelope-intake.md, spec/replay/matrix/tests.md, spec/kani/functional/FR-029-run-outcome-terminal-record.md"
review_set: subset
---

## Summary

This is a spec review of the PR's four spec edits. Ticket: QSL-2612, with the correction comment. They were checked against FSL FR-269 (the `derive_state_clause_witness` section and AC-6 to AC-9) at 2cb29fcd. All four files pass `quire validate`.

## Examined scope

- FR-024-AC-76 (examined). It matches FR-269's producer obligation: call with `witness: None`, store the record unchanged, build nothing in CG.
- FR-024-AC-77 (examined). `Ok(None)` and `RangeViolation` match the correction comment. `NotRefuted` goes beyond the comment, but it is consistent with FR-269 (`NotRefuted` means the payload is no counterexample, and FR-269-AC-9 says replay tolerates an absent record) and it is required by ticket step 3 (the crediting run stays `ReplayParity(Verdicts)`). FSL states no contrary caller rule.
- FR-029-AC-16 amended text (examined). The `Derivation` clause, "as the wrapped `ReplayRefusal` does, a fault as a fault and any other FSL refusal as `Inconclusive(ReplayRefused)` with its catalog code", is the right rule. Code-review FND-001 is where the code misses it.
- TC-035 step 50 (examined). It describes the tests as written.
- tests.md (examined). The FR-024 row for AC-76/77 is new, and the TC-035 traces list is extended with AC-76 and AC-77.
- FR-024 Behavior bullets at lines 389-396 and 448-451, and FR-024-AC-63 (examined). See FND-001.

## Verdict

**CHANGES REQUESTED (one medium).** The new criteria are sound and agree with FSL FR-269. The older FR-024 text they supersede was left saying the opposite.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Older FR-024 text now contradicts FR-024-AC-76. FR-024-AC-63 and the two Behavior bullets at lines 389-396 and 448-451 still say the generator "shall set the payload's `witness` record to none" and that a decisive basis makes the result "`inconclusive` with cause `Witness`, which the generator returns as it is". After this PR the envelope sent to FSL carries the derived record, and FSL FR-269-AC-9 settles an absent record as agreement anyway. None of these was amended, and no Behavior SHALL states the new derivation step. | spec/replay/functional/FR-024-counterexample-envelope-intake.md:389-396, :448-451, :604 |
| FND-002 | low | The FR-024-AC-77 evidence list puts `(src/replay/state_clause.rs)` after the two unit tests but gives no path for `tc_035_a_clause_that_holds_carries_no_witness_record`, which is in `tests/it/state_clause_derived_witness.rs`. | spec/replay/functional/FR-024-counterexample-envelope-intake.md:616 |
