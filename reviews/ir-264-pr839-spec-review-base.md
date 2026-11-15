---
id: SR-17940486709564847048004
title: "IR-264 PR #839 spec-review base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-contract-codegen PR #839 at f0d42093, git diff origin/main...HEAD; IR-264 PR1; spec/oracle/functional/FR-018-composite-equality-oracles.md, spec/oracle/matrix/TC-029-composite-equality-oracles.md, spec/oracle/matrix/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-contract-codegen/FR-018
    type: reviews
  - target: ix://agent-ix/filament-contract-codegen/TC-029
    type: reviews
---

## Summary

Ticket: IR-264. The spec edits drop "Planned (IR-241, IR-264)" from the three FR-018 statements
for AC-21..23, rewrite the AC-21 and AC-23 rows and mark AC-22 PARTIAL, rescope TC-029 steps 13
to 15 to a `bounded` corpus, and split the tests.md row into Covered (AC-21, AC-23) and
Partially covered (AC-22). `quire validate` on the three files passes.

The AC-21 form fix was checked against the runtime and the IR: `quire_semantic_value`
`CompositeShape` has exactly `Record`, `Tuple` and `Union`; an `Option` is
`ValueType::Option` with no `CompositeDeclaration`, and the native AC-21 test asserts the two
option items' environments carry no declarations. In Contract IR an option is a
`composite_type` node with `semantic_form` `option` (the AC-23 reader follows it), so "an Option
has no declaration [in the runtime], so it is recorded as a type" is correct, and the new
`operand_types` field is what makes the option items' closure non-empty. FR-018 statement,
AC-21 row, TC-029 step 13 and tests.md agree on record/tuple/union. AC-22's PARTIAL is honest:
the remainder (boundary and seeded selection above the case cap) is named and owned (IR-241),
and every bounded-corpus domain fits the cap. No tool versions appear in the spec edits.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | FR-018's failure-mode row for AC-21 still says "Member order is not a mutant: a permutation changes no compared observable", while the AC-21 row this PR rewrote now says a closure that "reordered members fails the comparison" and the native AC-21 test carries a "reordered members" mutant (and tests.md lists "reordered-member controls"). The two statements contradict each other about whether member order is an observable of the closure. Reconcile: since AC-21 compares "member by member (name, presence, value type, order)", update the failure-mode row. | spec/oracle/functional/FR-018-composite-equality-oracles.md:792, spec/oracle/functional/FR-018-composite-equality-oracles.md:727 |
| FND-002 | low | TC-029 steps 13 and 15 lag the rewritten ACs: step 13 does not mention the operand comparison types that AC-21 now requires to equal the constructor's prepared operand types, and step 15 lists three closure mutants while AC-23 now also claims "lost an operand's option payload" and "a reconstruction that narrowed a bound or read `T?` as `T` fails it through the package read". Bring the steps in line with the AC rows. | spec/oracle/matrix/TC-029-composite-equality-oracles.md:256-262, spec/oracle/matrix/TC-029-composite-equality-oracles.md:272-276, spec/oracle/functional/FR-018-composite-equality-oracles.md:727-729 |

## Verdict

The spec fix (record/tuple/union; Option as a type) is correct and consistent across FR-018,
TC-029 and tests.md, and AC-22 is honestly PARTIAL. FND-001 is an internal contradiction the PR
introduced by rewriting AC-21 without its failure-mode row; fix it and FND-002 in this PR.

## Dispositions
