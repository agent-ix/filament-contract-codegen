---
id: SR-17940486709564847048041
title: "IR-854 PR #852 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #852 at 4fdb8a44, git diff origin/main...HEAD (base 6b013e26); IR-854; src/oracle/scalar/native.rs, src/kani/generate/scalar.rs, src/kani/generate/scalar/division.rs, src/core/native_integer.rs, src/oracle/scalar/render.rs, src/kani/generate/outcome.rs, src/oracle/clause/native.rs, src/kani/generate/clause/v2.rs, tests/it/integer_division_harness.rs, tests/it/kani_gate.rs, Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-057
    type: reviews
---

## Summary

Ticket: IR-854. Code-review with the rust-review lane folded in. The PR routes native `div`,
`rem` and `mod` claims through the kernel's `NativeIntegerOperation::Divide` / `Modulo` plan
(filament-exact f1ef4963, unchanged `Cargo.lock`), deletes `NativeDivisionPlanUnavailable` (no
reference remains outside historical `reviews/` files), adds an optional `division_profile` to
`NativeIntegerOperationDecision`, renders the native oracle in `render_division`, and reuses the
general route's expected side and outcome mapping with an `i128` read of the native value.

Independence: the native harness embeds the same `expected_side` text as the general route (the
`i128` relational form with no `rt::`, `nx::` or division-family call); only the oracle and the
mapping's carrier change. The expected side is not derived from the plan or the oracle. Operand
order is unchanged (`interval(0)` dividend, `interval(1)` divisor). `rem` on the native route is
consistent with FR-014-AC-81 (implemented) and FR-057-AC-1; FR-014 is not widened.

`division_profile` serialization: the struct is `Serialize` only, the field is
`skip_serializing_if = "Option::is_none"`, so every existing decision serializes without the key
(byte-identical); a `Some` always emits `"divisionProfile"`, so a present and an absent profile
cannot serialize equal.

Mutation check (each applied alone in the review worktree, `cargo test --test it
integer_division_harness::`, then reverted): division_profile dropped to `None` (4 tests fail);
native `rem` accepting `Completed` at a zero divisor (host controls fail); result domain widened
by one in `select_division` (host controls and the MIN/-1 test fail); rendered law fixed to
truncating (host controls and native mutants fail); `div` identity mapped to the remainder member
(4 fail); `rem` rendered with the quotient member (2 fail); rendered operand intervals swapped
(host controls and MIN/-1 fail); `div` refusal arm removed (3 fail). All killed. Removing the
`skip_serializing_if` attribute is killed by no test (FND-003).

Gates at 4fdb8a44, run by the reviewer: `cargo fmt --check` clean; `cargo clippy --locked
--all-targets -- -D warnings` clean; `cargo deny check` ok; `make spec` exit 0; `--lib` 617 passed
/ 36 ignored; `--test it` 1165 passed / 1 failed / 135 ignored, the failure being
`layout::l_2_the_module_graph_is_acyclic` (import cycle `routed::capability` -> `routed::generic`),
identical on main 6b013e26 and not caused by this PR; the `kani_gate` filter test passes; `git
status` clean. Every new test carries `#[trace]`. No pins, compat layers or tool versions.

