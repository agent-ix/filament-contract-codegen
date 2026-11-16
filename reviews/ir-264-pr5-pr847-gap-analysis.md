---
id: SR-17940486709564847048027
title: "IR-264 PR #847 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #847 at 1ae756d3, git diff origin/main...HEAD (base 8c80f954); IR-264 PR5; FR-028-AC-23, FR-015-AC-73 (same-request clause and reference row), FR-018-AC-7, FR-021-AC-10 against src/kani/run/**, src/kani/generate/negotiate.rs, src/kani/generate/outcome.rs, src/kani/generate/shadow/composite.rs, src/oracle/equality/**, tests/it/composite_shadow_families.rs, tests/it/composite_equality_generation.rs, tests/it/shadow_kani.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
---

## Summary

Ticket: IR-264 (PR5). Planless gap check of the criteria this PR claims against the code and tests.
Plan completion: not assessed.

- FR-028-AC-23 PARTIAL, honest: the Kani and CBMC releases are recorded on the proof evidence, the
  witness evidence and batch members (stand-in tests, real-backend test re-run here: 0.72 s,
  292,483,072 B peak in a 24G scope); the Rust compiler version is not recorded and the status says
  so; the normative AC text is unchanged. The solver version is recorded as the CBMC release, which
  holds for `KaniSolver`'s only variant (`Cadical`, linked into CBMC). Cache defects are code-review
  FND-002/FND-003.
- FR-015-AC-73 same-request clause NOT MET, owner IR-806 PR-3: verified against the code.
  `ObligationItem` has `ConstrainedMethod`, `V2Clause`, `StateFrame`, `ScalarClaim`, `GraphClaim`
  and `CollectionClaim` and no composite kind; `negotiate.rs` never names a composite item;
  `generate_composite_shadow` is its own public entry. FR-029-AC-30 exists and is PLANNED (IR-806).
- FR-015-AC-73 reference row met: `reference_record_entry` generates the corpus `E_REFERENCE` item
  (two `R_WITH_REF` records) through FR-018; it is refused as `PackageAccounting` carrying
  `Unsupported` with node tag `model`, and the shadow carries it unchanged with no harness. Both
  mutants tried (refusal kind changed; refusal not carried) fail the test.
- `agent-ix/filament-spec-language#120` is CLOSED (2026-11-11); the wire value
  `UpstreamBlocker::FilamentSpecLanguage120` is kept.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `"reference"` arm of the equality type walk still returns `BlockedOnUpstream { node_tag: "composite_type.reference", issue: FilamentSpecLanguage120 }`. The PR shows the corpus reference record never reaches it (FR-018 refuses it first as package accounting of the `model` target), and no test anywhere asserts `composite_type.reference` for the equality oracle (the only other hit is FR-021's function path). The arm is now untested and, by the PR's own analysis, unreached: it emits a blocker for a closed issue. Either delete it or make it an `Unsupported` invariant, or add a test that reaches it; and say in FR-018-AC-7 which of these is true. Pre-existing code, but the PR's claim that the row is met depends on it being unreached. | src/oracle/equality/resolution.rs:372-378, spec/oracle/functional/FR-018-composite-equality-oracles.md:713 |

## Verdict

AC statuses agree with the code: AC-23 PARTIAL (honest), AC-73 same-request NOT MET owned by IR-806
PR-3 (claim verified), reference row met by a test that fails when the refusal is removed or
changed. One low gap: the now-unreached reference arm.
