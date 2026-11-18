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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at cb7b27a3: the doc on `support.rs:45-48` now says that a zero `aux` is outside the type, and that the owner refuses it as an invalid runtime input and `emit_method` as `InitialState`. This matches the probe measurement. |
| FND-002 | fixed | Round 1 at cb7b27a3: the comment now attributes the check to FSL's owner evaluator. The CG assertion is `contains("rt::modulo(left, right, &domain, meter)")`, and the string is the verbatim `render.rs:386` literal (pushed unformatted at `render.rs:174`). Measured: the `rt::modulo(right, left, ..)` mutant now fails the test at `constrained_method.rs:433`. |
| FND-003 | fixed | Round 1 at cb7b27a3: the file-wide allow is removed from `support.rs`, and `#[allow(dead_code)]` is now on the lib's `mod support;` (`src/kani/terminal/constrained/tests.rs:11-12`). `clippy --locked --all-targets -D warnings` is clean. |

## Extension review (disposition pass 2)

Scope extension at head 93fb4342: `git diff babf58d5...93fb4342` (63e4d2a4 and 93fb4342). The planner ruled that FSL's unproved-nonzero refusal is intended (FSL #2738) and belongs to the same AC family in the same PR. Ticket: IR-854 (PR title refs QSL-2286).

- `tests/it/integer_division_harness.rs` (examined). `source()` now writes every `x div|rem|mod y` as `if y != 0 then x <op> y else 0`, and `division_node()` routes the division application instead of the body root. No assertion was weakened and no expected harness text was changed. A probe at head on `catalog(Floor)`/`dv` shows that the routed division harness carries `path_condition=[]` and `guard_inputs=[]`, draws `right` over `[-3, 3]` with `kani::assume(right >= -3_i64 && right <= 3_i64)` (0 included), and keeps `expected_zero_divisor`. The guard is used only to make FSL compile the source. It is not folded into the harness, so the harness still checks div/rem/mod on the full declared operands, including the zero divisor, as it did before #851. The `y: Int[0, 0]` case still reaches CG's own `ResultBoundUnreachable`, because the routed item has no path condition to narrow first. No test in the file claims a zero-divisor FSL behaviour that the guard removed. The host tests (`Shape` ranges `(-4, 4)` and `(-8, 7)`) still run the zero divisor through the compiled oracle and expected side.
- Mutation at head (examined). Each mutant was run against the 14 non-ignored `integer_division_harness::` tests, and all 14 pass unmutated.
  - Operand range, `render.rs:210` `>=` to `>`: killed by `the_harness_draws_each_operand...`.
  - Operator, `oracle/scalar/render.rs:360` div `Quotient` to `Remainder`: killed by `host_controls_each_law...` and `host_differential...`.
  - Result bound, `scalar.rs:448` empty-reachable `(1, 0)` to `(lower, upper)`: killed by `lowering_refuses_..._a_zero_only_divisor`.
  - Zero divisor in the expected side, `scalar/division.rs:59` `b == 0` to `b == 1`: killed by `the_seeded_mutants...`, `host_controls...` and `host_differential...`.
  - One mutant first placed at `scalar.rs:1165` survived. That site is not the division render path, so it was re-placed at `render.rs:210` (above). No test survives a mutant on its own region.
- `src/kani/terminal/constrained/tests.rs` (examined). It adds an FSL compile-refusal assertion: `CallSiteRefusal::Compile { code: UndefinedExpression, message contains "unproved-nonzero" }` for `self.n mod self.aux = 0` over the `Small`-typed `aux`. The directly constructed `ClauseEvaluation::Undefined` is kept as the conversion input, per the lead ruling, since it covers the only Undefined-to-EvaluationViolation mapping. The undefined/refused/incomplete loop, the CallFailure case and the malformed/cross-method assertions are byte-unchanged. The test passes at head and fails on main 2ca0277a. The comment wording is FND-005.
- `tests/it/constrained_method/support.rs` (examined). `clause_source` is factored out with no behaviour change. `compile_refusal` uses `expect_err`. The clippy commit 93fb4342 only adds `#[allow(dead_code)]` to the one helper the `it` binary does not use, and switches to `expect_err`. It is clean.
- `tests/it/constrained_method.rs` (examined). Only comments changed: both `<=` stand-in sites now say the revert lands with the tag-pin PR (IR-1566). That ticket's record is gap-analysis FND-003.
- Hygiene (examined). `git diff babf58d5...93fb4342 -- Cargo.toml Cargo.lock deny.toml` is empty. No pin or tool change. The merge base is origin/main 2ca0277a.
- Gates at head (examined).
  - `cargo fmt --check`: exit 0. `cargo clippy --locked --all-targets -- -D warnings`: exit 0. `FSPEC_DIR=... make spec`: exit 0, with pre-existing grammar warnings only in files this PR does not touch.
  - Confined tests pass: `constrained_method::` 10/10, `integer_division_harness::` 14/14, `integer_division_bounds::` 7/7, `kani_tractability::tc_070*` 28/28, and the lib conversion test.
  - Whole `--lib` + `--test it` with FSPEC_DIR set: head has 11 + 6 = 17 failed, and a clean origin/main 2ca0277a worktree has 12 + 24 = 36 failed under the same command. The head failures are a strict subset of the main failures, so the PR adds none.
  - The remaining set is the known one. MalformedWire `check_mode` covers checked_package_evidence x2 (lib and it), opaque_metadata, published_source_associations and the exact_scalar_generation tc_024 x2. The rest are tc_025 x2, cvc5 missing (`FILAMENT_CVC5_BIN`), tc_027_staged_native_observer and bounded_single_run.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `division_node` finds the division by catching the panic `application_in` raises for each absent operator (`std::panic::catch_unwind` over three lookups). Any other panic inside `application_in`, such as a missing function declaration, is swallowed as "not found". It then surfaces as the misleading `holds exactly one division`. A non-panicking lookup (`Option`) would keep each failure distinct. | tests/it/integer_division_harness.rs:42-61 |
| FND-005 | low | The comment says the Undefined conversion rule is "checked on the owner's own value". The value is constructed directly by the test (`ClauseEvaluation::Undefined { reason: "division-by-zero" }`) and is not produced by the owner evaluator. The comment should say "directly constructed" so that it does not read as owner-produced coverage. | src/kani/terminal/constrained/tests.rs:198-202 |
