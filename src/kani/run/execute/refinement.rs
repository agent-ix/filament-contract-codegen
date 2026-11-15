//! The typed refinement execution entry (FR-028-AC-24): one native process, launched through
//! FR-017's bounded launcher and held to the wall-clock ceiling of its shadow harness identity.
//!
//! The runner measures no memory and enforces no cap (FR-028-AC-21, ADR-007): the memory ceiling
//! the evidence names is the one the caller's environment is expected to enforce, and a process
//! the environment ends by a signal is recorded as ended, never as a verdict.

use std::{fs, io, num::NonZeroUsize, path::Path, time::Instant};

use quire_exact::Cancel;

use serde::Serialize;

use super::{settle, Concluded, KaniExecutionRefusal};
use crate::kani::{
    identity::{
        refinement::{
            plan_refinement, CaseCount, RefinementBudget, RefinementClass, RefinementDisagreement,
            RefinementOutcome, RefinementPlanError, RefinementSummary, RefinementVersions,
        },
        shadow::KaniShadowHarness,
        ProofCeilings,
    },
    run::{
        launch::run_bounded_launcher,
        memory::MemoryObservation,
        tool::{KaniInstallation, KaniTool, KaniToolError},
    },
};

/// One refinement run of one shadow harness's obligation.
pub struct KaniRefinementRequest<'a> {
    /// Caller-owned cancellation handle.
    pub cancel: &'a Cancel,
    /// The installation whose launch owners and peak-memory source the run uses.
    pub installation: &'a KaniInstallation,
    /// The shadow harness whose obligation runs.
    pub harness: &'a KaniShadowHarness,
    /// The compiled native refinement program (the obligation's source, built with the
    /// production subject). It takes the path of a cases file as its one argument.
    pub program: &'a Path,
    /// A directory the run may write its cases file in.
    pub work_directory: &'a Path,
    /// The case cap, sample size and seed.
    pub budget: RefinementBudget,
    /// The versions the program was built with.
    pub versions: RefinementVersions,
}

/// What one refinement run did (FR-028-AC-24).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementEvidence {
    /// The `module::harness` path of the shadow harness this refinement accompanies.
    pub harness_path: String,
    /// Exhaustive or sampled, as planned.
    pub class: RefinementClass,
    /// The size of the domain.
    pub case_count: CaseCount,
    /// How many cases the process executed.
    pub cases_run: u64,
    /// How many cases the plan held.
    pub cases_planned: u64,
    /// Boundary cases in a sampled plan.
    pub boundary_cases: u64,
    /// Seeded cases in a sampled plan.
    pub sample_size: u64,
    /// The seed.
    pub seed: u64,
    /// The two ceilings the run was held to: the identity wall clock, and the memory ceiling
    /// the caller's environment is expected to enforce.
    pub ceilings: ProofCeilings,
    /// The peak the environment provided, when it provided one.
    pub memory: MemoryObservation,
    /// How the run ended.
    pub outcome: RefinementOutcome,
    /// The versions the run used.
    pub versions: RefinementVersions,
}

impl RefinementEvidence {
    /// The summary strength mapping reads.
    #[must_use]
    pub fn summary(&self) -> RefinementSummary {
        match &self.outcome {
            RefinementOutcome::Agreed => RefinementSummary::Agreed(self.class),
            RefinementOutcome::Disagreed { .. } => RefinementSummary::Disagreed,
            RefinementOutcome::ReachedWallClockCeiling | RefinementOutcome::EndedBySignal => {
                RefinementSummary::ReachedCeiling
            }
        }
    }
}

/// Why a refinement run was refused or produced no evidence.
#[derive(Debug)]
pub enum KaniRefinementRefusal {
    /// The domain has no plan.
    Plan(RefinementPlanError),
    /// The cases file could not be written.
    CasesUnwritable(io::Error),
    /// The launcher refused or could not start the program.
    Execution(KaniExecutionRefusal),
    /// The caller's cancellation handle stopped the run. A cancellation is neither a ceiling nor
    /// a signal (FR-028-AC-24), so it yields no evidence and no strength.
    Cancelled {
        /// The first observed cancellation source.
        source: quire_contract_ir::kani::KaniCancellationSource,
    },
    /// The program exited unsuccessfully on its own, which is neither a verdict nor a signal.
    ProcessFailed {
        /// Its exit code.
        exit_code: Option<i32>,
    },
    /// The program's output does not state a complete result.
    ReportMalformed(String),
    /// The program ran a different number of cases than the plan held.
    CasesRunMismatch {
        /// Planned.
        planned: u64,
        /// Reported.
        run: u64,
    },
}

