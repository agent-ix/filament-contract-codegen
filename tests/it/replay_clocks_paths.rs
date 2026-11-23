//! FR-024-AC-78: the frame and state-clause replay paths put the recorded clock rows before FSL's
//! call site. Each control replays a unit with a clause on a clock: the recorded row is accepted,
//! and the empty selection the paths passed before is refused as a call-site refusal.

use filament_contract_codegen::{FrameReplayError, StateClauseReplay, StateClauseReplayError};
use ix_trace_rs::trace;

use super::kani_obligations_state_frame::native_twin::recorded_clocks as recorded;
use super::kani_obligations_state_frame::{
    frame_harness,
    native_twin::{Twin, CLAUSES},
    replay_of, run_of,
};

const BALANCE: &str = "BalanceNeverDrops";

fn frame_refused_at_call_site(twin: &Twin) -> Option<String> {
    let harness = frame_harness(&Twin::new());
    match replay_of(twin, "deposit", &run_of(&harness, [5, 0])) {
        Err(FrameReplayError::CallSite(refusal)) => Some(refusal.to_string()),
        Err(_) | Ok(_) => None,
    }
}

/// The frame replay selects the recorded program row for a clocked unit.
#[trace("FR-024-AC-78", "TC-035")]
#[test]
fn tc_035_the_frame_replay_forwards_the_recorded_clocks() {
    let _ = CLAUSES;
    let refusal = frame_refused_at_call_site(&Twin::new().with_replay_clock_clause(recorded()));
    assert!(
        refusal.is_none(),
        "the recorded row selects the unit's clock: {refusal:?}"
    );
    assert!(
        frame_refused_at_call_site(&Twin::new().with_replay_clock_clause(Default::default()))
            .is_some(),
        "an empty selection leaves the unit's clock unselected"
    );
}

fn state_clause_refused_at_call_site(twin: &Twin) -> Option<String> {
    let (package, clause) = Twin::new().emitted_package(BALANCE);
    match StateClauseReplay::new(twin.state_clause_inputs(
        &package,
        &clause,
        BALANCE,
        (5, 7),
        (4, 7),
    )) {
        Err(StateClauseReplayError::CallSite(refusal)) => Some(refusal.to_string()),
        Err(_) | Ok(_) => None,
    }
}

/// The state-clause replay selects the recorded program row for a clocked unit.
#[trace("FR-024-AC-78", "TC-035")]
#[test]
fn tc_035_the_state_clause_replay_forwards_the_recorded_clocks() {
    let refusal =
        state_clause_refused_at_call_site(&Twin::new().with_replay_clock_clause(recorded()));
    assert!(
        refusal.is_none(),
        "the recorded row selects the unit's clock: {refusal:?}"
    );
    assert!(
        state_clause_refused_at_call_site(
            &Twin::new().with_replay_clock_clause(Default::default())
        )
        .is_some(),
        "an empty selection leaves the unit's clock unselected"
    );
}
