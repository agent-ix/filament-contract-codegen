---
id: SR-17940486709564847048063
title: "IR-1447 PR #857 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #857 at 4f33c3b2, git diff origin/main...4f33c3b2; IR-1447; spec/kani/functional/FR-015-bounded-kani-obligations.md (AC-152..173), spec/kani/matrix/TC-025-bounded-kani-obligations.md (step 57), spec/kani/matrix/tests.md, docs/qualification/ir1447-kani-control-clause-runs.md, tests/it/kani_obligations_symbolic_domains.rs, src/kani/generate/collection.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-1447. Plan completion: not assessed. The PR moves FR-015-AC-155 and FR-015-AC-159
from PARTIAL to Implemented. Each claim is backed:

- AC-155 is backed by `collection_element_if_lowers_to_the_direct_harness_with_one_if_per_element`
  (identity, one rendered `if`, inside the bounded iteration and before the assertion, length
  `0..=3`) and by `collection_element_if_outside_the_closed_shape_is_refused_with_the_typed_claim_shape`.
- AC-159 is backed by `tc_025_real_kani_collection_if_claim_verifies` and
  `tc_025_real_kani_collection_if_else_mutant_falsifies` (both in the `make kani-gate` lane). The
  reviewer re-ran both: Verified with the cover satisfied, and Falsified at the harness assertion
  with an element of at least 2.

The ACs left PARTIAL were checked. AC-154, 158, 161, 162, 163, 164, 166, 167, 170, 171 and 173,
and FR-025-AC-19, still read PARTIAL with their blocker. No row or test of this PR claims them.
The docs record `docs/qualification/ir1447-kani-control-clause-runs.md` matches the new AC text:
items 0 to 3 of `Int[0, 3]`, mutant else `3`, witness element at least 2, and the removed "Not
run" sentence.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The `spec/kani/matrix/tests.md` row for FR-015-AC-152 to FR-015-AC-173 still reads `PLANNED (IR-1447): no control-construct lowering yet; ... TC-025 step 57 controls have not run`. This has been stale since #840 (let/if/nested executed). It now also contradicts this PR's AC-155/AC-159 Implemented and the TC-025 step 57 status. It should read PARTIAL and name what is executed and what is open. The ticket is the same, so it belongs in this PR. | spec/kani/matrix/tests.md:17 |
| FND-002 | medium | The TC-025 step 57 procedure body still prescribes the retired AC-159 control: `The collection fixture has no subject: its mutant is the generated oracle's element if with else branch 501, falsified ... with a collection holding an element of 500`. The rewritten AC-159 and the executed test use a source-authored clause recompiled by FSL with else `3` and an element of at least `2`. The runner refuses an edited generated oracle. Only the step's status prefix was updated, so the procedure now contradicts its criterion. | spec/kani/matrix/TC-025-bounded-kani-obligations.md:1190-1192 |
| FND-003 | low | Accepted and refused variants of the closed shape are pinned only by the one fixture plus three refusals. The reviewer's FSL-compiled probes show correct behaviour, but no test pins it, for: swapped branches (then literal / else element), both literal, both element, `exists`, non-literal bound, `if` as the right operand, nested `if`, Boolean element, and negative literals. A table-driven extension of the refusal test, plus one accepted swapped-branch identity assertion, would make the closed set load-bearing. | tests/it/kani_obligations_symbolic_domains.rs:4120-4152, src/kani/generate/collection.rs:1094-1142 |

## Verdict

AC-155 and AC-159 are genuinely implemented for the direct collection harness, with executed,
source-bound verdicts. FND-001 and FND-002 are status and procedure staleness and should be
fixed in this PR. FND-003 can ride the same round.

## Dispositions

Round 1, checked against fix commit 8306af02.

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | The `tests.md` row now reads PARTIAL. It lists what is executed (AC-152, 153, 155, 156, 157, 159, 160, 165, 168, 169, 172), what is PARTIAL with its blocker, and the operation-clause `if` as refused (PLANNED). |
| FND-002 | fixed | The TC-025 step 57 procedure now prescribes the `items` 0..3 `Int[0, 3]` fixture, the healthy Verified run, and the source-clause else-`3` mutant Falsified at the harness assertion with an element of at least 2. |
| FND-003 | fixed | The table-driven `collection_element_if_accepts_exactly_the_closed_shape` pins 5 accepted identities and 13 refusals. It fails under each of these mutations: accept a `<=` condition, accept a `>=` comparison, and swap then/else in the reader. The coder left out the Boolean-element case and claimed FSL has no such body. That claim is wrong: in round 0, FSL emitted and IR admitted `forall(f in self.flags: (if f then 1 else 2) <= 2)`, and CG refused it. The omission is acceptable because the case is refused twice over. Dropping the Integer element guard (mutation M8) leaves every test green, because the `integer.lt` condition check refuses any `if` over a Boolean element, so a Boolean row would not make the guard load-bearing. |

Round 1 gates at 8306af02, reviewer-run in its own worktree: fmt ok, and clippy `--locked --all-targets -D warnings` ok. `make spec` returns rc 0 with 8 warnings, the same as main. `--lib` has 603 passed and 14 failed; `--test it` has 1139 passed and 28 failed. These are the same 42 tests that were verified failing on clean origin/main 2ca0277a in round 0. The diff 4f33c3b2..8306af02 touches only the operation guards, the operation render arm, tests, the spec status text and the SR copies. The direct-harness render (`spec.rs`) is unchanged, so the round-0 Kani verdicts (healthy Verified, else-3 mutant Falsified) stand and were not re-run.