/// Runs the refinement obligation of `request.harness` and records its evidence.
///
/// # Errors
///
/// A refusal where the run produced no complete result: no plan, an unwritable cases file, a
/// launcher failure, a program that failed on its own, or output that is not a complete result.
pub fn execute_kani_refinement(
    request: &KaniRefinementRequest<'_>,
) -> Result<RefinementEvidence, KaniRefinementRefusal> {
    let obligation = &request.harness.obligation;
    let plan =
        plan_refinement(&obligation.domain, request.budget).map_err(KaniRefinementRefusal::Plan)?;
    let planned = plan.cases.len() as u64;
    let cases_path = request.work_directory.join(format!(
        "refinement-cases-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    let text: String = plan
        .cases
        .iter()
        .map(|case| {
            let line: Vec<String> = case.iter().map(ToString::to_string).collect();
            format!("{}\n", line.join(" "))
        })
        .collect();
    fs::write(&cases_path, text).map_err(KaniRefinementRefusal::CasesUnwritable)?;

    let ceilings = request.harness.identity.ceilings;
    let mut command = std::process::Command::new(request.program);
    command.arg(&cases_path).current_dir(request.work_directory);
    let launched = run_bounded_launcher(
        command,
        ceilings.wall_clock,
        NonZeroUsize::MIN,
        Instant::now().checked_add(ceilings.wall_clock),
        request.cancel,
        request.installation.owners(),
    );
    let _ = fs::remove_file(&cases_path);
    let launch = launched.map_err(|error| {
        KaniRefinementRefusal::Execution(KaniExecutionRefusal::Tool(KaniToolError::Io {
            tool: KaniTool::Launcher,
            path: request.program.to_path_buf(),
            error,
        }))
    })?;
    let memory = launch.memory;
    let (outcome, cases_run) = match settle(launch.outcome)
        .map_err(KaniRefinementRefusal::Execution)?
    {
        Concluded::TimedOut => (RefinementOutcome::ReachedWallClockCeiling, 0),
        Concluded::Cancelled { source } => return Err(KaniRefinementRefusal::Cancelled { source }),
        Concluded::Completed {
            exit_code: None, ..
        } => (RefinementOutcome::EndedBySignal, 0),
        Concluded::Completed {
            exited_successfully: false,
            exit_code,
            ..
        } => return Err(KaniRefinementRefusal::ProcessFailed { exit_code }),
        Concluded::Completed { streams, .. } => {
            let report = parse_report(&String::from_utf8_lossy(&streams.stdout))?;
            if report.run != planned {
                return Err(KaniRefinementRefusal::CasesRunMismatch {
                    planned,
                    run: report.run,
                });
            }
            let outcome = match report.first {
                None if report.disagreements == 0 => RefinementOutcome::Agreed,
                Some(first) => RefinementOutcome::Disagreed {
                    first,
                    disagreements: report.disagreements,
                },
                None => {
                    return Err(KaniRefinementRefusal::ReportMalformed(
                        "disagreements counted but none listed".to_owned(),
                    ))
                }
            };
            (outcome, report.run)
        }
    };
    Ok(RefinementEvidence {
        harness_path: request.harness.harness_path(),
        class: plan.class,
        case_count: plan.case_count,
        cases_run,
        cases_planned: planned,
        boundary_cases: plan.boundary_cases as u64,
        sample_size: plan.sample_size,
        seed: plan.seed,
        ceilings,
        memory,
        outcome,
        versions: request.versions.clone(),
    })
}

struct ProgramReport {
    run: u64,
    disagreements: u64,
    first: Option<RefinementDisagreement>,
}

fn parse_report(stdout: &str) -> Result<ProgramReport, KaniRefinementRefusal> {
    let malformed = |detail: &str| KaniRefinementRefusal::ReportMalformed(detail.to_owned());
    let mut first = None;
    let mut done = None;
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["DISAGREE", case, production, shadow] => {
                if first.is_none() {
                    let case = case
                        .split(' ')
                        .map(str::parse)
                        .collect::<Result<Vec<i64>, _>>()
                        .map_err(|_| malformed("case value"))?;
                    first = Some(RefinementDisagreement {
                        case,
                        production: (*production).to_owned(),
                        shadow: (*shadow).to_owned(),
                    });
                }
            }
            ["DONE", run, disagreements] => {
                done = Some((
                    run.parse().map_err(|_| malformed("cases run"))?,
                    disagreements
                        .parse()
                        .map_err(|_| malformed("disagreements"))?,
                ));
            }
            _ => {}
        }
    }
    let (run, disagreements) = done.ok_or_else(|| malformed("no DONE line"))?;
    Ok(ProgramReport {
        run,
        disagreements,
        first,
    })
}
