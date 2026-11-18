---
id: TC-070
title: "Verify measured-table tractability refusal and decider corpus"
type: TC
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-056
    type: verifies
---
# TC-070: Verify measured-table tractability refusal and decider corpus

## Description

Verify [FR-056](../functional/FR-056-kani-tractability-refusal.md) through admitted V2 clause
packages, public CG generation, a small test table and corpus, and, for the last step only,
installed Kani through CG's runner. This case is planned. Every mutant fails at its named step.

## Test Procedure

1. Implement the table decider and a trivial always-`Run` decider against the one interface; assert
   both compile behind it and the interface parameter is the feature value alone, by inspection of
   the signature (FR-056-AC-1).
2. Admit `a * b + c` at three widths and compare the exposed per-operation widths with those the
   oracle rendering computes. Mutate to the declared width; the multiply-then-add case must fail
   (FR-056-AC-2). Negotiate the same clause on each route with a recording decider and assert the
   features it receives carry that route: `general` for an `IntegerRoute::General` negotiation and
   `native` for `IntegerRoute::Native`; a decider fed a fixed route fails.
3. Load a valid table, an empty table and tables with an unknown verdict, a missing field and a
   fractional width, a missing `route` and an unknown `route`; assert the load results and the failing row index. Add a row for a fresh
   operation id and assert the decision changes with no source change (FR-056-AC-3).
4. With a `not_tractable` row for `filament.op.integer.mul` at 21 bits, request a clause multiplying
   at width 21 and assert `V2ClauseNotTractable` with operation id, width, the route, the row, the versions and
   no harness, beside a supported sibling that keeps its harness (FR-056-AC-4).
5. Probe `Int[0, 2^32]` (33 bits), `Int[0, 2^64 - 1]` (64 bits) and `Int[0, 2^64]` (65 bits) against a
   row at 64 bits on the same route (the 33-bit operation runs, the 64- and 65-bit operations are refused), a clause
   holding only another operation, and a clause holding both (FR-056-AC-5).
6. Request a clause no row qualifies and diff its harness and identity record against the
   empty-table output; byte-identical required. Add a `tractable` row at the same width and repeat
   (FR-056-AC-6).
7. Repeat step 4 with the row's generator version, then its kernel version, changed: the clause
   runs. Change only the Kani, CBMC or solver version: still refused (FR-056-AC-7).
7a. With a `not_tractable` general-route row for `filament.op.integer.mul` at 21 bits, request the
   same multiply at 21 bits on `IntegerRoute::General` (refused) and on `IntegerRoute::Native`
   (the decider returns `Run`; the item is not refused as `V2ClauseNotTractable`, though the multiply
   fixtures end `RenderFailed` on native, which has no two-parameter precondition); swap the row to `native` and assert the reverse; add a `tractable` row on either route and
   assert no refusal. Mutant: matching without the route refuses the native clause (FR-056-AC-14).
8. Add two qualifying rows in both file orders and assert the reported one is unchanged
   (FR-056-AC-8).
9. With a fake backend that exceeds the wall-clock ceiling on a clause the decider runs, assert the
   `KaniRunOutcome::Inconclusive` timed-out reason of FR-028-AC-2 and no tractability refusal (FR-056-AC-9).
10. Inspect the corpus file for the fields including `route` on every row, the seed families, the
    native-route measurement families of FR-056-AC-10 (add, multiply and multiply-then-add chain as Boolean-result postconditions over one operand; the two-parameter native shape cannot render; reported as present or missing, not assumed)
    and a measurement source on every row (FR-056-AC-10).
11. Run the scoring test. Assert it prints false-refuse and false-accept, passes at the recorded
    baseline, and fails for an always-`Refuse` decider and an always-`Run` decider
    (FR-056-AC-11).
12. Append one corpus row and assert a count changes with no source change; score the
    always-`Run` decider from step 1 against the same file (FR-056-AC-12).
13. Execute one clause the table lets through via CG's runner under installed Kani, one at a time,
    and record verdict, wall time, Kani version and generator/kernel version. Request integer
    multiply at or above the measured-bad row's bound and assert it is refused before any Kani
    launch, and a narrower multiply is not; the
    Decimal multiply row is used only once IR-1444 lets it reach the decider (FR-056-AC-13).

