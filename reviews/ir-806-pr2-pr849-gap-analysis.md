---
id: SR-17940486709564847048036
title: "IR-806 PR-2 PR #849 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #849 at 29227cbd, git diff origin/main...HEAD (base e9918842); IR-806 PR-2; FR-025-AC-9, FR-033-AC-2, AC-3, AC-4, AC-5; tests/it/composite_playback_decode.rs, tests/it/composite_shadow_kani.rs, src/kani/identity/leaf_binding.rs, src/replay/composite_leaves.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-033
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-2). Planless gap analysis of the five criteria this PR moves from PLANNED to
PARTIAL against code and tagged tests. Plan completion: not assessed.

All five stay PARTIAL and none says Implemented or Covered; both tests.md rows say Partial. Each
AC lists what is covered and what is not. Checked claims:

- FR-025-AC-9: union members are excluded honestly (the shadow refuses unions with
  `ShadowFamilyNotBuilt`; `walk` refuses a union shape). Refusal reasons are each killed by
  mutation (see the code-review SR), except the generation-time call-order half (code-review
  FND-002) and the contiguity check (code-review FND-001).
- FR-033-AC-2: the reference-identity gap claim is accurate. At the locked FSL revision
  `AdmittedContext` (fsl-replay/src/admitted.rs:379-398) holds enums, composites, units and
  dimensions but no object types; `fsl-package` `object_declarations` is `pub(crate)`; CG builds
  `TypeEnvironment::new(composites, std::iter::empty())` (src/oracle/equality/mod.rs:1466).
  `object_reference_witness` exists publicly but builds a standalone witness value, not an
  admitted composite operand.
- FR-033-AC-4: the 100,000-link test is worded honestly in the AC ("pins FSL's value type
  only", "the composite shadow refuses a declaration that reaches itself"); the test name claims
  no recursion support.
- FR-033-AC-5 and the installed-backend test: the decode is fed by the real Kani counterexample;
  the reviewer's re-run reproduced Falsified (see the code-review SR).

Bindings checked: the 11 new tests and the ignored Kani test bind to FR-025-AC-9 and FR-033-AC-2
to AC-5 as tagged; each binding is correct.

## Verdict

**Approve with low findings.** The PARTIAL statuses are honest. Two covered-lists say slightly more
than the tests show.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-033-AC-4's covered list says the 100,000-link value "clones, compares, formats Debug and drops"; the criterion asks that it "decodes" and formats "redacted" Debug. The test decodes nothing and asserts only that the rendering is non-empty (FSL's Debug is redacted, but nothing here checks that). Neither decode nor redaction is named in the not-covered list, so the criterion is narrowed without saying so. Name both in the not-covered list, or assert that the rendering holds no leaf content. | spec/replay/functional/FR-033-composite-parity-replay-binding.md:363, tests/it/composite_playback_decode.rs:589-610 |
| FND-002 | low | FR-033-AC-3 lists "swapped leaf order" among the refusals that are covered. Only a swap across primitive types refuses (the test swaps a Boolean with an integer and gets `kani_witness_width_mismatch`). A swap of two integer leaves decodes into a different operand. FR-033-AC-5 says this correctly ("refuse or change the rebuilt operand"); AC-3 should say the same. | spec/replay/functional/FR-033-composite-parity-replay-binding.md:362, tests/it/composite_playback_decode.rs:496-514 |

## Coverage

Examined FR-025-AC-9 and FR-033-AC-2 to AC-5, TC-048 steps 2 to 4, TC-036 step 9, and the
FR-025 and FR-033 rows of spec/kani/matrix/tests.md and spec/replay/matrix/tests.md. Tool
versions are absent from the TC-048 run record.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at 7890dec0 (6afe2163): FR-033-AC-4 now reads "formats a redacted Debug rendering (asserted to hold none of the value's leaf content)" and lists "decoding a 100,000-deep value through FSL's canonical reader" as not covered. The test builds the chain over an `ExactInteger(7654321)` leaf and asserts that the rendering does not contain it. |
| FND-002 | fixed | Round 1 at 7890dec0 (6afe2163): FR-033-AC-3 now reads "a swap of leaves of different primitive types" and adds "A swap of two leaves of one primitive type is not a refusal: it decodes, and the rebuilt operand differs (FR-033-AC-5)." |
