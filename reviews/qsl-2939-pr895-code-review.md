---
id: SR-202611210022442313144108534
title: "code-review — QSL-2939 codegen PR895"
type: SpecReview
analysis: code-review
scope: agent-ix/filament-contract-codegen; git diff origin/main...dc472f76 (Cargo.toml, Cargo.lock, build.rs,
  src/core/profile.rs, CheckedTerm::Aggregate call sites in src/ and tests/, tc_068, tc_029_ac12)
review_set: subset
---

## Summary

Ticket: QSL-2939. PR #895, head `dc472f7642b422eb780f09be3d22a0379638bb57`, scoped to `git diff origin/main...HEAD`
(28 files). Code-review with the Rust lane (rust-review, rust-style; repository CLAUDE.md, clippy.toml,
rustfmt.toml and `[workspace.lints]` read). No cargo was run by this reviewer, as the dispatcher instructed;
build, clippy and test results are the coder's and arch's reports.

Pins. Cargo.toml moves FSL (`fsl-replay`, `fsl-package`) to 9af79beb and `filament-contract-ir` plus both
`filament-contract-model` entries (normal and dev) to 30d96b9c. Cargo.lock agrees: 8 FSL source lines and the 2
contract-ir/model lines change, and the lock holds exactly one `filament-contract-model`, one
`filament-contract-ir` and one `filament-specification-qualification`. FSL 9af79beb's own workspace pins
`filament-contract-model` at 30d96b9c, so the graph unifies on one copy. Between FSL 5275e508 and 9af79beb the
consumed crates change only dev-dependencies, so no lock dependency list should change, and none does. The new
`toml` 0.8.23, `toml_edit` 0.22.27, `toml_datetime` 0.6.11, `serde_spanned` 0.6.9 and `winnow` 0.7.15 entries are
a coherent cargo resolution of the new build-dependency. Nothing indicates a hand edit. `make test` runs with
`--locked`, so the coder's green run also confirms the lock. The `agent-ix-semantic-schema` advance
(filament-core-data `branch=main`, c23980ea to 9c4f6dd0) comes in through the unchanged `quire-rs` pin. Between
those commits nothing under `crates/semantic-schema/` changes, and nothing it `include_str!`s changes
(`schema/semantic/v1/*`, `packages/semantic-core/generated/json-schema/*`). The compiled crate is byte-identical,
so this is only a lock refresh.

build.rs (QSL-2939). It emits `cargo:rerun-if-changed=Cargo.toml`, which is the only input it reads. It parses the
manifest with `toml::Table`, reads only `[dependencies]`, and for each of the eight names spells
`git = "...", rev = "..."` or `git = "...", tag = "..."`. It panics, failing the build with a named message, on
these cases: a missing table, a non-git dependency, both rev and tag, or neither. A `branch =` dependency
therefore fails the build, which enforces the "no branch" rule structurally. `{source:?}` produces a valid Rust
string literal. The generated items carry `///` docs, so the public re-exports in `src/lib.rs:181-183` stay
documented. The build-dependency is minimal (`default-features = false`, `parse`), and no shell is involved. No
copy of a revision remains in `src/`, `tests/`, `scripts/`, `docs/` or `spec/`. Only historical `reviews/` files
mention old revisions. The deleted `manifest_sources_match_this_crates_manifest` is not re-created elsewhere.
Every remaining consumer (`tests/it/*`, `src/oracle/*/tests`) reads the constants, so emitted manifests carry
exactly the Cargo.toml sources by construction.

