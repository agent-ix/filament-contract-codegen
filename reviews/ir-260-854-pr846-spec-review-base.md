---
id: SR-17940486709564847048025
title: "IR-260 IR-854 PR #846 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #846 at 9e98185c, git diff origin/main...HEAD (base ebe100f7); IR-260, IR-854; spec/kani/functional/FR-053-float-scalar-harnesses.md, spec/kani/functional/FR-056-kani-tractability-refusal.md, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md, spec/kani/matrix/TC-070-kani-tractability-refusal.md, spec/kani/matrix/tests.md, docs/qualification/ir260-float-scalar-kani-measurements.md, docs/qualification/ir854-general-route-division-measurements.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-053
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-056
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-057
    type: reviews
---

## Summary

Ticket: IR-260 (table, float, rational) and IR-854 (FR-057-AC-7). The spec edits are status prose
and one new TC-070 step (15). No requirement statement or acceptance criterion changed. Checked
against the source of truth, filament-exact origin/main 85034c5c (merged #172): TC-953 evidence
table, TC-956 evidence table, TC-957 (division wall semantic), proofs-2 `NOTES.md` (2026-11-14),
and the CG lock (`Cargo.lock` pins filament-exact f1ef4963, unchanged by this PR).

Clean: the float and rational numbers quoted in FR-053, FR-056, TC-070 step 15 and the IR-260 doc
(44 = 32 + 12, seven at 600 s probe not extended, five at 1800 s after a timed-out 600 s probe;
14 = 8 CG-domain at 8 bits + 6 full-`i64` at 900 s, 14.3 to 14.5 GiB for add/sub/mul) all match
TC-953 and TC-956. The 42/20 division split matches the pre-existing FR-057 measured line and
proofs-2 (20 solver-hang rows, "the recorded rows are right"); the lemma split is correctly
claimed as nothing. The `wall_seconds` semantic (whole `cargo kani` wall, or kill limit) matches
TC-957's "Time is wall-clock seconds" used by the existing native division rows. No tool version
numbers added.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-053 and the FR-053 matrix row give the wrong blocker for AC-13's arithmetic half: "needs the dependency lock moved to a revision that has the arithmetic natives" / "stays PARTIAL until the lock moves". CG's locked filament-exact f1ef4963 already has the arithmetic natives (`evaluate_native_float` over `NativeFloatOperation::{Negate, Add, Subtract, Multiply, Divide, SquareRoot, FusedMultiplyAdd}`, landed in #159 b81e595), and `git diff f1ef4963 85034c5c -- src` touches only `native_float/kani_proofs.rs`, `native_fraction.rs` and the rational proofs/reference. The host differential over native Add/Subtract/Multiply can be written against the current lock; the residual is CG work owned by IR-260, not a lock move. PARTIAL is right; the stated reason is not. | spec/kani/functional/FR-053-float-scalar-harnesses.md:482-492, spec/kani/matrix/tests.md:18 |
| FND-002 | medium | The FR-053 matrix row is headed `✅ Implemented` while its own new text says the arithmetic half of AC-13 "stays PARTIAL" and the refusal ruling's revisit condition is met. A criterion with an open half under an open ticket cannot sit under Implemented; the header should be PARTIAL (owner IR-260). | spec/kani/matrix/tests.md:18 |
| FND-003 | medium | The FR-056 matrix row this PR edited still opens "🚧 PLANNED (IR-1526): no tractability decider, table or corpus ... TC-070 controls have not run", then appends that "the shipped table holds 74 rows ... checked by TC-070 step 15". FR-056 Status says PARTIAL with AC-1 to AC-9, AC-11, AC-12 and AC-15 implemented and host-tested. The edited row contradicts itself and FR-056. | spec/kani/matrix/tests.md:14, spec/kani/functional/FR-056-kani-tractability-refusal.md:185-200 |
| FND-004 | medium | The new FR-057-AC-7 disposition names "IR-1541 for the plan" as the residual owner. IR-1541 is Done and its native division plan (`native_integer/division_proofs.rs`, `NativeIntegerPlan` divide/modulo) is in CG's locked f1ef4963 (#171). What keeps AC-7 PARTIAL is CG's own `select_native_integer` still returning `NativeDivisionPlanUnavailable` (its comment "No kernel-checked native division plan exists yet" is stale), so the residual has no live owner. PARTIAL is honest; the owner is not. Name a CG ticket (IR-854 or a new one) for wiring the plan. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:427-431, docs/qualification/ir854-general-route-division-measurements.md:96-99, src/kani/generate/scalar.rs:588-595 |

## Verdict

The measured record is copied faithfully and the AC-7 and AC-13 PARTIAL verdicts are honest in
substance. All four findings are status-prose accuracy: two misattributed blockers or owners
(FND-001, FND-004) and two self-contradicting matrix rows (FND-002, FND-003). None changes a
number or a claim of verification. Fixable in this PR as text edits.
