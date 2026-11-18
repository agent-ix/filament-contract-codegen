---
id: SR-17940486709564847048059
title: "code-review of IR-1562 PR #856 Verus verdicts in the scalar tractability table"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; PR #856 at bd5ececf (git diff origin/main...bd5ececf); docs/qualification/ir1530-general-route-scalar-table.json, src/kani/tractability.rs, src/lib.rs, tests/it/kani_tractability.rs, tests/it/integer_division_harness.rs"
review_set: subset
---

## Summary

Ticket: IR-1562. Code review with the rust-review lane. The PR adds an optional `verus_proof` object to 21 existing `not_tractable` rows (12 float, 6 full-`i64` rational, 3 native division), three new `verus_proved` decimal rows, a `VerusProved` verdict, a nullable `wall_seconds`, and a loader consistency check. Every `verus_proof` claim was checked against filament-exact `origin/main` (be9359f): FR-402 Behavior 11 and Status, FR-403 Behavior 41 and Status, FR-405 Behavior 23 and the per-row Status table, FR-408 Behavior 18a and Status, FR-409 Behavior 24 and Status, and the Verus records of TC-950, TC-951, TC-953, TC-956 and TC-957.

## Examined scope

- The 74 existing rows (examined). They match origin/main positionally and byte for byte once `verus_proof` is removed, key order included. The only change is the appended `verus_proof` object, and no Kani verdict, wall time, peak or provenance changed.
- Float rows (examined). The 12 rows (26, 27, 29, 32, 45, 48, 49, 50, 51, 52, 54, 55) are exactly the 12 rows of the FR-405 Status table, as operation, width and spelling group. The four `Exact` rows (27, 49, 52, 55) alone add "rounded as nearest-even" and the strict refusal. The numeric-path operand domains match the table: finite nonzero for div and mul, positive finite for sqrt, non-infinite non-NaN for fma. Theorem names match TC-953 (`row_*`, `lemma_plan_*`, the `integer_square_root` loop invariant, `lemma_sum`, `lemma_round`, `lemma_decode_rounded`, `lemma_grid_round`). The run is 396 verified, 0 errors, with 33 of 33 mutants killed. `not_proved` repeats the FR-405 common set verbatim. The `assumes` text is wrong (FND-001).
- Rational rows (examined). Theorems, records and run (211 verified, 18/18 mutants) match TC-956. The `assumes` text matches FR-408 18a. The equality row says the production function is NOT proved in Verus, names the completing Kani harness `native_rational_equal_all_parts_is_structural` (FR-408-AC-15), and does not carry that harness's verdict. That is correct.
- Division rows (examined). Their text matches FR-409 Behavior 24 and the TC-957 record: `div_rem_post` and `lemma_truncating`, 221 verified, 10/10 mutants, the same assumptions, symbolic and constant divisors alike, and every width. The rows defer the member choice, the result interval, the outcome kinds and the divisor-shape split (IR-1561). They do not overclaim against FR-057-AC-9.
- Decimal rows (examined). The constructor row matches FR-402 Behavior 11 and TC-950: four theorems, 440 verified, 12/12 mutants, the `POWERS_OF_TEN` assumption, and the host test FR-402-AC-15. The operation and ordering rows match FR-403 Behavior 41 and TC-951 (630 verified, 53/53 mutants). Each is a mathematical statement and NOT equality with `evaluate_decimal`/`order_numbers`. The ordering row covers lt, le, gt and ge, which are the only four ordering operators of FR-403. FND-002 and FND-005 apply to these rows.
- Decider (examined). `decide` still skips every row whose verdict is not `NotTractable`, so a `verus_proof` on a `not_tractable` row changes no refusal. `verus_proof` is read nowhere in `src/`. The only consumers of `TractabilityRow` outside `tractability.rs` are `evidence_row` in `terminal/tractable.rs` and `generate/clause/v2.rs`, and only a `NotTractable` row can reach them, which the loader guarantees carries `wall_seconds`. No other path routes a harness through a `verus_proved` row or marks a clause Verified from one.
- Nullable `wall_seconds` (examined). Every reader in `src/`, in `tests/` (`.unwrap()`/`.expect("measured")` on Kani rows) and in `tests/it/float_scalar_harness.rs` (raw JSON, general-route float rows only) is safe. Nothing in `scripts/`, `docs/` or the sibling repos quire-driver, filament-spec-language, filament-exact and filament-specification parses the table JSON.
- Loader (examined). `inconsistency()` runs after deserialization and reports `MalformedRow` with the row index. `VerusProof` has `deny_unknown_fields`.
- Gates at head (examined): fmt and clippy `--locked --all-targets -D warnings` pass, and `make spec` passes. The test results are in the gap-analysis SR.

## Verdict

