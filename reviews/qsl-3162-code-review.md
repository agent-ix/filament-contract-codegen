---
id: SR-202611210022442313144108536
title: "Code review — temporal SMT settlement adapter"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; Cargo.toml, Cargo.lock, src/routed/smt/mod.rs, src/routed/smt/temporal.rs; QSL-3162"
review_set: subset
---

## Summary

Reviewed QSL-3162's temporal SMT production adapter and its FSL dependency update, including the applicable FR-314, FR-127, FR-030 and FR-621 contracts. The adapter calls the production settlement function, but no test exercises the new public orchestration path end to end.

## Verdict

**CONDITIONAL** — the public route needs a behavioral test that covers query execution and settlement; the existing tests only validate the private mapping helper.

## Scope

- `FR-127-AC-14` (examined): `settle_smt_proof` settles bounded and inductive Counter outcomes, no-certificate, mismatch, unchecked-rule, limit, and fault cases; a crate with only `fsl-replay` compiles against the settlement API.
- `FR-314-AC-23` (context_only): Counter TP-2 `always[0,3] holds(c.value <= 3)` with accepted bounded-complete depth 3 certificate settles proved and Certified, with FR-621 checker `smt-proof` and accepted `true`.
- `FR-030-AC-19` through `FR-030-AC-22` (context_only): local CG SMT terminal-map obligations for accepted/uncertified proofs, certificate rejection, cross-check/replay outcomes, and one terminal result per accepted settlement.
- `FR-621` (context_only): analyzed outcome document contract.
- `FR-282-AC-14` (context_only): qualified-core temporal certificate check contract; no implementation claim in this CG adapter diff.

## Findings

| ID      | Severity | Summary                                                                                                      | Refs                       |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------ | -------------------------- |
| FND-001 | medium   | The public production route has no behavioral test; all three new tests call only the private mapper.        | src/routed/smt/temporal.rs:160-203,327-450 |

## Review Notes

The three unit tests cover mapping synthetic process records into `SmtProofOutcome`. They do not call `run_and_settle_temporal_smt_proof`, so they cannot catch regressions in FSL query encoding, invocation through `run_bounded_process`, proof capture options, or the call/result handling for `settle_smt_proof`. The reported focused test run and clippy run were supplied by the coder; this reviewer did not rerun Cargo gates. The repo's final workspace/all-targets/all-features control is outside this code-review run.

## Tracker

Linear issue resolution/read-back and posting were unavailable: `linear issue view QSL-3162` returned `No usable API key for workspace "agent-ix"`. No GitHub PR exists, and findings were not posted to GitHub.
