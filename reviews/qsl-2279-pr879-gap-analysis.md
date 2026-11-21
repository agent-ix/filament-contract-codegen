---
id: SR-202611210022442313144108503
title: gap-analysis of codegen PR 879 dependency window
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-contract-codegen; QSL-2279; Cargo.lock, Cargo.toml, src/core/ir/checked_term.rs,
  src/core/ir/clause.rs, src/core/ir/clause_control_fixture.rs, src/core/profile.rs,
  src/kani/generate/clause/v2.rs, src/kani/generate/collection.rs, src/kani/generate/graph.rs,
  src/kani/generate/scalar.rs, src/oracle/equality/mod.rs, src/oracle/equality/resolution.rs,
  src/oracle/function/mod.rs, src/oracle/scalar/derive/tests.rs, src/oracle/scalar/lower/tests.rs,
  src/routed/capability.rs, src/routed/contract_package_tests.rs, tests/exact_function_support/package.rs,
  tests/it/capability_settlement.rs, tests/it/exact_function_agreement.rs, tests/it/exact_scalar_agreement.rs,
  tests/it/kani_obligations_state_frame.rs, tests/it/kani_obligations_symbolic_domains.rs,
  tests/it/routed_generation.rs, tests/it/scalar_source_forms.rs
review_set: subset
---

## Summary

Ticket QSL-2279. Frozen PR agent-ix/filament-contract-codegen#879, head cd204194ab68fe6dd712e95ac855a2508e1e31f1, base e127f4837faf21edbc155b329e373f2a33ed0229. gap-analysis reviewed the authorized dependency window and consumer accommodations.

## Verdict

CONDITIONAL — no scoped defects found; mergeable only after the root lead verifies the focused checks green on cd204194ab68fe6dd712e95ac855a2508e1e31f1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence

All 25 changed files were inspected. Normal/dev Cargo dependencies, Cargo.lock, existing resolved metadata and all eight profile source constants select one source per first-party crate. Selected runtime is 75688ead74e265bcb654de871577570b7b15e19c, IR/model 20a4dfbf29e979da4326ac81f85e8266e464519f, FSL 529282e9dc28cc6be8b382ae634636e3b4fe4a14, FSV 6e809434d5fd7bde4c5b88ec40cd809efcca595d, eval/checked 89573102cc309941786e3693b2f71e254dddc91c, FSpec 247a394157b58843643a61dfde16280c25d733c0, observation 1e356f99358aaf0f9bbc42dd3451c44cc61a4d06; exact remains b26a8e4c592503fccdc4a0e3d2cce23dd8487616.

The selected model defines optional binder only on control-let bindings; selected FSL emits it from its owning admitted ID. Changed consumer patterns borrow name/value without reconstructing the term or deleting binder metadata. Newly explicit None values construct ordinary record/type/function-body fixture members. No schema, reader, compatibility layer, feature disable, ignored test, threshold reduction or changed assertion was introduced. The compact location API defines relative up/down paths; each of the three authored entries is a separate function-body root, so up=0/down=[] retains its former empty path meaning. Existing source identities and admission calls remain intact.

Profile manifests still obtain dependencies from one authority. Inspected fixture controls compare producer IDs after real strict admission and compare changed bodies independently. The admitted function generation control retains source visit counts, repeated output equality and refusal checks. CI workflows, scripts and specs are unchanged. No new stub, duplicate implementation, copied schema or unsafe/concurrency surface was found in the patch.

Verification: independent cargo fmt --check and git diff --check exited 0. A read-only Python check verified all eight source constants occur in Cargo.toml and each selected lock package has one source. Existing /tmp/qsl-2279-codegen-wave2-metadata.json was read, without resolving a new graph. No Cargo build/test/clippy was run by this reviewer: the dispatch reserves all such checks to the root lead through draco:QSL:code. The eleven exact PR-body test commands plus focused lib/it clippy remain pending at review time. Native generated crate compilation and direct/generated agreement are therefore conditional, not claimed passed.

