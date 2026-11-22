---
id: SR-202611210022442313144108521
title: "gap-analysis \u2014 QSL-2757 codegen PR882"
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-contract-codegen; Cargo.toml, Cargo.lock, src/core/profile.rs;
  affected equality encoder and genuine enum/quantity callers; repository trace context
review_set: subset
relationships:
- target: ix://agent-ix/filament-contract-codegen
  type: references
---

## Summary

Reviewed immutable PR882 head `57afaab24aa60d3500f9c17f76e225f76f53e833` against `5777baf07230866c7164921002cb475d15c32d0b`. Ticket QSL-2757. Only Cargo.toml, Cargo.lock and src/core/profile.rs change. Sixteen lock records change source only, with no package addition/removal or dependency-edge change. Exactly one reachable package each for exact, semantic-value, IR/model, checked/eval, runtime, observation and walk. Direct FSpec247a394 is unchanged; no MLTL edge is introduced. Profile emitter sources match the new manifest cohort. No gate, native oracle assertion or specification changed.

Independent source trace: FSLb68 fsl-replay/src/admitted.rs760-766 binds the context to final compiled.emitted.package().bytes(); tests/it/admitted_enum_equality.rs48-64 reads those same returned bytes. FSLb68 fsl-package/src/emit.rs7787-7808 uses encode_checked_package_v2_with_protocol_correlation_occurrences with Some(&correlation_rows), even when empty. IRc46 checked_package/v2/mod.rs1102-1110 makes the legacy encoder pass None, which omits the member. Codegen src/oracle/equality/mod.rs782 uses that legacy encoder and855 rejects the differing bytes; kani/generate/enum_scalar.rs291 shares this helper. IR public ProtocolCorrelationProvenance distinguishes Unavailable from Complete, groups() exposes retained table order, clauses() exposes retained source order, and the public encoder writes supplied rows without sorting/deduplication. Preserving this metadata through those owner APIs is sufficient for this concrete mismatch; skipping comparison or synthesizing authority is not justified. No public API gap is established.

No builds or runtime tests were run by this reviewer, as instructed. Parent supplies exact57afa ARCH/Titan evidence: all-target check0, library+it Clippy0, profile6 PASS, accounting1 PASS, first-reader refusal1 PASS; genuine enum1 FAIL and quantity0 PASS/2 FAIL. Actual8 PASS/3 FAIL. The failure mechanism above is independently source-confirmed, while these execution counts remain parent-supplied. git diff --check passed. Final compiler gates and merge remain ROOT responsibility.

No applicable AssuranceProfile exists under spec/. Rust API, implementation, safety, wire and testing guidance was read; no asynchronous implementation changed.

## Verdict

FAIL — inherited repository trace gaps persist; no new trace gap introduced by this diff. No engine-backed repository PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Inherited208 criteria have no bound test, including FR-005-AC-4. | spec/core/functional/FR-005-cli-conformance.md:acceptance criteria |
| FND-002 | medium | Inherited19 criteria bind only ignored tests, including FR-015-AC-30. | spec/kani/functional/FR-015-bounded-kani-obligations.md:acceptance criteria |

## Examined scope

```yaml
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: filament-exact = { version = "0.3.0", git = "https://github.com/agent-ix/filament-exact",
    rev = "a58474cc105a86f3307a11def3fc001e2459679f" }
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/filament-spec-language?rev=b68f04c445396b1e66fa71063337d001f3717dcd#b68f04c445396b1e66fa71063337d001f3717dcd"
- id: src/core/profile.rs
  path: src/core/profile.rs
  role: examined
  excerpt: 'pub const FSL_REPLAY_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\",
    rev = \"b68f04c445396b1e66fa71063337d001f3717dcd\"";'
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
- id: encode_original_package
  path: src/oracle/equality/mod.rs
  role: examined
  excerpt: let bytes = encode_checked_package_v2(
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

Static matrix: quire0.52.0 (engine0.62.2), 1142 criteria:860 tagged,208 untagged,19 ignored-only,55 method-without-symbol. These counts equal PR881; spec/, tests/ and all non-profile source are unchanged against the reviewed base. Coverage reports222 untracked symbols and Rust census2024 candidates/1991 tagged/1950 bound; diagnostics retained in coverage.json. No run evidence was read. Whole-tree reverse-gap/stub inspection is carried forward from the accepted unchanged PR881 audit, with direct inspection of the changed profile and affected equality encoder/enum/quantity callers. No new public behavior, stub, copied owner code, weakening of tests or coverage threshold was introduced by the pin-only diff. This is not a renewed exhaustive semantic or reverse-gap inventory. Optional gap semantic fan-out was not requested and was skipped; targeted source correctness above belongs to code-review. Plan completion: not assessed

Nonblocking follow-up list: retain the inherited whole-repository trace debt (208 untagged,19 ignored-only and existing unmatched-tag diagnostics) as a single follow-up; no new style/test/refactor requests. Parent explicitly accepts the unchanged inherited groups for this minimal wave; whole audit remains FAIL, with no fabricated per-count architecture waiver.