Pin-move fixes. All 30-odd production and test reads now use `CheckedTerm::Aggregate { members, .. }`. Every
construction that sets `temporal_profile: None` is in a test fixture (`scalar.rs` tests, `equality` tests,
`function` tests, `lower/tests.rs`). Upstream (contract-model 30d96b9c `typed_terms.rs:2225-2233`, FSL #3553)
gives `temporal_profile` only to a finite-domain hyperproperty node's own body aggregate and leaves it absent
everywhere else. The fixtures model option, record, integer-range, function and parameter bodies, so `None` is
correct for them. CG refuses temporal and hyperproperty roots as unsupported (`src/oracle/mod.rs:32-44`,
`src/oracle/function/mod.rs:2465-2510`). None of the production readers that now ignore the field reads a
hyperproperty body. `scalar/lower.rs` `references()` looks for node references, and `temporal_profile` is a
definition reference, not a node id. No case is hidden.

tc_029_ac12 relaxation. The test is traced to FR-018-AC-12 ("names direct exact/FSV and root replay dependencies
through the one manifest authority ... compiles"). That AC requires no release tags. The old assertion (tag, or
the `fsl-replay` line) was already false on main: `filament-exact`, `filament-semantic-value` and the runtime have
been rev pins there. The new assertion still bans `branch =` and still requires a pinned tag or rev on every
`github.com/agent-ix/` line. Equality with Cargo.toml now holds by construction through build.rs. This does not
weaken the AC. The FR-018 Outputs prose that still says "release tags" is recorded by the gap-analysis artifact.

## Verdict

PASS with three low findings, all non-blocking. The pins are consistent, there is one model copy, the lock is a
cargo-generated refresh, the build script is correct and robust, the `None` constructions are right, and the test
relaxations do not weaken their criteria. Mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `scripts/cg_gates.rs` `touches_kani` lists `Cargo.toml`/`Cargo.lock` but not `build.rs`; a build.rs-only change rewrites every emitted oracle manifest yet the Kani scope reports "not required". | scripts/cg_gates.rs:74-85 |
| FND-002 | low | tc_068 asserts on `CheckedTerm`'s `Debug` text; it broke on an additive upstream field and will break on the next one. A pattern match on `Aggregate { members, temporal_profile: None }` with empty members would assert the intent. | tests/it/kani_obligations_v2_option.rs:287-290 |
| FND-003 | low | The `function_oracle_crate_manifest` doc still says the CG tag "predates the pins above"; no pins are spelled above any more (they are generated from Cargo.toml). | src/core/profile.rs:30-33 |

## Examined scope

```yaml
- {id: Cargo.toml, path: Cargo.toml, role: examined, excerpt: "fsl-replay/fsl-package rev 9af79beb; filament-contract-ir, filament-contract-model (dep and dev-dep) rev 30d96b9c; [build-dependencies] toml 0.8 parse"}
- {id: Cargo.lock, path: Cargo.lock, role: examined, excerpt: "8 FSL + 2 contract-ir source lines; toml/toml_edit/toml_datetime/serde_spanned/winnow added; agent-ix-semantic-schema branch=main c23980ea -> 9c4f6dd0"}
- {id: build.rs, path: build.rs, role: examined, excerpt: "SOURCES table of 8 constants; rerun-if-changed Cargo.toml; source_of panics unless exactly one of rev or tag"}
- {id: src/core/profile.rs, path: src/core/profile.rs, role: examined, excerpt: "include!(concat!(env!(\"OUT_DIR\"), \"/dependency_sources.rs\")); manifest_sources_match_this_crates_manifest deleted"}
- {id: CheckedTerm-Aggregate-reads, path: src/core/ir/clause.rs, role: examined, excerpt: "CheckedTerm::Aggregate { members, .. } across src/core/ir, src/kani/generate, src/oracle, src/routed, tests/it"}
- {id: CheckedTerm-Aggregate-constructions, path: src/oracle/equality/mod.rs, role: examined, excerpt: "temporal_profile: None in test fixtures only (scalar.rs, equality, function, lower/tests.rs)"}
- {id: tc_068, path: tests/it/kani_obligations_v2_option.rs, role: examined, excerpt: "Some(Aggregate { members: [], temporal_profile: None })"}
- {id: tc_029_ac12, path: tests/it/composite_equality_generation.rs, role: examined, excerpt: "line.contains(\"tag = \\\"\") || line.contains(\"rev = \\\"\")"}
- {id: FR-018-AC-12, path: spec/oracle/functional/FR-018-composite-equality-oracles.md, role: context_only, excerpt: "The generated crate declares publish = false, names direct exact/FSV and root replay dependencies through the one manifest authority, contains no charge amount and no planned pair count, and compiles."}
```

## Coverage

Rust lane ran (rust-style core, implementation and testing references). Gates were not run by this reviewer
(dispatcher: DO NOT BUILD); fmt, clippy and test results are the coder's report. The remaining reds (QSL-2953
seven, QSL-2955 four) are not plausibly caused by this diff. QSL-2955's doubled result type comes from FSL
693862f0c9 (#3266), which is already an ancestor of the old pin 5275e508. QSL-2953's groups (positional ids,
missing rekey helper, sidecar admission, stale measured rows, solver handoff) touch no code path this diff
changes behaviourally. The upstream delta (FSL #3573 rekey ordering, #3553 temporal_profile) could change failure
text but not create these failures. This was not confirmed by execution.
