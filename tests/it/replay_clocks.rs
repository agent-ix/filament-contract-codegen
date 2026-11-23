//! FR-016-AC-31 and FR-024-AC-78: a replay request carries the clock selections the proving
//! package recorded, program rows for the proved unit and dependency rows for its libraries, and
//! never an empty selection that FSL would refuse a clocked unit under.

use std::collections::BTreeMap;

use filament_contract_codegen::{
    DependencyLock, DependencyLockError, LockedSource, ReplayClocks, ReplayInputs, ReplayPackage,
    ReplayPackageError, WitnessDecodeLimits,
};
use fsl_package::ClockConfiguration;
use fsl_replay::{ReplayLimits, ScalarLimits};
use ix_trace_rs::trace;

const AUTHORITY: &str = "agent-ix";
const PROGRAM_ID: &str = "test:clocks-program";
const LIBRARY_ID: &str = "test:clocks-plant";
const LIBRARY: &str = "test/plant";
const PROFILES: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n";
const LIBRARY_BODY: &str = "record Reading { level: Int[0, 9]; }\n\
     function flag using v(): Boolean pure { true }\n\
     temporal Late using w over (r: Reading) clock \"lib-ms\" \
     on origin { eventually[0,5] holds(r.level > 0) }\n";
const PROGRAM_PLAIN: &str = "import \"test/plant\" as P;\n\
     function entry using v(): Boolean pure { true }\n";
const PROGRAM_CLOCKED: &str = "record ProgramReading { level: Int[0, 9]; }\n\
     function entry using v(): Boolean pure { true }\n\
     temporal Early using w over (r: ProgramReading) clock \"prog-ms\" \
     on origin { eventually[0,2] holds(r.level > 0) }\n";

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn locked(identity: &str, body: &str) -> LockedSource {
    LockedSource {
        authority: AUTHORITY.to_owned(),
        identity: identity.to_owned(),
        bytes: format!("{PROFILES}{body}").into_bytes(),
    }
}

fn millis() -> ClockConfiguration {
    ClockConfiguration::TimestampedEvent {
        timestamp_unit: "ms".into(),
    }
}

fn inputs(program: &str, with_library: bool, clocks: ReplayClocks) -> ReplayInputs {
    ReplayInputs {
        clocks,
        decode_limits: WitnessDecodeLimits {
            bytes: 4096,
            values: 2,
        },
        package_name: "clocks".to_owned(),
        source: locked(PROGRAM_ID, program),
        dependencies: if with_library {
            vec![DependencyLock {
                identity: LIBRARY.to_owned(),
                source: locked(LIBRARY_ID, LIBRARY_BODY),
                refinement_internals: None,
            }]
        } else {
            Vec::new()
        },
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        declared_domains: Vec::new(),
        replay_limits: ReplayLimits::default(),
    }
}

fn dependency_rows() -> ReplayClocks {
    ReplayClocks {
        program: BTreeMap::new(),
        dependencies: BTreeMap::from([("lib-ms".to_owned(), millis())]),
    }
}

/// A library clause on the clock `lib-ms` replays under the lock's dependency clock row and is
/// refused under the empty selection the replay path passed before.
#[trace("FR-016-AC-31", "TC-026")]
#[test]
fn tc_026_a_dependency_clock_request_replays_with_the_dependency_row() {
    let empty = ReplayPackage::new(
        inputs(PROGRAM_PLAIN, true, ReplayClocks::default()),
        "entry",
    )
    .expect_err("an empty selection leaves the library clock unselected");
    assert!(matches!(empty, ReplayPackageError::CallSite(_)), "{empty}");

    ReplayPackage::new(inputs(PROGRAM_PLAIN, true, dependency_rows()), "entry")
        .expect("the dependency clock row selects the library clock");
}

/// The program rows serve only the proved unit's own source: a program clause replays under a
/// program row, and the same row filed as a dependency row does not select it.
#[trace("FR-016-AC-31", "TC-026")]
#[test]
fn tc_026_program_and_dependency_clock_rows_are_separate_namespaces() {
    let program_rows = ReplayClocks {
        program: BTreeMap::from([("prog-ms".to_owned(), millis())]),
        dependencies: BTreeMap::new(),
    };
    ReplayPackage::new(inputs(PROGRAM_CLOCKED, false, program_rows), "entry")
        .expect("the program row selects the program clause's clock");

    let wrong_namespace = ReplayClocks {
        program: BTreeMap::new(),
        dependencies: BTreeMap::from([("prog-ms".to_owned(), millis())]),
    };
    let refusal = ReplayPackage::new(inputs(PROGRAM_CLOCKED, false, wrong_namespace), "entry")
        .expect_err("a dependency row does not serve the program source");
    assert!(
        matches!(refusal, ReplayPackageError::CallSite(_)),
        "{refusal}"
    );
}

/// A clock row that is no well-formed configuration is refused before any replay, as a
/// `DependencyLockError::Clock`, whichever namespace holds it.
#[trace("FR-016-AC-31", "FR-024-AC-78", "TC-026", "TC-035")]
#[test]
fn tc_026_a_malformed_clock_row_is_refused_before_replay() {
    let bad = ClockConfiguration::TimestampedEvent {
        timestamp_unit: "".into(),
    };
    for clocks in [
        ReplayClocks {
            program: BTreeMap::from([("p".to_owned(), bad.clone())]),
            dependencies: BTreeMap::new(),
        },
        ReplayClocks {
            program: BTreeMap::new(),
            dependencies: BTreeMap::from([("d".to_owned(), bad.clone())]),
        },
    ] {
        let refusal = ReplayPackage::new(inputs(PROGRAM_PLAIN, true, clocks), "entry")
            .expect_err("the malformed row is refused");
        assert!(
            matches!(
                refusal,
                ReplayPackageError::Dependencies(DependencyLockError::Clock(_))
            ),
            "{refusal}"
        );
    }
}