Installed Kani, re-run by the reviewer (systemd user scope, MemoryMax 24G, swap 0, scope
`memory.peak` read at the end): `native_constant_divisor_8_bit_div_floor_verifies` Verified,
23.0 s, scope peak 3159097344 bytes (coder 22.5 s, 1902702592; the reviewer's scope also holds the
cargo process); `native_div_floor_mutants_wrong_profile_and_missing_zero_guard_are_falsified`
healthy Verified 20.3 s, law->truncating Falsified 79.3 s playback (-3, 2), law->Euclidean 78.0 s
(7, -3), completes-at-zero 78.9 s (-8, 0), expected zero arm removed 74.3 s (2, 0); whole test
331.08 s, scope peak 7226908672 bytes (coder 331 s, 7228067840). Both reproduce.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-057 Status says the `MIN rem/mod -1` dispositions (AC-5) are delivered "on both routes", but the only native MIN test, `native_route_min_over_minus_one_and_zero_divisor_are_typed_refusals`, covers `div` under the three laws only. No native test shows `i64::MIN rem -1` (any law) or `i64::MIN mod -1` completing `0`, and the native host-control grid is `[-9, 9] x [-4, 4]`. Add the native `rem`/`mod` MIN cases (they need their own fixture because the host test requires a refused pair), or narrow the Status claim to `div`. | tests/it/integer_division_harness.rs:570-597, spec/kani/functional/FR-057-integer-division-scalar-harnesses.md:356 |
| FND-002 | low | `native_div_floor_mutants_wrong_profile_and_missing_zero_guard_are_falsified` asserts only `KaniRunOutcome::Falsified { .. }`; it does not check that the counterexample's operands lie in the fixture's ranges, which FR-057-AC-8 requires ("solver playback of in-domain operands") and the measurement record reports (`-3, 2`, `7, -3`, `-8, 0`, `2, 0`). Assert the decoded operands are inside `[-8, 7] x [-3, 3]`. | tests/it/integer_division_harness.rs:2017-2077 |
| FND-003 | low | The comment on `division_profile` says absent profiles keep the existing selections' identity bytes, but no test pins it: deleting the `skip_serializing_if` attribute leaves every `native` test in `--lib` and `--test it` green (68 + 29 run), while every native add/sub/mul/negate harness's identity bytes would change. Add one assertion that a non-division selection's annotation has no `divisionProfile` key. | src/core/native_integer.rs:52-55 |
| FND-004 | low | `render_division` and `render` do not check that `division_profile` is present exactly for `div`/`rem`: a `mod` decision carrying a profile, or an arithmetic decision carrying one, renders the same oracle as without it while its identity bytes differ. `select_division` never builds one, so it is latent; reject the mismatch in `NativeDivision::of` (return `None` for `mod` with `Some`) and in `render_selected` for the arithmetic arm. | src/oracle/scalar/native.rs:188-200, src/oracle/scalar/native.rs:287-299 |
| FND-005 | low | The Makefile comment says the native tests added to the kani gate take "about 20 s and 80 s"; the measurement record gives seven constant-divisor runs of 17-23 s and a mutant test of 331 s and 7.2 GB peak (healthy plus four mutants of about 75-80 s each). The gate grows by about 8 minutes; state the real figure. | Makefile:112-115 |

## Verdict

The native routing is correct and well guarded: every mutation of the new checks named in the
brief is killed, the expected side stays independent of the plan, and the gates are green apart
from the pre-existing `layout` failure. FND-001 is a real coverage gap behind a Status claim;
FND-002 to FND-005 are low. Mergeable after FND-001 is fixed or the claim narrowed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-006 | high | At dc8d911d the seven FR-057-AC-7 Kani lane tests `native_constant_divisor_8_bit_{div,rem}_{truncating,floor,euclidean}_verifies` and `native_constant_divisor_8_bit_mod_verifies` panic before Kani runs: `compile` fails with "FSL compiles the division source: Recompile(Check { .. Unproved(Range { required: [8, 8], proved: [0, 0], operand: \"0\" }) })". The merge kept main's `source()` guard (`if y != 0 then x div y else 0`), and `Shape::generate` with `narrow: None` compiles that guarded text for the constant divisor `y: Int[8, 8]`, which FSL refuses. All seven runs at dc8d911d: 0.0 s, FAILED (each scope peak about 56 MB, no Kani launch). `--test it` does not show it because the tests are ignored, and `make kani-gate` would fail. The executed AC-7 verdicts claimed in FR-057 Status and the measurement record are not reproducible at head. The general-route reproducer `general_route_constant_divisor_8_bit_floor_div_times_out` fails the same way, and its fixture is main's. Build the constant-divisor shape through the reader fixture (`narrow: Some(..)`) or with an unguarded source for a divisor range that excludes zero, then re-run the seven tests under Kani. | tests/it/integer_division_harness.rs:34-39, tests/it/integer_division_harness.rs:2056-2068, tests/it/integer_division_harness.rs:322-336 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | `native_route_min_rem_and_mod_over_minus_one_complete_zero_and_zero_divisor_is_typed` (6e25298a) host-runs native `rem` under each law and `mod` over `[MIN, MIN+1] x [-2, 0]` with result `[0, 0]`; passes at dc8d911d. |
| FND-002 | fixed | `expect_native_mutants_falsified` decodes the playback with `playback_operands` and asserts the operands lie in `[-8, 7] x [-3, 3]`; reviewer run at dc8d911d: four Falsified with playback (7, -2), (7, -3), (-5, 0), (2, 0). |
| FND-003 | fixed | `division_profile_is_absent_from_non_division_identity_bytes` in src/core/native_integer.rs asserts that an absent profile has no `divisionProfile` key and that a present one differs. |
| FND-004 | fixed | `render` rejects a decision with a profile, `NativeDivision::of` returns `None` for `mod` with a profile, `render_selected` rejects an arithmetic decision with one; `a_division_profile_is_rendered_only_for_div_and_rem` covers all three. |
| FND-005 | fixed | The Makefile comment now gives seven runs of 17 to 23 s and the mutant test at 331 s / 7.2 GB, about 8 minutes. |
