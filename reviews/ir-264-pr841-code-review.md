---
id: SR-17940486709564847048008
title: "IR-264 PR #841 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #841 at 5512fed0, git diff origin/main...HEAD; IR-264 PR3; src/kani/generate/shadow/composite.rs, src/kani/generate/shadow/source_check.rs, src/kani/generate/shadow.rs, src/kani/generate/render/shadow.rs, src/kani/identity/shadow.rs, src/lib.rs, tests/it/composite_shadow.rs, tests/it/composite_shadow_kani.rs, tests/it/composite_shadow_native.rs, tests/it/composite_equality_closure.rs, tests/it/exact_scalar_agreement.rs, tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
---

## Summary

Ticket: IR-264 (PR3 of 3, the composite family of the bounded shadow). The PR adds
`generate_composite_shadow` (record, tuple and option closures over Boolean and bounded-integer
leaves), a straight-line source check, the composite identity (closure, pair-node count,
coverage backing), a native refinement driver built against the TC-029 bounded corpus oracle,
and an installed-Kani lane (`composite_shadow_kani`, `#[ignore]`d, run by `make kani-gate` via
the `shadow_kani` filter). `SHADOW_PAIR_NODE_BUDGET` moves from 4 to 128.

Checked by reading and by running (rust-review lane folded in):

- Generator: closure resolution (`Model::resolve`/`declaration`, recursion via the in-progress
  stack, union and `Other` families refused as `ShadowFamilyNotBuilt`), leaf enumeration and the
  `when` guard for absent payloads, the shadow walk and the independent flat expectation, the
  draw renderer, literal pinning, the abstraction module (built through
  `subject::type_environment(..).record/tuple` and `OptionValue::present`), the identity.
  No `unwrap`/`expect`/`panic!`/`unreachable!` in non-test `src` code of the diff; the
  `.expect(..)` calls are inside the emitted `DRIVER` string literal only, which the TC-042 scan
  excludes.
- Kani lane, executed under the installed backend (results below): each mutant test asserts the
  exact failing assertion line (`failed_lines(..) == [line]`) and a non-empty counterexample, so
  each mutant is pinned to the check it claims.
- Native refinement: executed; exhaustive and sampled runs agree; the five production mutants
  disagree.
- Gate: fmt, clippy `-D warnings`, whole `--lib` and whole `--test it` at the PR head and on a
  clean `origin/main` worktree, failing sets diffed (see Verdict).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-015-AC-75 shadow mutant "one field of the left operand compared with the next field of the right" is realised as `let p1 = (l.f1 == l.f1, 1);` (label `right-field-never-read`). That makes `p1.0` constantly `true` with the count unchanged, which is behaviourally the same mutant as `field-left-out-of-the-comparison` (`p0.0 && p1.0 && p2.0` -> `p0.0 && p2.0`, the same field `b`): both remove field `b` from the verdict and nothing else. So the set has five distinct mutant behaviours, and the field-misalignment class the AC names (reading the wrong field of the right operand) is not exercised at all. The stated reason, "the corpus record has no two fields of one type", does not hold for the family: the same test files already build synthetic closures with `with_record`/`mixed_record` (e.g. members `f0` and `f3` both Boolean), and the Kani lane already proves such synthetic closures (the budget probe). FR-015-AC-75 is marked Implemented. Run the misalignment mutant on a synthetic record with two same-typed fields (`l.fi == r.fj`, i != j), or mark that mutant PARTIAL in FR-015-AC-75, FR-028-AC-22 and TC-025. | tests/it/composite_shadow_kani.rs:230-236, tests/it/composite_shadow_kani.rs:258-264, spec/kani/functional/FR-015-bounded-kani-obligations.md:926 |
