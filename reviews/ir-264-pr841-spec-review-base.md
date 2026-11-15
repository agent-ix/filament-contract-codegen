---
id: SR-17940486709564847048010
title: "IR-264 PR #841 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #841 at 5512fed0, git diff origin/main...HEAD; IR-264 PR3; spec/kani/functional/FR-015-bounded-kani-obligations.md, spec/kani/functional/FR-028-bounded-proof-ceilings.md, spec/kani/matrix/TC-025-bounded-kani-obligations.md, spec/kani/matrix/TC-039-bounded-proof-ceilings.md, spec/kani/matrix/tests.md, spec/oracle/functional/FR-018-composite-equality-oracles.md, spec/oracle/matrix/TC-029-composite-equality-oracles.md, spec/oracle/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-039
    type: reviews
---

## Summary

Ticket: IR-264 (PR3). Reviewed the status text the PR writes into the FR-015, FR-028 and FR-018
criteria, TC-025 / TC-039 / TC-029 and both matrix indexes for consistency with each other and
with the code, the new FR-015 unsupported-shape row for `union`, and the `SHADOW_PAIR_NODE_BUDGET`
derivation recorded in TC-025.

Clean: the union row (FR-015 table) matches `Model::declaration`/`resolve` (`ShadowFamilyNotBuilt`
naming `union` and the declaration node, also for a union payload). The budget derivation is
recorded in TC-025 with the probe test, the shape measured, the scope method (one cgroup, swap 0),
verdict, wall and `memory.peak` per size, and its caveat (a costlier shape at the same count is
refused only by its count); it states no tool versions. 192 nodes measured 18,917,900,288 B,
above 16 GiB, so 128 is the last measured size inside the ceiling: the derivation is honest.
The PARTIAL criteria FR-015-AC-73, FR-015-AC-74, FR-028-AC-17, FR-028-AC-23 and FR-018-AC-22 each
say "unowned, planner to assign", as do TC-025, TC-039, TC-029 step 14 and both tests.md rows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-028-AC-18 still reads "PARTIAL for the composite closure: the independent check of that helper is FR-018-AC-23, owner IR-264 PR1 (oracle side) and PR3", unchanged by this PR, which is PR3. `spec/kani/matrix/tests.md` (row FR-028-AC-13 through AC-24) now says the contract is implemented over the composite family and only AC-17 and AC-23 remain partial, and TC-039's new paragraph lists the steps the composite family implements. The FR row and the matrix disagree on AC-18, and the owner named is the PR that has now landed. Mark AC-18 implemented for the composite closure (`closure_reader` value-bearing with the FR-018-AC-23 independent check, `member_order` order-only, the permutation test) or keep it PARTIAL with "unowned, planner to assign" and say so in tests.md. | spec/kani/functional/FR-028-bounded-proof-ceilings.md:228, spec/kani/matrix/tests.md:94 |
| FND-002 | low | Status prose outside the AC tables still says planned: FR-015 "Planned (IR-264, IR-241): composite equality over records and tuples..." and "IR-264 (planned): the composite-equality family", and FR-028 "the composite family is PR3". TC-025's section heading now says "partly implemented". Update the three lines to match. | spec/kani/functional/FR-015-bounded-kani-obligations.md:57, spec/kani/functional/FR-015-bounded-kani-obligations.md:769, spec/kani/functional/FR-028-bounded-proof-ceilings.md:34 |
| FND-003 | low | FR-015-AC-75 is marked Implemented while its own text records that the "next field" mutant is realised as a self-comparison; that substitute is the same behaviour as the "field left out" mutant (code-review FND-001), so the criterion's six-mutant set is not met as written. Either the AC text is amended to the mutant actually wanted, or the row is PARTIAL until a same-typed-field misalignment mutant runs. | spec/kani/functional/FR-015-bounded-kani-obligations.md:926, spec/kani/matrix/TC-025-bounded-kani-obligations.md:768 |

## Verdict

Changes requested. FND-001 (FR-028-AC-18 left PARTIAL with this PR as owner, contradicting tests.md) is medium; FND-002 and FND-003 are low.

Gate, all at FSPEC_DIR filament-specification 17d49a98 (pulled at 03:59, before every run of this review, so the PR head and main ran in one FSPEC state):

- `cargo fmt --check`: clean. `cargo clippy --locked --all-targets --all-features -- -D warnings`: clean. `make spec`: exit 0 (warnings only, none in files this PR touches).
- Whole `--lib`: PR head 579 passed / 11 failed; clean `origin/main` 05f26a39 593 passed / 11 failed. Whole `--test it`: PR head 1106 passed / 9 failed / 108 ignored; main 1104 passed / 9 failed / 115 ignored.
- Failing-set diff: identical on both sides (`comm` of the sorted sets: 0 PR-only, 0 main-only, for `--lib` and `--test it`). Besides the known `native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`, `oracle_generation::tc_001_...` and `composite_equality_generation::tc_029_authoritative_fspec_recursive_items_generate_through_reader`, both sides fail the same `checked_package_evidence`, `checked_graph`, `opaque_metadata`, `exact_scalar_generation::tc_024_*`/`tc_031_ac33_*` and `tc_029_ac32_*` tests with `UnknownContractVersion` (`filament.checked-package/v2`): FSPEC drift, owned by the IR-805 catch-up PR, not by this PR.
- Kani lane (`composite_shadow_kani`, `--ignored`, serial, under the heavy-build lock): 4 passed. Healthy: flags `=` 0.3 s, flags `!=` 0.3 s, tuple 0.2 s, option 0.2 s, nested option 0.2 s, member order permuted 0.3 s, all Verified (cover satisfied asserted). Mutants: all six Falsified (0.4-0.6 s), each test asserting a non-empty counterexample and that the only failing non-cover check is the claimed assertion line. The walls match TC-025 (0.2-0.3 s healthy), not the 0.7-0.9 s quoted in the dispatch brief.
- Native (`composite_shadow_native`, --nocapture): exhaustive 2916 / 2916 / 36 / 36 / 64, sampled 80 at cap 100, all agreeing; mutants disagree on 2916 (opposite operator), 108 (absent = null), 9 (extra pair), 2592 (`ConstructionRefused` `MissingField` on `c`), 126 (`Refused`), as TC-025 records.
- Structural: `layout::l_2_*`, `cover_last::tc_025_*` (composite.rs spells no `kani::` text; draws and cover come from `render::shadow`), `role_parity`, `no_generation_panics`: 18 of 18 pass. No pin, compat layer or Cargo change in the diff; every new test carries `#[trace]`.

