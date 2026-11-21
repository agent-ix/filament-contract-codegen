---
id: SR-17940486709564847048109
title: "QSL-2660 PR 877 code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR 877 e9f9ae314360dad753ec2345dca40fd16f3171a6; src/oracle/function/mod.rs; tests/checked_package_support/rekey.rs; tests/it/exact_function_generation.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-contract-codegen/FR-021", type: reviews}
---

## Summary

Ticket: QSL-2660; QSL-2586 remains partial. Reviewed all three changed files against fresh origin/main 556f47c3, including the Rust review lane, owning FR-021, the admitted accounting/represented identity maps, and pinned producer body-location implementation. No source or branch edits and no build were performed. The owner requires filtered tests only and prohibits aggregate gates.

The production change correctly translates a lowered refusal through `original_represented_node`, an accounting refusal through `original_node`, and leaves the claim keyed by its original request. Direct temporal provenance follows the accounting translation. This removes the old erroneous attempt to interpret a represented identity as an accounting identity. Existing typed family blockers remain unchanged. No new unsafe, API, dependency, warning suppression, gate change or compatibility layer appears.

## Verdict

**FAIL** — the new fixture helper duplicates producer-owned body-location semantics. One additional trace binding needs correction. The four known Kani table hardening failures and the QSL-2586 measurement campaign are outside this PR's delivery scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new fixture helper reimplements producer-owned body-location traversal and emission; replace the algorithm with producer-emitted packages or explicitly authored fixture rows | tests/checked_package_support/rekey.rs:79-246 |
| FND-002 | medium | The reenabled direct-case test claims FR-021-AC-41 although it does not exercise the plain-reader temporal authentication refusal | tests/it/exact_function_generation.rs:1785 |

## Evidence

FND-001: searched the changed repository for `author_body_locations`, `WALKED_THROUGH`, and `body_locations`, then inspected pinned FSL 8f71444d `fsl-package/src/emit/body_locations.rs::checked_body_locations` and `emit/protocol_locations.rs::term_rows`. The added helper independently selects local function bodies, walks expression references, excludes three control operations, chooses occurrences, constructs function origins and locations, groups applications and assigns term ordinals. Those are the producer's FR-531/FSpec FR-322 semantics, now maintained a second time. The fixture algorithm even hardcodes a flat `[visit]` path while the producer retains checker expression paths. This is more than fixture data: an upstream change now needs a matching edit here. The code-review Duplication and Vendoring rule requires a high finding for a second implementation even in tests; a private upstream helper is not permission to copy its behavior. Use the already established genuine FSL fixture approach for these packages, or write the small expected rows as fixture data rather than a general semantic reconstruction algorithm.

FND-002: `tc_031_ac33_direct_case_call_node_is_unsupported` asserts the case/body Unsupported ids and healthy sibling equality, matching AC-33. It never constructs a plain strict-reader V1 temporal Unsupported request and never asserts `OracleGenerationError::UnauthenticatedRequestedRootKind`, required by AC-41. Remove the AC-41 marker from this test or provide the actual AC-41 scenario in an appropriate test. The other AC-41 binder is outside this PR and was not declared missing by this finding.

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

Examined: FR-021-AC-33, FR-021-AC-41; context: FR-021-AC-11 and the FR-021 Description. Binding checked: direct-case test to AC-33 correct, to AC-41 wrong. All three changed files inspected. No applicable AssuranceProfile exists under spec. No Tokio surface. Rust implementation, testing, safety and boundary references applied. Required external gate policy is superseded by the owner's explicit no-gate instruction.

Focused validation for the lead: `cargo test --locked -p filament-contract-codegen -j4 --lib -- oracle::function ir_988 --test-threads=2`; `cargo test --locked -p filament-contract-codegen -j4 --test it -- tc_031_ac33 tc_031_ac41 --test-threads=2`. For the fixture changes, select the existing `exact_function` cases that consume the shared support; known environment-dependent scratch-crate/FSPEC_DIR failures must be reported separately. Run every cargo command through qsl-slot with the assigned worktree target and requested debug/incremental environment. Test execution remains pending, not claimed green here.

## Dispositions

Round 1 reviewed fix head 24675fac96d9f54b787f0d310c9743ba21555e7a against the frozen review head e9f9ae314360dad753ec2345dca40fd16f3171a6. Original findings above are unchanged.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The full generic body reconstruction algorithm and its call were deleted. The two fixed fixture bodies now carry explicit application/location rows; the IR-988 fixture adds one explicit root row per additional known declaration sharing SOURCE_BODY, without discovering or interpreting arbitrary expressions. |
| FND-002 | fixed | The direct-case test now binds only FR-021-AC-33 and TC-031. The unrelated AC-41 tag was removed. |

After excerpts:

- FND-001, tests/exact_function_support/package.rs: `// Each of these two authored bodies has exactly one application and no expression children.` followed by explicit `body_locations` data whose location is `"path": []`. The former author_body_locations, body_target, expression_references and WALKED_THROUGH no longer exist in the shared fixture reader.
- FND-002: `#[trace("FR-021-AC-33", "TC-031")]` immediately precedes the direct-case regression.

## Disposition round 1 verdict

**MERGEABLE** for this PR's scoped fix: both findings are fixed and no new defect was found in the fix commits. The lead reports 54 filtered library tests and 70 filtered integration tests passing at the fix head. Three generated scratch-crate agreement cases are blocked before generated code executes: their metadata brings filament-contract-model from IR 4c91eff6 while the root lock uses af45a983. The scratch resolver, root manifest/lock, and src/core/profile.rs, which supplies the emitted IR source literal, are unchanged from merge base 556f47c3. This establishes an independent pre-existing pin mismatch, not a regression caused by explicit body fixture rows or the refusal remap. The reviewer did not rerun cargo and does not claim those three agreement cases passed. Full diagnostic text is retained at /private/tmp/qsl-2660-scratch-failures.txt. No new review, aggregate gate, worktree or source edit was performed for this disposition.
