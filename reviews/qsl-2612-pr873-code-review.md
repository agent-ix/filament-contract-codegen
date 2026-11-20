---
id: SR-17940486709564847048104
title: "code-review of QSL-2612 PR #873 derived state-clause witness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; PR #873 at 7350a284 (git diff origin/main...HEAD); src/replay/state_clause.rs, src/core/profile.rs, tests/it/state_clause_derived_witness.rs, tests/it/main.rs, Cargo.toml, Cargo.lock"
review_set: subset
---

## Summary

Code review with the rust-review lane, scoped to `git diff origin/main...HEAD`. Ticket: QSL-2612 (description plus the correction comment 9f030e7c, which supersedes it). Authority also: FSL FR-269-AC-6 to AC-9 and the public `fsl_replay::derive_state_clause_witness` at FSL 2cb29fcd (#3128 merge commit).

`StateClauseReplay::replay_through` now reconstructs the envelope with `witness: None`, calls `derive_state_clause_witness(wire, &envelope, replay_limits)`, stores `Ok(Some(record))` unchanged, keeps `None` on `Ok(None)`, `RangeViolation` and `NotRefuted`, and returns the new `StateClauseReplayError::Derivation(Box<DerivationRefusal>)` on any other refusal. The settlement maps `Replay` to `Refused`, `Fault` to `Fault`, and other refusals to `SetupRefused(code)` by matching `code()` against `Code::all()`, falling to `CgDefect` when nothing matches.

## Examined scope

- Single seam (examined). Every builder, `StateClauseReplay::new` (postcondition, `Invocation`), `new_invariant` (V2 clause harness: invariant, postcondition, precondition) and the graph builder, constructs `Self { wire, packet, replay_limits }`, and `replay()` delegates to `replay_through`. So the one seam covers all of them, and it derives over the exact envelope the executor receives. That is better than the ticket's per-builder wording. `graph.rs:158` is `GraphReplayInputs.witness` (the decoded `GraphWitness`), not the payload record, so it needs nothing. `graph.rs:577`, `invariant.rs:428` and `state_clause.rs:745` are the payload `witness: None` that the seam now fills.
- No CG-built record (examined). The record is moved from FSL's `Ok(Some(_))` into `payload.witness` and nothing else touches it.
- NotRefuted extension (examined). This is consistent with FSL FR-269. `NotRefuted` means "the payload is no counterexample", and FR-269-AC-9 says replay tolerates an absent record. Ticket step 3 needs the crediting run to stay `Inconclusive(ReplayParity(Verdicts))`. Mapping `NotRefuted` by its code would give `Inconclusive(ReplayRefused(invalid_runtime_input))` and break that. FSL specifies no caller treatment that contradicts it.
- RangeViolation (examined). It keeps `None` and the replay runs (correction comment). `tc_040_out_of_range_debit_post_state_is_exact_and_refuted` passes through the new path.
- Fault (examined). `Fault` maps to `ReplaySettlement::Fault`, so the run is `Failed` with code `runtime_invariant`, the same fault reading FR-029-AC-10 gives a `ReplayRefusal` fault.
- Ordering and cost (examined). The derivation runs FR-122's front half: it decodes the request, recompiles the package, resolves the clause, admits the observation and evaluates the clause once. The executor then does all of that again, so each replay compiles and evaluates twice. The envelope is also reconstructed twice. Each call is bounded by `replay_limits` on its own, so no limit is exceeded, but the two calls share no meter and the total cost is about double. This is inherent to FR-269's standalone entry and to the ticket's design, so it is not counted as a defect. An `Exhausted` derivation stops the run before the replay, as `SetupRefused(resource_exhausted)`.
- Pin (examined). `fsl-replay` and `fsl-package` use `rev = 2cb29fcd5a8dcdb85612bad90b34bec0045571fd` in `Cargo.toml`. `FSL_REPLAY_DEPENDENCY_SOURCE` and `FSL_PACKAGE_DEPENDENCY_SOURCE` in `src/core/profile.rs` match, and `manifest_sources_match_this_crates_manifest` passes. `Cargo.lock` changes only the 8 FSL `source` lines. These are rev pins, not tags. Mismatch to report, not judged as a defect: the driver is on FSL 8877b38b (2cb29fcd is 52 commits ahead of it), and arch PR #872 pins cc1c4d22 (4 commits ahead of 2cb29fcd). The ticket's "CG and the driver must stay on one FSL rev" needs merge-order coordination.
- Rust idioms (examined). The new variant is boxed. `Display` has its arm and the `From<&StateClauseReplayError>` match has no wildcard. There is no panic or unsafe. `rustfmt --check` is clean on the changed files.
- Tests run (focused, name-filtered). In `it`, 7 named tests pass, including `tc_040_the_state_clause_replay_reads_as_fr029_ac16` and both new `tc_035_*` tests. The `terminal_map`, `state_clause*` and `kani_obligations_state_clause_replay` modules pass 63 with 2 ignored. In `lib`, the two new unit tests and the `core::profile` tests pass, 8 in all. The full gate, clippy and Kani lanes were not run (thermal brief).

## Verdict

**CHANGES REQUESTED (one medium).** The seam, the record custody, the NotRefuted and RangeViolation handling, and the pin are correct. FND-001 is an honest-mapping hole: a coded FSL refusal can still end as `Failed` with no code. FND-002 is a simplification nit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Derivation` fall-through `map_or(Self::CgDefect, ..)` is reachable. `DerivationRefusal::Refused(SeparationRefusal)` carries the evaluation refusal's `CatalogCode` spelling, and kernel codes such as `integer_out_of_domain`, `division_out_of_domain`, `decimal_out_of_domain` and `rational_out_of_domain` are in FSL's catalog (`CATALOG_CATEGORIES`) but not in `fsl_foundation::Code::all()`. A state clause whose evaluation refuses with one of them therefore settles `CgDefect`, which is `Failed` with the code dropped. That violates the ticket's "never Failed without a code" and FR-029-AC-16's "any other FSL refusal as `Inconclusive(ReplayRefused)` with its catalog code", and it contradicts `CgDefect`'s doc ("carries no FSL catalog code"). | src/replay/state_clause.rs:457-465 |
| FND-002 | low | `Code::all().iter().copied().find(\|c\| c.as_str() == other.code())` re-implements `fsl_foundation::Code::from_code`, which FSL already exports and round-trip tests. | src/replay/state_clause.rs:461-464 |

## New findings (disposition pass 1)

Re-reviewed at head 0734e1f8 (fix commit 4a256a10, then a merge of origin/main). PR-only diff against main: 10 files. Pins now equal main's exactly: FSL cc1c4d22, the same in `Cargo.toml`, `Cargo.lock` and `src/core/profile.rs`, so the driver/#872 mismatch is resolved by #872 landing.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The `carry_derived_witness` doc says a refused, undefined or exhausted evaluation is "settled by the replay itself, which carries its own catalog code and cause in its result". FSL at cc1c4d22 does not do that. `execute/state_clause.rs:240-244` discards the refusal (`CallOutcome::Refused(_) => (Category::Refusal, None)`, `Incomplete(_) => Incomplete`, `Undefined { .. } => Inconclusive`), and the result settles `inconclusive` with `DisagreementCause::NoValue { proved: violation, replayed: <category> }`. Only the category reaches CG; no code and no cause. The behaviour is FSL's own settlement and is acceptable, but the comment states a property that does not hold. | src/replay/state_clause.rs:813-818 |

## New findings (disposition pass 2)

Re-checked at head da02e2ca (wording-only commit; `rustfmt --check` and `quire validate` clean).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The FND-003 rewrap left one 156-character doc line, "FR-029-AC-16), not by a catalog code, so each leaves it too. A refusal of the request ends the replay as the replay's own refusal, and an internal fault". The repo's width is 100 (`rustfmt.toml` `max_width = 100`, CLAUDE.md). rustfmt does not wrap comments, so no gate catches it. The only other over-width line in the file, now line 1346, predates this PR. | src/replay/state_clause.rs:817 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fall-through to `CgDefect` is gone. `Refused`, `Undefined` and `Exhausted` now keep `witness` absent and the replay settles them as FSL's `inconclusive` (`NoValue`), never `Failed`. Only `Replay` (to `Refused`) and `Fault` (to `DerivationFault`, settled `Fault`) end the replay. |
| FND-002 | fixed | The `Code::all()` scan was removed together with the `Derivation` variant. |
| FND-003 | fixed | Doc now reads "reports them as an inconclusive `NoValue` disagreement with the replayed category (FR-122, FR-029-AC-16), not by a catalog code". That matches FSL cc1c4d22 execute/state_clause.rs:240-244. |
