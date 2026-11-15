---
id: SR-17940486709564847047995
title: "IR-854 PR #836 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/quire-contract-codegen PR #836 at 89a7cc86, git diff origin/main...HEAD; IR-854; spec/kani/functional/FR-057-integer-division-scalar-harnesses.md, spec/kani/matrix/TC-071-integer-division-scalar-harnesses.md, spec/kani/functional/FR-015-bounded-kani-obligations.md, spec/kani/functional/FR-056-kani-tractability-refusal.md, spec/core/functional/FR-038-typed-operator-catalog.md, spec/oracle/functional/FR-014-exact-scalar-oracles.md, spec/oracle/matrix/TC-024-exact-scalar-oracles.md, spec/kani/matrix/TC-023-bounded-kani-profile-corpus.md, spec/routed/matrix/TC-033-routed-generation.md, spec/kani/matrix/tests.md, spec/oracle/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-contract-codegen/FR-057
    type: reviews
  - target: ix://agent-ix/quire-contract-codegen/FR-015
    type: reviews
---

## Summary

Ticket: IR-854. The spec edits are status flips and "planned" to "landed" amendment notes:
FR-057 and TC-071 Status (PARTIAL), FR-014-AC-81 (Implemented), FR-038-AC-1/AC-4, FR-015-AC-109,
FR-056's row list, TC-023, TC-024, TC-033 and both matrix rows. No requirement statement or
acceptance criterion changed meaning. The FR-056 sentence ("sub, div, rem and mod from 4 bits")
matches the shipped table. The status over-claims for AC-9/AC-11/AC-12 are recorded in the
gap-analysis artifact and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | FR-015-AC-109 now contradicts itself: its body still asserts in present tense "V2 does not serve divide or remainder, tested: a claim over `quire.op.integer.div` settles `Unsupported(OperationNotRendered)` ... and `quire.op.integer.rem` stays the refused catalogue identity", while the appended "Amended by IR-854 (landed)" sentence says those tests are superseded and division renders. Reword the body to the past tense or move the old disposition into the recorded-history clause. | spec/kani/functional/FR-015-bounded-kani-obligations.md:957 |
| FND-002 | low | FR-057's Description still says "Today these claims get no harness" (`div`/`mod` settle `OperationNotRendered`, `rem` is `Underived`) and "This row renders `rem` once FR-014-AC-81 holds", and TC-071's Description still says "This case is planned", although the same PR flips both Status sections to PARTIAL with those behaviours landed. Update the two Description paragraphs to match. | spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:38-65, spec/kani/matrix/TC-071-integer-division-scalar-harnesses.md:22 |

## Verdict

Spec edits are consistent with the code apart from two stale present-tense passages (low). No
EARS or structural change was made, so no sub-analysis beyond base applies.

## Dispositions

Round 1, checked at 1339b944.

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | FR-015-AC-109 now reads "At IR-598, V2 did not serve divide or remainder ... settled ... stayed", then the landed amendment. |
| FND-002 | fixed | FR-057 Description now reads "Before IR-854 these claims got no harness" and "FR-014-AC-81 now holds (IR-854), and this row renders `rem`"; TC-071 reads "This case is partly run (see Status)". |
