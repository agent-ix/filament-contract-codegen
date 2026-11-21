---
id: SR-202611210022442313144108507
title: gap-analysis — QSL-2757 codegen PR881
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-contract-codegen; QSL-2757; Cargo.toml, Cargo.lock, src/core/profile.rs;
  existing generated ABI tests and repository trace context
review_set: subset
---

## Summary

Reviewed frozen PR881 head `af976d99da418b5094d9c099c0400833cc35fdd8` against `6e85e782a7de6bf1fd6842660d78600df3e72997`. Ticket: QSL-2757. The entire diff is Cargo.toml, Cargo.lock and six dependency-source strings in src/core/profile.rs. No implementation, API, tests, gate, oracle, measurement table or specification changes occur.

Cargo.toml normal/dev dependencies and all eight emitted dependency-source constants were parsed and checked statically for agreement; Cargo.lock contains one selected package for each direct owner. All 15 changed lock records keep package names/versions; no package is added or removed. Fourteen records change source only; fsl-eval additionally gains sha2, which is present in the exact bf74 owner manifest. Exact b26a8e4c and FSpec247a394 remain unchanged. The FSL workspace at bf74 names precisely SVfa5e78a, IR/modela03c86b, checked/evala0e8e12 and obsa583c51. The exact runtime284cb6d manifest was independently inspected in /home/peter/.cache/cargo-qsl2757/git/checkouts/filament-contract-runtime-121b4a7c51bd0d37/284cb6d (HEAD verified284cb6df9e2c917deba987d952e47e5ffc384ca1) and selects the identical exact/SV revisions. The f5c913 source in FSL's own manifest is the semantics dev-only fsv-failing-allocator, absent from CG Cargo.lock, and is not a second semantic-value crate.

The three oracle manifest generators and scalar observer consume the common source constants. The pre-existing function oracle CG branch-main line is unchanged. The explicit matched-cohort authorization governs this bounded refresh; no source-policy rewrite was made. Inspection found no copied upstream implementation/schema/fixture, private API, fallback, weakened budget, changed expected side or new quantity ABI support.

Retained tests are substantive: the real admitted enum test compiles actual generated code in direct/transitive contexts; its inner Cartesian pairs compare completed outcomes, ordered charges, all counters and each denial with direct owner evaluation, plus distinct-declaration and mutation controls. The two quantity tests keep authentic source, exact unsupported-quantity refusals, generated healthy siblings, complete output isolation, missing context and foreign context checks. Their source and assertions are unchanged. The bf74 versus dab8c60624 reachable owner delta is confined to simulation model/order/sample and integer-range endpoint retention in lowering; replay/package and enum/quantity owner APIs are unchanged. Int[0,9] healthy sibling is within the changed lowering surface.

No applicable AssuranceProfile was found in spec/. Repository AGENTS.md delegates to CLAUDE.md; deny.toml and rustfmt.toml were inspected. The existing interim exact-rev policy is unchanged. No safety/concurrency, wire or lifecycle implementation changed.

Local checks: git diff --check and direct rustfmt --check --edition2021 src/core/profile.rs exited0. No Cargo, build, compilation, Clippy, test, Kani, runtime or measurement command was run by this reviewer. The dispatch reports initial587987e5 all-target check0, Clippy0, six profile tests, genuine enum1 and quantity2 passing; exactaf976 reports all-target0, profile6 and quantity2 passing. Initial enum/Clippy evidence is explicitly earlier-cohort evidence retained for unchanged paths, not rerun ataf976. Distinct tests9, run at new cut8. These execution results are ROOT-supplied exact-head one-line service attestations under the architecture evidence policy; they are not independent reviewer executions. Raw log transfer is not required by that policy. Final-current-main rebase, exact final-head CG all-target check, separate exactbf74 FSL analyze/root all-target check, freshness and match-head merge remain the lead's responsibility.

## Verdict