14. Load the embedded general-route table (assert it equals
    `docs/qualification/ir1530-general-route-scalar-table.json`), build the general-route routed
    `x + 1` scalar harness and submit it to `execute_kani_obligation_decided` and
    `execute_kani_obligations_decided` with a launcher and crate that do not exist; assert
    `NotTractable` carrying `V2ClauseNotTractable` with node, operation id, width 4, route `general`
    and the row with its versions, so no launch happened. Add a mixed batch (native first, general
    second) and assert it is refused whole. Submit the native-route harness and assert it is not
    refused (the row is general-route); assert general-route add features at 2, 3 and 4 bits are
    refused (the add row starts at 2 bits) and at 1 bit run, sub features at 4 bits are refused and
    at 3 bits run (the sub row starts at 4 bits), and native-route features at the same widths run
    (FR-056-AC-15).

15. Load the shipped table and assert it holds 77 rows, of which the native-route float rows are 44
    (32 `tractable`, 12 `not_tractable`: seven at 600 s and five at 1800 s) and the native-route
    rational rows 14 (8 `tractable` at 8 bits, 6 `not_tractable` at 64 bits and 900 s); assert each
    row names the filament-exact record, the harness and, when `not_tractable`, the measuring
    record (FR-056-AC-15).

16. Assert the `verus_proof` field sits on exactly 24 rows: the twelve float, six full-`i64` rational
    and three native `div`, `rem` and `mod` rows, each keeping its `not_tractable` verdict, and the
    three decimal rows, each `verus_proved` with null `wall_seconds` and `peak_bytes`; assert each
    proof's filament-exact record ids, merged changes and theorem names equal the kernel's record
    (TC-953, TC-956, TC-957, TC-950, TC-951), that the float rows keep the common unproved set, the
    `Exact` rows the strict refusal, the equality row its Kani completion, the decimal rows the
    `POWERS_OF_TEN` assumption and the statement that the proof is not equality with `evaluate_decimal`
    and `order_numbers`; assert a `verus_proved` row never refuses; and assert the loader rejects a
    `verus_proved` row without a proof or with a Kani measurement, a Kani verdict without a wall time,
    and a proof with an unknown field (FR-056-AC-16 for the
    rows and their text, FR-056-AC-17 for the loader and the never-refuse rule).

    Re-transcription procedure. The expected `proved`, `not_proved`, `assumes` and `run` text, the
    record ids and the theorem names are transcribed by hand into the test
    (`tests/it/kani_verus_record.rs`, `tests/it/kani_tractability.rs`) from filament-exact `main`: the
    Status and the Verus behaviors of FR-402 (11), FR-403 (41), FR-405 (23), FR-408 (18a) and FR-409
    (24), and the Verus record of TC-950, TC-951, TC-953, TC-956 and TC-957. Nothing reads the upstream
    record, so redo the transcription, update the rows and the test together, and bump the merged
    change ids whenever one of those records changes (a new proof change, a renamed theorem, a new
    mutant count). A failing test names the upstream records to compare. The deferral text in
    filament-exact's FR-402, FR-403, FR-405 and FR-409 Status is dropped by a second, spec-only
    filament-exact change under IR-1562 after #856 merges; IR-1562 stays open until then.

## Expected Results

Steps 1 to 12 (including 7a) and 14 pass on the host with no Kani. Step 13 records one executed verdict and one
pre-launch refusal. Every refusal keeps its own reason and no harness; a clause with no bad row is
unchanged.

## Status

Steps 1 to 9, 11 and 12 are implemented and tagged (IR-1526) and the route additions (steps 2 to 4, 7a and 10) are implemented (IR-1530); step 14 is implemented and tagged (IR-1530); steps 15 and 16 (FR-056-AC-15, AC-16 and AC-17) are implemented and tagged (IR-260, IR-258, IR-854, IR-1562); step 10 is partial (see FR-056 Status). Step 13 is coded as a kani-lane test and was executed under installed Kani 0.68.0 (Verified, 235.5 s); the multiply refusal half runs as a host test until an installed-Kani run covers it.
