---
id: SR-872001
title: "QSL-2646 PR #872 pin batch code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; PR #872 head deb300445274693716fb0d1d717ff781b2e5d46c vs main 889e999589c98d3b2dfd96abc1b0c9b233d864de; Cargo.toml, Cargo.lock, src/oracle/function/owner_path.rs, plus unchanged src/core/profile.rs and src/oracle/function/owner_render.rs"
review_set: subset
---

## Summary

Ticket: QSL-2646 (title also names QSL-2633). Code-review with rust-review lane of the pin batch: FSL c053c3d1 -> cc1c4d22, filament-eval/checked 6689fa06 -> fa2b7220, contract-ir/model e576f749 -> c2146579, filament-verification-contracts tag v0.2.1 -> rev 27142196. Cargo.toml and Cargo.lock agree with FSL cc1c4d22 for every shared first-party crate, with one source per crate in the lock (the remaining agent-ix-semantic-ir double entry predates this PR). No `[patch]`, no path overrides. The two test-only `Body::resume` implementors in owner_path.rs take `&mut LiveContext` correctly.

## Verdict

CHANGES REQUIRED — the emitted-crate pin constants in src/core/profile.rs still name the old revisions, and the rendered `Body::resume` templates in owner_render.rs still use the three-argument signature. The pins are not carried into generated oracle crates, and the TC-024 guard test fails at this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | profile.rs MODEL/EVAL/FSL_REPLAY/FSL_PACKAGE_DEPENDENCY_SOURCE still name e576f749, 6689fa06 and c053c3d1; emitted oracle crates resolve the old crates and manifest_sources_match_this_crates_manifest (FR-014-AC-8, TC-024) fails. Set them to the Cargo.toml revs. | src/core/profile.rs:27,31,38,40 |
| FND-002 | high | Rendered generated `Body::resume` (leaf and nested-call templates) lacks the `context: &mut LiveContext` parameter that filament-eval fa2b7220 requires; once FND-001 is fixed, every generated FR-485 function oracle crate fails to compile. Add `_context: &mut eval::LiveContext` (ignoring it is correct: leaf bodies open no frames, and nested calls go through Transition::Call to the scheduler). | src/oracle/function/owner_render.rs:362,441 |
| FND-003 | medium | The PR title claims QSL-2633, which cannot be met at these pins: body_locations (QSL-2625) is only in contract-ir spec at c2146579, with no reader code in contract-ir or writer code in FSL cc1c4d22, so the sidecar test is unchanged and the format break has not landed. Remove QSL-2633 from the title so the merge does not close it. | PR #872 title |
| FND-004 | low | An extra blank line was added after the dev-dependencies table. Remove it. | Cargo.toml:69 |
| FND-005 | low | The filament-specification-qualification dev-dep stays at tag v0.1.1 (1946638c), while FSL cc1c4d22 uses rev f32bca6c. The lock is unaffected (dev-only), but this crate is not on the FSL rev. Align it or record why not. | Cargo.toml:59 |

## Coverage

Plan completion: not assessed. Gap-analysis was skipped: there are no production behavior changes beyond dependency pins, and the only Rust edit is in a test module. Tests were not run (review only); the coder's focused run is pending.
