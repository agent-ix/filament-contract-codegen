//! FR-024-AC-76 and FR-024-AC-77 (QSL-2612): the separating witness record the state-clause replay
//! carries is the one FSL derives (FR-269), and equals the one FSL's replay holds on the result.

use filament_contract_codegen::StateClauseReplay;
use fsl_replay::{replay_state_clause, ReplayResult, SeparatingWitnessRecord};
use ix_trace_rs::trace;

use crate::kani_obligations_state_frame::{emitted_fixture, native_twin::Twin};

/// Replays the twin's `BalanceNeverDrops` clause from `pre` to `post`, returning the record the
/// envelope carried to FSL's replay and the one the result holds.
fn replayed(
    pre: (i64, i64),
    post: (i64, i64),
) -> (
    Option<SeparatingWitnessRecord>,
    Option<SeparatingWitnessRecord>,
) {
    let twin = Twin::new();
    let fixture = emitted_fixture(&twin, "BalanceNeverDrops");
    let replay = StateClauseReplay::new(twin.state_clause_inputs(
        &fixture.package,
        &fixture.clause,
        "BalanceNeverDrops",
        pre,
        post,
    ))
    .expect("the replay is built");
    let limits = replay.replay_limits();
    let mut carried = None;
    let result = replay
        .replay_through(|wire, envelope| {
            carried = envelope.family_payload().witness.clone();
            replay_state_clause(wire, envelope, limits)
        })
        .expect("the replay settles");
    let ReplayResult::Witness(arm) = result.result() else {
        panic!("a witness-arm envelope settles on the witness arm");
    };
    (carried, arm.record().cloned())
}

/// A debiting run falsifies the clause: the record the counterexample carries is present and is
/// the record FSL's replay holds on the result.
#[trace("TC-035", "FR-024-AC-76")]
#[test]
fn tc_035_the_carried_witness_record_is_the_record_the_replay_result_holds() {
    let (carried, held) = replayed((5, 0), (4, 0));
    let held = held.expect("the replay holds the decisive record");
    assert_eq!(carried.as_ref(), Some(&held));
}

/// A crediting run satisfies the clause, so FSL derives no record (`NotRefuted`): none is carried
/// and none is held, and the replay settles the disagreement of verdicts itself.
#[trace("TC-035", "FR-024-AC-77")]
#[test]
fn tc_035_a_clause_that_holds_carries_no_witness_record() {
    let (carried, held) = replayed((5, 0), (6, 0));
    assert_eq!(carried, None);
    assert_eq!(held, None);
}
