---
id: SR-17940486709564847048110
title: "QSL-2660 PR 877 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR 877 e9f9ae314360dad753ec2345dca40fd16f3171a6; repository computed matrix and reverse inventory; changed function refusal and test fixture surface"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-contract-codegen/FR-021", type: references}
---

## Summary

Ticket: QSL-2660. Repository-driven matrix, surface inventory and stub scan were performed, with detailed implementation inspection of the three changed files. The production refusal remap is owned by FR-021-AC-33. The reenabled case test exercises this behavior but carries an unrelated AC-41 marker. This artifact is a PR-scoped decision and does not claim the repository's historic assurance backlog is closed.

## Verdict

**FAIL** for repository-wide assurance: the computed matrix contains 208 untagged criteria and 19 tagged only by ignored tests. Those baseline obligations are outside the explicitly assigned PR scope. The PR-local trace finding below is **CONDITIONAL** until fixed; it is distinct from the code-review duplication finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The reenabled case test binds AC-41 without testing the required plain-read temporal authentication refusal | tests/it/exact_function_generation.rs:1785; FR-021-AC-41 |

## Coverage

Examined scope (verbatim excerpts, truncated to 600 characters where needed):

```yaml
[
  {
    "id": "FR-021-AC-33",
    "path": "spec/oracle/functional/FR-021-function-application-oracles.md",
    "role": "examined",
    "excerpt": "With an IR-admitted `temporal`/`formula`, `temporal`/`fairness` or `expression`/`case` node used directly as an item's call-node id, and an authenticated requested-root kind for an IR `Unsupported` lowering record, the claim-map entry retains that id and is `Refused { refusal: ExactFunctionRefusal::Unsupported }`. For the separate declared-body-root case, an item names that function through a distinct, IR-admitted `expression`/`call` node that lowers successfully and has no duplicate, capability, or earlier item refusal; its claim retains the call-node id and carries the body's `ExactFunctionR"
  },
  {
    "id": "FR-021-AC-41",
    "path": "spec/oracle/functional/FR-021-function-application-oracles.md",
    "role": "examined",
    "excerpt": "For a plain strict-reader-admitted V1 package, after existing duplicate, capability and item-precedence checks, an eligible selected call item or declaration body with an admitted `Unsupported` accounting record whose `node_tag` is `\"temporal\"` and whose requested-root-kind view is `NotAdmitted` causes whole-call `OracleGenerationError::UnauthenticatedRequestedRootKind { request_index, node_id }` before claim map, schedule, oracle or partial publication. These V1 facts trigger refusal without deciding whether the requested root is direct `formula`, direct `fairness` or a temporal clause. For a"
  },
  {
    "id": "FR-021-AC-11",
    "path": "spec/oracle/functional/FR-021-function-application-oracles.md",
    "role": "context_only",
    "excerpt": "With an admitted requested-root kind, model and relation lowering records map to the typed refusal whose wire value is filament-spec-language#120 (a stable identifier; that issue is closed); state, temporal nodes other than directly requested `formula` and `fairness` roots, and protocol lowering records map to filament-spec-language#121; the blockers remain distinct. A requested temporal clause keeps that blocker even if its first unsupported reachable node is a formula or fairness child. A plain-read V1 temporal-tag `Unsupported` record with `NotAdmitted` requested-root kind follows AC-41 ins"
  }
]
```

Reconciliation: quire matrix, quire 0.52.0 / engine 0.62.2, explicit scope /Users/peter/dev/qsl-2660-codegen; no run evidence read. Matrix value saved in the scratch review directory. Engine totals: 1142 criteria, 860 tagged, 208 untagged, 19 tagged-by-ignored-test, 55 method-without-symbol. FR-021-AC-33 and AC-41 both compute tagged. AC-33 binds the case test, genuine temporal roots test and scalar direct-control test. AC-41 binds the case test plus selected-imported-call eligibility test. Bindings were checked against test assertions, not inferred from matrix status.

Plan completion: not assessed

Reverse inventory: 726 public function/type/trait declarations observed across src, with subsystem ownership organized by core, oracle, kani, replay, evidence, routing and publication specification trees. Detailed new-surface inventory: no production public API added, two modified refusal helpers and one new fixture authoring entry point with its helpers. New production untraced behavior: 0. Source stubs introduced: 0. Test stubs introduced: 0. The reenabled regression uses real FSL producer admission and asserts cause identity and sibling output equality. The fixture reconstruction duplicates upstream behavior, recorded once in the code-review artifact rather than repeated here.

Examined units: FR-021-AC-33 and FR-021-AC-41. Context: FR-021-AC-11 and FR-021 Description. Semantic review opt-in was not requested; no standalone semantic review was run. Matrix emitted existing trace-grammar warnings for source commentary and old bare requirement/test-case tags; the requested AC binders were present in the engine output. No baseline warning was represented as a new PR defect. No aggregate or cargo test run was performed by this review.
