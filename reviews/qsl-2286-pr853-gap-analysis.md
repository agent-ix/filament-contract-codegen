---
id: SR-17940486709564847048042
title: "gap-analysis of QSL-2286 PR #853 routed module cycle fix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-contract-codegen; PR #853 at 1cca13c0; src/routed/{capability.rs,generic.rs,vocabulary.rs}, tests/it/layout.rs (AD-004 L-2)"
review_set: subset
---

## Summary

Gap analysis scoped to the PR diff. Ticket: QSL-2286. Plan completion: not assessed. The PR adds no AC, test or trace tag, and it changes no behaviour. The AD-004 L-2 evidence test `l_2_the_module_graph_is_acyclic` was red on main and is green at head. FR-019 (AC-1, AC-29) and FSpec FR-711 settlement code is unchanged, so its tagged tests in `tests/it/capability_settlement.rs` and `tests/it/item_entry.rs` keep their coverage. The whole `--test it` run passes (1164/127/0). The spec paths that name `src/routed/capability.rs` (FR-039 Verification, NFR-005 row 137 and AC-1, FR-019 line 409, AD-001 line 288) still name items that stay in that file: `negotiate_contract_package`, `ItemSettlement::warning`, `BackendKind::from_identity`, and the negotiation arms. `vocabulary.rs` has no panic tokens and is scanned by the src/-walking NFR-005 scan.

## Examined scope

- AD-004 L-2 acyclic clause (examined): fails at base, passes at head, and the test is unedited.
- FR-019-AC-1 / FR-019-AC-29 (context_only): the code that implements them is moved, not changed.
- NFR-005-AC-1 (examined): the measured-site file list still holds `src/routed/capability.rs`, and the `unreachable!` site stays there.

## Verdict

Clean. No coverage gaps were introduced and no stale spec paths were left.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
