---
id: SR-17940486709564847048067
title: "spec-review base of filament-contract-codegen PR #859 latest first-party pins"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen; PR #859 at 101943e2; spec/assurance/AD-002, spec/kani/functional/FR-048 (AC-6), spec/kani/matrix/TC-062, spec/oracle/functional/FR-018 (Behavior, AC-4, AC-50, mutant row), spec/oracle/functional/FR-031, spec/oracle/matrix/TC-029"
review_set: subset
---

## Summary

Spec-review base over the six spec files the PR edits. Ticket: IR-1566. `make spec` with FSPEC_DIR at filament-specification origin/main af8dd1d3 exits 0. The edits re-spell the pin policy (AD-002, FR-018 Behavior, FR-031, TC-029), extend FR-018-AC-4, add FR-018-AC-50 with its mutant row, and carry the #854 fold's FR-048-AC-6 and TC-062 wording. No relationships edges changed, so the dependency sub-analysis does not apply. The integrity and EARS concerns are covered here, because the change is a few rows.

## Examined scope

- AD-002 `fsl-replay` row (examined). It now says CG names an FSL revision, an interim `rev` until the fsl-* tag exists. This is accurate.
- FR-018 Behavior manifest bullet (examined). It says tags, plus the revision for `fsl-replay`, "so no first-party crate resolves from a branch or twice". That is accurate for the equality manifest the bullet describes.
- FR-031 `filament-exact` bullet, TC-029 step 3 (examined). Both say tag and no branch, which matches `src/core/profile.rs`.
- FR-018-AC-4 extension (examined). The added sentence says the record mirrors every owner schedule, `Float` included. This is testable and tested (tc_029_ac50).
- FR-018-AC-50 and its mutant row (examined). The mutant row is concrete. The AC's FSL parenthetical is wrong, recorded as FND-006 in the gap-analysis SR. The AC also bundles three claims: the reader refuses, FSL does not emit, and the claim map records `float`. Only the first and last are CG-observable. It is noted here and not raised separately.
- FR-048-AC-6 (examined). FND-010.
- TC-062 paragraph (examined). It says the Undefined conversion is checked on a directly constructed `Undefined` evaluation, beside the typed compile refusal. That matches `src/kani/terminal/constrained/tests.rs:198`.

## Verdict

The pin-policy wording is consistent across AD-002, FR-018, FR-031 and TC-029, and it matches the code. The coordinator-requested FR-048-AC-6 change removed the test-method narrative, but the row is still a compound criterion, so "atomic" is not met. The FR-018-AC-50 text needs its FSL clause corrected (gap-analysis FND-006).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | FR-048-AC-6 is still compound. The new clause ("a source divisor FSL cannot prove nonzero is refused at compile time with `undefined_expression` (`unproved-nonzero`) and never reaches Undefined") is joined by a semicolon to the Undefined-retention sentence, beside the Refused/Incomplete and CallFailure sentences: four claims in one row. The compile refusal is FSL behaviour and could be its own AC (or a cited FSL dependency). | spec/kani/functional/FR-048-constrained-state-clause-method.md:208 |

## Dispositions

Round 1 at d42182dc. `make spec` with FSPEC_DIR af8dd1d3 exits 0, and its warnings are only on rows this PR did not edit.

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | FR-048-AC-6 is now the single Undefined-retention claim. The split adds AC-12 (unproved-nonzero compile refusal), AC-13 (Refused/Incomplete keep their causes) and AC-14 (CallFailure stops as runtime_invariant), one claim each. The lib test `conversion_preserves_original_causes...`, which asserts all four, is tagged AC-6/12/13/14. The it tests at constrained_method.rs:145 and :400 keep AC-6 only, which is correct: they check the Undefined stop and the rendered "undefined constraint evaluation" branch. |
