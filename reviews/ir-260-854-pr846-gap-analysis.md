---
id: SR-17940486709564847048024
title: "IR-260 IR-854 PR #846 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #846 at 9e98185c, git diff origin/main...HEAD (base ebe100f7); IR-260, IR-854; FR-056-AC-15, FR-053-AC-6, FR-053-AC-13, FR-057-AC-7; docs/qualification/ir1530-general-route-scalar-table.json, tests/it/kani_tractability.rs, tests/it/float_scalar_harness.rs, spec/kani/matrix/TC-070-kani-tractability-refusal.md, spec/kani/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-056
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-070
    type: reviews
---

## Summary

Ticket: IR-260 and IR-854. Planless gap analysis. Plan completion: not assessed. `quoin matrix
--json` at 9e98185c: FR-056-AC-15 is `tagged` (binders include the new
`tc_070_the_shipped_table_holds_the_kernel_native_float_and_rational_rows` and
`tc_070_the_embedded_general_route_table_is_the_docs_file`); FR-057-AC-7 stays `tagged` by the
pre-existing native division row test, unchanged here; FR-053-AC-6 keeps
`the_shipped_float_rows_are_a_version_free_record_with_measured_peaks`.

Row-by-row check of all 58 added rows against filament-exact 85034c5c, scripted for the 44 float
rows (operation id, `max_operand_bits`, verdict, `wall_seconds` = TC-953 wall or limit,
`peak_bytes` = TC-953 peak, harness id, rounding class, probe/extension text, owner IR-1551):
0 mismatches. The 14 rational rows checked by hand against TC-956 (op ids, 8/64 bits, verdict,
wall, peak, harness, owner IR-1544): 0 mismatches. No row claims `tractable` for a TC-953 or
TC-956 timeout, and none is missing; the 14 tier-lemma and charge-denial harnesses of TC-956 are
correctly left out as non-operations. No production code changed (`src/` untouched), so there is
no unowned code; the only code-level gap (AC-13's arithmetic differential, AC-7's CG native
division harness) is declared PARTIAL in the spec and recorded in the spec-review artifact for
its stated cause.

The `float_scalar_harness` filter change (`route == "general"`) keeps that test on exactly the
five rows it was written for; without it the count would become 49. It does not weaken the test.
The test-strength finding on the new TC-070 test is recorded in the code-review artifact, not
repeated here.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every added row is traceable to the kernel's record and every criterion touched is tagged
and honestly statused in substance (see the spec-review artifact for prose fixes).
