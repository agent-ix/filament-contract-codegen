# IR-854 general-route division measurements

Measurement record for FR-057-AC-7 and FR-057-AC-9 (TC-071 steps 7 and 9). It records every
shape that was run, and names the shapes that were not run. A shape that is not listed here
carries no claim.

## Method

- Target: the generated scalar crate of one routed `div`, `rem` or `mod` claim, with the general
  `rt::Integer` carrier (`ScalarIntegerRepresentation::General`), built from FSL source admitted
  through Contract IR (`tests/it/integer_division_harness.rs`, `Shape::generation`).
- Backend arguments, in order: `-Z function-contracts --harness
  <module>::<proof> --exact --unwind 10 --solver cadical --output-format regular`, with the
  generated crate's own `[package.metadata.kani]` flags (`--max-field-sensitivity-array-size 1024`).
- Limits: one job at a time under `flock /tmp/cargo-slot.lock systemd-run --user --scope -p
  MemoryMax=24G -p MemorySwapMax=0`, wall-clock ceiling 600 s, peak read from the scope's cgroup
  `memory.peak`; both the wall time and the peak include the cargo build of the crate.
- Unwind 10 for every run. A larger unwind only enlarges the formula, so it cannot turn a shape
  that did not finish into one that does.
- Generator version 0.1.0, kernel (filament-exact) revision
  b81e595046c96739ef738a390dbe4b536db26e49, host titan, 2026-11-14. Tool versions are not
  recorded (QSL-406, EA-49).
- Each harness is the healthy generated harness: two `i64` operands over the operand ranges,
  the oracle call, the one assertion and the one completion cover.

## Results

| Operation | Law | Shape and operand ranges | Route | Unwind | Outcome | Wall (s) | Peak (bytes) |
|---|---|---|---|---|---|---|---|
| `filament.op.integer.div` | floor | constant divisor `Int[8, 8]`, dividend `Int[-128, 127]` (8 bits) | general | 10 | TimedOut | 600.0 | 8502534144 |
| `filament.op.integer.div` | truncating | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8541622272 |
| `filament.op.integer.div` | floor | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8579940352 |
| `filament.op.integer.rem` | truncating | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8522305536 |
| `filament.op.integer.mod` | Euclidean | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8538312704 |
| `filament.op.integer.rem` | truncating | constant divisor `Int[8, 8]`, dividend `Int[-128, 127]` (8 bits) | general | 10 | TimedOut | 600.0 | 8525271040 |
| `filament.op.integer.mod` | Euclidean | constant divisor `Int[8, 8]`, dividend `Int[-128, 127]` (8 bits) | general | 10 | TimedOut | 600.0 | 8550948864 |
| `filament.op.integer.div` | truncating | constant divisor `Int[8, 8]`, dividend `Int[-128, 127]` (8 bits) | general | 10 | TimedOut | 600.0 | 8563646464 |
| `filament.op.integer.div` | Euclidean | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8566710272 |
| `filament.op.integer.rem` | floor | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8541540352 |
| `filament.op.integer.rem` | Euclidean | symbolic divisor, both operands `Int[-8, 7]` (4 bits, zero and both signs) | general | 10 | TimedOut | 600.0 | 8547880960 |
| `filament.op.integer.div` | floor | constant divisor `Int[-8, -8]`, dividend `Int[-128, 127]` (8 bits) | general | 10 | TimedOut | 600.0 | 8573677568 |
| `filament.op.integer.div` | floor | mutant: oracle completes at a zero divisor; dividend `Int[-9, 9]`, divisor `Int[-4, 4]`, explicit result `Int[-3, 3]` (5 bits) | general | 10 | TimedOut | 600.0 | 8556896256 |

`TimedOut` is `Inconclusive { reason: TimedOut }`: no verdict, no proof credit. No shape finished,
so no harness classified `Verified` and the one mutant tried classified neither `Falsified` nor
anything with a playback.

## Shapes not run

The shape list of FR-057 (Measured widths and refusal) is larger than this record. Not run, and so
carrying no claim:

- Constant divisor `Int[8, 8]`: `div` Euclidean, `rem` floor and Euclidean. Constant divisor
  `Int[-8, -8]`: `div` truncating and Euclidean, `rem` and `mod`. Constant divisors at 16, 32 and 63
  bits.
- Symbolic divisor at 8, 16, 32 and 63 bits (the 4-bit shape did not finish, so wider symbolic
  shapes were not tried).
