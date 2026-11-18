---
id: SR-17940486709564847048049
title: "spec-review base of filament-contract-codegen PR #854 FSL unproved-nonzero fixture fix"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen; PR #854 at 7667c4b9; spec/** and plan/** (no changes)"
review_set: subset
---

## Summary

Trivial spec-review base. The PR has no ticket of its own. Its title refs QSL-2286, and findings are posted to IR-854 as the dispatch directs. `git diff --name-only origin/main...HEAD -- spec plan` is empty, so the PR changes no spec or plan. `make spec` exits 0 at head, with pre-existing grammar warnings only in files this PR does not touch. No spec sub-analyses apply.

## Examined scope

- spec/** and plan/** diff (examined): empty.

## Verdict

Clean. Nothing to review in the spec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Extension review (disposition pass 2)

Scope extension at head 93fb4342 (`git diff babf58d5...93fb4342 -- spec plan`). Ticket: IR-854. The only spec change is the FR-048-AC-6 row.

- FR-048-AC-6 (examined). The added clause is true at the pinned FSL: the lib conversion test asserts the `UndefinedExpression` / `unproved-nonzero` compile refusal. It is matrix-tagged (TC-062: `src/kani/terminal/constrained/tests.rs:74`, `tests/it/constrained_method.rs:145`, `:400`) and is tested at head. `make spec` exits 0, and FR-048 has no grammar warning.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The amended FR-048-AC-6 puts test-design prose into the criterion: "so the conversion rule is checked on the owner's own Undefined evaluation and the refusal is checked as the typed compile cause". The AC grows to four claims and mixes a statement about FSL's behaviour with how CG's test is built. "The owner's own Undefined evaluation" also reads as owner-produced, while the test constructs the value directly. Keep the behaviour, for example "an unproved-nonzero source divisor is refused at compile time with `undefined_expression` and never reaches Undefined", and state in TC-062 or the test that the Undefined input is directly constructed. | spec/kani/functional/FR-048-constrained-state-clause-method.md:208 |
