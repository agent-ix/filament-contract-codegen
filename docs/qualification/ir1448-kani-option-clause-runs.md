# IR-1448 option clause runs through CG's runner

Each Kani row is a CG-generated harness compiled from FSL source (`amount` typed by an option over
`Int[0, 1000]`) and executed through the two-phase runner (FR-017-AC-63) with the installed backend.
One job at a time, 24 GiB cgroup, no swap, each verdict in its own scope. The memory column is the
scope's cgroup `memory.peak`, which covers the test binary and the backend. Probe context:
`ir1448-kani-option-probe.md`, which is not CG evidence.

| Criterion | Fixture | Verdict | Wall | cgroup memory.peak | Witness (presence, payload) |
| --- | --- | --- | --- | --- | --- |
| FR-054-AC-3 | `result = present(amount)`, subject `present` | Verified, cover satisfied | 3.1 s | 392,069,120 B | none |
| FR-054-AC-3 | equivalent package `result = (not present(amount))`, same subject | Falsified | 18.4 s | 1,925,926,912 B | presence 0, payload 0 |
| FR-054-AC-4 | `result = (present(amount) implies value(amount) <= 1000)`, subject `!present \|\| value <= 1000` | Verified, cover satisfied | 15.0 s | 1,122,131,968 B | none |
| FR-054-AC-4 | same claim, subject `!present \|\| value < 1000` | Falsified | 45.1 s | 4,631,707,648 B | presence 1, payload 1000 |
| FR-054-AC-7 | same claim, subject `present && value <= 1000` | Falsified | 45.5 s | 4,851,875,840 B | presence 0, payload 0 |

Falsified runs fail at the step "the produced V2 Boolean result agrees with the independent checked
condition". Wall is the runner's own time; the lock queue is not counted.

## Oracle-side mutants

The runner refuses a crate whose library does not contain the harness source byte for byte, so a
hand-edited generated oracle cannot go through it. The oracle mutant of FR-054-AC-3 (the presence
read returns its negation) is therefore run as the equivalent package above. The oracle mutants of
FR-054-AC-5 and FR-054-AC-6 have no equivalent package, because the FSL checker refuses an unguarded
`value(...)` (`unproved-presence`); they are host stop probes of the generated oracle compiled in a
scratch crate, not Kani verdicts. Only the guarded oracle's `value` step is called; the unguarded
claim's produced result and an assume-presence mutant are not run, so AC-5 and AC-22 are PARTIAL:

- absent read: `Err(Undefined)`; present read of 7: `Ok(7)`; a default payload 0 in place of the
  stop completes with `Ok(0)`.
- guarded claim at presence 0, payload 0: `Ok(true)`; the same oracle with the consequent
  evaluated before the presence test: `Err(Undefined)`.

## Host enumeration

The unchanged harness body runs on the host over presence x payload `-2..=1003`:
1002 draws are admitted (the absent draw at payload 0 and the present draws `0..=1000`) and the
healthy subject passes each. The `not present` oracle against the subject `present` fails all 1002
admitted draws, presence 1 / payload 0 included (a host enumeration, FR-054-AC-3). With the canonical-absent assumption removed the absent draw is
admitted at payload 1 as well; with the present bound widened payload 1001 is admitted.

## Not reachable or not run (PARTIAL, unowned, planner to assign)

- Composite option (FR-054-AC-8, AC-9): FSL accepts `Option<Record>` and `value(x).n`, but the
  admitted checked package holds the record type as a `record_value_type` model node whose term is
  an empty aggregate. No field layout reaches the lowering, so the option is refused
  `PayloadNotScalar` naming `record_value_type`. A constructed package would need the checked
  package schema to carry record fields.
- Option of an option (FR-054-AC-11): Contract IR intake refuses it (`Option<Option<T>> has no
  spelling`), so no package reaches the lowering. A record payload holding an option is not run.
- Unbounded integer option (FR-054-AC-16): a declared value type `integer` is refused at admission
  (`OperatorIneligible`), but `Option<native Integer>` is admitted, reaches the lowering and is
  refused by the generic `V2ClauseConditionNotRendered`. `requires_bound` is not emitted.
- Record field read as an option (FR-054-AC-12): a `field?` slot and a required `Option<T>` field
  are both typed option by the checked package, and both are refused `NullablePresence`.
- `Some` (FR-054-AC-13): the FSL checker elides it, so no `filament.op.option.some` node exists and
  the clause is refused by the generic `V2ClauseConditionNotRendered`.
- Option reads outside a Boolean-result postcondition over a parameter: the generic refusal.
- Non-scalar payload (FR-054-AC-10): only a bounded text payload is run (reason `PayloadNotScalar`,
  family `text_bounds`); the other families are not run.
- Not run: FR-054-AC-14, AC-17 to AC-20, and the Kani mutant of AC-6.
