---
id: SR-202611210022442313144108505
title: "gap-analysis — QSL-2234 codegen PR 880"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; QSL-2234; Cargo.toml, Cargo.lock, src/core/profile.rs, src/kani/generate/frame.rs, tests/it/kani_obligations_state_frame.rs, tests/state_frame_support/native_twin.rs; FR-015-AC-78/81"
review_set: subset
---

## Summary

Latest review: final head `68d08803458d4816508e3185e931620a8046992e`; final narrow pin delta clear. Original b4b review and matrix snapshot are retained below. Final three remote checks now passed; the runtime merge condition is satisfied.

Reviewed exact PR 880 head `b4b5064618d44e866d0f6a283c979abccfd49708` against `f327415b488d3c91c4b7384eff1e81a53fed8bfd`. Ticket: QSL-2234. Six-file implementation change; no source, branch, commit, dependency cache, or PR mutation by this reviewer.

The selected-model override uses the existing `admit_emitted_package` seam: FSL `call_site` emits the balance clause using the ordinary domain, public IR `CheckedPackageEvidence::insert_domain_package_document` supplies the selected clock-reference domain, and public `CheckedPackageV2::read` must admit before public CG `generate_state_frame_obligations` runs. The initial accessor assertion checks IR classified the actual selected audit member as `ClockReference`. This is a consumer classification regression, not a claim that FSL emits or evaluates a clock-reference expression. The existing out-of-i64 fixture uses the same seam; no upstream fixture or artifact was copied.

The production exhaustive match maps `ClockReference` to `ModelMemberNotI64Range { reason: NonRangeType }`; `state_domains` maps that present member to persisted `TypeNotRange`. The test asserts one exact balance domain (0..1000), one audit unranged entry, the typed reason and complete record/identity round-trip. Giving the audit an i64 range, dropping its entry or changing its reason fails assertions. No clock evaluation, new solver, provider process limit implementation or Kani proof support was added.

Direct final-head `cargo metadata --locked --offline --format-version 1`, `cargo fmt --check` and `git diff --check` exited 0. Metadata proves resolution only. One IR/model source at exact1cbd and eight FSL sources at exact7303 were confirmed against manifest, emitted sources, Cargo.lock and exact7303's resolved workspace manifest. RT75688ead, exactb26a8e4c, SV6e809434, eval/checked89573102, obs1e356f99 and FSpec247a394 remain unchanged. Comparing 9ad8c11b to this head proves exactly four FSL manifest/profile substitutions and eight FSL lock-source substitutions; all other bytes are unchanged.

No compilation/runtime/Clippy/Kani/proof command ran locally. Root ran b4b/FSL7303 library check, six exact tests and focused Clippy through draco successfully; final68d/FSLf597 then passed the three focused changed-input checks recorded below. Earlier 9ad8c11b/d0ac evidence remains explicitly prior-cohort evidence. No whole-crate/all-target/full-gate claim is made. No applicable AssuranceProfile was found under spec/. No lint, CI gate or ignored-test setting was changed.

## Verdict

FAIL for the whole-repository computed matrix baseline: 208 untagged and 19 ignored-only criteria remain. The PR delta introduces no criterion edits or new trace gap; its changed behavior inspection is clear. This is not an engine-backed repository PASS. The lead must explicitly disposition the inherited baseline separately from final-head execution evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: filament-contract-ir = { version = "0.1.5", git = "https://github.com/agent-ix/filament-contract-ir",
    rev = "1cbd86a48f02e4623e558a3d3b23c066c7f280bf" }
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/filament-spec-language?rev=7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef#7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef"
- id: src/core/profile.rs
  path: src/core/profile.rs
  role: examined
  excerpt: 'pub const FSL_PACKAGE_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\",
    rev = \"7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef\"";'
- id: field_range
  path: src/kani/generate/frame.rs
  role: examined
  excerpt: '            | CheckedMemberType::ClockReference'
