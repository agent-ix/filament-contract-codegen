---
id: SR-17940486709564847048021
title: "IR-264 PR #845 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #845 at 1f6c08ec, git diff origin/main...HEAD (base ebe100f7); IR-264 PR4; FR-015-AC-72/73/74, FR-028-AC-16/17/20/23/24, FR-018-AC-21/22 against src/kani/**, src/oracle/equality/closure.rs, tests/it/composite_shadow_*.rs, tests/it/shadow_refinement.rs and spec/kani/**, spec/oracle/**"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-264 (PR4). Planless gap check of the criteria this PR claims against the code and the
tests it adds, closing the gaps the PR3 round recorded (reviews/ir-264-pr841-*.md: AC-72's
evidence clause, AC-73's family rows and same-request clause, AC-74's second case).
Plan completion: not assessed.

Per criterion:

- FR-015-AC-72 Implemented: the evidence clause is now met in code (`KaniExecutionEvidence.shadow_size`,
  `shadowSize` with `sizeBudget`, `size`, `pairNodes`), asserted against the identity on every
  composite installed-Kani run; the absence-for-other-families claim has no test (code-review
  FND-002).
- FR-015-AC-73 PARTIAL: the sequence row reaches `ShadowFamilyNotBuilt` naming `sequence` from a
  checked package; quantity carries FR-018's `Unsupported(quantity)`; set, bag and ordered set
  carry FR-018's `Unsupported(bounded_domain)`; executed and passing. The same-request clause is
  exercised only through a test-only map (code-review FND-001). Reference row and the negotiated
  item kind honestly listed as not met.
- FR-015-AC-74 PARTIAL (versions only): the second case (four `i64`-range draws) is sampled
  natively with an overflowed case count and agrees; its shadow is `Verified` with the cover
  satisfied (re-run by this review, below).
- FR-028-AC-17, AC-23 PARTIAL and FR-018-AC-22 PARTIAL: statuses consistent with the code; AC-23
  prose is spec-review FND-001.
- FR-028-AC-24 (tc_039 flake): every assertion kept; see code-review Summary.

Gate, run by this review at 1f6c08ec in its own worktree and target directory, under the
cargo-slot lock, FSPEC_DIR filament-specification, `--locked`:

- Whole `--lib`: 607 passed, 0 failed, 36 ignored. Whole `--test it`: 1147 passed, 0 failed,
  126 ignored (the coder's 607 / 1147 reproduced). `layout::l_2_*`, `cover_last::tc_025_*`,
  `role_parity`, `no_generation_panics`, both `composite_shadow_families` tests, the wide native
  test and the tc_039 ceiling test all pass in that run.
- `cargo fmt --check` clean; `cargo clippy --locked --all-targets --all-features -- -D warnings`
  clean; `cargo deny check` ok (advisories, bans, licenses, sources); `make spec` exit 0
  (warnings only). Cargo.lock and Cargo.toml unchanged.
- Kani spot-check, each in its own `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`
  under the lock, test binary run directly, scope wall measured from process start to end and
  the scope's `memory.peak`: `flags-equal` Verified, scope 0.42 s (prove 0.3 s), 120,573,952 B
  (TC-025: 0.5 s, 116,314,112 B); mutant `absent-equals-null` Falsified, scope 0.61 s (prove
  0.5 s), 126,951,424 B (TC-025: 0.7 s, 120,963,072 B); `wide-i64` Verified, scope 0.31 s,
  123,834,368 B (TC-025: 0.4 s, 126,316,544 B). The TC-025 table's "Wall" is the scope total as
  its header says, not the Kani-internal or `prove` wall (those are 0.1-0.2 s lower); no
  mislabelling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | With the closure reader now naming each collection kind, no checked package produces the recorded family name `collection`, yet `tc_025_each_unsupported_shape_settles_with_its_reason_and_no_harness` still feeds `"collection"` as a recorded type name and FR-015-AC-73's status lists `collection` among the families exercised at the generator seam. That row now exercises a name the reader cannot emit; replace it with `sequence` (or the four kind names) and update the status. | tests/it/composite_shadow.rs:643, spec/kani/functional/FR-015-bounded-kani-obligations.md:926 |
| FND-002 | low | FR-018-AC-22's status now cites the `i64`-range record ("the sampled plan above the cap is shown for the record item and for the `i64`-range record (IR-264 PR4)"), but `tc_025_two_unconstrained_i64_leaves_are_refined_by_a_sampled_run_that_agrees` is tagged FR-015-AC-74 and FR-028-AC-16 only, and it asserts agreement with the shadow (verdict and pair count), not AC-22's admitted schedule (`equality.plan-form`, `equality.plan`, one `equality.pair` per node, `equality.result-retain`). Either tag and assert the schedule, or drop the citation from AC-22. | spec/oracle/functional/FR-018-composite-equality-oracles.md:728, tests/it/composite_shadow_native.rs:671-711 |

## Verdict

Gate green (all of it, reproduced). Two low gaps here; the medium items are code-review FND-001
and FND-002 and spec-review FND-001. The PR closes the PR3 round's AC-72 evidence gap and AC-74
second-case gap genuinely, and AC-73's family rows from checked packages; the AC-73 same-request
clause is not genuinely closed.
