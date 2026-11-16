# IR-260 IEEE float scalar Kani measurements

Target `x86_64-unknown-linux-gnu`, `--solver cadical`, unwind 2 (the comparison path has no loop; 2
is above its largest loop bound of 0 plus 1). Every option is the harness's own vector, recorded in
`identity.options` and in the run evidence's `arguments`: `-Z function-contracts --harness
<module>::<proof> --exact --unwind 2 --solver cadical --output-format regular`, then the runner's
`-Z unstable-options --export-json <report>`. No tool versions are recorded in this evidence
(QSL-406, EA-49). Source: `tests/it/float_scalar_harness.rs`, one `#[ignore]` test per verdict, each
run in its own `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0` under one `flock
/tmp/cargo-slot.lock`, one job at a time, a 30 minute hard cap. Wall is the whole test process (crate
build plus proof); memory.peak is the scope cgroup's, which includes the test process and `cargo
kani`. Every job ended well inside the 10 minute probe cap.

## Corpus

A V1-draft QSL source (`filament.value.complete/v1`), functions over `Float64[nearest-even]` and
`Float32[nearest-even]` parameters whose body is `filament::value::ieee::numericEqual(a, b)`,
`totalOrder(a, b)` or `bitIdentical(a, b)`, compiled by QSL, admitted by Contract IR, lowered through
`generate_routed` by CG. QSL main at the time of the run (`f32450ae9`) admits float parameters, the operators `=` and `!=`
(lowered to `filament.op.ieee.numeric_equal`), the three relation intrinsics, `+ - * /` over one
width, and `convert` to `Rational` (its `fsl-replay` tests `compile_ieee_profile` and the fixtures
under `fsl-replay/tests/fixtures/ieee_float`). A float ordering operator lowers to three
`numeric_equal` nodes and one `total_order`, so it is not one row node. A float literal enters as a
bit-constructor value node and is not a drawn parameter: this row refuses such an operand with
`float.literal_operand`, and an expression operand with `float.operand_not_parameter`. QSL types a
bare `Float64` as `exact`-pinned, so an operand with no `float_rounding` bound is only reachable as
hand-edited Contract IR. The `ir260-float-source` branch of QSL (a commutativity case through
`convert<Rational[...]>(a + b)`) is not on QSL main, and its operation is float addition, which this
row refuses. The QSL-948 source admission the ticket comments name has therefore landed for the
comparison case run here; no synthetic IR is used anywhere in this evidence.

## Verdicts

The harness draws each operand as one `i64` bit pattern (every `i64` for `float64`, `0..=0xFFFF_FFFF`
for `float32`), builds `filament_exact::NativeFloat::binary64(bits as u64)` or `binary32(bits as u32)`,
calls `compare_native_float` under a meter that never denies and compares the Boolean with an
expected side that reads the bit patterns directly (an exponent and fraction classification for
`numeric_equal`, bit equality for `bit_identical`, the host's `total_cmp` for `total_order`).

| Case | Verdict | Wall (s) | memory.peak (MB) |
| --- | --- | --- | --- |
| numeric_equal, float64 | Verified | 3.5 | 348 |
| total_order, float64 | Verified | 3.3 | 335 |
| bit_identical, float64 | Verified | 7.1 | 753 |
| numeric_equal, float32 | Verified | 5.3 | 352 |
| total_order, float32 | Verified | 3.3 | 333 |
| bit_identical, float32 | Verified | 3.0 | 326 |
| numeric_equal float64 asked as bit_identical | Falsified (counterexample) | 18.5 | 3131 |
| bit_identical float64 asked as numeric_equal | Falsified (counterexample) | 27.1 | 4891 |
| total_order float32 asked as bit_identical | Falsified (counterexample) | 21.5 | 3766 |
| numeric_equal float32, expected side with a reflexive NaN | Falsified (counterexample) | 28.0 | 5106 |
| numeric_equal float64 asked as bit_identical, decoded and replayed through the owner route | Falsified (counterexample) | 18.8 | 3132 |

| comparison mutants c, e, f, g, h, i, j, k, l, m of FR-053-AC-8, ten expected-side replacements in one test | each Falsified (counterexample) | 236.9 (ten runs) | 5539 (the test's scope) |
| run evidence of numeric_equal float32 (target, every option, unwind, solver, limits, checks, one satisfied cover, no version key) | Verified | 6.8 | 712 |
| numeric_equal float32 with its cover forced unreachable | `CoverUnsatisfied` (0 of 1), not Verified | 3.2 | 348 |

Each mutant asserts that the replaced text occurs in the routed harness and that the replacement
changed it. The replayed counterexample is decoded to exact `u64` bit patterns and evaluated by QSL's
`replay` over `WitnessValue::Float64`; the owner's answer is numeric equality and differs from the
mutant's bit identity on that witness.

## Not tractable and refused, no harness

Every arithmetic identity of the row (`filament.op.ieee.float32|float64` `add`, `sub`, `mul`, `div`,
`sqrt`, `fma`) is a typed refusal naming its capability (`float.<op>.not_kani_tractable`) with no
harness. The measured basis is filament-exact FR-405's evidence: an arithmetic operation is a supported
row only when its three harnesses (`nearest-even`, `exact`, and the four directed spellings) are
all Verified. Only the `nearest-even` harnesses of `add` and `sub` at both widths and of `mul` at
`binary32` are Verified; every `exact` and directed harness, and every harness of `div`, `sqrt` and
`fma`, reached no verdict within the 600 s limit (18 of the 31 above the 16 GiB ceiling). The five
rows are in `ir1530-general-route-scalar-table.json` (`add`/`sub`, `mul`, `div`, `sqrt`, `fma`, both
widths, `max_operand_bits` 32).

Kani symbolic `f32` division finding. Kani's model of the host `f32` division is not bit-exact: at
the operands `0x800000A9 / 0x9C873333` it returns a value other than the hardware's and the
correctly rounded `0x1BA00000`. A host `/` is therefore not usable as the expected side of a `div`
harness, and the bracketing of the quotient in exact integers that filament-exact's `div` harnesses use
is solver-bound (no `div` verdict at either width). The finding is recorded in this document, in
the `div` row of the tractability table and in the refusal detail of
`float.div.not_kani_tractable`.

## Native-route float and rational rows (IR-260, IR-258)

The shipped scalar table (`ir1530-general-route-scalar-table.json`, FR-056) holds the kernel's measured
record as native-route rows. They are copied from filament-exact #172 (merged as 85034c5c): FR-405 and
the TC-953 evidence record for float, FR-408 and the TC-956 evidence record for rational. They are the
kernel's own harnesses, not CG-generated ones, and CG did not re-measure them. `wall_seconds` is the
wall time of the whole `cargo kani` invocation (Kani's own verification time is in each row's
`provenance`), or the limit a run was killed at; `peak_bytes` is the scope's `memory.peak`.

Float, 44 rows (one per operation, width and rounding class of TC-953): 32 are VERIFIED with their
covers reached. 12 are timeouts that hang in the solver and return no verdict, so nothing is claimed
for them. They are measured not-tractable rows owned by IR-1551 and wait on the deductive lane. Seven are at the
600 s probe, not extended: Divide, SquareRoot and FusedMultiplyAdd `NearestEven` at Binary32; Multiply,
Divide, SquareRoot and FusedMultiplyAdd `NearestEven` at Binary64. Five are at the 1800 s cap after a
600 s probe that also timed out: Divide `Exact` at both widths, Divide directed at Binary64,
SquareRoot `Exact` at Binary64 and FusedMultiplyAdd `Exact` at Binary64. The all-VERIFIED combinations
(every spelling group) are Negate, the three comparisons, Add and Subtract at both widths and Multiply
at Binary32. The rounding class is not a key of the table, so the rows of one operation and width share
a key and are told apart by their `provenance` (harness id).

Fused multiply-add. CBMC's `fma` model has two faults of its own (an infinite addend and a zero product),
so the fused harnesses decide those two regions by IEEE rule and use the model elsewhere. The faults
are recorded in TC-953 with their reproductions; no row records a kernel failure.

Rational, 14 rows (TC-956): eight CG-domain rows are VERIFIED (equality, ordering, multiply, add,
subtract, subtract with carriers from `from_i64_parts`, and the two composed add-commutativity forms),
with `max_operand_bits` 8 because the formed magnitudes are below 256. Six full-`i64` rows are TIMED OUT at
900 s with no verdict: admission from `i64` parts, equality, ordering, add, subtract and multiply
(`max_operand_bits` 64). They are number theory (two gcd algorithms against each other, a coprimality
fact) or a 128-bit product against a reference product, owned by IR-1544, and add, subtract and
multiply sit at 14.3 to 14.5 GiB, below the 16 GiB ceiling. The tier-lemma and charge-denial harnesses
of TC-956 are not operations and have no row; their record is TC-956. The admission row's id
`filament.native.rational.from_i64_parts` names the harness subject and is not a catalog operation.

IR-258 AC-11 names the full-`i64` admission (a) and the CG-domain operations (b) and the composed
predicate (c). (b) and (c) are executed VERIFIED rows above; (a) is a measured not-tractable row, so
AC-11 cannot be fully executed in Kani, and whether it closes on that is the planner's ruling.
