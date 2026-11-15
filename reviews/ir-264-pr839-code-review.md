---
id: SR-17940486709564847048002
title: "IR-264 PR #839 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #839 at f0d42093, git diff origin/main...HEAD; IR-264 PR1; src/oracle/equality/closure.rs, src/oracle/equality/mod.rs, src/lib.rs, tests/it/composite_equality_closure.rs, tests/composite_equality_support/closure_cases.rs, tests/composite_equality_support/authentic_native.rs, tests/it/composite_equality_agreement.rs, tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-029
    type: reviews
---

## Summary

Ticket: IR-264 (PR1 of 3, oracle side). The PR adds `declaration_closure` and `operand_types`
to every generated composite-equality claim (`closure.rs`, read from the same
`generated.composites` / `composite_bindings` the environment constructor is rendered from),
and TC-029 steps 13 to 15 on a new `bounded` corpus (record `Flags { a: Boolean; b: Int[0,2];
c: Int[0,2]?; }` under `=` and `!=`, tuple `Duo(Boolean, Int[0,2])`, `Option<Int[0,2]>`,
`Option<Option<Boolean>>`).

What was checked, by reading and by reproduction:

- AC-23 independence. The test's reader (`read_type` / `read_declaration`) uses only
  `CheckedPackageV2::resolved_graph()` node tags, semantic forms and serde-serialised `body`
  terms; it calls no crate function. Its traversal differs from the generator's: it treats the
  `bounded_domain` node as the type and reads `min`/`max` from its aggregate, while the
  generator (`mod.rs` ~1733) starts at the `scalar_type` and filters attached `integer_range`
  bounds through `read_integer_range`; the optional field is unwrapped from the `optional`
  binding by the test, by `resolution.rs` ~440 in the generator. No shared logic. The operand
  check reads the request descriptor's `left_source_type` (built by the test), so a wrong
  `binding.node` would also be caught. Two generator-source mutants were run out of tree
  (reconstruction narrows the integer bound; reconstruction reads `c?` as required): see
  Verdict for results.
- AC-22. `Meter::new(UNLIMITED)` has all ten limits `u64::MAX`. Verdict (`model_equal`) and
  node count (`model_nodes`/`slot_nodes`) are computed from a test-side value model and match
  FSpec FR-149's occurrence-pair plan (none/none and none/present terminal, present/present +1
  payload, absent/absent and null/null terminal equal, other slot combinations terminal unequal,
  no early exit). Domain sizes 30, 6, 4, 4 and pair total 900*2+36+16+16 are correct. The
  operator-flip control replaces `sv::EqualityOperator::Equal` in the emitted `src/lib.rs`, and
  that token is the `{operator}` argument of `environment.check_equality(...)` in the emitted
  oracle, so the flip changes runtime behaviour; the mutated crate compiles (valid variant).
- AC-21. The native test compares `claim-map.json` with `prepared.environment.composites()` and
  `prepared.left_source`/`right_source`, with four JSON mutants of the closure.
- Whole `--lib` and whole `--test it` run once at head (results in Verdict).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | `recorded_declarations` flattens a union to one `RecordedMember` per payload position (`"{identifier}.{index}"`), so a nullary union member (`UnionMemberDeclaration::positions()` is documented "empty for a nullary member") records nothing: the claim-map closure of a union silently omits its nullary members, and member boundaries are lost (`A(Boolean, Boolean)` and `A(Boolean)` + a second member are told apart only by name parsing). AC-21 requires "members in declaration order with each member's name", for every Boolean/bounded-integer item, which includes unions over such leaves; a closure that dropped a nullary member cannot be detected by the AC-21 or AC-23 comparison. No union is in the bounded corpus, so this path is untested. Record one member per union member (identifier plus its ordered payload types), and add a union with a nullary member to the corpus or to a unit test. | src/oracle/equality/closure.rs:166-180 |
| FND-002 | medium | The AC-23 control labelled "option payload dropped from the operand" mutates item 0 (`flags_equal`, whose operand is the record `Flags`) and replaces `Composite { key }` with `Boolean`; no option is involved. The AC-23 text now claims "lost an operand's option payload fails the check", but no mutant drops an option layer from the `option_equal` (`Option<Int[0,2]>`) or `nested_option_equal` operand, so that branch of the AC is not exercised. Apply the mutant to `option_equal` / `nested_option_equal` (`Option{Option{Boolean}}` -> `Option{Boolean}`) and keep the record case under an honest label. | tests/it/composite_equality_closure.rs:394-396, spec/oracle/functional/FR-018-composite-equality-oracles.md:729 |
| FND-003 | low | The AC-22 operator-flip control accepts any panic of `run_composite_agreement_workspace` (`catch_unwind(...).is_err()`), including a scratch-workspace build failure, a lockfile/offline resolution error or a test-count mismatch, so it would pass vacuously if the mutated crate stopped compiling or the workspace broke for an unrelated reason. Assert on the panic payload: the workspace ran and the failing test is `tc_029_ac22_oracle_completes_every_domain_pair...` with a `Completed(` verdict mismatch. | tests/it/composite_equality_closure.rs:140-162 |

