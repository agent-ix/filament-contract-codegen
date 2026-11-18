---
id: SR-17940486709564847048062
title: "IR-806 PR-3 PR #858 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #858 at 124ea590, git diff c2931a1b...124ea590 (stacked on PR #849); IR-806 PR-3; src/replay/composite_shadow_route.rs, src/replay/composite_builder.rs, src/replay/composite_artifact.rs, src/kani/identity/leaf_binding.rs, src/lib.rs, src/replay/mod.rs, tests/it/composite_shadow_route.rs, tests/it/composite_shadow_kani.rs, tests/it/composite_playback_decode.rs, tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-033
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-048
    type: reviews
---

## Summary

Ticket: IR-806 (PR-3). Code review with the rust-review lane folded in, of the shadow-aware entry
`settle_composite_shadow`, the new `OriginalCompositeEqRequest::settle_falsified`, the
`integer_positions` key derivation and the route tests.

Design check: the entry builds one FSL request over the original proving context, sends any
non-cancelled retained `Disagreed` and every verified run to `settle_verified_shadow`, falsified
runs to `replay_composite_parity`, and inconclusive, cover-unsatisfied and cancelled runs to the
unchanged `run_terminal_value` (`src/kani/terminal.rs` is not in the diff). It adds no FSL producer
and no second route. The F-1 to F-7 rows and V-1 to V-5 match the pinned FSL `fsl-replay`
(9c09cc9e) docs of `replay_composite_parity` and `settle_verified_shadow`. Bounds are the identity's
drawn ranges; FSL `Positions::harness/covered` decides coverage.

Reviewer probes (temporary test, reverted): exact drawn bounds give `Proved 5`; one bound omitted
and an empty list give `Tested`; a duplicate key is refused by CG's packet builder; an unknown key
gives FSL's `Inconclusive(ReplayRefused(InvalidRuntimeInput))`; a leaf missing from the identity
refuses `OperandMismatch`; a drawn range wider than the declared one gives `Proved` (AC-8 allows a
wider legal range). Position keys are child-index paths, unique per position; one level of
record nesting is exercised (Flags).

Mutation (env-switched, one build, the 9 route tests): Disagreed->NotExhausted, bounds from the
declared range, the whole artifact tie, the operand tie, the native-consistency check,
Inconclusive through verified, CoverUnsatisfied through verified, falsified refinement forced
NotExhausted, the Disagreed shortcut removed, verified checks forced >= 1, the operand-source
check and a member index +1 each fail at least one test. Four survive: the claim/bounds tie and
the sent-context/limits tie of the artifact (FND-004), Cancelled routed through the verified
entry and the Disagreed shortcut applied to Cancelled (FND-005).

Real Kani (24G scope, inside the cargo slot): `tc_048_installed_kani_runs_settle_through_the_production_route`
passed; verified control `Verified` 0.7 s -> `Tested`; mutant `p0.0 && p1.0 && p2.0,` ->
`p0.0 && p2.0,` `Falsified` 0.6 s at the verdict assertion only -> `Failed` (exact
`{equal: false, pair_count: 4}`, shadow `{equal: true}`); whole test 2.2 s wall, cgroup
memory.peak 558 518 272 bytes (includes the cargo test process). The run writes the harness's
own generated source into the crate, and the mutant drops field `b` from the comparison while the
rebuilt operands differ only on `b`, so it is not trivially false.

Gates at 124ea590: fmt and clippy `--locked --all-targets -D warnings` clean; Cargo files
untouched; no new dependency.

## Verdict

**Changes requested.** The routing is sound for the outcomes the tests cover and matches the
recorded design, but the entry does not tie the run's evidence to the harness it settles, the
native-consistency check accepts any pair of non-completed outcomes, and the artifact tie and the
Cancelled rows are not exercised by any test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `settle_composite_shadow` never checks that `run.evidence` is the run of `run.harness` (no `harness_path`, `shadow_size` or symbolic-argument comparison). Reproduced: verified evidence with `harness_path = "some::other::harness"`, exhaustive refinement and 5 checks settles `Proved 5` under this harness's drawn bounds. FR-029-AC-27 requires refusing another claim's run binding with no value. | src/replay/composite_shadow_route.rs:329-352 |
| FND-002 | medium | `consistent` ends in `(_, _) => true`, so any non-completed artifact outcome matches any non-completed native observation. Reproduced: artifact `Undefined(DivisionByZero)` with native `Refused(InvalidRequest)` and an agreeing shadow settles `Inconclusive(ScalarAgrees)`. An artifact that stopped (`Incomplete`) observed as `Refused` skips F-2 and hides a generated-artifact fault. Match each pair explicitly. | src/replay/composite_shadow_route.rs:308-318 |
| FND-003 | medium | `DriverFalsification::shadow`, the retained shadow comparison that alone decides F-7 (`Failed` vs `Inconclusive(ScalarAgrees)`), is caller input tied to nothing: not to the playback, the harness or the run. Reproduced: a falsified run whose shadow is supplied equal to FSL's exact outcome settles `ScalarAgrees`. Derive it from the run or bind it, or record it as a named trust assumption with an owner. | src/replay/composite_shadow_route.rs:82-93, 469-474 |
| FND-004 | medium | Two of the three artifact ties survive deletion: the claim identity tie (`tie(&artifact_claim) != tie(request.claim())`, which carries the bounds and limits) and `check_sent_context`. Only the operand swap is tested, yet the FR-029-AC-27 note says an artifact authenticated for other bounds or limits refuses. Add an artifact authenticated with other bounds and one with other limits. | src/replay/composite_shadow_route.rs:454-461, tests/it/composite_shadow_route.rs:766-777 |
| FND-005 | medium | No route test drives a `Cancelled` outcome: routing `Cancelled` through `verified_entry`, and applying the Disagreed shortcut to `Cancelled`, both survive. The FR-029 design decision says "any retained disagreement" settles through the verified entry, but the code exempts `Cancelled` (ordinary rows); pick one and test it. | src/replay/composite_shadow_route.rs:346-350, 386-388 |
| FND-006 | low | `shadow_harness_bounds` maps `FiniteBound::integer_range` and `ProofBound::new` errors to `OperandMismatch` (`map_err(\|_\| ...)`), dropping the cause and misnaming an inverted drawn range. The `claim_request` `result_large_err` allow copies a PackageRead justification that does not apply here. | src/replay/composite_shadow_route.rs:238-252, 392-393 |

