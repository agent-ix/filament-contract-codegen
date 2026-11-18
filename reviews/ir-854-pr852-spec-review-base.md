---
id: SR-17940486709564847048043
title: "IR-854 PR #852 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #852 at 4fdb8a44, git diff origin/main...HEAD (base 6b013e26); IR-854; spec/kani/functional/FR-057-integer-division-scalar-harnesses.md, spec/kani/matrix/TC-071-integer-division-scalar-harnesses.md, spec/kani/matrix/tests.md, docs/qualification/ir854-general-route-division-measurements.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-057
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-071
    type: reviews
---

## Summary

Ticket: IR-854. Base spec review of the FR-057 delta (native-route Behavior paragraph, AC-1, AC-6,
AC-7 and Status), TC-071 steps 6, 7 and 8, the FR-057 matrix row and the native section of the
measurement record.

Examined clean: the native Behavior paragraph (plan from the full operand intervals and result
range, `evaluate_native_integer_arithmetic`, profile identity in the selection) matches the code;
AC-1's refusal list (`NativeIntegerCapacity` replaces `NativeDivisionPlanUnavailable`); AC-6's
rewrite; AC-7's text with the IR-1541 conditional removed; `rem` on the native route is consistent
with FR-014-AC-81 (implemented) and FR-057-AC-1, and does not widen FR-014. The FR-057 row's
PARTIAL marker for the requirement as a whole is consistent, since AC-8, AC-9, AC-11 and AC-12
remain PARTIAL. The measurement record carries no tool versions. `make spec` exits 0.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-057 Status contradicts itself on AC-7: the heading says "AC-7 and AC-6 are delivered" and the delivered paragraph lists "the executed native verdicts (AC-7)", and the matrix row says "AC-1 to AC-7 and AC-10 are tested", but the "Partly delivered (PARTIAL)" list still opens with an AC-7 bullet. That bullet's content (the 16-bit native rows over-refusing constant divisors) is an AC-9 matter ("No shipped row refuses a shape measured `Verified`"). Move it under AC-9 and drop AC-7 from the PARTIAL list, or keep AC-7 PARTIAL everywhere. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:347-348, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:381-390, spec/kani/matrix/tests.md:13 |
| FND-002 | low | The FR-057 matrix row says the native `div` floor mutants "for the wrong profile and the missing zero guard" classify `Falsified`. AC-8's wrong-profile group also includes the expected side's window table rendered for the wrong law, and the `div` quotient/remainder member swap is rendered too; neither ran under Kani (`wanted` lists only the two law swaps, `completes-at-zero` and `expected-zero-arm-removed`). Say "the two embedded-law swaps" as FR-057 Status does. | spec/kani/matrix/tests.md:13, tests/it/integer_division_harness.rs:2062-2077 |

## Verdict

The spec delta is accurate to the code and honest about what stays PARTIAL, apart from the AC-7
delivered-and-partial contradiction (FND-001) and one loose phrase in the matrix row (FND-002).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-003 | low | FR-057 Status says, in present tense, that the AC-9 "decided run wrappers refuse at their width", and that the 16-bit native rows "refuse every native harness at 16 bits and above", so a constant-divisor harness "is refused at run time". At dc8d911d every shipped row is `kernel_version` 1 and `KERNEL_VERSION` is 2. The decider matches a row only when `kernel_version` is equal (src/kani/tractability.rs:259), so no row refuses anything, as AC-9's own "Known red (IR-1566, IR-1568)" preface says. The IR-1561 gap is latent until IR-1568 re-measures the rows under kernel 2, then live. Say that, and record IR-1561's dependency on IR-1568. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:384-401, src/kani/tractability.rs:259 |
| FND-004 | low | FR-057 Status says "AC-6 and AC-7 are delivered", but the FR-057-AC-7 row opens with "Known red (IR-1566, IR-1568)", and its tagged `the_shipped_table_holds_the_kernel_native_division_rows_on_the_native_route` is one of the 42 known reds. The native verdicts do not depend on the rows, but the Status does not say that one AC-7 test is red. Add that line, or keep AC-7 PARTIAL until IR-1568. This depends on code-review FND-006, which leaves AC-7's own native Kani tests failing at head. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:347-348, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:317 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | The AC-7 bullet is gone from "Partly delivered"; the over-refusal now sits under AC-9. |
| FND-002 | fixed | The matrix row now reads "the two embedded-law swaps, the oracle completing at a zero divisor and the expected side without its zero arm". |
| FND-003 | fixed | The AC-9 bullet now says every row is kernel_version 1 against KERNEL_VERSION 2, so the decider matches no row and refuses nothing until IR-1568. The open gap is latent until IR-1568, then live, and IR-1561 depends on IR-1568. |
| FND-004 | fixed | The Status heading now notes that the AC-7 row is Known red (IR-1566, IR-1568), that the tagged table test is a known red, and that the seven native Kani runs pass. |
