---
id: SR-17940486709564847048015
title: "IR-1448 PR #843 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #843 at ce2221e9, git diff origin/main...HEAD; FR-054-AC-1..23 against tests/it/kani_obligations_v2_option.rs, docs/qualification/ir1448-kani-option-clause-runs.md and src/**"
review_set: subset
---

## Summary

Ticket: IR-1448. This analysis is planless. Plan completion: not assessed. Each FR-054 criterion
was checked against the tagged tests and the code, and every Implemented claim was checked against
what its test actually asserts.

Gate at ce2221e9:
- whole `--lib`: 604 passed, 0 failed, 36 ignored.
- whole `--test it`: 1127 passed, 0 failed, 120 ignored.
- The option module run alone: 12 passed, 5 ignored (the Kani lane).
- The coder's counts match.

The Kani lane was not re-executed in this review. The five `#[ignore]` controls assert the
verdicts and playback vectors that the run doc records:
- AC-3 mutant: `[0],[0]`.
- AC-4 mutant: `[1], 1000`.
- AC-7 mutant: `[0], 0`.

The draw order (presence, then payload) matches `draw_option`. The doc's wall and `memory.peak`
columns are external measurements of a systemd scope. The tests do not compute or assert them, and
the doc says so ("the scope's cgroup memory.peak"). That is acceptable under FR-054-AC-21 ("peak
memory only where the environment supplies a source").

Clean, as claimed:
- AC-1 (no-allocation source scan with the enum-arm detector mutant).
- AC-2 (host enumeration: 1002 admitted draws, with the free-absent and widened-bound mutants).
- AC-4 (Kani healthy and `< 1000` mutant).
- AC-7 (Kani `present && value <= 1000` mutant).
- AC-23 (the FR-015-AC-42 amendment text is on main and matches FR-054).

AC-6 PARTIAL is honest: FSL refuses an unguarded `value()` (`unproved-presence`, measured by
`tc_068_the_fsl_checker_refuses_an_unguarded_value_read...`). The AC-8/AC-9, AC-10, AC-11 and
AC-21 PARTIAL wording and the "not run" list agree across FR-054 Status, TC-068 Status, tests.md
and the run doc.

## Verdict

**FAIL.** AC-5 and AC-22 are claimed Implemented beyond what their tests show. AC-3's "at every
draw" is not shown. One test is tagged to a criterion the status calls not run. AC-16 is covered
in the spec-review SR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-054-AC-5 is marked Implemented, but its control is not run. The AC needs four things: (a) the unguarded claim `result = (option.value(x) <= 1000)` on a direct oracle fixture; (b) that it stops at the absent draw; (c) that it fails the produced-result step at presence 0 / payload 0; (d) two mutants, default payload 0 and assumed presence, each making the fixture verify. The test calls only the `value_*` step function of the GUARDED claim's oracle on the host and observes `Err(Undefined)`, plus the default-payload mutant `Ok(0)`. No unguarded-claim fixture exists, the produced-result step is never run (not even via the host playback the AC-2 enumeration already uses), and the assume-presence mutant is absent. Keep AC-5 PARTIAL, or run the harness body over a constructed unguarded oracle on the host. | tests/it/kani_obligations_v2_option.rs:350-375; spec/kani/functional/FR-054-option-clause-harnesses.md:176,208 |
| FND-002 | medium | FR-054-AC-22 is marked Implemented: "the controls of FR-054-AC-5 and FR-054-AC-6 are run ... and each stated verdict is observed". Both AC-5 and AC-6 state a produced-result-step failure, and neither test observes one: the scratch crate only prints the oracle's `Result`. AC-6 is itself PARTIAL. AC-22 cannot be Implemented while the stated verdicts of the controls it covers are unobserved. | tests/it/kani_obligations_v2_option.rs:350-402; spec/kani/functional/FR-054-option-clause-harnesses.md:193,208 |
| FND-003 | medium | FR-054-AC-3 is marked Implemented, but its mutant clause is not shown. The criterion says the negated-presence mutant "fails the produced-result step at every draw, including presence 0 with payload 0 and presence 1 with payload 0". The equivalent-package Kani run gives one witness (`[0],[0]`). No test shows the failure at presence 1 / payload 0, or at every draw. The AC-2 host enumeration (`enumerate`) could show it over the `NOT_PRESENT` harness with subject `present`. | tests/it/kani_obligations_v2_option.rs:550-561; spec/kani/functional/FR-054-option-clause-harnesses.md:174 |
| FND-004 | medium | Matrix inflation. `tc_068_a_non_scalar_payload_is_refused_naming_its_family` is tagged `FR-054-AC-14`, but FR-054 Status lists AC-14 as "Not run", and the test asserts nothing about `NoFiniteEncoding`, `StateFrameRefused` or the other three reasons. The computed matrix will count AC-14 as tested. The same test-to-claim gap appears where the reachability tests are tagged `FR-054-AC-8`, `AC-9` and `AC-11`: those tests show the shape is unreachable, not the criterion's lowering or `NestedOption`. Tag only what each test asserts. | tests/it/kani_obligations_v2_option.rs:276,428,439 |

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-054-AC-1 | examined | draws one presence bool and one i64 payload as Copy values with no allocation |
| FR-054-AC-2 | examined | The abstract domain of x: Option<Int[0, 1000]> is exactly 1002 draws |
| FR-054-AC-3 | examined | returning its negation fails the produced-result step at every draw |
| FR-054-AC-4 | examined | Mutant subject with consequent x.value < 1000 fails ... presence 1 and payload 1000 |
| FR-054-AC-5 | examined | the unguarded claim ... stops at the absent draw with ClauseStop::Undefined and fails the produced-result step |
| FR-054-AC-6 | examined | The guarded claim of FR-054-AC-4 completes at the absent draw with the oracle's value true |
| FR-054-AC-7 | examined | A subject answering option.present(x) and option.value(x) <= 1000 ... presence 0 and payload 0 |
| FR-054-AC-8..11 | examined | composite option; non-scalar payload; option of an option |
| FR-054-AC-12..14, 17..20 | examined | not run (status) |
| FR-054-AC-15 | examined | reports the sibling supported with its harness unchanged |
| FR-054-AC-16 | examined | refused at package admission with OperatorIneligible |
| FR-054-AC-21 | examined | healthy and mutant runs through CG's two-phase runner |
| FR-054-AC-22 | examined | each stated verdict is observed |
| FR-054-AC-23 | examined | FR-015-AC-42 states ... refused with OptionNotLowered |

## Dispositions

Disposition pass round 1: fix commit 651e88f5 (was ce2221e9). Gate at 651e88f5: fmt, clippy `-D warnings`, `cargo deny check` and `make spec` exit 0; whole `--lib` 604 passed, 0 failed, 36 ignored; whole `--test it` 1131 passed, 0 failed, 120 ignored (the option module alone: 16 passed, 5 ignored). The committed `reviews/` copies of the three SR files cmp-match the review-pass files.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-5 is PARTIAL in FR-054 Status, tests.md and TC-068 Status; the run doc says only the guarded oracle's `value` step runs on the host and that the unguarded claim's produced result and the assume-presence mutant are not run. |
| FND-002 | fixed | AC-22 is PARTIAL on the same grounds, and the `FR-054-AC-22` tags are removed from the two host-probe tests. |
| FND-003 | fixed | `tc_068_the_negated_presence_oracle_fails_the_presence_claim_at_every_admitted_draw` enumerates the `not present` harness against subject `present` on the host: 1002 admitted draws, none passes, presence 1 / payload 0 among them. The run doc labels it a host enumeration. |
| FND-004 | fixed | The `FR-054-AC-14` tag is removed from the AC-10 test, and the record-option test is retagged `FR-054-AC-10` (it asserts `PayloadNotScalar`); AC-8 and AC-9 are no longer tagged. The AC-11 tag stays on the intake-refusal test, which matches AC-11 PARTIAL. |