**CONDITIONAL.** The data claims are accurate for 23 of 24 proofs. The float `assumes` field understates the trusted base. The decimal `verus_proved` rows never refuse, although the kernel's record shows Kani non-return on those subjects. The free-text claims are not asserted by any test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | All 12 float rows' `assumes` says "The record states no assumption beyond the prover library's specifications". FR-405 Behavior 23 also assumes the crate-authored spec of `u128::leading_zeros` in `src/verus_std_specs.rs`, the library axiom `axiom_u64_leading_zeros`, and from `src/verus_kernel_model.rs` that `RoundingMode`, `IeeeWidth` and `IeeeFlag` are the declared enumerations and that the derived equality of `RoundingMode` compares variants. The row understates the trusted base, so the claim exceeds the proof. | docs/qualification/ir1530-general-route-scalar-table.json:452 (and the 11 other float rows) |
| FND-002 | medium | The provenance of the decimal `verus_proved` rows says "No Kani measurement exists for this row", and the rows never refuse. FR-403 Status records that the kernel's Kani harnesses for the six operations, the ordering, the constructor and the normalized view never returned at 600 s, the operation harnesses at 19 GiB or more, which is above the 16 GiB ceiling of CG FR-028. The row therefore lets a 128-bit native decimal clause reach Kani despite a recorded non-return. This is unreachable today (the V2 path accepts only integer operations, and IR-259 is in backlog), but once IR-259 lands nothing refuses it. | docs/qualification/ir1530-general-route-scalar-table.json:1473,1513,1561; src/kani/tractability.rs:441 |
| FND-003 | medium | `assert_proof` checks `proved`, `not_proved`, `assumes` and `run` only for `len() > 40`, plus a few substrings. The verified counts, the mutant counts (for example 33 of 33 changed to 34 of 34), the assumption lists (FND-001 passes) and most of the `not_proved` common set (for example dropping "the provenance") can change and every test still passes. | tests/it/kani_tractability.rs:1820-1853 |
| FND-004 | low | The loader test covers "a Kani verdict without a wall time" only for `not_tractable`. Removing `RowVerdict::Tractable` from the third arm of `inconsistency()` still passes. | tests/it/kani_tractability.rs:2137; src/kani/tractability.rs:249 |
| FND-005 | low | Clause (b) of the decimal-operations `proved` text says that otherwise "the retained coefficient is the exact result rounded in the type's mode". It drops the part of FR-403 Behavior 41 (b) that says a rounding step under `exact` is `Inexact`, not a retained rounded coefficient, so it reads wider than the proof for an `exact` type. | docs/qualification/ir1530-general-route-scalar-table.json:1540 |
| FND-006 | low | The 18 float and rational `not_tractable` rows still say "claims no proof; waits on the deductive lane, owner IR-1551" (or IR-1544). Both tickets are Done, and the same rows now carry a `verus_proof`. FR-056 (lines 63 and 66) and the edited ir260 doc line also keep IR-1551 as the owner, so the unproved remainder (the common set, Meter admission) has no live owner. | docs/qualification/ir1530-general-route-scalar-table.json:433; spec/kani/functional/FR-056-kani-tractability-refusal.md:63; docs/qualification/ir260-float-scalar-kani-measurements.md:94 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The stale-owner fix is incomplete. The tests.md FR-053 row still says "12 solver-hang rows, owner IR-1551". That text predates this PR, but it is the same claim FND-006 covers. The `owner` field of `NATIVE_FLOAT_AND_RATIONAL_ROWS` (IR-1551/IR-1544) is now dead data, and the only assertion on it, `assert!(!owner.is_empty())`, checks a constant. | spec/kani/matrix/tests.md:18; tests/it/kani_tractability.rs:1680 |
| FND-008 | low | The header of `tests/it/kani_verus_record.rs` says the text is "transcribed from FR-402 ... on filament-exact main", and TC-070 step 16 says the text is "transcribed by hand". The constants were in fact generated from the table JSON, so the test pins CG's own text against drift and does not check it independently against filament-exact. The text happens to match upstream (verified this round against be9359f), but the stated provenance is wrong. | tests/it/kani_verus_record.rs:1-4; spec/kani/matrix/TC-070-kani-tractability-refusal.md:94-95 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The 12 float `assumes` texts now list FR-405 Behavior 23's trusted base (`axiom_u64_leading_zeros`, `u128::leading_zeros` in verus_std_specs.rs, the verus_kernel_model.rs facts, the library lemmas), verified against be9359f. |
| FND-002 | fixed | The decimal provenance now records the Kani non-return (600 s, 19 GiB or more, above 16 GiB) and that a 128-bit decimal clause needs its own not_tractable row owned by IR-259. FR-056 says the same. |
| FND-003 | fixed | `assert_proof` now pins `proved`, `not_proved`, `assumes` and `run` exactly (kani_verus_record.rs). Mutants (34 of 34; `u64::BITS` dropped from the assumptions) are killed. |
| FND-004 | fixed | A tractable-without-wall case was added. The mutant that drops the Tractable arm is killed. |
| FND-005 | fixed | Clause (b) now reads "Inexact under the exact spelling (no rounded coefficient is retained)". A mutant of that text is killed. |
| FND-006 | fixed | The 18 provenances, FR-056 and the ir260 doc now say "the remainder is not scheduled" and name no Done owner. The leftover in tests.md and in the test data is FND-007. |
| FND-007 | fixed | Round 2 (6468a5bc). The tests.md FR-053 row now says "the part of them the Verus proof does not state is not scheduled", and the dead `owner` field and its constant check are removed from all 58 `NativeRow` entries. No IR-1551/IR-1544 text is left. |
| FND-008 | fixed | Round 2 (6468a5bc). The kani_verus_record.rs header and the TC-070 step 16 "Re-check procedure" now describe the file as a drift pin over CG's own text, generated from the table JSON and checked against filament-exact be9359f. |
