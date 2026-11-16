---
id: SR-17940486709564847048026
title: "IR-264 PR #847 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #847 at 1ae756d3, git diff origin/main...HEAD (base 8c80f954); IR-264 PR5; --version, src/kani/run/versions.rs, src/kani/run/tool.rs, src/kani/run/execute.rs, src/kani/run/execute/two_phase_tests.rs, src/kani/run/mod.rs, src/kani/run/scalar.rs, src/kani/terminal/constrained/tests.rs, src/lib.rs, tests/it/shadow_kani.rs, tests/it/terminal_map.rs, tests/it/composite_equality_generation.rs, tests/it/composite_shadow_families.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-039
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-264 (PR5). Code review with the rust-review lane folded in: the FR-028-AC-23 tool-version
probe (`KaniToolVersions`, `KaniInstallation` cache, the probe calls in `run_phase_single` and
`run_group`), the five stand-in tests and the ignored real-backend test, and the reference-row test
`tc_025_a_reference_composite_settles_with_fr_018s_refusal_and_no_harness` with its helper
`reference_record_entry`.

Checked clean: the probe goes through `run_bounded_launcher` (new process group, group signal and
reap before return, 60 s wall budget, capture limit); only the run's own launcher is invoked, no PATH
lookup; absence is typed and does not refuse the run; every production `KaniExecutionEvidence` is
built through `with_launch`, which now takes the versions (single run, both phases of a two-phase
run, every batch member); `KaniExecutionEvidence` is `Serialize` only (no `Deserialize`) and no
golden JSON fixture holds serialized evidence, so the added `toolVersions` member is additive for
consumers; Cargo.lock and Cargo.toml unchanged; no pins or compat layers; new tests carry `#[trace]`
except the parser unit test (FND-004).

Mutation runs (own worktree, each reverted): M1 single-run versions replaced by a constant absence
fails 4 of the 5 stand-in tests (the batch test stays green, correctly); M2 batch versions replaced
fails `tc_039_a_batch_members_evidence_records_the_versions`; M3 cache removed fails
`tc_039_the_launcher_is_probed_once_per_installation`; R1 FR-018's `PackageAccounting` refusal
replaced by `Unsupported(model)` and R2 the shadow generator not carrying the refusal unchanged
each fail the reference test. The tests are not tautological.

