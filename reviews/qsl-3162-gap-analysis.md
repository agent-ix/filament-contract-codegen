---
id: SR-202611210022442313144108537
title: "Gap analysis — temporal SMT settlement adapter"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; repository trace matrix and changed temporal SMT adapter; QSL-3162"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen
    type: references
---

## Summary

The computed matrix reports 208 untagged criteria repository-wide. Four local SMT outcome-map criteria directly governing this route (FR-030-AC-19 through AC-22) are untagged, and the new adapter tests do not exercise the production route.

## Verdict

**FAIL** — the computed Test Matrix has untagged criteria, including the local SMT settlement criteria this route contributes to.

## Coverage

- Plan completion: not assessed.
- Matrix: `quoin matrix --repo <repo> --json` (`quoin 0.36.10`, `quire 0.52.1`, engine `0.62.2`); 862 tagged, 208 untagged, 20 tagged-by-ignored-test, 55 method-without-symbol, 1,145 with no run evidence, and zero qualified/retired/bound/stale/suspect/undischarged rows.
- Relevant local rows: FR-030-AC-19, FR-030-AC-20, FR-030-AC-21, and FR-030-AC-22 each have `static_status: untagged`, no binders, and no run evidence.
- Changed-source stub scan: no `TODO`, `todo!`, or `unimplemented!` in the new module; no source stub identified in the changed route.
- Reverse-gap scan: the new public temporal adapter is owned by the local FR-030 SMT map and references FSL FR-127/FR-314; no unowned changed behavior identified.
- Semantic intent review was skipped; it is optional and was not requested.
- The independent test-run evidence was not available in this review. Coder-reported focused test and clippy results were not treated as matrix run evidence.

## Findings

| ID      | Severity | Summary                                                                                          | Refs                                                        |
| ------- | -------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| FND-001 | high     | The local SMT certified/uncertified terminal mapping criterion has no tagged test evidence.       | spec/kani/functional/FR-030-ir-outcome-terminal-map.md:239 |
| FND-002 | high     | The local SMT certificate-rejection mapping criterion has no tagged test evidence.                | spec/kani/functional/FR-030-ir-outcome-terminal-map.md:240 |
| FND-003 | high     | The local SMT parity, missing-backend, and replay mapping criterion has no tagged test evidence.  | spec/kani/functional/FR-030-ir-outcome-terminal-map.md:241 |
| FND-004 | high     | The local criterion requiring every accepted SMT settlement to produce one non-Tested terminal is untagged. | spec/kani/functional/FR-030-ir-outcome-terminal-map.md:242 |

## Tracker

Linear issue resolution/read-back and posting were unavailable: `linear issue view QSL-3162` returned `No usable API key for workspace "agent-ix"`. No GitHub PR exists, and findings were not posted to GitHub.
