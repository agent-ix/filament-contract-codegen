---
id: SR-17940486709564847048109
title: "QSL-2997 PR #898 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #898 at f1ffeae1, git diff origin/main...HEAD; QSL-2997; Cargo.toml, Cargo.lock, src/replay/**, src/kani/generate/negotiate.rs, tests/it/replay_clocks.rs"
review_set: subset
---

## Summary

Ticket: QSL-2997 (also QSL-2999 context). `ReplayInputs` gains `clocks: ReplayClocks`; `ReplayInputs::admit` builds
`RequestClocks::for_replay` once via `ReplayClocks::request`; ReplayPackage::new, FrameReplay::new and the three
state-clause paths pass it to `call_site`. A malformed row becomes `DependencyLockError::Clock`, settled as `CgDefect`.

Checked and sound: no non-test `RequestClocks::default()` remains in src; the two remaining test-module kani sites
use the helper with empty rows. `ReplayInputs.source` as program source is sound: `LockedSource::wire()` and
`source_identity()` are the same (authority, identity) pair FSL's `get_for_source` compares. A program row filed as a
dependency row is refused (test and swap mutation both fail). Mutation: helper returning `RequestClocks::default()`
fails all 3 new tests; swapping program/dependencies fails 2 of 3. Pins: only Cargo.toml/Cargo.lock, no `[patch]`,
no duplicate filament/fsl/agent-ix crate in `cargo tree --locked -d`; FSL 82a764c5 pins contract-ir ddbc69ef, equal to
this repo's pin. `InvalidClock` carries no FSL catalog code, so `CgDefect` agrees with FR-029-AC-16's no-code reading.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | Frame and the three state-clause paths are not tested for clock forwarding; replacing `&clocks` by the empty selection in all four leaves the suite green | src/replay/frame.rs:775, src/replay/state_clause.rs:668 |
| FND-002 | medium | No test asserts `DependencyLockError::Clock` settles as `CgDefect`, which FR-016-AC-31 states | src/replay/function.rs:1496 |
| FND-003 | low | PR adds rustfmt drift: 5 src files plus tests/it/main.rs (mod placement) fail `cargo fmt --check` | src/lib.rs:377, src/replay/composite_builder.rs:741 |

## Verdict

Mergeable after FND-001 and FND-002 are tested, or accepted by the lead; neither is a behavioural bug.