| FND-004 | medium | The PR body says the "reader-narrowed bound" control was "verified by really mutating source", but the mutant is not in the PR and could not be reproduced. A reviewer mutant of the reconstruction's integer-bound read (`resolve_scalar`'s `integer_range` arm, `Ok(ValueType::Int(interval))` -> an interval `[lower, lower]`) left all three `composite_equality_closure` tests passing (AC-21, AC-22 and AC-23). Either that arm is not on this corpus's path, so the bound reaches the closure and environment some other way that the PR does not name, or AC-23 misses a narrowed reconstruction bound. The second presence mutant (`resolution.rs` reading `c?` as `Required`) does fail AC-23 through the package read, and also AC-21/AC-22 natively. Name the exact source mutant used for "narrowed bound" (file:line and edit), show that it fails `tc_029_ac23_...`, and explain why the `resolve_scalar` integer arm does not reach this corpus (or fix the gap). | src/oracle/equality/mod.rs:1733-1753, src/oracle/equality/resolution.rs:214-221, tests/it/composite_equality_closure.rs:363-412 |

## Verdict

Gates at f0d42093, run by the reviewer: whole `cargo test --lib` gave 584 passed, 1 failed
(`routed::scalar_native_private::native_meter::routed_native_oracle_preserves_denial_cancellation_counters_and_refusal`:
the staged `meter_control` bin cannot resolve `filament_exact`, a rename issue untouched by this
diff). Whole `cargo test --test it` gave 1093 passed, 1 failed
(`oracle_generation::tc_001_boolean_oracle_bundle_is_deterministic_traceable_and_schema_valid`,
rust-validator assertion). Both are the known failures that also fail on main. The `layout::`
tests, including `l_2_no_file_but_the_crate_root_re_exports_an_item`, pass after the coder's
`pub mod closure` fix. The reviewer's own origin/main baseline run was cut short at the
coordinator's request, so the "also fails on main" status rests on the brief and on the failure
causes, which lie outside this diff.

Confirmed: the AC-23 reader shares no logic with the generator. The AC-22 meter limits, the
FR-149 verdict and node-count model, and the domain sizes are right. The operator-flip control is
real: with `--nocapture`, the native AC-22 test fails at `flags_equal ... left: Completed(false)
right: Completed(true)` while AC-21 passes. The AC-23 claim mutants each fail the package read.
The presence source mutant fails AC-21, AC-22 and AC-23. The bound source mutant survives
(FND-004). No tool versions appear in the diff. The only file shared with the other IR-264 lane
(PR #838) is `src/lib.rs` (additive re-exports). Not mergeable until FND-001, FND-002 and FND-004
are fixed or answered in this PR. FND-003 is low and can ride the same round.

## Dispositions

Round 1, fix commit 02e476d6 (reviewed against f0d42093..02e476d6).

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | `closure.rs` now records one `RecordedMember` per union member with `RecordedType::Payload { positions }` (empty for a nullary member). The bounded corpus gains `union Shape { Empty, Flag(Boolean), Both(Boolean, Int[0, 2]) }`; AC-21 native controls (dropped nullary member, dropped payload position, narrowed payload bound) and AC-23 package-read controls (nullary member dropped, payload position dropped) each fail; the member count includes the union's 3 members. |
| FND-002 | fixed | The record mutant is relabelled "record operand replaced by Boolean"; new option mutants run on the option items: `option_equal` payload bound narrowed, `nested_option_equal` layer dropped, `option_equal` read as its payload, plus the retained `option` case narrowed to `[-100, 99]`; each must fail the package read and the test passes at 02e476d6. |
| FND-003 | fixed | The flip control now takes the panic payload and requires `composite agreement workspace failed`, no `could not compile`, `test tc_029_ac22_oracle_completes_every_domain_pair_with_the_fr149_verdict_and_schedule ... FAILED`, `flags_equal` and `Completed(`. |
| FND-004 | rejected: reviewer measurement error | The same edit at `mod.rs:1753` (`[lower, lower]`), re-run by the reviewer at 02e476d6 with `--locked`, fails all 4 `composite_equality_closure` tests. The fix commit does not touch `mod.rs`, so the round-1 survivor at f0d42093 was a stale or invalid reviewer build, not a test gap. The emitted-render mutant at `mod.rs:2466` (`exact::ValueType::option({child})` -> `{child}`) fails the native AC-21 and AC-22 tests; AC-23 reads the claim, not the emitted source, and its `T?`-as-`T` reconstruction case (`resolution.rs:445` Optional->Required) failed AC-23 in round 1. |
