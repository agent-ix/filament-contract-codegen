---
id: SR-17940486709564847048041
title: "code-review of QSL-2286 PR #853 routed module cycle fix"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; PR #853 at 1cca13c0 over main 6b013e26; src/lib.rs, src/routed/{capability.rs,generic.rs,vocabulary.rs,mod.rs,generate.rs,contract_package_tests.rs}, tests/it/kani_scalar_native_private.rs"
review_set: subset
---

## Summary

Code review with the rust-review lane, scoped to `git diff origin/main...HEAD`. Ticket: QSL-2286. The PR fixes the red-main test `layout::l_2_the_module_graph_is_acyclic`. Merged #850 left `generic.rs` importing settlement types from `capability.rs` while `capability.rs` imported `negotiate_generic` and the generic arm vocabulary from `generic.rs`. The fix moves the shared value types into a new `src/routed/vocabulary.rs`. The import direction is now capability -> generic -> vocabulary, and capability -> vocabulary.

## Examined scope

- Pure move (examined). Every block deleted from `capability.rs` (old lines 25, 182-188, 194-304, 319-409, 484-595, 1220-1390) and from `generic.rs` (old 13, 18-180) appears contiguously, byte for byte, in `vocabulary.rs`. The only exception is three visibility prefixes. Comparing the sorted removed and added line multisets (LC_ALL=C) leaves only the import lines, the `vocabulary.rs` module doc, its `use` lines, and `pub(crate)` on `advertises`, `label` and `decides`. Derives, serde attributes (`rename_all`, `tag`, `content`, the variant `rename`s, `serialize_with = "serialize_domain_kind"`), field and variant order, the explicit `Ord`/index arms and the `Serialize` impls are all unchanged.
- Visibility (examined). `KaniFormName::label` is used from `capability.rs:962-979`, `BackendDescriptor::advertises` from `capability.rs:1363`, and `BackendProfile::decides` from `generic.rs:193`. Each one is now called across a module boundary, so `pub(crate)` is the minimum that compiles. The new module is `pub(crate) mod vocabulary`.
- Public API (examined). `mod routed` is private (`lib.rs:48`), so `routed::capability::X` was never a public path. The crate-root re-exports from `routed::*` are the same 55 names before and after. The `routed::capability::`/`routed::generic::` hits in sibling repos under /home/peter/dev are all snapshot copies of this crate under qualification-logs. No compat re-export or shim was added.
- Layout (examined). `layout.rs` is untouched. At base 6b013e26 `l_2_the_module_graph_is_acyclic` fails with "an import cycle". At head it passes, together with all eight layout tests, including `l_2_every_import_follows_the_dependency_direction`. `vocabulary.rs` sits inside `routed/` and imports only `fsl_replay`, `serde` and `std`. `MODULE_FILES` and the AD-004 map do not list `generic.rs` either, so the absence of `vocabulary.rs` follows the existing precedent.
- Ride-alongs (examined). The other changes are import-path edits only: in `contract_package_tests.rs`, `generate.rs`, `kani_scalar_native_private.rs` and the `generic.rs` test's `Candidate` path. Logic, `Cargo.toml` and `Cargo.lock` are unchanged.
- Gate at head (examined). `cargo fmt --check`, `clippy --locked --all-targets -D warnings`, `cargo deny check`, `make spec`, and `make rustdoc` (-Dwarnings, private items) all pass. `--lib` gives 617 passed, 36 ignored. `--test it` gives 1164 passed, 127 ignored, 0 failed. `git status` is clean.

## Verdict

Clean. This is a pure move with minimal, justified visibility widening. The layout test passes because the cycle is gone, and the test itself is not weakened. Mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