FAIL — the whole-repository matrix retains208 untagged and19 tagged-by-ignored-test criteria. No new criterion or binding changes are introduced by this diff; this is not an engine-backed repository PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Inherited208 criteria have no bound test, including FR-005-AC-4. | spec/core/functional/FR-005-cli-conformance.md |
| FND-002 | medium | Inherited19 criteria bind only ignored tests, including FR-015-AC-30. | spec/kani/functional/FR-015-bounded-kani-obligations.md |


## Examined scope

```yaml
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: filament-semantic-value = { version = "0.1.5", git = "https://github.com/agent-ix/filament-semantic-value",
    rev = "fa5e78aa71fcea5418492b67f13a33124af84459" }
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/filament-spec-language?rev=bf74ebb2cd9fda495aaff52a75db8ea23309eba0#bf74ebb2cd9fda495aaff52a75db8ea23309eba0"
- id: src/core/profile.rs
  path: src/core/profile.rs
  role: examined
  excerpt: 'pub const FSL_REPLAY_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\",
    rev = \"bf74ebb2cd9fda495aaff52a75db8ea23309eba0\"";'
- id: FR-014-AC-8
  path: spec/oracle/functional/FR-014-exact-scalar-oracles.md
  role: examined
  excerpt: The generated crate declares `publish = false`, names the direct exact/FSV
    and Runtime residue dependencies from the one profile authority without a replay-context
    dependency for Boolean/integer scalar proofs, contains no charge amount (every
    charge comes from runtime metering), and compiles.
- id: FR-018-AC-34
  path: spec/oracle/functional/FR-018-composite-equality-oracles.md
  role: examined
  excerpt: The actual generated ABI executes ordered and unordered enum comparisons
    over all declared cases, including a case absent from every selected literal and
    an enum parameter with no member literals. Exact shape variants and canonical
    positions come from genuine facade/FSV values. Same-spelled cases from distinct
    declarations retain distinct identities; matching and mismatching declaration
    comparisons preserve the owning equality disposition.
- id: FR-018-AC-36
  path: spec/oracle/functional/FR-018-composite-equality-oracles.md
  role: examined
  excerpt: 'Emitted source uses only structural checked-package `CheckedNodeId` and
    case identifiers for lookup and contains no NodeKey constructor/decoder or generated
    nominal preimage/hash. Only the prepared wrapper''s private genuine index reaches
    check_equality. A private environment-only fault fixture still satisfies AC-8,
    every original charge-stop case still satisfies AC-9, and a wrong item/descriptor
    prepared wrapper refuses before charging. Record/tuple/recursive/collection cases
    remain required by AC-2 and AC-21 through AC-30; an absent owner lookup capability
    is recorded as unresolved migration '
- id: FR-018-AC-37
  path: spec/oracle/functional/FR-018-composite-equality-oracles.md
  role: examined
  excerpt: Equality's generated manifest and native scratch consumers use the one
    profile authority's direct exact/FSV/replay dependencies, while ordinary Boolean/integer
    FR-014 scalar proof manifests and oracle bodies acquire no replay source-compilation/context
    leg. Native generated enum acceptance is reported separately from original TC025/TC027
    proof qualification; neither result substitutes for the other.
- id: FR-018-AC-45
  path: spec/oracle/functional/FR-018-composite-equality-oracles.md
  role: examined
  excerpt: After the existing setting/accounting and original-package context guards
    succeed, generation-time construction and emitted preparation supplied Some(context)
    attach a clone of that context's complete authoritative unit_table() to the constructed
    TypeEnvironment, retaining declared but unused units without rebuilding a table,
    deriving UnitId/NodeKey or substituting another package's table. Valid context-free
    None behavior remains unchanged. Wrong-package refusal retains its whole-request
    precedence; direct and nested Quantity items retain their existing typed Unsupported
    behavior and refusal p
- id: FR-021-AC-32
  path: spec/oracle/functional/FR-021-function-application-oracles.md
  role: examined
  excerpt: Neither the schedule crate nor any concrete oracle requires `filament_contract_runtime::exact`,
    an alias, wrapper or compatibility layer; a concrete oracle uses `fsl_replay::acquire_checked_closure`,
    `CheckedClosureHandle::lookup_callable` and `registry()`, `ApplicationRegistry::register`
    and `begin_call`, and `CheckedApplicationSession::step` or `run` through the permitted
    FR-485 facade, without direct FSL evaluator or checker internals. CG and any emitted
    oracle crate hosting a generated body depend directly on `filament-eval`, `filament-exact`
    and `filament-semantic-value` beside `fsl-repla