| FND-002 | medium | FR-015-AC-72 is marked Implemented, and requires "the budget and the count are recorded in the identity and the evidence". The identity records them (`ShadowIdentity::size_budget`, `size`, `CompositeIdentity::pair_nodes`), but `KaniExecutionEvidence` has no budget or pair-count field and the test asserts the identity only; nothing in the evidence carries them. Either add them to the evidence (and assert it), or reword AC-72 / mark that clause PARTIAL. | src/kani/run/execute.rs:327-366, tests/it/composite_shadow.rs:490-531, spec/kani/functional/FR-015-bounded-kani-obligations.md:923 |
| FND-003 | low | The production-mutant test does not assert the reason for `mutant_opposite_operator`: its arm only checks `shadow.starts_with("Completed(verdict=")`, which the shadow result always satisfies, so any disagreement (for example a construction refusal or a build-side artefact) passes as "the opposite operator". Assert that the production result is `Completed` with the negated verdict and the same `pairs` (as the `extra_pair_charge` arm does for the count). | tests/it/composite_shadow_native.rs:499-508 |
| FND-004 | low | Doc comments disagree with the code and the measurement: `SHADOW_PAIR_NODE_BUDGET` says 192 nodes "needs most of" the 16 GiB ceiling, while TC-025 records 18,917,900,288 B at 192 nodes, more than 16 GiB (17,179,869,184 B), which is the stated reason the budget stops at 128; it also refers to "the default identity", but `src` has no default ceilings (the 16 GiB is the test's `ceilings()`). `CompositeIdentity::closes_family_row` is documented "Whether any covering criterion is unbacked", the inverse of the code (`all(.. == Backed)`) and of the field name. | src/kani/generate/shadow.rs:38-43, src/kani/identity/shadow.rs:153-154, src/kani/generate/shadow/composite.rs:1180-1182 |
| FND-005 | low | The test helper `parameter(entry, true)` gives the right operand the claim's `semantic_type` node as its "parameter node", so FR-015-AC-70's "the drawn arguments ascend by parameter node id" is exercised over one real node and one type node, not two parameter nodes. The ordering code is generic, but the evidence for the ordering clause is synthetic; name it as such in TC-025 or use two parameter nodes. | tests/it/composite_shadow.rs:62-79, tests/it/composite_shadow.rs:344-354 |

## Verdict

Changes requested, not blocking on the gate. The generator, the source checks, the Kani lane and the native refinement are sound and executed; the mutant tests pin the failing check. FND-001 (duplicated field mutant stands in for the misalignment mutant) and FND-002 (AC-72 evidence clause) are medium: each either needs a small code/test change or an honest PARTIAL in the spec. FND-003 to FND-005 are low.

Gate, all at FSPEC_DIR filament-specification 17d49a98 (pulled at 03:59, before every run of this review, so the PR head and main ran in one FSPEC state):

- `cargo fmt --check`: clean. `cargo clippy --locked --all-targets --all-features -- -D warnings`: clean. `make spec`: exit 0 (warnings only, none in files this PR touches).
- Whole `--lib`: PR head 579 passed / 11 failed; clean `origin/main` 05f26a39 593 passed / 11 failed. Whole `--test it`: PR head 1106 passed / 9 failed / 108 ignored; main 1104 passed / 9 failed / 115 ignored.
- Failing-set diff: identical on both sides (`comm` of the sorted sets: 0 PR-only, 0 main-only, for `--lib` and `--test it`). Besides the known `native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`, `oracle_generation::tc_001_...` and `composite_equality_generation::tc_029_authoritative_fspec_recursive_items_generate_through_reader`, both sides fail the same `checked_package_evidence`, `checked_graph`, `opaque_metadata`, `exact_scalar_generation::tc_024_*`/`tc_031_ac33_*` and `tc_029_ac32_*` tests with `UnknownContractVersion` (`filament.checked-package/v2`): FSPEC drift, owned by the IR-805 catch-up PR, not by this PR.
- Kani lane (`composite_shadow_kani`, `--ignored`, serial, under the heavy-build lock): 4 passed. Healthy: flags `=` 0.3 s, flags `!=` 0.3 s, tuple 0.2 s, option 0.2 s, nested option 0.2 s, member order permuted 0.3 s, all Verified (cover satisfied asserted). Mutants: all six Falsified (0.4-0.6 s), each test asserting a non-empty counterexample and that the only failing non-cover check is the claimed assertion line. The walls match TC-025 (0.2-0.3 s healthy), not the 0.7-0.9 s quoted in the dispatch brief.
- Native (`composite_shadow_native`, --nocapture): exhaustive 2916 / 2916 / 36 / 36 / 64, sampled 80 at cap 100, all agreeing; mutants disagree on 2916 (opposite operator), 108 (absent = null), 9 (extra pair), 2592 (`ConstructionRefused` `MissingField` on `c`), 126 (`Refused`), as TC-025 records.
- Structural: `layout::l_2_*`, `cover_last::tc_025_*` (composite.rs spells no `kani::` text; draws and cover come from `render::shadow`), `role_parity`, `no_generation_panics`: 18 of 18 pass. No pin, compat layer or Cargo change in the diff; every new test carries `#[trace]`.

