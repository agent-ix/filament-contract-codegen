---
id: SR-17940486709564847048023
title: "IR-260 IR-854 PR #846 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #846 at 9e98185c, git diff origin/main...HEAD (base ebe100f7); IR-260, IR-854; docs/qualification/ir1530-general-route-scalar-table.json, tests/it/kani_tractability.rs, tests/it/float_scalar_harness.rs, src/kani/tractability.rs (context), src/kani/terminal/tractable.rs (context), src/kani/generate/clause/v2.rs (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-056
    type: reviews
---

## Summary

Ticket: IR-260 and IR-854. Code-review with the rust-review lane folded in. The PR adds 58
`route: native` rows to the shipped scalar table (16 to 74), one new host test, and narrows one
existing float test's filter to the general route. No `src/` change; `Cargo.lock` and
`Cargo.toml` unchanged; no pins or compat layers; `#[trace("TC-070", "FR-056-AC-15")]` on the new
test, matching the precedent `tc_070_the_embedded_general_route_table_is_the_docs_file`.

Data: all 44 float rows scripted against TC-953 at filament-exact 85034c5c (op id, width,
verdict, wall or limit, `memory.peak`, harness id, rounding class, probe/extension text, owner
IR-1551): 0 mismatches. All 14 rational rows checked against TC-956: 0 mismatches. New rows carry
no tool versions (the fields are optional and never select a row) and stamp CG's current
generator and kernel versions, as the existing native division rows do.

Concern (1), the id `filament.native.rational.from_i64_parts`: the decider matches only by
string equality of `operation_ids` against a clause's operation ids, and the loader does no
catalog validation, so a non-catalog id loads and can never match a clause; it is inert. It
contradicts the row field's doc ("The catalog operation ids the probe exercised"), recorded as
FND-002.

Concern (2), duplicate keys: the decider ignores `tractable` rows and picks the qualifying
`not_tractable` row with the smallest `max_operand_bits`, then file order. It is never
first-wins over a tractable row. Probed on the shipped table (scratch test, native route): float32
`div` with `bits: None` returns `Run`; with `bits: Some(32)` it refuses, citing a hang row, so
the VERIFIED directed class would also be refused, as FR-056 documents. float32 `add` at 32 runs;
rational `add` at 8 runs and at 64 refuses; `le` at 64 refuses (the order row lists `lt` and
`le`), `gt` at 64 runs; integer `add` at 64 runs; the general route is unaffected. Today it is
unreachable: float operations reach the V2 decider with no width, the operator check admits only
integer operations before the decider, and the run-level decider only sees exact-scalar integer
harnesses. Safe now; the documented over-refusal is latent. No finding.

Gate at 9e98185c: `cargo fmt --check` clean; `cargo clippy --locked --all-targets -- -D
warnings` clean; `cargo deny check` ok; `make spec` exit 0 (warnings only, none in changed
text); `cargo test --locked --no-fail-fast --lib --test it`: lib 606 passed / 0 failed / 36
ignored, it 1145 passed / 0 failed / 125 ignored (matches the coder).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | `tc_070_the_shipped_table_holds_the_kernel_native_float_and_rational_rows` pins only counts by verdict, the 600 s/1800 s/900 s limits and provenance substrings, so a mis-copied row passes. Measured on a mutated copy of the table, all four mutants survive in one run: a float row's `max_operand_bits` 32 to 64 (row 17), a float row's op id `float32.div` to `float64.div` (row 26), a `peak_bytes` set to 1 (row 39), and a tractable `wall_seconds` set to 1.0 (row 40). A verdict flip (row 17 to `not_tractable`) is killed. Pin each row's (operation id, width, verdict, wall, peak, harness) against a per-row list, or at least assert that `max_operand_bits` equals the width in the op id and that the harness id in `provenance` names the same operation and width. | tests/it/kani_tractability.rs:1086-1159, docs/qualification/ir1530-general-route-scalar-table.json |
| FND-002 | low | The row `filament.native.rational.from_i64_parts` puts a non-catalog id in `operation_ids`, whose field doc says "The catalog operation ids the probe exercised", and FR-056 calls the field the operation ids a clause is matched on. It is inert (no clause can carry it), and FR-056 and the provenance say it is not a catalog operation, but the field contract now has an exception that the type and loader do not record. Either amend the field doc to allow a non-catalog subject id for record rows, or keep the admission row out of the table and leave its record in TC-956 with the tier and charge-denial harnesses. | src/kani/tractability.rs:163-164, docs/qualification/ir1530-general-route-scalar-table.json |

## Verdict

The data is right: every added row matches the kernel's record and none claims a proof a timeout
denies. The decider handles the new rows safely today (concerns 1 and 2 above). The
`float_scalar_harness` filter keeps that test on its original five general-route rows (without
it the count would be 49), so it is not weakened. FND-001 is the one real gap: the new test
cannot catch a mis-copied width, op id, wall or peak. FND-002 is a contract nit.

## New findings (disposition pass 1)

