---
id: SR-17940486709564847048047
title: "code-review of filament-contract-codegen PR #854 FSL unproved-nonzero fixture fix"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; PR #854 at 7667c4b9 over main 1da743a3; src/kani/terminal/constrained/tests.rs, tests/it/constrained_method.rs, tests/it/constrained_method/support.rs, tests/it/integer_division_bounds.rs, tests/it/kani_tractability.rs"
review_set: subset
---

## Summary

This is a code review with the rust-review lane, scoped to `git diff origin/main...HEAD`. The PR has no ticket of its own. Its title refs QSL-2286, and findings are posted to IR-854 as the dispatch directs. The PR changes tests only. Merged #851 bumped filament-spec-language from 14c0fd8a to 9c09cc9e, and FSL now refuses a source `div`/`mod`/`rem` whose divisor it cannot prove nonzero (`undefined_expression (unproved-nonzero)`). The PR repairs the fixtures that rule broke.

## Examined scope

- Baseline (examined). A scratch detached worktree at main 1da743a3, with the target shared with the review worktree, reproduces the failures: the lib `conversion_preserves_original_causes...` test, the 5 `integer_division_bounds` tests, 2 `constrained_method` tests, `kani_tractability::tc_070_a_measured_bad_row...`, the 2 `kani_obligations_v2_control::tc_025_*` tests and the 10 `integer_division_harness::*` tests. That is 11 fixable failures plus 10 that belong to #852. Each one is `unproved-nonzero`, except tc_025 and the harness tests.
- Head results (examined). `--lib` gives 617 passed, 36 ignored, 0 failed. `--test it` gives 1152 passed, 12 failed, 127 ignored. The 12 failures are exactly the 10 `integer_division_harness` tests and the 2 `tc_025` tests. QSL-2361 B owns tc_025.
- `integer_division_bounds` `division_package` (examined). It uses genuine FSL source when the divisor range excludes zero, and otherwise the pre-existing `reader_division_operation`. The reader package's result is typed plain Integer, so derivation takes the same unnarrowed operand path (`lower.rs:1161-1227`) as the FSL package did. Generation and native execution of the scratch crate then run unchanged. For `mod`, forcing `profile = None` matches FSL, which lowers `mod` without a law.
- Mutation `render.rs:386` `rt::modulo(right, left, ..)`: 18 of 18 mod native cases fail in both generated-values tests, including the reader-path `y: Int[0, 0]` cases 33 to 35.
- Mutation `lower.rs:1207` zero-magnitude guard removed: killed only by the reader-path zero-divisor cases (`bounded_division...`, `genuine_modulo...`, `remainder_inferred_interval...`). This shows the reader path really guards the derivation lines that FSL can no longer reach.
- Lib `conversion_preserves...` (examined). The hand-built `ClauseEvaluation::Undefined { reason: "division-by-zero" }` is still a real input to `classify_constrained_method`. Mutating `terminal/constrained.rs:222` (constraint Undefined to `S::Refused`) fails the test at `tests.rs:204`. The weakening is acceptable here: the classifier is the code under test, and the test already hand-builds `owner_fault` the same way.
- `same_named_general_clause_bodies...` (examined). Changing `=` to `<=` keeps the test's intent, because the two sources still differ only in a constant. Mutating `generate/constrained.rs:351` (a constant oracle-content digest) fails the test at line 709. `rt::modulo` mutants cannot kill this test by design: it is about method identity, not arithmetic.
- `boolean_and_partial_conditions...` (examined). Rendering `mod` as `rt::divide(Euclidean, Remainder, ..)` is killed at line 430 by `contains("rt::modulo")`. The `rt::modulo(right, left, ..)` mutant survives (FND-002).
- `kani_tractability::tc_070` (examined). Adding `offset > 0 and` lets FSL prove the divisor nonzero. The clause still holds the 21-bit multiply, and all three asserted dispositions are unchanged, so the intent of FR-056-AC-4 is preserved. Not mutated.
- Hygiene (examined). No production file is changed, because `src/kani/terminal/constrained/tests.rs` sits behind `#[cfg(test)] mod tests;`. `Cargo.lock` and `Cargo.toml` are unchanged. No pins or tool versions changed. No new test function was added; only helpers were added, so no new `#[trace]` tag is needed, and the existing tags are unchanged.
- Gates at head (examined). `cargo fmt --check` passes. `clippy --locked --all-targets -D warnings` passes. `make spec` exits 0, with pre-existing grammar warnings only in files this PR does not touch. `cargo deny check` fails only on advisories: RUSTSEC-2026-0543 (chrono unmaintained), the known failure also present on main. Bans, licenses and sources pass. `git status` is clean.

## Verdict

The fixes are sound, and the reader-fixture substitution genuinely exercises the same derivation, generation and native-execution code. Mutants confirm this, including mutants that only the reader path can kill. One medium coverage finding is in the gap-analysis SR. The findings here are all low: a doc comment that measurement shows is false, a test comment that overstates what the test guards, and a file-wide `dead_code` allow where a narrower placement works.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `nonzero_aux_document` doc says a zero `aux` "still reaches the owner evaluator as a runtime outcome"; measured: the typed snapshot refuses it (`Refused(Coded{Admit, invalid_runtime_input, invalid-value})`), and `emit_method` with initial aux=0 gives `Err(InitialState)`. This contradicts the test comment in `boolean_and_partial...`. | tests/it/constrained_method/support.rs:48-50 |
| FND-002 | low | The new comment "only a real modulo separates it from a constant-true oracle" describes FSL's owner evaluator, not the CG method. The CG output is checked only by `contains("undefined constraint evaluation")`, which `render/constrained.rs:212` emits for every condition, and by `contains("rt::modulo")`. The `rt::modulo(right, left, ..)` mutant survives this test (it is killed in `integer_division_bounds`). | tests/it/constrained_method.rs:419-431 |
| FND-003 | low | `#![allow(dead_code)]` is file-wide and also applies in the `it` binary, where every helper is used. Measured: removing it and putting `#[allow(dead_code)]` on the lib's `mod support;` passes `clippy --all-targets -D warnings`, and keeps dead-code detection for the integration binary. | tests/it/constrained_method/support.rs:5 |
