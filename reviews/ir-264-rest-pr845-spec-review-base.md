---
id: SR-17940486709564847048022
title: "IR-264 PR #845 spec-review (base)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #845 at 1f6c08ec, git diff origin/main...HEAD (base ebe100f7); IR-264 PR4; spec/kani/functional/FR-015-bounded-kani-obligations.md, spec/kani/functional/FR-028-bounded-proof-ceilings.md, spec/kani/matrix/TC-025-bounded-kani-obligations.md, spec/kani/matrix/TC-039-bounded-proof-ceilings.md, spec/kani/matrix/tests.md, spec/oracle/functional/FR-018-composite-equality-oracles.md, spec/oracle/matrix/TC-029-composite-equality-oracles.md, spec/oracle/matrix/tests.md"
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

Ticket: IR-264 (PR4). Base spec-review of the status and requirement edits: FR-015-AC-72
(Implemented), AC-73 and AC-74 (PARTIAL), the FR-015 unsupported-shape table, FR-028-AC-17 and
AC-23 (PARTIAL), FR-018-AC-22 (PARTIAL), the TC-025 Kani table and step 32-34 notes, TC-039,
TC-029 and both `tests.md` rows. Sub-analyses: none beyond the base pass (no new FR, no new
relationship edges; EARS and integrity checks folded in below). `make spec` exits 0; the only
warning on a touched line is `ears:non-singular` on FR-015 line 793 (the AC-72 statement), which
had two `shall`s before this PR and three now.

Checked clean: AC-72, AC-73 and AC-74 status lines name the tests that exist and keep AC-73 and
AC-74 PARTIAL; AC-74's second case matches the AC text (two unconstrained `i64`-range leaves,
`sampled`); FR-028-AC-17's owner (IR-806 covers FR-029 AC-17/21..27 per its description);
FR-018-AC-22's QSL-2249 dependency (In Progress in Linear); no tool version numbers added to any
doc; FR-015/FR-028 headers and the FR-015 `tests.md` row cite PR4 consistently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-028-AC-23's status adds, after "Not covered", a present-tense sentence describing behaviour no code has: "a run reads the versions of the backend it launched (the launcher reports the Kani and CBMC versions; the Rust compiler and the bundled solver are read the same way), and a stand-in launcher that reports none records none rather than a guess". The installed launcher does print both versions (`cargo-kani --version`), but `KaniExecutionEvidence` has no version field and nothing in `src/kani` runs a version probe, so no run reads them. The AC's normative text is unchanged, so this is not a narrowing of the requirement, but the sentence reads as implemented design and grants an exemption (a stand-in records none) the AC does not grant ("Every execution evidence records the Kani version..."). Mark it as a planned design note or drop it; the AC stays PARTIAL either way. | spec/kani/functional/FR-028-bounded-proof-ceilings.md:233 |
| FND-002 | low | The normative unsupported-shape table moves `quantity`, `set`, `bag` and `ordered set` out of the `ShadowFamilyNotBuilt` row into a new row carrying FR-018's refusal, "while FR-018 does not read its bound". The correction is sound (the old row said FR-018 "still generates" for them, which it does not), but the time-conditional clause puts implementation state into a requirement row, and the AC-73 status and the FR-015 `tests.md` row still list "the `ShadowFamilyNotBuilt` rows for ... `quantity`" as not exercised, a row the table no longer has. | spec/kani/functional/FR-015-bounded-kani-obligations.md:815-817, spec/kani/functional/FR-015-bounded-kani-obligations.md:926, spec/kani/matrix/tests.md |
| FND-003 | low | The FR-028 row of `spec/kani/matrix/tests.md` still reads "Partial (IR-264 PR2, PR3)" although this PR edits that row and FR-028's header now names PR4 for the composite family. | spec/kani/matrix/tests.md |

## Verdict

Changes requested on FND-001 (medium); FND-002 and FND-003 are wording fixes. On the brief's
question about FR-028-AC-23: the AC was not reworded, only its status note; the note is not an
honest narrowing (it describes an unbuilt design in the present tense and adds an exemption), so
it should not stand as written. The AC remains PARTIAL, as the PR says.