Gate at 1ae756d3 (own worktree and target, cargo-slot lock, `--locked`, FSPEC_DIR and
FILAMENT_CVC5_BIN set): `cargo fmt --check` clean; `cargo clippy --locked --all-targets -- -D
warnings` clean; `cargo deny check` ok; `make spec` exit 0 (pre-existing warnings only); whole
`--lib` 613 passed / 0 failed / 36 ignored; whole `--test it` 1150 passed / 0 failed / 127 ignored
(the coder's counts reproduced). Ignored real-backend test
`tc_039_installed_kani_evidence_records_the_launchers_own_versions`, one job inside the lock under
`systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`: passed, recorded
`Reported { kani: "0.68.0", cbmc: "6.11.0" }`, equal to a separate `cargo-kani --version`; scope
wall 0.72 s, cgroup `memory.peak` 292,483,072 B.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR commits a stray file named `--version` at the repository root (a stand-in Kani report, `{"metadata":{"version":"1.0"},"verification_results":{"results":[]}}`). Cause: the probe builds `Command::new(&installation.launcher).arg("--version")` with no `current_dir`, so it runs in the caller's working directory; every stand-in whose script writes its report to its last argument (`> "$last"`) writes `./--version` there. Reproduced: after deleting it, the whole `--lib` run recreates `./--version`, and the whole `--test it` run creates an untracked `./arguments.txt` (the symbolic-graph/domain stand-ins write `"$@"` to `arguments.txt` relative to cwd). Every test run now writes into the source tree. Delete the file, and give the probe a deterministic working directory (or make the stand-ins answer `--version` without writing) so tests leave the tree clean. | --version, src/kani/run/versions.rs:80-82, src/kani/run/execute.rs:2645-2665, tests/it/kani_obligations_symbolic_graphs.rs:788 |
| FND-002 | medium | A failed probe is cached forever. `KaniToolVersions::of` stores whatever `probe` returns in the installation's `OnceLock`, including `ProbeFailed`, and the probe is called with the run's own `Cancel`. A run whose token is cancelled therefore probes with a cancelled token (not started, `ProbeFailed`), and every later run of that installation and its clones records `ProbeFailed` without probing again. Reproduced with a temporary test: a pre-cancelled run followed by a normal run on the same stand-in records `Unreported { ProbeFailed }` on the second run with zero probes, where `Reported { 9.8.7, 6.5.4 }` is expected. A transient timeout or spawn failure is cached the same way. Cache only a successful read (or retry on `ProbeFailed`), and do not probe under a cancelled token. | src/kani/run/versions.rs:72-77, src/kani/run/execute.rs:998-1000, src/kani/run/execute.rs:1375 |
| FND-003 | medium | The cache is keyed by the installation object, not by the launcher it probed. `versions: Arc<OnceLock<..>>` is shared by every clone, while `launcher` is a `pub` field, so `let mut b = a.clone(); b.launcher = other;` makes `b` report `a`'s cached releases under a different launcher. That is the misattribution FR-028-AC-23's adverse case names (two runs under different backends read as one). `PartialEq` also ignores the cache. Key the cache by the launcher path, give a clone a fresh cache when the launcher changes, or make `launcher` private. | src/kani/run/tool.rs:66-72, src/kani/run/tool.rs:119-122 |
| FND-004 | low | The parser unit test `the_real_launcher_output_parses_and_noise_does_not` and the `KaniToolVersions::Reported` doc comment hard-code the installed releases `0.68.0` and `6.11.0` (`cargo-kani --version` on this host prints exactly those). The brief rules out tool versions hard-coded in docs and tests; use neutral sample releases as the stand-in tests do (`9.8.7`, `6.5.4`). The test also has no `#[trace]` tag (FR-028-AC-23). | src/kani/run/versions.rs:28, src/kani/run/versions.rs:111-134 |

## Verdict

Not mergeable. The probe is bounded and typed, the field reaches every evidence path, and the tests
fail under each mutant tried; the real backend records the same releases `cargo-kani --version`
prints. FND-001 has to go before merge: the PR commits a junk file and makes every test run write
into the source tree. FND-002 and FND-003 are real cache defects (reproduced and read
respectively) in the AC this PR claims.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The probe now runs under `&Cancel::new()`, so it no longer sees the run's cancellation. A run whose token is cancelled still probes after it settles (the reproduction below records `Cancelled` with `Reported` versions and spends one probe), and a launcher that hangs on `--version` holds a cancelled run for up to the 60 s `PROBE_BUDGET` before it returns. The fresh handle fixes the cache poisoning, but a run's cancellation should not wait on an unrelated probe. Skip the probe (record `ProbeFailed`, uncached) when the run's token is already cancelled, or probe under the run's token and leave the failure uncached. | src/kani/run/versions.rs:97-121 |

## Dispositions

Round 1, at a48c7b25 (fix commits 4a0181c1 and a48c7b25). Gate at a48c7b25, own worktree and target, cargo-slot lock, `--locked`: fmt, clippy `--all-targets -D warnings`, `cargo deny` and `make spec` pass; whole `--lib` 616 passed / 0 failed / 36 ignored; whole `--test it` 1150 passed / 0 failed / 127 ignored; `git status --porcelain` empty after both runs (no file written into the tree). Ignored real-backend test `tc_039_installed_kani_evidence_records_the_launchers_own_versions`, one job, 24G scope: passed, Kani 0.68.0 / CBMC 6.11.0, scope wall 0.28 s, `memory.peak` 108,904,448 B.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `--version` git-rm'd; the probe sets `current_dir` to the launcher's own directory (temp dir if none); the symbolic-graph and symbolic-domain stand-ins exit on `--version` without writing; `tc_039_the_probe_runs_in_the_launchers_directory` fails when `current_dir` is removed (mutant run). Tree clean after whole `--lib` and `--test it`. |
| FND-002 | fixed | Only a `Reported` result is cached, and the probe uses its own `Cancel`. `tc_039_a_failed_probe_is_not_remembered` fails when failures are cached (mutant run). A cancelled run followed by a normal run (temporary review test) records `Reported` on the second run, so the structural argument holds. The new cancellation side effect is FND-005. |
| FND-003 | fixed | The cache is keyed by `LauncherIdentity` (canonical path, length, mtime), checked on every use; `tc_039_a_clone_with_another_launcher_reports_that_launchers_versions` fails when the key check is removed (mutant run). |
| FND-004 | fixed | The parser test uses synthetic releases `1.2.3-test` / `4.5.6-test` and carries `#[trace("TC-039", "FR-028-AC-23")]`; the doc comment no longer names a release. |

Round 2, at 39571271 (fix commit 39571271). Gate at 39571271, own worktree and target, cargo-slot lock, `--locked`: fmt, clippy `--all-targets -D warnings`, `cargo deny` and `make spec` pass; whole `--lib` 617 passed / 0 failed / 36 ignored; whole `--test it` 1150 passed / 0 failed / 127 ignored; `git status --porcelain` empty afterwards.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | `of` takes the run's `Cancel`. An already-cancelled token with no cached success returns `Unreported { NotProbed }` without spawning or caching anything, and the probe runs under the run's token, so a cancellation during the probe stops it (`ProbeFailed`, not cached). The non-cancelled path is otherwise unchanged: same cache lookup, same `current_dir`, same budget, only a `Reported` result cached; the 14 `tc_039_` tests pass. Mutants: with the cancelled check removed, `tc_039_a_cancelled_run_does_not_probe_and_a_later_run_still_records_the_versions` fails (`ProbeFailed` != `NotProbed`); with the check removed and the probe given a fresh `Cancel`, it fails after the 45 s hang (`Reported` != `NotProbed`). `NotProbed` is listed in FR-028-AC-23. |