## Coverage

- Route tests at head: 9/9 pass. Mutants: 16 run plus a no-mutant control; 12 killed, 4 survived (FND-004, FND-005).
- Probes: exact, tightened, omitted, empty, duplicate, unknown key, missing leaf, wider range, foreign evidence, wildcard consistency, untied shadow.
- Real Kani lane test: pass, figures above.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `check_evidence` also compares `evidence.solver`, and the `EvidenceMismatch` doc lists the solver, but no test edits the solver: removing that comparison fails none of the 14 route tests. FR-029-AC-27's tested list leaves the solver out, so the spec is honest. Add a solver case to `tc_048_the_evidence_of_another_harness_refuses_with_no_value`, or drop the comparison and the doc word. | src/replay/composite_shadow_route.rs:299-306, tests/it/composite_shadow_route.rs:879-917 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at baecb8c0 (52c5f83a, baecb8c0): `check_evidence` runs before bounds and before the Disagreed shortcut. It compares the generated harness file (`harness.rust.path`), the size record, unwind, solver and the symbolic arguments rebuilt from the drawn leaves, and refuses `EvidenceMismatch`. `tc_048_the_evidence_of_another_harness_refuses_with_no_value` covers path, size, missing size, unwind and arguments; deleting the call or any of those four comparisons fails it. The real-Kani route test still passes, so real evidence matches. |
| FND-002 | fixed | Round 1 at baecb8c0 (52c5f83a): `consistent` matches every (artifact, native) pair with no wildcard. Completed pairs only with Completed (verdict equal), Refused with Refused, Incomplete with Incomplete or ExecutionFault, and Undefined with nothing. `tc_048_each_native_observation_pairs_only_with_its_own_artifact_outcome` checks the whole 4x4 table; flipping Refused/Refused, Incomplete/Fault, Incomplete/Refused or Undefined/any fails it. |
| FND-003 | accepted-no-change | The shadow outcome stays a driver measurement. The `DriverFalsification::shadow` doc and the FR-033-AC-19 PLANNED remainder record it as a named trust assumption owned by IR-1567. This is honest. The harness's `expected_verdict` comes from an in-harness reference, not the authenticated artifact, and a Kani playback carries only the draws, so CG's library cannot compute the shadow's outcome at the operands without executing the generated shadow, which is the driver's job. Binding it needs that executed capture, already IR-1567 scope. |
| FND-004 | fixed | Round 1 at baecb8c0: `tc_048_an_artifact_of_other_bounds_limits_or_context_refuses_with_no_value` covers other bounds, accounting limits, a stage limit and replay limits. Both round-0 survivors (claim tie, sent-context check) now fail it. The `check_sent_context` argument is now `request.replay_limits()`; it used to compare the artifact's limits with themselves, and restoring that also fails the test. |
| FND-005 | fixed | Round 1 at baecb8c0: `tc_048_a_cancelled_run_takes_the_ordinary_row_even_with_a_retained_disagreement` asserts `Incomplete(Cancelled)` on the ordinary route for both sources under None, Exhaustive and Disagreed. The spec states the Cancelled row (design decision, AC-24). Both round-0 survivors (Cancelled through verified, Disagreed shortcut on Cancelled) now fail it. |
| FND-006 | fixed | Round 1 at baecb8c0: `CompositeShadowError::DrawnBound { node, path, cause }` carries FSL's cause, and `tc_048_an_inverted_drawn_range_names_its_leaf_and_the_cause` asserts it; restoring `OperandMismatch` fails it. The copied `result_large_err` allow on `claim_request` is removed, and clippy `-D warnings` is clean. |