The installed tools report quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.36.0. These are newer than the supplied release notice; no older binary fallback was used. No applicable AssuranceProfile is declared in spec/. Composition PR #3287 is outside this selected window; this review claims no composition-text coverage.

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix --scope /home/peter/dev/worktrees/qsl-2279-codegen-wave2 --format json exited 0; no run evidence read. This is the dispatch-authorized manual pin-scoped gap review, not a repository-wide gap PASS. Selected FR-014-AC-8, FR-037-AC-6 and FR-037-AC-9 are engine-tagged. Other criteria were read as context only; pre-existing whole-repository gaps, including FR-039-AC-4 untagged, were not newly attributed to this dependency-only change. Reverse-gap/stub/integrity inspection covers the changed consumer and fixture paths only. Optional semantic repository audit and plan audit were not run. Spec-review and spec sub-analyses were not applicable because spec/ and plan/ are unchanged.

## Reviewed units

```yaml
scope:
- id: FR-037-AC-6
  path: spec/core/functional/FR-037-fixture-identity.md
  role: examined
  excerpt: Both fixture builders preserve every producer-assigned structural application
    `CheckedNodeId` in the assembled node and its dependent references; an ungrouped
    fixture's requested ID equals the reader-admitted node ID. Inspection finds no
    local application ID derivation, application preimage encoder, version constant
    or digest call for that identity.
- id: FR-037-AC-7
  path: spec/core/functional/FR-037-fixture-identity.md
  role: context_only
  excerpt: A recursion-group fixture retains the complete group, each member's exact
    producer-assigned structural ID, including its own ordinal, and references to
    those exact IDs; the strict reader admits the valid group without imposing contiguous
    group positions. A missing referenced member or a declaration path inconsistent
    with `node_id.path` produces the reader's typed refusal and no admitted fixture.
    Exceeding the complete-package byte ceiling retains the reader's typed limit result
    and admits no fixture.
- id: FR-037-AC-8
  path: spec/core/functional/FR-037-fixture-identity.md
  role: context_only
  excerpt: A canonical enum-member fixture carries its producer-assigned structural
    `CheckedNodeId`; the reader admits its member declaration and exact reference.
    Changing the case through the producer yields the corresponding structural member
    ID without a CG nominal preimage or digest. In-process agreement keys never compared
    with a checked-package identity remain classified as fixture-only.
- id: FR-037-AC-9
  path: spec/core/functional/FR-037-fixture-identity.md
  role: examined
  excerpt: Positive exact-scalar and composite-equality fixture packages remain reader-admitted.
    A declaration whose path disagrees with `node_id.path` is refused by the reader.
    A changed body under the same valid structural key is detected separately by comparison
    with the expected admitted source body and is not silently credited as the original
    fixture; neither negative control is repaired before its assertion. A different
    valid unused ordinal with consistent references is not itself a reader refusal.
- id: FR-037-AC-10
  path: spec/core/functional/FR-037-fixture-identity.md
  role: context_only
  excerpt: Deterministic fixture-only keys remain distinct from canonical checked-package
    identities. Ordinary canonical-value identities use their owning API where needed;
    originating package identity comes from the producer and reader admission, with
    no CG package preimage, package digest refresh or independent application identity
    rule.
- id: FR-039-AC-1
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: Each of two genuine FSL-admitted source graphs with a different clause
    or pure-function operation lowers through IR to its own admitted FR-330 package.
    CG's production settlement and generation use each package's typed graph and accounting
    for its exact request index and node ID; supplying the two packages with each
    one's corresponding node ID at the same request position changes the generated
    bytes to reflect the represented operation while backend and generation context
    stay fixed. If the production API still accepts an original checked graph, holding
    that legacy input fixed across the two c
- id: FR-039-AC-2
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: A mixed request with a representable clause or function and an independent
    `unsupported`, `requires_bound` or `invalid_input` sibling retains one typed decision
    per bound driver item, including duplicate requested IDs, and emits only the representable
    sibling's artifact. An interleaved context-only root retains its own exact accounting
    position and key without creating a driver decision. For two FSL scalar items
    sharing one expression key but selecting different checked application/conversion
    pairs, CG retains each pair's own admitted FR-049 result bound; a conversion outside
    its driver's cont
- id: FR-039-AC-3
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: A selected dependency function reference resolves only through the reader-validated
    selected package identity and target key. A missing, wrong-identity or stale selected
    dependency refuses before CG; an identically keyed local node or extra unselected
    dependency cannot redirect generation.
- id: FR-039-AC-4
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: For a genuinely admitted FR-330 package, CG binds a `protocol`/`protocol_reference`
    carrier's exact `body.target` to the local `protocol` node, retaining its typed
    key, requested identity and source occurrences; changing that local target and
    recomputing the admitted carrier and package identities changes the selected target
    and package identity. A matching key only in a selected dependency does not bind
    locally. An absent local target, a present non-`protocol` target and a target
    with unequal `semantic_type` are refused by the FR-322 reader before CG with the
    respective `missing_declaration`/
- id: FR-039-AC-5
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: A malformed or noncanonical FR-330 wire, wrong version, invalid accounting/source
    map, stale semantic identity or incomplete reader result reaches no CG consumer
    and emits no artifact. CG preserves the reader's typed refusal or limit result
    and performs no second parse or checked-graph fallback.
- id: FR-039-AC-6
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: An item whose admitted package accounting is `failed`, `invalid_body` or
    `body_incomplete`, or whose `lowered` form is unrepresentable by the selected
    backend, emits no artifact and retains a distinct typed cause; the package byte-ceiling
    case emits no artifact for any sibling.
- id: FR-039-AC-7
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: RETIRED, replaced by FR-039-AC-11; the former `ContractPackageV1.identity()`
    and digest-shaped source assumption is obsolete and earns no implementation credit.
- id: FR-039-AC-8
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: 'For each admitted `lowered` bound driver item, Contract IR''s authoritative
    traversal is total over the required root, bounds and selected dependencies, and
    the generated claim''s reached claim, bound and dependency keys and dependent
    declaration closure equal its output. An unresolved required reference cannot
    reach CG as an admitted `lowered` item: Contract IR records a typed item refusal
    or its strict reader refuses the package; a refused or incomplete reader result
    yields no item claim or artifact. Inspection finds no per-node ID from a second
    lowering or wire-fragment hash.'
- id: FR-039-AC-9
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: Before matching positions, CG verifies that the owning reader-admitted
    V2 source's exact top-level name and complete lock equal the admitted V1 `package.source`
    name and complete lock, either directly or through Contract IR's explicit authenticated
    source handoff. A wrong-name or wrong-lock source refuses the whole request before
    any artifact even when its requested key resolves to the same structural key as
    V1 accounting. For a matching source, CG matches a bound driver item, an excluded
    accounting-only position and another bound driver item to `feature_accounting`
    by position and equal sourc
- id: FR-039-AC-10
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: Given a bound driver item followed by an excluded accounting-only position,
    a selected conversion context root outside the preceding item's contiguous span
    yields typed `IncompleteContext` for that item when the complete root sequence
    is otherwise valid. A context-only root directly after the excluded position and
    before the next bound driver item is orphaned and causes a typed whole-request
    correspondence refusal before any artifact, even when its accounting position
    and source-bound structural key match. A conversion root immediately before the
    excluded position remains in the preceding item
- id: FR-039-AC-11
  path: spec/core/functional/FR-039-contract-package-consumption.md
  role: context_only
  excerpt: Generated scalar, composite equality and exact function claims each serialize
    `contract_package_identity` equal to the admitted FR-330 `package.source` structural
    package name plus complete lock, replacing the removed `ContractPackageV1.identity()`
    accessor, and `node_id` equal to the original requested V2 `CheckedNodeId`, including
    graph-local indexes; their containing claim-map entries use that same `node_id`,
    with no generated-claim `ir_id` key or second public V1 node identity. CG matches
    the request to V1 accounting through equal source-bound structural keys resolved
    by `ContractSourceKey
- id: FR-014-AC-8
  path: spec/oracle/functional/FR-014-exact-scalar-oracles.md
  role: examined
  excerpt: The generated crate declares `publish = false`, names the direct exact/FSV
    and Runtime residue dependencies from the one profile authority without a replay-context
    dependency for Boolean/integer scalar proofs, contains no charge amount (every
    charge comes from runtime metering), and compiles.
bindings:
- test_id: tests::manifest_sources_match_this_crates_manifest
  ac_id: FR-014-AC-8
  trace: correct
- test_id: fixture_identity_tests::composite_application_keeps_producer_assigned_node_id
  ac_id: FR-037-AC-6
  trace: correct
- test_id: fixture_identity_tests::changed_body_and_unused_ordinal_are_measured_separately_from_identity
  ac_id: FR-037-AC-9
  trace: correct
```
