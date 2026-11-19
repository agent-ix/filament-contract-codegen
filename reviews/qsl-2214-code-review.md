---
id: SR-17940486709564847048002
title: "Code review — filament-contract-codegen FSL pin update"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen; Cargo.toml, Cargo.lock, src/core/profile.rs; PR #866; QSL-2214"
review_set: subset
---

## Summary

Reviewed PR #866 at `86a464bd0b3737b6c8ca6b0789829c5c52853128` against `origin/main` and the stated requirement to pin codegen's FSL dependencies to `13b0b11e7b00e7412976d7a4b55cc93aed528c38`. The workspace manifest and lockfile use the requested commit, but emitted oracle manifests retain the previous FSL revision, and the existing consistency test fails.

The PR title has no ticket key and its body contains `Refs QSL-2214`, as requested. Linear resolves QSL-2214 to a protocol documentation request unrelated to this dependency pin; this review follows the explicit PR requirement in the dispatch brief.

## Verdict

**CONDITIONAL** — one medium finding remains: generated oracle manifests still pin the prior FSL revision. The manifest consistency test reproduces the mismatch. No aggregate gate was run.

## Scope

| Unit | Path | Role | Excerpt |
| --- | --- | --- | --- |
| PR-866-pin | PR #866 body | examined | Pin the codegen workspace's FSL packages to current FSL main `13b0b11e7b00e7412976d7a4b55cc93aed528c38`. |
| Cargo.toml-fsl-deps | Cargo.toml | examined | fsl-replay = { version = "0.3.0", git = "https://github.com/agent-ix/filament-spec-language", rev = "13b0b11e7b00e7412976d7a4b55cc93aed528c38" } |
| Cargo.lock-fsl-packages | Cargo.lock | examined | source = "git+https://github.com/agent-ix/filament-spec-language?rev=13b0b11e7b00e7412976d7a4b55cc93aed528c38#13b0b11e7b00e7412976d7a4b55cc93aed528c38" |
| generated-manifest-source | src/core/profile.rs | examined | pub const FSL_REPLAY_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\", rev = \"a2f4ae873c6f0eb8628bb1e79541f6af46ddfa17\""; |
| QSL-2214-context | Linear QSL-2214 | context_only | The docs show role projection, realizability and composition with running examples. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Generated oracle manifests still pin the old FSL revision | src/core/profile.rs:38-40 |

## Review Details

The changed `Cargo.toml` dependencies and all eight FSL package source entries in `Cargo.lock` resolve to `13b0b11e7b00e7412976d7a4b55cc93aed528c38`. `cargo test --locked manifest_sources_match_this_crates_manifest -- --exact core::profile::tests::manifest_sources_match_this_crates_manifest` failed: `src/core/profile.rs` still embeds `a2f4ae873c6f0eb8628bb1e79541f6af46ddfa17` in `FSL_REPLAY_DEPENDENCY_SOURCE` and `FSL_PACKAGE_DEPENDENCY_SOURCE`, so emitted function/equality oracle `Cargo.toml` files target a different FSL commit than the codegen workspace. Update those source constants with the dependency pin so generated artifacts preserve the same pin.

The FSL dependency update is limited to `Cargo.toml` and `Cargo.lock`; no CI workflow or spec/plan files changed. The PR reports workspace `cargo check --workspace --all-targets` and two targeted integration tests as passed, plus `git diff --check`. This review ran only the targeted manifest consistency test; it failed as above. No separate Quoin gap-analysis ran because the diff changes dependency declarations and lock resolution only, with no production source implementation changed. No applicable AssuranceProfile was found.

## Findings Context

The code review includes the Rust lane. No API, safety, concurrency, or wire behavior was changed in the diff. The dependency source mismatch is concrete and independently covered by an existing focused test, which fails on this head.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Both generated-manifest FSL source constants now pin `13b0b11e7b00e7412976d7a4b55cc93aed528c38`; the focused consistency test passes on `9038a2137d8cabeecf2a1753dbef87141b7e075a`. |
