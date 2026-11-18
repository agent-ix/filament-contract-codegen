---
id: SR-17940486709564847048060
title: "gap-analysis of IR-1562 PR #856 Verus verdict rows"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; PR #856 at bd5ececf; IR-1562 AC-1 to AC-3, FR-056-AC-3, FR-056-AC-15, TC-070 steps 15-16; tests/it/kani_tractability.rs, src/kani/tractability.rs, docs/qualification/ir1530-general-route-scalar-table.json"
review_set: subset
---

## Summary

Ticket: IR-1562. Plan completion: not assessed. This is a manual check of the acceptance criteria against the tests, scoped to the PR diff and the IR-1562 acceptance criteria:

- AC-1: each of the twelve float rows names the Verus verdict and its coverage.
- AC-2: the tests fail if a row cites a verdict absent from the filament-exact record.
- AC-3: the FR-056 and FR-405 Status texts drop the deferral.

The cross-repo sources were read from filament-exact `origin/main` (be9359f).

## Examined scope

- IR-1562 AC-1 (examined). Delivered: the twelve float rows carry `verus_proof`, and the coverage text matches FR-405. The assumption text is a defect, recorded in the code-review SR as its FND-001.
- IR-1562 AC-2 (examined). Partial: see FND-002.
- IR-1562 AC-3 (examined). The FR-056 half is done. The FR-405 half is not done (FND-001).
- FR-056-AC-15, with the bindings of the three new tests (examined). See FND-003.
- FR-056-AC-3 (context_only). The spec-review SR covers it.
- The new tests (examined). All three pass at head, and `tc_070_the_shipped_table_holds_the_kernel_native_float_and_rational_rows` and `the_shipped_table_holds_the_kernel_native_division_rows_on_the_native_route` pass too. Mutating a record id, a change id, a theorem name, the verdict, the set of rows carrying a proof, the Exact-only clauses, or the loader's proof/measurement arms fails them. The free-text and `run` fields are only weakly asserted (code-review SR, its FND-003).
- Gates at head (examined). `--test it` gives 1139 passed, 28 failed and 127 ignored. `--lib` gives 605 passed, 12 failed and 36 ignored. Each failure set is identical to the one on a clean origin/main 2ca0277a worktree (same test names): MalformedWire in the lib tests, the FSL `Unproved(Nonzero)` failures fixed by #854, and tc_025 / QSL-2361. None is caused by this PR.

## Verdict

**CONDITIONAL.** The CG data is delivered. The upstream half of AC-3 and an AC that owns the new behaviour are missing, and AC-2 is met only against CG's own transcription.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | IR-1562 AC-3 is not met for FR-405. filament-exact main FR-405 Status still says the recording "is a CG change, deferred to IR-1562, and is not part of this change". FR-402 and FR-403 Status still say "IR-1562 adds the decimal row ... not part of this change". FR-409 Status says the division recording is "deferred to IR-1561", yet this PR does it under IR-1562, and IR-1556 AC-2 (three CG rows record the Verus verdict) is still open in Review. No ticket owns the filament-exact follow-up edit. | filament-exact spec/functional/FR-405-kani-tractable-native-float-carrier.md:114; FR-409-kernel-checked-native-integer-divide-and-modulo-plan.md:161 |
| FND-002 | medium | IR-1562 AC-2 ("tests fail if a row cites a verdict that is absent from the filament-exact record") is met only against constants hand-transcribed into the test. Nothing reads or pins the upstream record. CG tracks filament-exact `branch = "main"`, so a later upstream change to TC-953, TC-956, TC-957, TC-950 or TC-951 (renamed theorems, new mutant counts) leaves the rows stale and the tests green. No owned ticket or documented refresh procedure covers this. | tests/it/kani_tractability.rs:1820; tests/it/kani_tractability.rs:1862 |
| FND-003 | medium | The three new tests are tagged `FR-056-AC-15`, but they assert none of what AC-15 states (routed exact-scalar harness, decided-run refusal before Kani, per-route rows). They assert the `verus_proof` contents, the `verus_proved` never-refuse rule and the loader consistency checks, and no AC owns those. A new FR-056-AC-16 is required, with these tests re-tagged to it. | tests/it/kani_tractability.rs:1860; tests/it/kani_tractability.rs:2019; tests/it/kani_tractability.rs:2111 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred: a second, spec-only filament-exact PR under IR-1562 after #856 merges drops the FR-402/FR-403/FR-405/FR-409 Status deferral. It is recorded in TC-070 step 16, and IR-1562 stays open until it lands. | owned |
| FND-002 | fixed | TC-070 step 16 now holds a re-transcription procedure, and the exact-text pins catch drift. Upstream equality was checked by the reviewer this round (see code-review FND-008 for the provenance wording). |
| FND-003 | fixed | FR-056-AC-16 and AC-17 were added. Two tests are re-tagged to AC-16 and the loader test to AC-17, and `quoin matrix` shows both tagged. AC-17's never-refuse clause is now asserted (mutant killed). |
