---
id: SR-17940486709564847047993
title: "IR-854 PR #836 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-contract-codegen PR #836 at 89a7cc86, git diff origin/main...HEAD; IR-854; src/kani/generate/scalar.rs, src/kani/generate/scalar/division.rs, src/kani/generate/outcome.rs, src/kani/identity.rs, src/kani/abi.rs, src/oracle/operator_catalog.rs, src/oracle/scalar/{derive.rs,lower.rs,mod.rs,render.rs}, src/routed/capability.rs, Makefile, tests/it/integer_division_harness.rs, tests/it/integer_division_bounds.rs, tests/it/capability_settlement.rs, tests/it/kani_obligations.rs, tests/it/routed_generation.rs, tests/it/scalar_playback.rs, tests/it/kani_tractability.rs, tests/it/kani_gate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-contract-codegen/FR-057
    type: reviews
  - target: ix://agent-ix/quire-contract-codegen/FR-014
    type: reviews
---

## Summary

Ticket: IR-854. The PR derives `quire.op.integer.rem` (FR-014-AC-81), adds `Divide`, `Remainder`
and `Modulo` to the scalar Kani projection, renders a division harness
(`src/kani/generate/scalar/division.rs`) with an independent expected side and an exact outcome
mapping, refuses the native route as `NativeDivisionPlanUnavailable`, records 13 general-route
measurements (all `TimedOut` at 600 s), and ships three `not_tractable` rows at 4 bits.

Reviewer reproduction at 89a7cc86 (`CARGO_TARGET_DIR=/home/peter/dev/.review-targets/ir854`,
under `flock /tmp/cargo-slot.lock`): see Verdict for the `--lib` / `--test it` result. No Kani
run was repeated (13 x 600 s at about 8.5 GB each, and the measured outcome is non-completion).

Soundness walk of the expected side (by reading the renderer and the generated text):

- Windows: truncating `a >= 0 ? [0, |b|-1] : [-(|b|-1), 0]`; floor `b > 0 ? [0, b-1] : [b+1, 0]`
  (code `(-reach, 0)` with `reach = -b - 1`); Euclidean `[0, |b|-1]`. Matches FR-057's table.
- `div` admission: `b > 0`: `r(L) >= minW && r(U) <= maxW`; `b < 0`: `r(L) <= maxW && r(U) >= minW`,
  gated by `!zero_divisor`. Checked: `r` is strictly monotone in `q` with step `|b|` and `W` holds
  exactly `|b|` values, so each inequality is equivalent to the bound on `q*`. A completed
  quotient is checked by `r = a - q * b` in `W` (quotient then divisor). All arithmetic is `i128`;
  the largest product, `2^63 * 2^63`, fits. Quotient magnitude `2^63` (`MIN / -1`) decodes through
  `to_u64` without loss.
- `rem`/`mod`: `i64::rem_euclid(a, b)` only when `b != 0` (the mapping computes `expected` only
  when `!zero_divisor`) and `b != -1`; the shift `e - |b|` lands in `W` and stays congruent.
- The `div` side holds no `/`, `%` or division-family call; the `rem`/`mod` side holds exactly the
  one permitted `i64` `rem_euclid`; no side divides at 128 bits. The `rt::` names in
  `outcome_agrees` belong to the mapping, not the expected side.

The native refusal (`select_native_integer`) fires for every division operation after lowering
and before any capacity check, carries `node_id` and `operation_identity`, and emits no harness;
lowering refusals (`BoundNotI64`, `ResultUnreachable`) still precede it, which FR-057-AC-1 allows.
The inferred `rem` interval (`[0, B-1]` Euclidean, `[-(B-1), B-1]` truncating/floor from the
divisor magnitude) matches FR-014-AC-81. `ScalarOperands::reachable` bounds a quotient by the
dividend magnitude and a remainder by the divisor magnitude (sound), and a zero-only divisor
settles `ResultUnreachable`.

