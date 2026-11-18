# IR-1447 control clause runs through CG's runner

Each row is a CG-generated harness (`let`, `if` and nested forms compiled from FSL source; the
laziness, selected-refusal and unread-initializer rows from constructed packages whose addition
node is typed by the amount range, because FSL types arithmetic as mathematical integers) executed
through the two-phase runner (FR-017-AC-63) with the installed backend. One job at a time, 24 GiB
cgroup, no swap. Probe context: `ir1447-kani-control-probe.md`.

| Criterion | Fixture | Verdict | Wall | cgroup memory.peak | Witness |
| --- | --- | --- | --- | --- | --- |
| FR-015-AC-156 | let healthy (`amount + 1 <= 1000`) | Verified, cover satisfied | 31.8 s | 2,172,379,136 B | none |
| FR-015-AC-156 | let mutant (`amount + 2`) | Falsified (witness phase) | 119.4 s | 9,749,979,136 B | amount = 999 |
| FR-015-AC-157 | if healthy | Verified | 50.3 s | 3,228,286,976 B | none |
| FR-015-AC-157 | if mutant (`amount <= 500`) | Falsified | 192.4 s | 15,193,313,280 B | amount = 500 |
| FR-015-AC-165 | nested healthy | Verified | 51.3 s | 3,214,577,664 B | none |
| FR-015-AC-165 | nested mutant | Falsified | 201.3 s | 14,974,558,208 B | amount = 500 |
| FR-015-AC-168 | lazy branch healthy (`Int[0, 999]`, else `amount + 1000`) | Verified | 31.4 s | 2,142,208,000 B | none |
| FR-015-AC-168 | eager-branch equivalent clause (strict `let` binds the never-selected `amount + 1000`) | Falsified at "the V2 postcondition holds for the produced result" | 109.3 s | 9,235,607,552 B | amount = 31 (in domain) |
| FR-015-AC-169 | selected refusing branch | Falsified | 126.4 s | 10,371,416,064 B | amount = 1000 |
| FR-015-AC-172 | unread refusing initializer | Falsified | 142.0 s | 14,534,299,648 B | amount = 1000 |
| FR-015-AC-172 | clause without the unread `let` (`result = (amount >= 0)`, `Int[0, 1000]`) | Verified | 6.1 s | 488,370,176 B | none |
| FR-015-AC-159 | collection if healthy (`forall(item in self.items: (if item < 2 then item else 2) <= 2)`, `items` 0 to 3 of `Int[0, 3]`, direct collection harness, unwind 8) | Verified, cover satisfied | 0.46 s test run (cold build plus run: 5.3 s for healthy and mutant together, peak 1,142,304,768 B) | 3,267,690,496 B (whole scope, including the test rebuild) | none |
| FR-015-AC-159 | collection if mutant (else branch `3`) | Falsified (witness phase) | 0.94 s test run | 180,699,136 B (whole scope, test binary already built) | an element of at least 2 |

Falsified runs of the let, if and nested rows fail at the step "the produced V2 Boolean result
agrees with the independent checked condition".

## Oracle-side mutants

The runner refuses a crate whose library does not contain the harness source byte for byte, so a
hand-edited generated oracle cannot go through it. Where a mutant has an equivalent clause, the
mutant is run as that package: the eager-branch mutant of AC-168 is the clause that binds the
never-selected branch value with a strict `let` (Falsified at the postcondition step, as above),
and the skip-unread mutant of AC-172 is the clause without the unread `let` (Verified, as above).
The hand-edited generated oracles are also run as host stop probes: eager gives
`Err(IntegerOutOfDomain)` at amount 0 where the healthy oracle gives `Ok(true)`, the
`false`-mapping mutant of AC-169 gives `Ok(false)` where the healthy oracle gives
`Err(IntegerOutOfDomain)`, and skip-unread gives `Ok(true)` where the healthy oracle stops. The
`false`-mapping mutant has no equivalent clause, so its Kani verdict is not run; only its stop is.

The collection rows run `tc_025_real_kani_collection_if_claim_verifies` and
`tc_025_real_kani_collection_if_else_mutant_falsifies` one Kani job at a time, each in its own
`systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0` under `flock /tmp/cargo-slot.lock`.
The mutant is a source-authored clause recompiled by FSL, because the runner needs the generated
source byte for byte. The collection `if` is emitted by the FSL pinned in `Cargo.lock`
(`be88cde3`); no pin moved.

## Not run (PARTIAL)

Union `case` (AC-154, 158, 162, 163, 171; FR-025-AC-19) needs a union operation parameter in the
subject signature; shadowing
(AC-170, 173) needs constructed packages; AC-161 and the four other reasons of AC-167 have no
reachable FSL clause fixture (their `control_operands` shape checks and the reader's `case_term`
are tested over FSL-compiled function packages); the `let` of `filament.op.option.value` clause of
AC-166 is not tested because the FSL checker refuses `value(...)` over the fixture's nullable
parameter. The AC-160 and AC-166 tests are default-lane tests in
`tests/it/kani_obligations_v2_control.rs`, not Kani runs.
