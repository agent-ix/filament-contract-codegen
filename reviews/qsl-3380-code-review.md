---
id: SR-9302
title: "QSL-3380 PR #909 code-review (rust-review lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-contract-codegen PR #909 at a8d9dae4e1c8d4472c8860187dd2184ded36d5f6, git diff 521796cc...a8d9dae4; QSL-3380; Cargo.toml, Cargo.lock"
review_set: subset
---

## Summary

Ticket: QSL-3380. Step 2 of a pin bump. The PR is one commit that changes only `Cargo.toml` and
`Cargo.lock`. It moves `filament-contract-ir` and `filament-contract-model` (normal and dev) from
fcefa8460e to 67ba7b9e2a. It moves `fsl-replay`, `fsl-package`, `fsl-eval` and `fsl-semantics`
from 43561bc0fc to c7d8ea8cb9. The PR says Refs QSL-3380 in its title, body and commit, not
Closes.

What was checked:

- Lock and manifest agree. Every contract-ir manifest line names 67ba7b9e2a. Both contract-ir
  lock entries (ir, model) carry that revision. All eight FSL lock entries carry c7d8ea8cb9.
  The lock holds exactly one `filament-contract-model` and one `filament-contract-ir`.
  `filament-exact` is still one entry at 08a5a1763e and was not touched. The only other lock
  change is `fsl-replay` now listing `filament-walk`. That crate was already in the lock at tag
  v0.1.1, and FSL's `fsl-replay/Cargo.toml` adds it within the range. No crate is duplicated
  because of this PR.
- Pins only move forward. fcefa8460e is an ancestor of 67ba7b9e2a, which is on contract-ir
  main. The range is #1179, #1180, #1176, #1178, #1183 and #1182. It leaves out #1184/#1185 as
  the PR says. 43561bc0fc is an ancestor of c7d8ea8cb9.
- The FSL commit. c7d8ea8cb9 has the single parent 075f52ac38, which is on FSL main. It changes
  only the FSL `Cargo.toml` contract-model pin and `Cargo.lock`, which matches its stated
  change. FSL's contract-model pin at c7d8 is 67ba7b9e2a, the same as codegen's, so the graph
  does not split.
- No other file has to move. No file outside the manifest and lock names the old or new
  revisions. CI (`ci.yml`, workflow_dispatch only) and the Makefile `LOCAL_PATCHES` list crate
  names, not revisions. `deny.toml` lists repository URLs only.
- Changes from contract-ir that codegen consumes:
  - #1179 adds `CertificateMember::ZeroWeight`. Codegen does not name `CertificateMember`, so no
    exhaustive match can break, and `check --all-targets` passed.
  - #1176 makes the strict reader refuse an owned `protocol_clause` that has no owner operand.
    Every hand-built codegen node sets `owner_model: None`. The published FSpec fixture that the
    `FSPEC_DIR` tests read (`positive-all-families.json`, both at contract-ir's CI ref a6968fb8
    and at FSpec main) already carries `{"operand": 3}` on its owned protocol clause, so it is
    still admitted.
  - #1178 encodes checked terms iteratively and is documented as producing the same bytes.
  - #1180, #1182 and #1183 change only tests or spec text in contract-ir.
- Gate logs (`qsl-3380-cg-{check,test-it-pins,test-lib-prep,clippy}.log`) show exit 0, 7 passed
  and 1 passed. The `head=` lines read 521796cc, not a8d9dae4 (see FND-002).
- `quoin:gap-analysis` was skipped. The diff has no production source and no acceptance
  criterion, so there is nothing to match against the test matrix.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The FSL move 43561bc0..c7d8 brings in 37 FSL main commits (46 files under fsl-package/src and fsl-replay/src: protocol-clause owner-operand keying #3920, dimension_equivalence key change #3957, SmtRunLimits moved into fsl-replay plus monitor.max_activations refused #3955, source-operation route #3956). Codegen consumes these through compile_admitted, WireGraph and the replay facade. The PR body describes only the contract-ir range, and the tests that ran did not include codegen's replay unit tests (src/replay, about 47 tests). | Cargo.toml:42-43, Cargo.toml:61-62 |
| FND-002 | low | The gate logs record head=521796cc (base plus uncommitted edits), not the PR head a8d9dae4. They show the tests ran on a tree that was committed later at 13:20:55, not on the head itself. | Cargo.lock |
| FND-003 | low | Once this PR merges, codegen main pins FSL c7d8ea8cb9, which is reachable only from branch titan/qsl-3380-pin-bump until the FSL PR lands with a merge commit. A squash merge, or deleting the branch first, would leave codegen main's lock pointing at a commit that cannot be fetched. | Cargo.toml:42 |

## Verdict

The manifest and lock are consistent and every pin moves forward. There is one copy each of
contract-ir and contract-model, and filament-exact is unchanged. The contract-ir behaviour
changes either do not reach codegen or keep its fixtures admitted. FND-001 is the only finding
that blocks merge: amend the PR body to state the FSL range, and run
`cargo test --locked --lib -- replay::` on a8d9dae4. FND-002 is closed by the same run, which
records head=a8d9dae4. FND-003 needs no code change, only the stated order (merge-commit the FSL
PR and do not delete its branch first). The lead can accept it with no change.

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | The PR body now has a section "FSL range consumed: 43561bc0fc..c7d8ea8cb9" that names #3920, #3957, #3955 and #3956 and what codegen uses of each. `cargo test --locked --lib -- replay::` gave 42 passed and 0 failed, and its log records head=a8d9dae4. |
| FND-002 | fixed | Logs `qsl-3380-cg-head-{check,test-replay,test-units}.log` record head=a8d9dae4e1c8d4472c8860187dd2184ded36d5f6 with exit 0. Workspace check passed, replay:: gave 42 passed, and `--test it -- admitted_quantity_units::` gave 2 passed. |
| FND-003 | accepted-no-change | The lead will merge the FSL PR with a merge commit and keep branch titan/qsl-3380-pin-bump until then, so c7d8ea8cb9 stays reachable. |