Lane hygiene (brief question 3): five commits, all IR-854; every changed file and hunk belongs to
this change; the worktree is clean; the shared stash stack still holds the IR-260 entry
(`stash@{0}`, `code/ir-260-float-scalar-row`), so it was not popped into this branch, and the two
files that entry also touches (`src/routed/generate.rs`, `tests/it/kani_tractability.rs`) carry
only this PR's one-line `division_scalar: None` and row-count hunks. No foreign content leaked.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | Tests carry trace tags for criteria they cannot evidence. `the_seeded_mutants_of_each_claim_change_the_routed_source_and_cover_both_groups` (default lane, source-diff only) and `general_route_zero_completing_mutant_times_out` (asserts `TimedOut`) are tagged `FR-057-AC-8`, which requires installed-Kani `Falsified` with playback replay; `quoin matrix` now reports FR-057-AC-8 as `tagged`. The two `*_times_out` tests are tagged `FR-057-AC-7`, whose text says "Timeouts alone do not satisfy this criterion". Retag them to TC-071 and AC-9 (measurement) only, or untag. | tests/it/integer_division_harness.rs:1381-1384, tests/it/integer_division_harness.rs:1400-1403, tests/it/integer_division_harness.rs:1544-1546, tests/it/integer_division_harness.rs:1616-1619 |
| FND-002 | medium | `integer_division_harness` joins `KANI_FILTERS`, so every `make kani` / `make kani-gate` now runs three ignored tests that pass only by burning the full 600 s ceiling at about 8.5 GB each (about 30 min of the serial lane per gate) to re-assert non-completion. Keep the reproducers runnable by name but out of the default gate filter (or behind an explicit opt-in), and record that in the measurement doc. | Makefile:112, tests/it/kani_gate.rs:18-29, tests/it/integer_division_harness.rs:1365-1379 |
| FND-003 | low | Adding `Divide/Remainder/Modulo` to `integer_scalar_operand_count` silently widens the FR-032 scalar playback decoder to division identities (the routed replay builder still rejects them with `ScalarBuildError::Operation`). The decoder doc still says it "Accepts only integer add, subtract, multiply and negate", and the FR-032-AC-3 test simply dropped `quire.op.integer.div` from its refused list with no assertion of the new behaviour. Either keep the decoder refusing division until AC-8 replay exists, or update the doc and assert decode-accepts plus builder-refuses. | src/kani/abi.rs:30-36, src/replay/scalar.rs:20, src/replay/scalar_builder.rs:191-206, tests/it/scalar_playback.rs:688-694 |
| FND-004 | low | The rewritten `kani_selected_division_settles_and_renders` keeps the old doc ("The FR-015 refusal wins before mode settlement") and its refusal-era trace tags (FR-015-AC-15 `no_derivable_claim`, FR-019-AC-29 form-before-mode precedence) although it now asserts only `Supported` and `harness.is_some()`; the dropped assertions on the typed checked-bound domain were not replaced by any check of the rendered harness (profile, ranges). Update the doc, drop tags it no longer evidences, and assert the harness's `division_scalar`. | tests/it/capability_settlement.rs:2134-2146, tests/it/capability_settlement.rs:2226-2231, tests/it/capability_settlement.rs:2283-2291 |

## Verdict

Code is correct on every path checked: the expected side is independent and division-free as
specified, the outcome mapping is exact (healthy accepts real outcomes; the five mapping mutants
are each killed on the host over the real oracle), the host differential compares the rendered
expected side and the generated oracle with quire-exact over the named vectors and the
`i64::MIN`/`MAX` grid, and the native refusal matches FR-057-AC-6. The findings are trace,
gate-cost and stale-test hygiene; none is a wrong result. Mergeable after FND-001 and FND-002
are fixed in this PR (both small); FND-003 and FND-004 are low and may ride the same fix round.

Reviewer gates at 89a7cc86: whole `cargo test --locked --no-fail-fast --lib --test it` with
`QUIRE_CVC5_BIN` set: lib 585 passed / 36 ignored, `it` 1090 passed / 103 ignored, 0 failed (a
first run without `QUIRE_CVC5_BIN` failed only the environmental
`production_refinement_live_solver_handoff_reaches_qsl_checker`, untouched by this PR);
`cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean. The six rewritten
tests that asserted "division unrendered" now assert the landed behaviour and pass; the claim is
sound, with the hygiene gaps of FND-003 and FND-004.
