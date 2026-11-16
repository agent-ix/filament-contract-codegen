---
id: SR-17940486709564847048028
title: "IR-264 PR #847 spec-review (base)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #847 at 1ae756d3, git diff origin/main...HEAD (base 8c80f954); IR-264 PR5; spec/kani/functional/FR-015-bounded-kani-obligations.md, spec/kani/functional/FR-028-bounded-proof-ceilings.md, spec/kani/matrix/TC-025-bounded-kani-obligations.md, spec/kani/matrix/TC-039-bounded-proof-ceilings.md, spec/kani/matrix/tests.md, spec/oracle/functional/FR-021-function-application-oracles.md, spec/oracle/matrix/TC-029-composite-equality-oracles.md; context spec/oracle/functional/FR-014-exact-scalar-oracles.md, spec/oracle/functional/FR-018-composite-equality-oracles.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-025
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/FR-021
    type: reviews
---

## Summary

Ticket: IR-264 (PR5). Base spec-review of the status and requirement edits; no sub-analyses beyond
the base pass (no new FR, no new relationship edges). `make spec` exits 0 with no warning on a
touched line.

Checked clean: FR-028-AC-23's normative sentence is unchanged and its status names what is covered
(Kani and CBMC releases, proof, witness, batch, typed absence) and what is not (the Rust compiler
version, with no PATH substitute); TC-039 and both `tests.md` rows agree. FR-015-AC-73's reference
clause is reworded to "a `reference` composite reached through a record field settles with FR-018's
typed refusal (FR-018-AC-7), carried unchanged, with no harness", matching the table row and the
test; the same-request clause text is unchanged and its NOT MET status names IR-806 PR-3 and
FR-029-AC-30 consistently in FR-015, TC-025 and `tests.md`. #120 is no longer called an open blocker
in FR-015, TC-025's AC-73 note, `tests.md`, FR-021-AC-10, TC-029 step 3 or FR-028's deferred list;
the wire value is kept. No tool version numbers in spec text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several places still say "blocked on" #120, against the PR's own wording ("a closed issue; its wire value is kept as an identifier"): TC-025 line 1226 "Model and graph bounds: refused as blocked on `agent-ix/filament-spec-language#120`" (in a file this PR edits), FR-014 line 535 "model and relation families as blocked on filament-spec-language#120", and FR-021-AC-11 "map to the filament-spec-language#120 blocker". Reword them the same way. | spec/kani/matrix/TC-025-bounded-kani-obligations.md:1226, spec/oracle/functional/FR-014-exact-scalar-oracles.md:535, spec/oracle/functional/FR-021-function-application-oracles.md:540 |
| FND-002 | low | TC-025's AC-73 note now says "by design the composite family is not an item kind of the `negotiate_kani_obligations` request", but the same paragraph ends "the generator is a public entry that takes the claim-map entry, and is not yet an item kind of the negotiated request". "Not yet" contradicts "by design". FR-015-AC-73's status also has a garbled clause, "NOT MET: and the `ShadowFamilyNotBuilt` outcome for a `set`, `bag` or `ordered set` leaf". | spec/kani/matrix/TC-025-bounded-kani-obligations.md:736-747, spec/kani/functional/FR-015-bounded-kani-obligations.md:925 |

## Verdict

The spec edits are honest and consistent with the code on the three questions (AC-23 PARTIAL, the
AC-73 same-request ownership, the reference row). Two low wording defects remain.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The sentence added to FR-018-AC-7, "a `reference` composite reaches it only after the lowering profile has refused its `model` target, so a `reference` form there falls to the generic `Unsupported` refusal", contradicts itself: an item whose `model` target the profile refused is settled as `PackageAccounting` and never reaches the closure reader. Say the reader is not reached for a `reference` composite and that a `reference` form would fall to the generic `Unsupported` arm; no test exercises that arm with a `reference` form. | spec/oracle/functional/FR-018-composite-equality-oracles.md:713 |

## Dispositions

Round 1, at a48c7b25 (fix commits 4a0181c1 and a48c7b25). Gate at a48c7b25, own worktree and target, cargo-slot lock, `--locked`: fmt, clippy `--all-targets -D warnings`, `cargo deny` and `make spec` pass; whole `--lib` 616 passed / 0 failed / 36 ignored; whole `--test it` 1150 passed / 0 failed / 127 ignored; `git status --porcelain` empty after both runs (no file written into the tree). Ignored real-backend test `tc_039_installed_kani_evidence_records_the_launchers_own_versions`, one job, 24G scope: passed, Kani 0.68.0 / CBMC 6.11.0, scope wall 0.28 s, `memory.peak` 108,904,448 B.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-025 line 1226, FR-014 line 535 and FR-021-AC-11 now say "the typed refusal whose wire value is ...#120 (a stable identifier; that issue is closed)"; no "blocked on #120" or "#120 blocker" wording is left in spec/ or src/. |
| FND-002 | fixed | The TC-025 note ends "the generator is a public entry that takes the claim-map entry; by design it is not an item kind of the negotiated request"; FR-015-AC-73's status now reads "NOT MET: the `ShadowFamilyNotBuilt` outcome for a `set`, `bag` or `ordered set` leaf ...". |

Round 2, at 39571271 (fix commit 39571271). Gate at 39571271, own worktree and target, cargo-slot lock, `--locked`: fmt, clippy `--all-targets -D warnings`, `cargo deny` and `make spec` pass; whole `--lib` 617 passed / 0 failed / 36 ignored; whole `--test it` 1150 passed / 0 failed / 127 ignored; `git status --porcelain` empty afterwards.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | The rewritten sentence is still not true as written, on two counts. (1) "every `reference` composite targets a model object type" contradicts the same AC's earlier sentence, which lists the permitted targets as "model object type, process, state machine, systems interface, or relationship"; only the model case is tested (`tc_025_a_reference_composite_settles_with_fr_018s_refusal_and_no_harness`). (2) "The closure reader has no `reference` arm of its own" is false: the closure reader (`src/oracle/equality/closure.rs`, FR-018-AC-21, the "closure reader" FR-028-AC-18 names) has `ValueType::Reference(_) => RecordedType::Other { name: "reference" }` at line 134. The arm this PR removed, and the generic `Unsupported { node_tag: "composite_type" }` arm left behind, are in the operand type resolution (`resolve_type`, `src/oracle/equality/resolution.rs`), not in the closure reader. Fix: name `resolve_type` (operand type resolution) as the component; say that each permitted target (model object type, process, state machine, systems interface, relationship) is outside the equality lowering profile and is refused as `PackageAccounting` first (or limit the claim to model-object targets); keep "no test reaches that arm". Spec-only change. |
