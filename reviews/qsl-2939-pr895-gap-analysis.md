---
id: SR-202611210022442313144108535
title: "gap-analysis — QSL-2939 codegen PR895"
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-contract-codegen; git diff origin/main...dc472f76 and the criteria its tests trace
  (FR-014-AC-8, FR-018-AC-12, FR-018-AC-37)
review_set: subset
relationships:
- target: ix://agent-ix/filament-contract-codegen
  type: references
---

## Summary

Ticket: QSL-2939. PR #895, head `dc472f76`. This is a planless audit scoped to the diff. `quire matrix --scope .
--format json` (quire 0.52.1, engine 0.62.2) exited 0.

FR-014-AC-8 loses its binder `manifest_sources_match_this_crates_manifest` but is still tagged by 7 tests
(`src/core/profile.rs` 2, `src/kani/run/scalar.rs` 1, `tests/it` 4). FR-018-AC-12 is tagged by 2 tests and
FR-018-AC-37 by 3. The diff introduces no untagged criterion.

Reverse gap: `build.rs` is the "one manifest authority" that FR-014-AC-8 and FR-018-AC-12 require, now derived
from Cargo.toml. It adds no unowned behaviour. Stubs: none. The `..` reads and `temporal_profile: None` test
constructions are mechanical, and no assertion was weakened (see the code-review artifact for tc_029_ac12 and
tc_068).

The diff changes no spec. Two spec passages now describe the old state.

## Verdict

CONDITIONAL. There is one low documentation-drift finding and no traceability gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Spec prose no longer matches the source authority. FR-018 Outputs says the profile supplies "release tags (and, for fsl-replay, the revision)", but every first-party source except ix-trace-rs is a rev pin and tc_029_ac12 now accepts either. AD-004 says the profile names the runtime "git URL and branch" and that spellings are "defined in core/profile.rs once", but they are now generated from Cargo.toml by build.rs. | spec/oracle/functional/FR-018-composite-equality-oracles.md:222-225, spec/assurance/AD-004-cg-crate-layout.md:920-923 |

## Examined scope

```yaml
- {id: FR-014-AC-8, path: spec/oracle/functional/FR-014-exact-scalar-oracles.md, role: examined, excerpt: "The generated crate declares publish = false, names the direct exact/FSV and Runtime residue dependencies from the one profile authority without a replay-context dependency for Boolean/integer scalar proofs, contains no charge amount (every charge comes from runtime metering), and compiles."}
- {id: FR-018-AC-12, path: spec/oracle/functional/FR-018-composite-equality-oracles.md, role: examined, excerpt: "The generated crate declares publish = false, names direct exact/FSV and root replay dependencies through the one manifest authority, contains no charge amount and no planned pair count (every charge and every pair comes from runtime metering), and compiles."}
- {id: FR-018-AC-37, path: spec/oracle/functional/FR-018-composite-equality-oracles.md, role: examined, excerpt: "Equality's generated manifest and native scratch consumers use the one profile authority's direct exact/FSV/replay dependencies, while ordinary Boolean/integer FR-014 scalar proof manifests and oracle bodies acquire no replay source-compilation/context leg."}
```

## Coverage

- Matrix: `quire matrix` exit 0; the three criteria in scope are all `tagged`.
- Reverse gap and stubs: checked over the diff; none found.
- Semantic review: skipped (non-interactive dispatch; no opt-in).
- Plan completion: not assessed