Re-review of 9e98185c..1c768610 (9ca8d099, 1c768610). New code: the CG host differential
`the_native_arithmetic_equals_the_owner_and_the_differential_kills_native_mutants`. Independence
checked: the native side is filament-exact `evaluate_native_float` (`native_float.rs`, its own
`u128`/`i32` arithmetic; it imports only types from `ieee`), the owner side is `evaluate_ieee`
(`ieee.rs`), so the differential is not tautological. A no-op mutant (identity) returns `None`.
Each of the four committed mutants makes the test's `is_some()` pass, but two are killed for a
reason other than the one claimed (scratch probe printing `first_arithmetic_disagreement`).

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-003 | medium | The `tie-is-away` mutant rewrites the rounding provenance to `NearestEven` for every mode, not only for nearest-even, so it is killed at the first grid point by provenance (`Add Binary32 Exact 0x0 0x0: native (0, {}, NearestEven), owner (0, {}, Exact)`), not by a tie. The test therefore does not show that the differential catches a wrong tie rule, though FR-053 says it does. A correct mutant (map NearestEven to NearestAway, keep each mode's own provenance) IS killed by real ties in the grid (scratch probe: Add B32 `0x400000 + 0xc00001` gives 0x1000001 against 0x1000000; also Add B64, Sub B32, Mul B32). Fix the closure: relabel only when the requested mode was `NearestEven`, or pass `rounding` through. | tests/it/float_scalar_harness.rs:1455-1463, spec/kani/functional/FR-053-float-scalar-harnesses.md:492-493 |
| FND-004 | low | The `inexact-dropped` mutant clears every flag (`IeeeFlags::default()`), so it is killed by `invalid` on `Multiply Binary32 Exact 0 * inf`, not by a missing `inexact`. A real inexact-only mutant (drop only `Inexact`) is also killed (scratch: `Multiply Binary32 TowardZero 0x1 * 0x1`, flags 8 against 24), so the claim holds in substance, but the committed mutant does not test it. Drop only `IeeeFlag::Inexact` (for example `flags.iter().filter(|f| *f != IeeeFlag::Inexact).collect()`), or rename it "flags-dropped". | tests/it/float_scalar_harness.rs:1464-1467 |

## Dispositions

Round 1, checked at 1c768610. Gate: `cargo fmt --check` clean; `cargo clippy --locked
--all-targets -- -D warnings` clean; `cargo deny check` ok; `make spec` exit 0; `cargo test
--locked --no-fail-fast --lib --test it`: lib 606 passed / 0 failed / 36 ignored, it 1146 passed /
0 failed / 125 ignored. `Cargo.lock`/`Cargo.toml` unchanged against origin/main.

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | `tc_070_the_shipped_table_holds_the_kernel_native_float_and_rational_rows` now compares all 58 rows to a literal `NATIVE_FLOAT_AND_RATIONAL_ROWS` list (ids, bits, verdict, wall ms, peak, harness, owner). Run separately on a mutated copy of the table, each mutant fails: row 17 width 64 (`left: 64, right: 32`), row 26 op id `float64.div` (left/right ids differ), row 39 peak 1 (`Some(1)` against `Some(946696192)`), row 40 wall 1.0 (`1000` against `197700`). |
| FND-002 | fixed | The `operation_ids` field doc and FR-056's Measured table text now allow a record row to name a non-catalog subject and say no clause carries that id, so it never selects. |

Round 2, checked at ec373dc1 (`git diff 1c768610..ec373dc1`). Gate: `cargo fmt --check` clean;
`cargo clippy --locked --all-targets -- -D warnings` clean; `cargo deny check` ok; `make spec`
exit 0; `cargo test --locked --no-fail-fast --lib --test it`: lib 606 passed / 0 failed / 36
ignored, it 1146 passed / 0 failed / 125 ignored. Scratch runs on a copy of the test, printing
each mutant's first disagreement, then putting back each round-0 mutant to check that the new
reason assertions can fail.

| FND | outcome | reason |
|---|---|---|
| FND-003 | fixed | `tie-is-away` now relabels the provenance only when nearest-even was requested, and is run on Add B32, Add B64, Sub B32 and Mul B32. Each first disagreement is a real NearestEven tie: Add B32 `0x400000 + 0xc00001` (0x1000001 against 0x1000000), Add B64 `0x8000000000000 + 0x18000000000001`, Sub B32 `0x400000 - 0x80c00001`, Mul B32 `0x3fffff * 0x3fc00000` (6291455 against 6291454). The test now asserts `d.rounding == NearestEven` and that the owner's nearest-even and nearest-away bits differ. With the round-0 closure put back, the test fails at that assertion (`left: Exact, right: NearestEven`, Add B32 Exact 0+0), so the reason check can fail. |
| FND-004 | fixed | `inexact-dropped` now removes only `IeeeFlag::Inexact`. Its first disagreement is Mul B32 TowardZero `0x1 * 0x1` (native flags 8, owner 24), and the test asserts the owner's flags contain `Inexact`. With the round-0 all-flags mutant put back, the test fails at that assertion (`float_scalar_harness.rs:1568`; the first disagreement moves to the `invalid`-only Exact `0 * inf`). The other two mutants are killed for their stated reason too: `nan-payload-cleared` on Sub B64 Exact `0 - 0x7ff0000000000001` (owner keeps payload bit 0x...001), and `subtract-as-add` on Sub B32 Exact `0 - 0x1` (owner 0x80000001 against 0x1). |
