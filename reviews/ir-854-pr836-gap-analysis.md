---
id: SR-17940486709564847047994
title: "IR-854 PR #836 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-contract-codegen PR #836 at 89a7cc86, git diff origin/main...HEAD; IR-854; FR-057-AC-1..AC-12, FR-014-AC-81; spec/kani/functional/FR-057-integer-division-scalar-harnesses.md, spec/kani/matrix/TC-071-integer-division-scalar-harnesses.md, spec/kani/matrix/tests.md, docs/qualification/ir854-general-route-division-measurements.md, docs/qualification/ir1530-general-route-scalar-table.json, tests/it/integer_division_harness.rs, tests/it/integer_division_bounds.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-contract-codegen/FR-057
    type: reviews
  - target: ix://agent-ix/quire-contract-codegen/TC-071
    type: reviews
  - target: ix://agent-ix/quire-contract-codegen/FR-014
    type: reviews
---

## Summary

Ticket: IR-854. Planless gap analysis of FR-057 and FR-014-AC-81 against the tagged tests, the
measurement record and the shipped table. `quoin matrix --json` at 89a7cc86: every FR-057
criterion and FR-014-AC-81 is `tagged` (AC-3 `method-without-symbol`, AC-7
`tagged-by-ignored-test`); none has run evidence. Plan completion: not assessed.

Per-criterion reading (examined):

- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6: evidenced by the default-lane tests named in TC-071; the
  host controls build and run a crate over the real quire-exact; the differential compiles the
  rendered expected side; AC-5's inferred-range refusal surfaces publicly as
  `DomainNotRepresentableInI64` with the canonical endpoints.
- AC-7: honestly NOT delivered. No shape finished; the Status claims no `Verified`.
- AC-8: honestly NOT delivered in prose (one general-route mutant timed out), but tagged (see the
  code-review artifact, FND-001).
- AC-9: the table side holds: three `not_tractable` rows, route `general`, 4 bits; the peaks are
  the per-operation maxima of the 4-bit rows (div 8579940352, rem 8547880960, mod 8538312704); the
  4-bit symbolic shape was measured under all seven operation/law pairs; refusal at 4 bits and
  pass-through at 3 bits are tested; no shape is `Verified`, so no split and no
  `divisor_constant`. The record side is incomplete (FND-001).
- AC-10: classifications of unsatisfied cover, missing cover summary, exhausted unwind, a real
  timeout and a no-verdict (out-of-memory stub) run are tested; none is `Verified`.
- AC-11, AC-12: claimed delivered; evidence is narrower than the criteria (FND-002, FND-003).
- FR-014-AC-81: derivation, inferred intervals, explicit-result refusal, grid agreement with
  quire-exact and the `modulo`/`Quotient` mutants are tested.

The general-route `TimedOut` claims are faithful to the evidence: the 600 s ceiling is the test
fixture's `proof_ceilings()` (FR-028), every recorded outcome is `Inconclusive { TimedOut }`, the
peak (about 8.5 GB) is far below the 24 GiB scope and the 16 GiB ceiling, so `memory_exhausted`
is correctly not claimed, and the doc states that wall and peak include the crate build.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-057-AC-9 is reported tested (`tests.md`: "AC-1 to AC-6 and AC-9 to AC-12 are tested") but its record clause is not met: AC-9 requires the record "for every listed operation, shape, route and width", and the record's own "Shapes not run" lists the constant divisor at 16, 32 and 63 bits (the "up to the first width that does not finish" allowance in FR-057 applies only to the symbolic shape), the 8-bit constant shape for `div` Euclidean and `rem` floor/Euclidean, the negative constant divisor for six of seven pairs, and every native-route shape. Mark AC-9 PARTIAL in `tests.md`, FR-057 Status and TC-071 Status (or run the missing constant shapes), and name what remains. | spec/kani/matrix/tests.md:13, docs/qualification/ir854-general-route-division-measurements.md:47-58, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:256-262 |
| FND-002 | medium | FR-057-AC-12 is reported delivered ("the identity record (AC-12)") but the tagged test checks only the generation identity (node, operation, profile, result range, unwind, that `--unwind` and `cadical` occur in the options) and representation. AC-12 also requires the source association, the full ordered backend arguments, target, limits, raw checks and cover, playback, exit, the typed terminal outcome, and replay from the retained artifact; no run in this PR produced a verdict or playback, so those clauses cannot be evidenced yet. Mark AC-12 PARTIAL (owner IR-1541 with AC-7/AC-8) or add the missing assertions over the AC-10 launcher runs' evidence. | tests/it/integer_division_harness.rs:1076-1103, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:321 |
| FND-003 | low | FR-057-AC-11 is reported tested, but its test compares each integer sibling generated together with a division claim against the same sibling alone (an isolation check), not output before versus after this change, and covers only add, sub, mul and negate; the integer comparisons, Boolean implication, enum and float comparisons and the FR-015-AC-115 reference-copy rerun named by AC-11 are not cited. Cite the existing tests that pin those outputs (if any) or narrow the Status to what is checked. | tests/it/integer_division_harness.rs:1032-1074, spec/kani/matrix/tests.md:13 |
| FND-004 | low | The measurement record gives the kernel version as "quire-exact revision as locked in `Cargo.lock`", which is not a recorded value and changes with the next lock bump; FR-057 requires generator and kernel versions per measurement. Write the locked quire-exact revision (or tag) into the record. | docs/qualification/ir854-general-route-division-measurements.md:20-21 |

## Verdict

The status claims for the hard items are honest: no `Verified` verdict, no `Falsified` mutant,
AC-7 and AC-8 explicitly not delivered and owned by IR-1541, the table rows match the measured
data, and nothing refuses a measured-`Verified` shape. Over-claims remain for AC-9 (record
incomplete), AC-12 (run-evidence clauses unevidenced) and, mildly, AC-11. Fix the status lines
(FND-001, FND-002) in this PR; FND-003 and FND-004 are low.
