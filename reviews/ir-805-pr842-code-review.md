---
id: SR-17940486709564847048011
title: "IR-805 PR #842 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #842 at a1a4c6b6, git diff origin/main...HEAD (304 files); Cargo.toml, Cargo.lock, Makefile, deny.toml, src/kani/generate/float_scalar.rs, src/kani/generate/corpus/bounded_kani_corpus.rs, src/replay/**, tests/it/scratch_crate.rs, tests/exact_scalar_support/package.rs, tests/it/integer_division_harness.rs, tests/it/kani_obligations_v2_clause_controls.rs"
review_set: subset
---

## Summary

Ticket: IR-805 (with IR-260, IR-854, IR-1547 and IR-1509). This is code-review with the
rust-review lane folded in. I reviewed the semantics of the change, not every renamed line.

Rename and lock:
- All Cargo dependencies are `branch = "main"` and there is no `[patch]`, `rev`, `tag`, compat
  re-export, symlink or environment workaround.
- `quire-canonical` keeps its name because the upstream repo still has that name
  (`filament-canonical` does not exist).
- The `quire-rs?rev=71a1f97` lock source comes from `filament-core-data`'s own rev pin (main had
  `rev=5b4be36`). It is upstream's pin, not CG's.
- Cargo.lock changes net once against main. The WIP commit f9834488 touched it and f1b15877
  supersedes that change.

The `acfilament_checked_closure` spelling:
- Confirmed upstream at the locked fsl-replay 41d41a8:
  `fsl-replay/src/application.rs:787 pub fn acfilament_checked_closure` (the sed rename turned
  `acquire` into `acfilament`).
- A scan of every locked filament-* checkout finds exactly three mangled identifiers:
  `acfilament_checked_closure`, `acfilament_checked_closure_with_integer_division` and the private
  `refilament_protocol_results_for_refusals` (fsl-package `emit.rs`).
- CG uses only `acfilament_checked_closure`, in `src/oracle/function/kani_harness/tests.rs:5` and
  in its spec prose (AD-004, FR-021, TC-031).
- CG introduces no mangled name of its own.

Other checks:
- deny.toml: the exceptions and allow-git removed are for crates and sources no longer in the
  lock. There is no blanket allow, and `cargo deny check` passes (main fails 16 license and
  10 source errors).
- `#[ignore]` count is 157 under tests/ (0 under src) on both main and HEAD, and the diff adds or
  removes no ignore.
- No `#[test]` was removed.
- IR-260 code: the Contract IR `RequiresBound` record is read through `CheckedPackageV2::lower`
  under `scalar_profile()` (`require_bounds: true`), which gives one record per request, so
  `.first()` is sound. The stopgap branch is removed and the unreadable-bound check is kept.
- New `ReplayResult::ProtocolAnalysis` arms are mapped to `WrongArm` and `CgDefect`, which is
  sound.

## Verdict

**PASS with low findings.** The gate was run at the head:
- fmt, clippy `-D warnings`, `cargo deny check` and `make spec`: all exit 0.
- whole `--lib`: 604 passed, 0 failed, 36 ignored.
- whole `--test it`: see the gap-analysis SR for the counts.

IR-260 mutant: I removed the Contract IR `RequiresBound` read in `settle()`. Then
`an_unpinned_float_operand_requires_a_bound_naming_its_type` fails, and it passes again with the
read restored. Details are in the gap-analysis SR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Rename leftovers: the bounded Kani corpus still names its emitted FSL artifacts `corpus/{name}.qsl` and `corpus/{name}.dependency.qsl`, but upstream filament-spec-language at the locked rev uses `.fsl` (17 string uses, zero `.qsl`). Scratch directories in two tests are still prefixed `quire-codegen-`. Scenario: a consumer that keys on the source extension sees a stale language name in generated corpus artifacts. Fix: `.fsl`, and `filament-codegen-` prefixes. | src/kani/generate/corpus/bounded_kani_corpus.rs:690-691; tests/it/bounded_kani_corpus.rs:485; tests/it/capability_settlement.rs:1093 |
| FND-002 | low | The PR says deny.toml "drops stale entries", but four remain, and `cargo deny check` at the head warns about each: `advisory-not-detected` for `ignore = ["RUSTSEC-2026-0009"]` (also `make cargo-audit --ignore`), and `license-not-encountered` for `BSD-3-Clause`, `CDLA-Permissive-2.0` and `ISC`. They already existed on main and do not loosen anything. Fix: remove them, or keep them with a stated reason. | deny.toml:7-9,63-65; Makefile:170 |

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| Cargo.toml | examined | first-party deps renamed to filament-* / fsl-*, all `branch = "main"` |
| Cargo.lock | examined | one net bump; sources filament-*, quire-canonical, quire-rs rev from filament-core-data |
| deny.toml | examined | removed ix-cli-kit, quire-rust-extraction, quire-code-parse, filament-spec-language, fsl-attrs exceptions and two allow-git sources |
| Makefile | examined | LOCAL_PATCHES renamed; fsl-attrs dropped |
| src/kani/generate/float_scalar.rs | examined | `settle` reads `CompleteLoweringRecordV2::RequiresBound` from Contract IR; stopgap removed |
| src/kani/generate/corpus/bounded_kani_corpus.rs | examined | `.qsl` artifact paths |
| tests/it/scratch_crate.rs | examined | `per_label_package`, `tests_ran` |
| tests/exact_scalar_support/package.rs | examined | float comparison operands carry `float_rounding`; cohort size 4 |
| src/oracle/function/kani_harness/tests.rs | examined | `acfilament_checked_closure` import |
