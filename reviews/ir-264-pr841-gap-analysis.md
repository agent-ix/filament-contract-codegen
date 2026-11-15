---
id: SR-17940486709564847048009
title: "IR-264 PR #841 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #841 at 5512fed0, git diff origin/main...HEAD; IR-264 PR3; FR-015-AC-69 to FR-015-AC-76, FR-028-AC-13 to FR-028-AC-17, FR-028-AC-20, FR-028-AC-22, FR-028-AC-23, FR-018-AC-22 against src/kani/generate/shadow/composite.rs and tests/it/composite_shadow{,_kani,_native}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
---

## Summary

Ticket: IR-264 (PR3). Planless gap analysis, manual AC-to-test check (Plan completion: not
assessed). Each AC the PR moves to Implemented or PARTIAL was read against its tagged test and
the code it reaches; the Kani-lane and native tests were executed.

Implemented and backed by an executed, asserting test: FR-015-AC-69 (`tc_025_each_supported_corpus_item_yields_...`),
FR-015-AC-70 (`tc_025_the_harness_draws_one_value_per_leaf_...`), FR-015-AC-71
(`tc_025_the_harness_asserts_shadow_against_an_independent_expectation_...`), FR-015-AC-76
(`tc_025_the_identity_records_...`, which reads each backing state from
`spec/oracle/matrix/tests.md`), FR-028-AC-13 (seven spliced faults), FR-028-AC-14/15
(construction refusal `MissingField` asserted), FR-028-AC-16 (every Boolean, presence and slot
state drawn by a sampled plan), FR-028-AC-20, FR-028-AC-22 (subject to the code-review FND-001 on
the mutant set). FR-015-AC-72 and FR-015-AC-75 have code-review findings (evidence fields;
the duplicated field mutant). PARTIAL wording of FR-015-AC-74, FR-028-AC-17, FR-028-AC-23 and
FR-018-AC-22 matches what is and is not exercised and uses "unowned, planner to assign"
consistently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-015-AC-73's PARTIAL text says "every row is produced and asserted at the generator seam" except the `reference` row, but the family row is not: the table lists text, enum, rational, decimal, quantity, float, sequence, set, bag and ordered set, and the test covers `text`, `enum`, `rational`, `decimal`, `float` and `collection` only. `quantity` has no case (the closure reader records it as `Other { name: "other" }`), and sequence, set, bag and ordered set cannot be "named" at all because the reader records every collection as `"collection"`. The family names are also injected as arbitrary `RecordedType::Other { name }` strings that the generator copies into the refusal, so the assertion `named == family` checks an echo, not a mapping. The "supported item in the same request settles as it does alone" clause is exercised by separate `generate_composite_shadow` calls, since no negotiated request exists (AC-69's own caveat). List quantity and the four collection families as not exercised (or exercise them through the reader) and state the request caveat in AC-73. | tests/it/composite_shadow.rs:581-598, tests/it/composite_shadow.rs:642-657, src/oracle/equality/closure.rs:118-124, spec/kani/functional/FR-015-bounded-kani-obligations.md:814, spec/kani/functional/FR-015-bounded-kani-obligations.md:924 |

## Verdict

Changes requested. One medium: FR-015-AC-73's PARTIAL overstates which family rows are produced. Plan completion: not assessed.

Gate, all at FSPEC_DIR filament-specification 17d49a98 (pulled at 03:59, before every run of this review, so the PR head and main ran in one FSPEC state):

- `cargo fmt --check`: clean. `cargo clippy --locked --all-targets --all-features -- -D warnings`: clean. `make spec`: exit 0 (warnings only, none in files this PR touches).
- Whole `--lib`: PR head 579 passed / 11 failed; clean `origin/main` 05f26a39 593 passed / 11 failed. Whole `--test it`: PR head 1106 passed / 9 failed / 108 ignored; main 1104 passed / 9 failed / 115 ignored.
- Failing-set diff: identical on both sides (`comm` of the sorted sets: 0 PR-only, 0 main-only, for `--lib` and `--test it`). Besides the known `native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`, `oracle_generation::tc_001_...` and `composite_equality_generation::tc_029_authoritative_fspec_recursive_items_generate_through_reader`, both sides fail the same `checked_package_evidence`, `checked_graph`, `opaque_metadata`, `exact_scalar_generation::tc_024_*`/`tc_031_ac33_*` and `tc_029_ac32_*` tests with `UnknownContractVersion` (`filament.checked-package/v2`): FSPEC drift, owned by the IR-805 catch-up PR, not by this PR.
- Kani lane (`composite_shadow_kani`, `--ignored`, serial, under the heavy-build lock): 4 passed. Healthy: flags `=` 0.3 s, flags `!=` 0.3 s, tuple 0.2 s, option 0.2 s, nested option 0.2 s, member order permuted 0.3 s, all Verified (cover satisfied asserted). Mutants: all six Falsified (0.4-0.6 s), each test asserting a non-empty counterexample and that the only failing non-cover check is the claimed assertion line. The walls match TC-025 (0.2-0.3 s healthy), not the 0.7-0.9 s quoted in the dispatch brief.
- Native (`composite_shadow_native`, --nocapture): exhaustive 2916 / 2916 / 36 / 36 / 64, sampled 80 at cap 100, all agreeing; mutants disagree on 2916 (opposite operator), 108 (absent = null), 9 (extra pair), 2592 (`ConstructionRefused` `MissingField` on `c`), 126 (`Refused`), as TC-025 records.
- Structural: `layout::l_2_*`, `cover_last::tc_025_*` (composite.rs spells no `kani::` text; draws and cover come from `render::shadow`), `role_parity`, `no_generation_panics`: 18 of 18 pass. No pin, compat layer or Cargo change in the diff; every new test carries `#[trace]`.