- id: selected_model_clock_reference_field_is_unranged_in_generated_frame_identity
  path: tests/it/kani_obligations_state_frame.rs
  role: examined
  excerpt: fn selected_model_clock_reference_field_is_unranged_in_generated_frame_identity()
    {
- id: emitted_package_with_unread_clock_reference_audit
  path: tests/state_frame_support/native_twin.rs
  role: examined
  excerpt: '    /// Admit the balance clause against a selected model whose unread
    audit names a clock.'
- id: FR-015-AC-78
  path: spec/kani/functional/FR-015-bounded-kani-obligations.md
  role: examined
  excerpt: PLANNED (IR-624), IR-628 accessor merged; CG implementation pending. `field_range`
    returns `MemberAbsent` for a requested name absent from the selected model declaration's
    accessor, including a clause's own field; an accessor `UnknownNode`, `NotModelObjectType`
    or `AmbiguousField(name)` returns `ModelFieldsUnavailable` with that error and
    object id, with no body or read fallback. A non-model object and an admitted unselected
    model/object_type with a nonempty body both yield `NotModelObjectType` and refuse
    even if body members claim valid ranges. A selected model/object_type whose nonempty
    body
- id: FR-015-AC-81
  path: spec/kani/functional/FR-015-bounded-kani-obligations.md
  role: examined
  excerpt: PLANNED (IR-624), IR-628 accessor merged; CG dependency update and implementation
    pending. A present listed model field with `member_type()` of `None`, a non-`IntRange`
    variant, or `IntRange` outside `i64` is recorded as unranged with `TypeNotRange`
    in `StateFrameIdentity` and its persisted record, even when no read names it;
    a present unread `IntRange` within `i64` is recorded in `domains` and is absent
    from the unranged list. Equal inputs produce byte-identical records; a record
    naming a field twice is invalid. The model declaration path records no `NoRead`
    reason. FSL-emission gaps for `Non
```

## Coverage

Plan completion: not assessed

Reconciliation: real `quoin matrix --repo . --json` and `quire matrix --scope . --format json` succeeded using quoin0.36.8 and quire0.52.0 (engine0.62.2), spec-artifacts-process@v0.36.0. These are newer installed tools than the release notice; no older binary fallback was used. The module declares the Rust trace attribute and the engine binds the new test to both FR-015-AC-78 and AC-81.

Whole repository: 1142 criteria; tagged860, untagged208, tagged-by-ignored-test19, method-without-symbol55. Evidence: no run evidence on all1142. `quire coverage` reports222 untracked symbols repository-wide, with Rust candidates2024/tagged1991/bound1950. The new test is bound and is absent from the untracked set. Existing aggregate gaps and marker diagnostics were recorded as baseline context; this bounded PR review does not re-review unrelated historical gaps or create tickets for them. No engine failure or unavailable matrix is asserted.

Reverse-gap inspection used the spec/spec.md current ownership inventory plus changed source: field_range and state_domains belong to FR-015/FR-025. New regression binds the existing non-range and persisted identity criteria. Changed lines introduce no source/test stub, no ignored test, no weakened gate, and no unowned new behavior. Full historical reverse-gap re-audit was outside the dispatched bounded review; no full repository clearance follows.

Optional semantic-review fan-out was skipped (no opt-in). Ordinary code-test alignment inspection was performed as part of code-review. No plan selected. Final-head focused runtime evidence now passed as recorded below; the matrix evidence store was not updated or rerun.


## Narrow final pin review

Latest reviewed head: `68d08803458d4816508e3185e931620a8046992e` (PR 880), narrow delta from previously reviewed `b4b5064618d44e866d0f6a283c979abccfd49708`. No new findings; no prior real findings required a fix disposition.

Independent mechanical comparison establishes exactly two manifest FSL strings, two emitted profile FSL strings, and eight Cargo.lock FSL source records changed from 7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef to f5977125981288ea1bd36fb67f71780182055aac. All ClockReference production/regression source and the declared spec/test graph remain byte-identical. All non-FSL package sources are unchanged; final metadata contains one package source for each IR/model and FSL public crate with no mixed FSL revisions. IR/model1cbd, RT756, exactb26, SV6e, eval/checked895, obs1e and FSpec247 remain selected. Emitted model/FSL source strings match the consumer manifest.

`gh api repos/agent-ix/filament-spec-language/compare/f5977125981288ea1bd36fb67f71780182055aac...main` reports merge-base f597, behind_by0, ahead_by4: the exact cutoff is reachable from the default branch. Later main commits were not selected. Exact cached f597 workspace Cargo.toml/Cargo.lock and fsl-replay/package/eval manifests are byte-identical to 7303. The upstream delta is limited to analyze/command, protocol simulation and related spec/review artifacts. It changes no FSL replay/package/semantics source. CG source/tests contain no calls to fsl_eval/fsl_analyze/fsl_command, PackageProtocolProgram, simulation::protocol, Program::from_package, run_protocol or analyze_package. A transitive fsl-eval compile remains warranted; no CG protocol runtime behavior is claimed.

Independent final-head locked/offline metadata, fmt-check and diff-check exit0. Metadata establishes resolution only. No local compilation, tests, Clippy or proofs ran. Final f597 library check, one exact manifest/profile test and focused library/integration-target Clippy now passed on the remote draco runner; their raw evidence is recorded below.

Retained measured runtime evidence is tied to exact heads: b4b/FSL7303 library check and Clippy succeeded, and six selected exact tests each report1 passed/0 failed/0 ignored. I inspected the raw logs under /home/peter/.cache/handoff/qsl-2234-cgb4b and verified its3387-byte raw.tar.gz SHA256 02ae4aaf452896f6f2646693ba49d830b94de9bb4827a08e14456eac6caba729. The initial unprivileged check log failed while opening Cargo FETCH_HEAD; the successful escalated check is a separate retained log. Earlier9ad/FSLd0ac nine-test evidence remains prior-cohort evidence. Unchanged long agreement tests were not rerun for this pin delta. Final68d focused checks subsequently passed, satisfying that runtime merge condition.

## Owner conditional PR disposition

Architecture owner ruling, delivered explicitly through the dispatching lead: conditionally accept the inherited matrix baseline for this bounded ClockReference consumer PR because no new gap is introduced; do not expand scope or create historical-gap tickets. Merge requires source review PASS, focused final-cohort checks and final-pin review. This is an owner conditional PR disposition, separate from the whole-repository audit verdict. The whole audit remains FAIL (1142 criteria,208 untagged,19 ignored-only); no engine PASS or full-audit CONDITIONAL is claimed. The spec/test graph is byte-identical, so the real prior matrix snapshot is retained without a redundant rerun. No newly unowned behavior, unbound new test or stub is introduced by revision substitutions. The final focused runtime condition is now satisfied; root retains responsibility for latest artifact custody and merge. QSL-2234 remains open as an umbrella.


## Final runtime evidence

Final exact runtime evidence at `68d08803458d4816508e3185e931620a8046992e`, FSLf597 / IR1cbd / exact FSpec247: remote `cargo check --locked -j4 --lib` exit0 (48.99s); `cargo test --locked -j4 --lib core::profile::tests::manifest_sources_match_this_crates_manifest -- --exact` exit0 with exactly1 passed,0 failed,0 ignored,636 filtered; `cargo clippy --locked -j4 --lib --test it -- -D warnings` exit0 (56.39s). I independently inspected the three retained logs and verified the2066-byte raw.tar.gz SHA256 eb02cf68c0c007333666e905d4359759b7dc4465dbabf22deb992f03805af92b at /home/peter/.cache/handoff/qsl-2234-cg68d. Root reports the leased remote checkout was restored to its original clean head after execution.

The final runtime merge condition is satisfied. Source/final-pin review remains clear at this exact head. The previous9ad nine-test and b4b six-test results retain their original heads; they are not68d test executions. These three focused successful commands do not discharge1142 criteria or change the whole-repository matrix FAIL. No new review methods, source inspection, matrix computation, metadata resolution, compilation or tests were rerun in this evidence-only refresh. Latest two reviewer artifacts are ready for coder custody and the lead's merge of PR880; QSL-2234 remains open as an umbrella.
