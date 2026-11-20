---
id: SR-17940486709564847048108
title: "QSL-2561 PR #876 gap-analysis (diff-scoped)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen PR #876 at 5f835865, git diff origin/main...HEAD; QSL-2561; Cargo.toml, Cargo.lock, src/replay/function.rs, src/replay/scalar_builder.rs, tests/it/composite_parity_converter.rs, tests/it/kani_obligations_symbolic_domains.rs"
review_set: subset
---

## Summary

Ticket: QSL-2561. This is a diff-scoped gap check, not a full repository audit. The PR adds
no requirement, no acceptance criterion and no new behaviour. It is a dependency re-pin plus
the compile-forced addition of one wire member, so no matrix row changes. The ticket text could
not be read (the linear CLI timed out on the host keyring), so the PR's own claims were used as
the acceptance list and each was re-measured:

1. FSL crates pinned to 8f71444d: measured in Cargo.toml and in all eight Cargo.lock entries.
2. contract-ir/model pinned to af45a983, the revision FSL 8f71444d pins: measured in both
   lock entries and in FSL's own Cargo.lock at 8f71444d.
3. Non-hybrid requests set `certificate: None`, and wire comparison includes it: measured in
   src/replay/function.rs:505 and src/replay/scalar_builder.rs:487 and 504.
4. Existing replay tests still pass at the new pins: the coder's
   `cargo test --lib replay::` (42 passed) was re-run, and every target was compiled.

No stub, no `todo!`, no `unimplemented!` and no new untraced production code are introduced.
`quire matrix` was not run: the diff changes no spec, test tag or traced item, so the matrix
cannot change.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for the diff scope. The FSL QSL-2184 paired-pin comment (Cargo.toml) asks for the driver
to sit at the same FSL revision. That is the driver repository's own wave ticket, outside this
PR, but the lead should land both together.
