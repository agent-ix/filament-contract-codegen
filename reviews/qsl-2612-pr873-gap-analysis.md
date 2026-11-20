---
id: SR-17940486709564847048105
title: "gap-analysis of QSL-2612 PR #873 derived state-clause witness"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; PR #873 at 7350a284 (git diff origin/main...HEAD); FR-024-AC-76, FR-024-AC-77, FR-029-AC-16; src/replay/state_clause.rs, tests/it/state_clause_derived_witness.rs, tests/it/terminal_map.rs"
review_set: subset
---

## Summary

This is a planless gap analysis of the PR's criteria against their tagged tests and code. Ticket: QSL-2612. Plan completion: not assessed. `quire trace` resolves FR-024-AC-76 to one claim, `tc_035_the_carried_witness_record_is_the_record_the_replay_result_holds`. It resolves FR-024-AC-77 to three claims: the two unit tests in `src/replay/state_clause.rs` and `tc_035_a_clause_that_holds_carries_no_witness_record`. `tc_040_the_state_clause_replay_reads_as_fr029_ac16` (FR-029-AC-16) is green at head.

## Examined scope

- FR-024-AC-76 (examined). The test records the payload's `witness` inside the executor and compares it with `arm.record()` of FSL's result. Dropping the store fails it, because the carried value becomes `None` while the result holds a record. The binding is correct.
- FR-024-AC-77, no-record arms (examined). The unit test covers `Ok(None)`, `RangeViolation` and `NotRefuted`, and the crediting integration test covers `NotRefuted` end to end. Both bindings are correct.
- FR-024-AC-77, other-refusal arms (examined). Only `Exhausted`, which gives `SetupRefused(ResourceExhausted)`, and `Fault`, which gives `Fault`, are asserted.
- FR-029-AC-16, amended `Derivation` clause (examined). It shares the coverage of the unit test above.
- The two `tc_025` state_frame tests (context_only). The coder baselined them as failing identically on origin/main (`/claim_names/1` admission at `exact_scalar_support/package.rs:1210`). They were not re-run here and are not this PR's.
- Mutants (examined). The coder claims 8 mutants killed. One spot mutant was checked here (FND-001) and it survives.

## Verdict

**CHANGES REQUESTED (one medium).** AC-76 and the no-record half of AC-77 are well evidenced. The "`Replay` as the wrapped `ReplayRefusal`" clause of AC-77 and FR-029-AC-16 has no test, and its mutant survives.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-024-AC-77's "`Replay` as the wrapped `ReplayRefusal`" arm has no test. Replacing `DerivationRefusal::Replay(refusal) => Self::Refused(refusal)` with `=> Self::CgDefect` and re-running the lib `replay::state_clause` tests plus the `it` tests `terminal_map`, `state_clause*` and `kani_obligations_state_clause_replay` leaves everything green: lib 8 pass, it 68 pass. The `Refused` and `Undefined` arms and the unmatched-code fall-through (code-review FND-001) are also unasserted. TC-035 step 50's mutation control "losing the code fails the mapping case" holds only for `Exhausted`. | src/replay/state_clause.rs:457-465; src/replay/state_clause.rs:1484-1502 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tc_035_a_request_refusal_and_a_fault_end_the_replay` now feeds `DerivationRefusal::Replay(WrongModelGraphEntry)` through `carry_derived_witness` and asserts `StateClauseReplayError::Refused` and `ReplaySettlement::Refused(WrongModelGraphEntry)`, so the Replay-to-defect mutant is killed. `Fault` is asserted as `DerivationFault`, settled `Fault`. `tc_035_a_closed_scope_and_every_evaluation_outcome_carry_no_record` covers `Refused` (code `integer_out_of_domain`), `Undefined` and `Exhausted`. At 0734e1f8, lib `replay::state_clause` and `core::profile` pass 14, and `it` `terminal_map`, `state_clause*` and `kani_obligations_state_clause_replay` pass 68 with 3 ignored, `tc_040_the_state_clause_replay_reads_as_fr029_ac16` included. |
