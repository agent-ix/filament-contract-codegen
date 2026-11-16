---
id: SR-17940486709564847048029
title: "IR-806 PR-1 PR #848 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #848 at 653b0170, git diff origin/main...HEAD (base 149c98d2); IR-806 PR-1; src/kani/terminal.rs, tests/it/composite_parity_converter.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-1). Code review with the rust-review lane folded in: the new
`ProofStrength::parity_refinement` (a `const fn` with one wildcard-free match, `#[must_use]`) and
five new integration tests in `tests/it/composite_parity_converter.rs`.

Checked clean: the projection rows match FR-029-AC-17; no wildcard arm in production or test
matches; no `unsafe`, no panics in production code; the tests drive FSL's real
`settle_verified_shadow` and the public `verified_shadow_terminal_value` converter, with no verdict
double; every new test carries `#[trace]` with its FR-029 criterion and TC id; Cargo.toml and
Cargo.lock unchanged; no pins, compat layers or tool versions in code or docs.

Mutation runs (own worktree, each reverted, tree clean afterwards): M1 `ShadowProvedRefinementSampled`
projected to `Exhausted` fails `tc_048_every_strength_projects_onto_its_refinement_state` and
`tc_048_each_strength_settles_to_the_terminal_value_fsl_reports`; M2 the settled expectation for
`inconclusive` changed from `ResourceExhausted` to `TimedOut` fails the settle test; M3
`shadow_proved_refinement_sampled` removed from the FR-028-AC-17 spec row fails
`tc_048_the_enumerated_strengths_equal_the_closed_set_of_the_spec`; M4 a new `TerminalPairError`
variant fails to compile (E0004 non-exhaustive). Removing a projection arm fails to compile by
construction. The tests are not tautological. The second assertion of the settle test
(`bound == report.terminal_value()`) is close to an identity, but the per-strength expectation table
in the same test carries the check.

Gate at 653b0170 (own worktree and target, cargo-slot lock, `--locked`, FSPEC_DIR and
FILAMENT_CVC5_BIN set): `cargo fmt --check` clean; `cargo clippy --locked --all-targets -- -D
warnings` clean; `cargo deny check` ok; `make spec` exit 0 (pre-existing warnings only); whole
`--lib` 617 passed / 0 failed / 36 ignored; whole `--test it` 1155 passed / 0 failed / 127 ignored
(the coder's counts reproduced); `git status --porcelain` empty afterwards; Cargo.lock unchanged.

## Verdict

**PASS for the code-review subset.** No code defect. Evidence and spec-status findings are in the
gap-analysis and spec-review SRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined `src/kani/terminal.rs` (the `parity_refinement` impl and import changes) and every added
test in `tests/it/composite_parity_converter.rs` (`every_strength`, the five `tc_04x_*` tests).