- The other AC-8 mutants, on the general route (the first did not finish).
- Every shape on the native route (FR-057-AC-6: the native route refuses until IR-1541).

## Reproducers

Three of the rows above have an ignored reproducer in `tests/it/integer_division_harness.rs`
(`general_route_constant_divisor_8_bit_floor_div_times_out`,
`general_route_symbolic_divisor_4_bit_floor_div_times_out`,
`general_route_zero_completing_mutant_times_out`). Each runs for the full 600 s at about 8.5 GB, so
they are not in the `make kani` / `make kani-gate` filter list; run one by name with
`cargo test --test it -- --ignored <name>` under the lane lock.

## Table rows

The shipped scalar table (`docs/qualification/ir1530-general-route-scalar-table.json`) gains one
`not_tractable` row per operation (`filament.op.integer.div`, `rem`, `mod`), route `general`,
`max_operand_bits` 4, `wall_seconds` 600 and the largest peak measured for that operation's 4-bit
shapes. 4 bits is the smallest listed width that did not finish. The constant-divisor shape at 8
bits also did not finish, so no general-route shape was measured `Verified` and the rows show no
constant/symbolic split: no `divisor_constant` extension is added for these general-route rows and none refuses a shape
measured `Verified`. The native 16-bit rows are a different matter: they refuse constant divisors the
kernel verified (the open gap under FR-057-AC-9). Each row applies to every law of its operation; the 4-bit symbolic shape was measured under each
law.

## Native-route disposition of FR-057-AC-7 (IR-854)

Rule (planner): each native shape ends as an executed native verdict, or as a recorded measured
not-tractable row. Input: the proofs-2 analysis of the kernel's record (filament-exact TC-957, #171).

- Executed native verdicts: the shapes the kernel's record lists as VERIFIED, including the symbolic
  8-bit width for `div`, `rem` and `mod` under every profile and member. They are the `tractable`
  native rows already in the shipped table.
- Measured not-tractable: the 20 timeout rows. proofs-2 found each is one solver query that never
  returns (a divider against an independent multiply, about a million variables, under 2.3 GiB),
  independent of width and of a constant divisor; the recorded rows are right and a rerun changes
  nothing. They are the `not_tractable` native rows at 16 bits.
- Not a row: proofs-2 probed a split of the division plan at the truncating pair (five harnesses over
  all `i128`, each verified in under 15 s). It is an unlanded scratch refactor with no committed
  harness, so no row and no claim.

## Native route, CG's own generated harnesses

CG routes `select_native_integer` through the kernel's `Divide` and `Modulo` plan, so its own
generated native harnesses exist. Each run is `cargo test --test it -- --ignored --exact` of the named
ignored test, one job at a time under the 24 GiB scope (memory swap off), `--unwind 3 --solver cadical`
through the execution module, wall seconds as recorded by the run and memory peak as the scope's cgroup
`memory.peak` (it includes the test process, so it is an upper bound for the prover).

| Shape | Operation, law | Verdict | Wall s | Scope peak bytes |
| --- | --- | --- | --- | --- |
| dividend `[-128, 127]`, divisor `8` | div, floor | Verified | 22.5 | 1902702592 |
| same | div, truncating | Verified | 18.8 | 1492271104 |
| same | div, Euclidean | Verified | 18.8 | 1465257984 |
| same | rem, truncating | Verified | 17.7 | 1368702976 |
| same | rem, floor | Verified | 17.3 | 1352966144 |
| same | rem, Euclidean | Verified | 17.2 | 1349300224 |
| same | mod | Verified | 17.0 | 1348071424 |

Mutant fixture: dividend `[-8, 7]`, divisor `[-3, 3]`, explicit result `[-3, 3]`, native `div` floor. The
healthy harness is `Verified` (20.0 s). Each seeded mutant is `Falsified` with solver playback of
in-domain operands: the embedded law swapped to truncating (79.5 s, playback `-3, 2`), to Euclidean
(78.0 s, `7, -3`), the oracle completing at a zero divisor (78.4 s, `-8, 0`) and the expected side
without its zero arm (74.1 s, `2, 0`). The four mutants and the healthy run together peaked at
7228067840 bytes in the scope. AC-7's native minimum (the 8-bit constant divisor, every operation and
law) is met by CG's own harnesses.

The general-route rows above are unchanged: the same 8-bit constant divisor did not finish there.
