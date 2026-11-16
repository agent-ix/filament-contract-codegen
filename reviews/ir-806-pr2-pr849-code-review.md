---
id: SR-17940486709564847048035
title: "IR-806 PR-2 PR #849 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #849 at 29227cbd, git diff origin/main...HEAD (base e9918842); IR-806 PR-2; src/kani/identity/leaf_binding.rs, src/replay/composite_leaves.rs, src/replay/witness.rs, src/kani/generate/shadow/composite.rs, src/kani/generate/shadow/source_check.rs, src/kani/identity/shadow.rs, src/lib.rs, tests/it/composite_playback_decode.rs, tests/it/composite_shadow_kani.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-033
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-2). Code review with the rust-review lane folded in, of the composite leaf
binding check, the playback decoder and the operand rebuild.

Mutation runs in the review worktree (env-switched mutants, one build, the 11 new tests plus
`composite_shadow`): each refusal reason in `check_drawn` (missing, duplicate path, unknown path,
kind, declared, drawn, out-of-order), the global duplicate-name and unknown-node checks, the
pinned value bound, guard evaluation (`admits` always true: 4 tests fail), Absent/Null swap,
option presence, pinned rebuild value, decoder bound check, decoder binding check, surplus entry,
byte limit at `>=` and at limit+1, value-count limit at limit+1, and the generation-time
`check_leaf_bindings` call each fail at least one test. Four mutants survive: the contiguity check,
the generation-time `check_leaf_call_order` call, the pinned-on-a-parameter check and the
per-operand pinned path duplicate check (FND-001 to FND-003).

Layering: `src/kani/identity/leaf_binding.rs` imports only `core`, `kani::identity::shadow` and
`oracle::equality::closure`; nothing under `src/kani` imports `crate::replay`, and the layout
tests pass in the whole `--test it` run. Playback input reaches no `unwrap`, `expect` or
indexing that can panic: guard indices always point at earlier leaves of the same operand, and
`i64::try_from` is checked. Recursion in `walk` and `Builder::build` follows the declaration
closure, and a declaration that reaches itself is refused before either runs on playback.

Real Kani: the reviewer re-ran
`tc_048_installed_kani_playback_of_a_falsified_mutant_decodes_and_rebuilds_both_operands` once
(24G scope, 600 s cap); see Coverage. The test feeds `KaniRunOutcome::Falsified { counterexample }`
from the real run into `parse_assertion_playback` and `reconstruct_composite_operands`; it builds
no value by hand. It carries the lane's `#[ignore = "kani lane: run serially through make
kani-gate"]` and is selected by the Makefile's `shadow_kani` filter, like its neighbours.

## Verdict

**Changes requested (no high findings).** The refusals, guard evaluation, slot-state distinction,
literal pinning and limits are all backed by tests that fail under mutation. Two checks the spec
relies on (contiguity, generation-time call order) have no test that can fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The contiguity check in `check_leaf_bindings` (a parameter's leaves form one block) survives deletion: the `interleaved` case swaps leaf 1 with the last leaf, which also breaks the per-node order, so `check_drawn` refuses it as `OutOfOrder` anyway, and the test asserts only `is_err()`. The decoder reads each node's leaves as one contiguous run (`draw_order`, composite_leaves.rs:108), so an interleaved identity whose per-node order is intact would be misread. Add a case that interleaves two nodes' blocks with each node's own order kept and assert `OutOfOrder`. | src/kani/identity/leaf_binding.rs:422-438, tests/it/composite_playback_decode.rs:225-228, src/replay/composite_leaves.rs:108 |
| FND-002 | medium | Removing the generation-time `check_leaf_call_order` call fails no test. FR-025-AC-9 says "generation runs both and yields no harness on a refusal", but only the function is tested on hand-edited source; no test reaches a `CompositeShadowRefusal::LeafBinding { CallOrder }` from generation. Add a crate-internal test that drives `generate_composite_with` with a renderer or identity that disagrees, or mark the generation half as Inspection. | src/kani/generate/shadow/composite.rs:1272-1275, tests/it/composite_playback_decode.rs:239-264 |
| FND-003 | low | Two pinned-leaf checks survive deletion: the check that a pinned leaf names a literal position (`on_a_parameter` asserts only `is_err()`; `Missing` for the right operand fires first), and the per-operand pinned path duplicate (the `duplicated` case reuses the name, so the global name check fires first). Assert the typed reason, and add a duplicate path with a fresh name. | src/kani/identity/leaf_binding.rs:338-344, src/kani/identity/leaf_binding.rs:447-456, tests/it/composite_playback_decode.rs:335-344 |
| FND-004 | low | `check_leaf_call_order` turns a source that does not parse, or has no proof function, into an empty draw list (`unwrap_or_default`). An identity with no drawn leaves (literal/literal) therefore passes against any source, and a parse failure is reported as `CallOrder` with an empty `drawn` instead of its own reason. | src/kani/generate/shadow/composite.rs:1324-1328 |
| FND-005 | low | `LeafBindingRefusal::Conflict.member` is a `String` holding one of four fixed words (`kind`, `declared`, `drawn`, `value`); a small enum keeps it typed. The leaf-kind-to-primitive map is written twice (`primitive_of` in composite.rs, `primitive` in composite_leaves.rs). | src/kani/identity/leaf_binding.rs:68-74, src/kani/generate/shadow/composite.rs:1301-1306, src/replay/composite_leaves.rs:46-51 |

## Coverage

Gate at 29227cbd in the review worktree: `cargo fmt --check` clean; `cargo clippy --locked
--all-targets -- -D warnings` clean; `cargo deny check` ok; `make spec` exit 0 (pre-existing
grammar warnings only); whole `--lib` 617 passed / 36 ignored; whole `--test it` 1166 passed /
128 ignored, no failures; `git status` clean after the mutation runs were reverted. Cargo.toml
and Cargo.lock are unchanged in the diff. Every new test carries `#[trace(...)]`.