- id: FR-005-AC-4
  path: spec/core/functional/FR-005-cli-conformance.md
  role: examined
  excerpt: 'OPEN: after IR-348 V1 retirement, agreement must be remeasured across
    surviving executable, proptest, Kani and coverage paths; removal alone is not
    agreement. Original criterion retained: Executable, proptest, Kani, and coverage
    semantics agree or retain an explicit difference.'
- id: FR-015-AC-30
  path: spec/kani/functional/FR-015-bounded-kani-obligations.md
  role: examined
  excerpt: With the installed backend, the operation contract verifies for a healthy
    subject and is falsified, naming the postcondition, for a subject mutated to debit.
```

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix --scope /home/peter/dev/worktrees/qsl-2757-codegen-current --format json, quire0.52.0 engine0.62.2, exits0.1142 criteria:860tagged,208untagged,19tagged-by-ignored-test,55method-without-symbol. No run evidence read. The complete coverage_matrix is exactly equal to the retained PR880 static matrix; inherited marker warnings persist, not an engine failure. Installed quoin0.36.8 and spec-artifacts-process@v0.36.0 are newer than the release notice; no old binary fallback occurred.

Reverse inventory: spec/spec.md owns the oracle, strategy, Kani, replay, publication and evidence modules. For this bounded PR, existing profile/manifests belong to FR-014/018/021 and the preserved enum/quantity ABI tests bind their existing criteria. Changed behavior count0; new untraced behaviors0; new source/test stubs0; new coverage-inflation changes0. Full historical reverse-gap/stub re-audit was outside dispatch scope; no full repository assurance clearance is claimed. No implementation source was added. No optional semantic fan-out (no opt-in) and no plan assessment.

The two findings record inherited debt. The lead may append accepted-no-change under the explicitly authorized coherent wave, with a bounded rationale. Architecture authorization is not a claimed explicit per-count waiver. This review does not repair historical traces, adjust measurement/kernel tables, claim current CLI acceptance of QSL-2756/2214, or close QSL-2757.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | ROOT accepts the unchanged inherited trace debt for this three-file coherent dependency/profile wave under ARCH's explicit whole-wave authorization and no-further-approval instruction. This is ROOT's scoped disposition, not an ARCH per-count waiver. Spec/tag lines and the computed matrix exactly match PR880; the update introduces no new trace gap. No unrelated repair or follow-up tickets are required. Whole-repository gap verdict remains FAIL; no engine-backed PASS is claimed. QSL-2757 remains open pending driver and four CLI acceptances. |
| FND-002 | accepted-no-change | ROOT accepts the unchanged inherited trace debt for this three-file coherent dependency/profile wave under ARCH's explicit whole-wave authorization and no-further-approval instruction. This is ROOT's scoped disposition, not an ARCH per-count waiver. Spec/tag lines and the computed matrix exactly match PR880; the update introduces no new trace gap. No unrelated repair or follow-up tickets are required. Whole-repository gap verdict remains FAIL; no engine-backed PASS is claimed. QSL-2757 remains open pending driver and four CLI acceptances. |

This section records ROOT’s owner disposition only. No additional source review, matrix computation, runtime checks or review round was performed. Original findings and FAIL verdict are retained.
