---
id: SR-17940486709564847048020
title: "IR-264 PR #845 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #845 at 1f6c08ec, git diff origin/main...HEAD (base ebe100f7); IR-264 PR4; src/kani/generate/shadow/composite.rs, src/kani/identity/shadow.rs, src/kani/run/execute.rs, src/kani/run/harness.rs, src/kani/run/scalar.rs, src/kani/terminal/constrained/tests.rs, src/lib.rs, src/oracle/equality/closure.rs, tests/composite_equality_support/authentic_native.rs, tests/it/composite_equality_closure.rs, tests/it/composite_shadow_families.rs, tests/it/composite_shadow_kani.rs, tests/it/composite_shadow_native.rs, tests/it/main.rs, tests/it/shadow_refinement.rs, tests/it/terminal_map.rs"
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

Ticket: IR-264 (PR4, the remainder of the composite shadow criteria plus the tc_039 flake fix).
The PR adds `ShadowSizeRecord` and an optional `KaniExecutionEvidence.shadow_size`
(serialised `shadowSize`, skipped when `None`), set from `HarnessView` for the `Shadow` arm only;
records collection kinds, quantity and reference under their own names in FR-018's closure
reader; adds `generate_composite_shadows` (a map of `generate_composite_shadow` over items);
adds the wide `i64` corpus case (native sampled refinement and an installed-Kani shadow run);
and gives the signal, panic and peak subjects of the tc_039 ceiling test a 600 s wall ceiling.

Checked by reading and by running (rust-review lane folded in):

- `shadow_size`: every `HarnessView` arm except `Shadow` sets `None`, and the field carries
  `skip_serializing_if = "Option::is_none"`, so every other family's serialised evidence is
  unchanged by construction (no key emitted). The composite Kani lane's `prove` helper asserts
  the typed field and the serialised `sizeBudget`, `size` and `pairNodes` against the identity;
  a `None` field would serialise no key and fail all three assertions, so the check can fail.
- Closure reader: the `CollectionKind` match is exhaustive; `quantity` covers all three quantity
  variants; the unit test checks all four kind names.
- tc_039 flake fix: the slow subject keeps the 2 s ceiling and still asserts
  `ReachedWallClockCeiling`, `stopped.ceilings == ceilings(2)`, the 30 s upper bound, versions,
  class/seed, inconclusive strength and the peak. The signal, panic and peak subjects keep every
  assertion they had (`EndedBySignal`, inconclusive strength, `ProcessFailed{101}`, peak and
  `Exact(49)`) and gain `ended.ceilings == ceilings(600)`; the memory-ceiling assertion compares
  two equal constants now (it did a real comparison before) but the new whole-ceilings equality
  subsumes it. No assertion weakened: none of those three outcomes depends on the wall ceiling
  being short, it only made them racy under load.
- Wide case: the test asserts four leaves drawn over `(i64::MIN, i64::MAX)`, `Overflowed` case
  count, `Sampled`, sample size 64, seed 11 and `cases_run == boundary + 64`.
- No `unwrap`/`expect`/`panic!` in non-test `src` of the diff; no pin, compat layer, Cargo.toml
  or Cargo.lock change; every new test carries `#[trace]`; per-label scratch package names
  (`generated-shadow-{label}`, `composite_shadow_wide`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `generate_composite_shadows` is a new public entry with no production caller: its body is `items.iter().map(generate_composite_shadow).collect()`. FR-015 uses "request" for the negotiated `negotiate_kani_obligations` request (the PR's own status says the composite family is not an item kind of it), so `tc_025_a_request_settles_each_item_as_it_does_alone` exercises a test-only map of a pure function, not the request FR-015-AC-73's "a supported item in the same request settles as it does alone" refers to. Only a mutation of that one-line map can fail it. The status calls the clause "exercised"; it is a re-labelling of per-item calls. Drop the wrapper and list the clause as not met with the negotiated item kind, or route a real request through it. | src/kani/generate/shadow/composite.rs:831-838, tests/it/composite_shadow_families.rs:122-164 |
| FND-002 | medium | FR-015-AC-72's status claims `shadowSize` is "present exactly for a `bounded_shadow` harness and absent from every other family's evidence", but no test asserts either half outside the ignored Kani lane: nothing checks absence for a non-shadow family, nor the stand-in shadow family (no `pairNodes`), and the default-lane launcher stand-in tests (`two_phase_tests.rs`, `kani_obligations.rs`) never read `shadowSize`. Removing `skip_serializing_if` (emitting `"shadowSize": null` into every family's evidence) or dropping the `Shadow` arm's `Some` passes the whole default `--lib` and `--test it` suites. | src/kani/run/execute.rs:385-388, src/kani/run/harness.rs:381-384, tests/it/composite_shadow_kani.rs:79-90 |

## Verdict

Changes requested (two medium, neither a wrong result). The evidence plumbing, the closure
reader change, the wide case and the tc_039 fix are correct; the tc_039 change weakens no
assertion. FND-001 is the reviewer's answer to the brief's question: the `generate_composite_shadows`
exercise is a re-labelling, not an honest exercise of the "same request" clause. FND-002 is a
missing default-lane test for a claim the status makes.

Gate results are recorded in the gap-analysis artifact (SR-17940486709564847048021) and the
final report.
