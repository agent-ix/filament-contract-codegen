---
id: SR-17940486709564847048064
title: "IR-1447 PR #857 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #857 at 4f33c3b2, git diff origin/main...4f33c3b2; IR-1447; spec/kani/functional/FR-015-bounded-kani-obligations.md (AC-155, AC-159), spec/kani/matrix/TC-025-bounded-kani-obligations.md (step 57), docs/qualification/ir1447-kani-control-clause-runs.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-1447. This review covers the rewritten AC-155 and AC-159.

AC-155 now names the closed predicate `IntegerIfAtMost` and the fixture: field `items` of
cardinality 0 to 3 with elements `Int[0, 3]`, body `(if element < 2 then element else 2) <= 2`,
rendered `(if item < 2 { item } else { 2 }) <= 2`. It also names the refusal
(`CollectionClaimNotRendered` / `UnsupportedClaimShape`). The text matches what the tests assert.

AC-159 now has a source-authored else-`3` mutant, Falsified at the harness assertion with an
element of at least 2. It records that the `<= 2` condition mutant is equivalent. This also
matches the test and the executed runs.

The old figure (0 to 4 elements of `Int[0, 1000]`, split 500, mutant 501) was not impossible.
The existing `model_document` fixture field `items` is `SmallInt` `Int[0, 3]` with cardinality 0
to 3, so the old figure needed only a second model document. The direct harness cost is
insensitive to the element range (one `kani::assume` per `i64` slot).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | AC-155 and AC-159 were narrowed from the specified example (cardinality 0 to 4, `Int[0, 1000]`, split/bound 500, mutant 501, witness 500) to the existing fixture (0 to 3, `Int[0, 3]`, 2, mutant 3, witness at least 2), and the Status cell does not say so. The shape and the property proved are unchanged, so this is not a semantic weakening. But a criterion rewritten to the delivered fixture should disclose the narrowing and why, for example "fixture narrowed to the existing `items` field; the predicate shape is range-independent", so a reader does not take the new figures as the originally required control. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1003, spec/kani/functional/FR-015-bounded-kani-obligations.md:1007 |
| FND-002 | low | AC-155 states "each branch the element or an integer literal and the bound an integer literal". A negative literal (`-1` as split, branch or bound) is a source integer literal, but FSL emits it as a negation, so CG refuses it with `UnsupportedClaimShape`. The reviewer confirmed this by FSL-compiled probe. The refusal is safe, but the AC promises more than the code accepts. It should say "a non-negative integer literal (FSL emits a negative literal as a negation, which is refused)", or the reader should accept `integer.neg` of a literal. | spec/kani/functional/FR-015-bounded-kani-obligations.md:1003, src/kani/generate/collection.rs:1239-1281 |

## Verdict

The AC rewrites are accurate to the tests and the executed evidence. The two findings are
disclosure and precision nits. The staleness of TC-025 step 57's procedure body and of the
`tests.md` row is recorded in the gap-analysis (SR-17940486709564847048063).
