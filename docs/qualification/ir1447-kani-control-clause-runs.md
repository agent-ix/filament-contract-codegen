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
| FR-015-AC-169 | selected refusing branch | Falsified | 126.4 s | 10,371,416,064 B | amount = 1000 |
| FR-015-AC-172 | unread refusing initializer | Falsified | 142.0 s | 14,534,299,648 B | amount = 1000 |

Falsified runs of the let, if and nested rows fail at the step "the produced V2 Boolean result
agrees with the independent checked condition".

## Host probes of oracle-side mutants

The runner refuses a crate whose library does not contain the harness source byte for byte, so a
mutated generated oracle cannot go through it. The eager-branch mutant (AC-168), the
`false`-mapping mutant (AC-169) and the skip-unread mutant (AC-172) are instead run as host tests
of the generated oracle recording its stop: eager gives `Err(IntegerOutOfDomain)` at amount 0
where the healthy oracle gives `Ok(true)`, the `false`-mapping mutant gives `Ok(false)` where the
healthy oracle gives `Err(IntegerOutOfDomain)`, and skip-unread gives `Ok(true)` where the healthy
oracle stops.

## Not run (PARTIAL)

Union `case` (AC-154, 158, 162, 163, 171; FR-025-AC-19) needs a union operation parameter in the
subject signature; collection `if` (AC-155, 159) needs a collection predicate arm; shadowing
(AC-170, 173) needs constructed packages; AC-161 and the four other reasons of AC-167 have no
reachable FSL fixture.
