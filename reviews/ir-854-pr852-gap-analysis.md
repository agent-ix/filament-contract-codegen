---
id: SR-17940486709564847048042
title: "IR-854 PR #852 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #852 at 4fdb8a44, git diff origin/main...HEAD (base 6b013e26); IR-854; FR-057-AC-1, FR-057-AC-5 to FR-057-AC-9, FR-057-AC-12; tests/it/integer_division_harness.rs, src/oracle/scalar/native.rs, src/kani/generate/scalar.rs, docs/qualification/ir854-general-route-division-measurements.md, docs/qualification/ir1530-general-route-scalar-table.json (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-057
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-071
    type: reviews
---

## Summary

Ticket: IR-854. Planless gap analysis of FR-057 over the computed matrix (`quire matrix --scope
.`) and the tagged tests. Plan completion: not assessed.

Examined: AC-1 (refusal list now names `NativeIntegerCapacity`), AC-5 (native MIN/-1 and zero
divisor), AC-6 (native plan, identity, sibling byte-identity), AC-7 (native 8-bit constant
divisor Verified), AC-8 (native `div` floor mutants), AC-9 (shipped rows) and AC-12 (run
evidence). Every new test is `#[trace]`-tagged.

Reviewer Kani re-runs (24 GiB scope, swap off, `--unwind 3 --solver cadical` via the
execution module): see the code-review SR verdict and the comment on IR-854; both reproduced
the coder's verdicts.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The seven FR-057-AC-7 native tests (`native_constant_divisor_8_bit_*_verifies`) carry their `#[trace("TC-071", "FR-057-AC-7")]` inside `macro_rules! native_constant_divisor_verifies`, and the computed matrix does not see them: `quire matrix --scope .` lists only `tests/it/integer_division_harness.rs:1588` (`the_shipped_table_holds_the_kernel_native_division_rows_on_the_native_route`, the copied kernel rows) for AC-7. AC-7 is flipped to delivered on evidence the matrix cannot trace. Write the seven tests as plain functions (or one test over the seven shapes) so the trace attribute is visible. | tests/it/integer_division_harness.rs:1984-2015 |
| FND-002 | medium | The shipped 16-bit native `not_tractable` rows for `div`, `rem` and `mod` refuse at run time every native harness at 16 bits and above, including constant divisors the kernel measured `Verified` at 64 bits (TC-957). FR-057-AC-9 says "No shipped row refuses a shape measured `Verified` at that width and route". Until this PR the over-refusal was latent behind `NativeDivisionPlanUnavailable` (accepted on that basis in ir-805-pr842 gap-analysis FND-002); this PR makes CG generate native division harnesses, so it is now live. FR-057 Status records it under AC-7 as "a known limitation until a profile or divisor-shape feature exists" with no owner, and the AC-9 PARTIAL bullet does not mention it. List it under AC-9 and name an owner ticket for the divisor-shape (and profile) feature. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:381-390, docs/qualification/ir1530-general-route-scalar-table.json |
| FND-003 | medium | FR-057-AC-6's new clause "A plan the kernel refuses settles `NativeIntegerCapacity`" has no test: no test in `tests/it` names `NativeIntegerCapacity`, and every AC-6 assertion is on a supported claim. The refusal path is `select_division(..).ok_or_else(refusal)` in `select_native_integer`. Add a division claim whose plan the kernel refuses (or, if no `i64` claim can reach a kernel refusal, say so and drop the clause). | src/kani/generate/scalar.rs:593-626, tests/it/integer_division_harness.rs:1061-1130 |

## Verdict

AC-6 and the AC-7 native minimum are implemented and the verdicts reproduce. Three gaps: the AC-7
evidence is invisible to the matrix (FND-001), the native 16-bit rows now over-refuse live
harnesses against AC-9 with no owner (FND-002), and AC-6's refusal clause is untested (FND-003).
AC-8, AC-9, AC-11 and AC-12 stay honestly PARTIAL.

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | The seven AC-7 tests are plain functions; `quire matrix --scope .` now lists them for FR-057-AC-7 (lines 2075 to 2153, ignored). |
| FND-002 | fixed | FR-057 Status lists the over-refusal under AC-9 as an open gap owned by IR-1561 (Backlog, acceptance items match); the matrix row names IR-1561. The run-time accuracy of that text is spec-review FND-003. |
| FND-003 | fixed | `native_division_plan_the_kernel_refuses_settles_native_integer_capacity` (src/kani/generate/scalar.rs) settles `div`, `rem` and `mod` with a reversed divisor interval as `NativeIntegerCapacity` with no selection and no oracle source. |
