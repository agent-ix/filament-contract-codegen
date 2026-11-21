---
id: SR-17940486709564847048112
title: "QSL-2279 PR 878 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; Cargo.toml, Cargo.lock, src/core/profile.rs, tests/checked_package_support/temporal.rs; repository matrix"
review_set: subset
relationships:
  - {target: ix://agent-ix/filament-contract-codegen, type: references}
---

## Summary

Frozen head d84ecf10296eae7d2ee47de83f44ec18edde9be0; exact base 038592fdcf6ff2eef9d18cd4c8699c2556e07517. Ticket: QSL-2279. No local Cargo, builds or tests run; delegated focused evidence is pending.

Repository-driven review. Static review found no defect in the four-file compatibility change. All eleven changed lock records preserve package versions and other fields except the intended Git sources and fsl-replay num-traits edge. GitHub commit endpoints resolve all three pins; adopted FSL Cargo.toml names the same model and FSpec pins and fsl-replay requires num-traits 0.2.19. Generated-manifest constants match every corresponding root dependency. The temporal helper obtains source_unit from the existing matching claim occurrence region, checks full source-reference equality against lock.sources, and uses that same region in TemporalClauseName. This preserves source ownership rather than choosing an arbitrary locked source. Upstream TemporalBinding requires Some for clauses, None otherwise, and locked membership. Both existing scalar/equality fixture builders call this helper and submit to strict admission. No node identity, graph body or #877 production refusal path changes. Existing #877 scalar/equality regression tests and production files are byte-identical to base. No new duplicate implementation, stub, mock, gate reduction, unsafe, concurrency, or production panic path is introduced. No applicable AssuranceProfile exists under spec/.

## Verdict

MERGEABLE for scoped pin/adaptation review; whole-repository gap assessment FAIL (227 unchanged baseline trace gaps), outside this consuming pin. Delegated focused checks remain pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reconciliation: quoin matrix (quoin 0.36.8, quire 0.52.0 engine 0.62.2), corroborated by quire matrix exit 0. Quoin JSON completed with empty stderr; original command exit status was not retained, so no exit-code claim is made. No fallback binary used. The original release notice names older versions; this installed version discrepancy was reported to the lead.

Whole-repository criteria: 1,142; tagged 860; untagged 208; tagged-by-ignored-test 19; method-without-symbol 55; no run evidence 1,142. All spec files and all trace tags are unchanged from base, establishing these trace gaps were not introduced here. Matrix outputs retained alongside this artifact. Coverage command exit 0 reports 222 untracked symbols, 141 unmatched tags, 1,082 diagnostics; these include legacy TC metadata and unrelated markers and have not been individually adjudicated as defects. Quire matrix diagnostics (353181 bytes) retained in quire-matrix.stderr. No full repository assurance PASS is claimed.

Plan completion: not assessed

Semantic review: skipped (not requested). Reverse-gap/stub inspection covered both changed behaviors: emitted manifest sources (FR-014-AC-8) and synthetic fixture temporal source retention (FR-037/FR-039); 2 behaviors, 0 untraced introduced behaviors, 0 source/test stubs found in changed scope. Whole-tree reverse-gap census was not performed; this is a scoped PR review with a whole-repository matrix, not a whole-repository implementation audit.

## Examined scope

```yaml
scope:
- id: FR-014-AC-8
  path: spec/oracle/functional/FR-014-exact-scalar-oracles.md
  role: examined
  excerpt: The generated crate declares `publish = false`, names the direct exact/FSV
    and Runtime residue dependencies from the one profile authority without a replay-context
    dependency for Boolean/integer scalar proofs, contains no charge amount (every
    charge comes from runtime metering), and compiles.
- id: FR-014-AC-44
  path: spec/oracle/functional/FR-014-exact-scalar-oracles.md
  role: examined
  excerpt: For each IR-admitted direct request naming a `temporal`/`formula`, `temporal`/`fairness`
    or `expression`/`case` node, the scalar claim map has exactly one entry with that
    requested node id and `ExactScalarRefusal::Unsupported`. For an IR `Unsupported`
    lowering record, the refusal's `unsupported_node_id` and `node_tag` equal that
    record's first unsupported reachable node and tag, even if that node is not the
    request root; for a lowered `case` root they name the root and `expression`. A
    lowered `case` is refused as `Unsupported`, not `FormMismatch`. Each refused item
    has no generated function or
- id: FR-018-AC-32
  path: spec/oracle/functional/FR-018-composite-equality-oracles.md
  role: examined
  excerpt: Each IR-admitted direct `temporal`/`formula`, `temporal`/`fairness` and
    `expression`/`case` request has one claim-map entry keyed by its requested node
    id and refused as `CompositeEqualityRefusal::Unsupported`. For an IR `Unsupported`
    lowering record, the refusal's `unsupported_node_id` and `node_tag` equal that
    record's first unsupported reachable node and tag, which need not be the requested
    root; for a lowered `case` root they name the root and `"expression"`. The request
    contributes no generated symbol or source, never panics or disappears, and leaves
    a supported sibling's claim and source
- id: FR-037-AC-6
  path: spec/core/functional/FR-037-fixture-identity.md
  role: examined
  excerpt: Both fixture builders preserve every producer-assigned structural application
    `CheckedNodeId` in the assembled node and its dependent references; an ungrouped
    fixture's requested ID equals the reader-admitted node ID. Inspection finds no
    local application ID derivation, application preimage encoder, version constant
    or digest call for that identity.
- id: FR-037-AC-9
  path: spec/core/functional/FR-037-fixture-identity.md
  role: examined
  excerpt: Positive exact-scalar and composite-equality fixture packages remain reader-admitted.
    A declaration whose path disagrees with `node_id.path` is refused by the reader.
    A changed body under the same valid structural key is detected separately by comparison
    with the expected admitted source body and is not silently credited as the original
    fixture; neither negative control is repaired before its assertion. A different
    valid unused ordinal with consistent references is not itself a reader refusal.
- id: FR-039-AC-11
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: examined
  excerpt: Generated scalar, composite equality and exact function claims each serialize
    `contract_package_identity` equal to the admitted FR-330 `package.source` structural
    package name plus complete lock, replacing the removed `ContractPackageV1.identity()`
    accessor, and `node_id` equal to the original requested V2 `CheckedNodeId`, including
    graph-local indexes; their containing claim-map entries use that same `node_id`,
    with no generated-claim `ir_id` key or second public V1 node identity. CG matches
    the request to V1 accounting through equal source-bound structural keys resolved
    by `ContractSourceKey
```
