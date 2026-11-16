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
