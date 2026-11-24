// SPDX-License-Identifier: AGPL-3.0-or-later

//! Production execution and FSL settlement for one temporal SMT proof.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    num::NonZeroU64,
    path::{Path, PathBuf},
};

use filament_exact::Cancel;
use fsl_replay::{
    CertificateCheckLimits, CertificateRequest, ProofFormat, RefutedQuery, SmtProofBasis,
    SmtProofCertificate, SmtProofOutcome, SmtQueryKind, TerminalValue,
};

use super::{
    captured_alethe_proof, next_attempt, parse_solver_status, ProcessLimits, ProcessOutcome,
    ProcessRequest, ProcessRun, ReachedLimit, SolverRunError, SolverStatus,
};

/// A genuine FSL query and the direct process invocation that answered it.
#[derive(Debug)]
pub struct TemporalProofQuery {
    kind: SmtQueryKind,
    attempt: NonZeroU64,
    query_bytes: Vec<u8>,
    proof_bytes: Option<Vec<u8>>,
    status: SolverStatus,
    process: ProcessRun,
}

impl TemporalProofQuery {
    /// The FR-315 query kind submitted to the solver.
    #[must_use]
    pub fn kind(&self) -> SmtQueryKind {
        self.kind
    }

    /// The distinct CG invocation identity.
    #[must_use]
    pub fn attempt(&self) -> NonZeroU64 {
        self.attempt
    }

    /// The exact canonical query bytes sent on stdin.
    #[must_use]
    pub fn query_bytes(&self) -> &[u8] {
        &self.query_bytes
    }

    /// Alethe proof bytes captured from this invocation, when present.
    #[must_use]
    pub fn proof_bytes(&self) -> Option<&[u8]> {
        self.proof_bytes.as_deref()
    }

    /// The interpreted response or typed non-success.
    #[must_use]
    pub fn status(&self) -> SolverStatus {
        self.status
    }

    /// The bounded process evidence for this query.
    #[must_use]
    pub fn process(&self) -> &ProcessRun {
        &self.process
    }
}

/// The result of running the temporal proof queries.
#[derive(Debug)]
pub enum TemporalSmtProofResult {
    /// At least one solver invocation was not a completed `Unsat` query.
    /// No FSL proof outcome or terminal value was constructed.
    NonProof {
        /// The complete typed query records, in FSL query order.
        queries: Vec<TemporalProofQuery>,
        /// The first query that did not supply an `Unsat` response.
        status: SolverStatus,
    },
    /// FSL accepted the mapped proof basis and certificate for settlement.
    Settled(Box<TemporalSmtProofSettlement>),
}

impl TemporalSmtProofResult {
    /// Query records in FSL order, for either result branch.
    #[must_use]
    pub fn queries(&self) -> &[TemporalProofQuery] {
        match self {
            Self::NonProof { queries, .. } => queries,
            Self::Settled(settlement) => &settlement.queries,
        }
    }
}

/// One solver outcome mapped to an FSL `SmtProofOutcome` and settled by FSL.
#[derive(Debug)]
pub struct TemporalSmtProofSettlement {
    /// The proof basis and certificate offered to FSL.
    pub outcome: SmtProofOutcome,
    /// FSL's terminal settlement after checking the certificate.
    pub terminal: TerminalValue,
    /// The genuine solver invocations that supplied the certificate fields.
    pub queries: Vec<TemporalProofQuery>,
}

/// Why the production adapter could not prepare or execute a temporal query.
#[derive(Debug)]
pub enum TemporalSmtProofError {
    /// FSL refused to encode the selected query.
    Encoding {
        /// The rejected query kind.
        kind: SmtQueryKind,
        /// FSL's original refusal.
        refusal: fsl_replay::SmtEncodingRefusal,
    },
    /// CG could not produce a trustworthy bounded process record.
    Solver {
        /// The query whose process failed.
        kind: SmtQueryKind,
        /// The original runner error.
        error: SolverRunError,
    },
}

impl fmt::Display for TemporalSmtProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding { kind, refusal } => {
                write!(
                    formatter,
                    "FSL refused {} query encoding: {refusal:?}",
                    kind.name()
                )
            }
            Self::Solver { kind, error } => {
                write!(formatter, "{} query solver run: {error}", kind.name())
            }
        }
    }
}

impl std::error::Error for TemporalSmtProofError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encoding { refusal, .. } => Some(refusal),
            Self::Solver { error, .. } => Some(error),
        }
    }
}

/// Run FR-315's temporal query or query pair, map complete solver evidence to
/// `SmtProofOutcome`, and pass that outcome through FSL's certificate checker.
///
/// CG creates no terminal value itself: non-proof process results remain
/// typed evidence, and every proof outcome is settled by
/// [`fsl_replay::settle_smt_proof`].
pub fn run_and_settle_temporal_smt_proof(
    request: &CertificateRequest<'_>,
    basis: SmtProofBasis,
    executable: &Path,
    arguments: &[OsString],
    process_limits: ProcessLimits,
    certificate_limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalSmtProofResult, TemporalSmtProofError> {
    let kinds = query_kinds(basis);
    let mut queries = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let query_bytes = fsl_replay::encode_smt_query(
            &request.subject,
            request.item,
            kind,
            &request.smt_encoding,
        )
        .map_err(|refusal| TemporalSmtProofError::Encoding { kind, refusal })?;
        let attempt =
            next_attempt().map_err(|error| TemporalSmtProofError::Solver { kind, error })?;
        let process = super::run_bounded_process(ProcessRequest {
            executable: PathBuf::from(executable),
            arguments: arguments.to_vec(),
            stdin: query_bytes.clone(),
            limits: process_limits,
            cancel: cancel.clone(),
        })
        .map_err(|error| TemporalSmtProofError::Solver {
            kind,
            error: SolverRunError::Process(error),
        })?;
        queries.push(TemporalProofQuery::new(kind, attempt, query_bytes, process));
    }

    let Some(outcome) = map_proof_outcome(basis, &queries) else {
        let status = queries
            .iter()
            .find(|query| query.status != SolverStatus::Unsat)
            .map_or(SolverStatus::ProcessFailed, |query| query.status);
        return Ok(TemporalSmtProofResult::NonProof { queries, status });
    };
    let terminal = fsl_replay::settle_smt_proof(request, &outcome, certificate_limits, cancel);
    Ok(TemporalSmtProofResult::Settled(Box::new(
        TemporalSmtProofSettlement {
            outcome,
            terminal,
            queries,
        },
    )))
}

impl TemporalProofQuery {
    fn new(
        kind: SmtQueryKind,
        attempt: NonZeroU64,
        query_bytes: Vec<u8>,
        process: ProcessRun,
    ) -> Self {
        let mut status = match &process.outcome {
            ProcessOutcome::MissingTool => SolverStatus::MissingTool,
            ProcessOutcome::InputOverLimit => SolverStatus::Stopped(ReachedLimit::InputBytes),
            ProcessOutcome::TimedOut => SolverStatus::Stopped(ReachedLimit::WallClock),
            ProcessOutcome::OutputOverLimit { .. } => {
                SolverStatus::Stopped(ReachedLimit::OutputBytes)
            }
            ProcessOutcome::Cancelled { .. } => SolverStatus::Cancelled,
            ProcessOutcome::Completed { success: true, .. } if process.started => process
                .stdout
                .as_deref()
                .map(parse_solver_status)
                .unwrap_or(SolverStatus::MalformedResponse),
            ProcessOutcome::Completed { .. }
            | ProcessOutcome::OutputUnread { .. }
            | ProcessOutcome::InputUnread { .. } => SolverStatus::ProcessFailed,
        };
        let proof_bytes =
            if status == SolverStatus::Unsat && cvc5_proof_dump_selected(&process.arguments) {
                match process.stdout.as_deref().map(captured_alethe_proof) {
                    Some(Ok(Some(proof))) => Some(proof.to_vec()),
                    Some(Err(error)) => {
                        status = error;
                        None
                    }
                    Some(Ok(None)) | None => None,
                }
            } else {
                None
            };
        Self {
            kind,
            attempt,
            query_bytes,
            proof_bytes,
            status,
            process,
        }
    }
}

fn query_kinds(basis: SmtProofBasis) -> Vec<SmtQueryKind> {
    match basis {
        SmtProofBasis::BoundedComplete { depth } => {
            vec![SmtQueryKind::Unrolling { depth }]
        }
        SmtProofBasis::Inductive { depth } => vec![
            SmtQueryKind::InductionBase { depth },
            SmtQueryKind::InductionStep { depth },
        ],
    }
}

fn map_proof_outcome(
    basis: SmtProofBasis,
    queries: &[TemporalProofQuery],
) -> Option<SmtProofOutcome> {
    let expected = query_kinds(basis);
    if queries.len() != expected.len()
        || queries
            .iter()
            .zip(expected.iter())
            .any(|(query, expected)| query.kind != *expected || query.status != SolverStatus::Unsat)
    {
        return None;
    }
    let certificate =
        match (basis, queries) {
            (SmtProofBasis::BoundedComplete { .. }, [unrolling]) => unrolling
                .proof_bytes
                .as_ref()
                .map(|proof| SmtProofCertificate::BoundedComplete {
                    format: ProofFormat::Alethe,
                    unrolling: RefutedQuery {
                        query: unrolling.query_bytes.clone(),
                        proof: proof.clone(),
                    },
                }),
            (SmtProofBasis::Inductive { .. }, [base, step]) => {
                match (base.proof_bytes.as_ref(), step.proof_bytes.as_ref()) {
                    (Some(base_proof), Some(step_proof)) => Some(SmtProofCertificate::Inductive {
                        format: ProofFormat::Alethe,
                        base: RefutedQuery {
                            query: base.query_bytes.clone(),
                            proof: base_proof.clone(),
                        },
                        step: RefutedQuery {
                            query: step.query_bytes.clone(),
                            proof: step_proof.clone(),
                        },
                    }),
                    _ => None,
                }
            }
            _ => return None,
        };
    Some(SmtProofOutcome { basis, certificate })
}

fn cvc5_proof_dump_selected(arguments: &[OsString]) -> bool {
    arguments
        .iter()
        .any(|argument| argument.as_os_str() == OsStr::new("--dump-proofs"))
        && arguments
            .iter()
            .any(|argument| argument.as_os_str() == OsStr::new("--proof-format-mode=alethe"))
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        num::{NonZeroU64, NonZeroUsize},
        path::PathBuf,
        time::Duration,
    };

    use fsl_replay::{ProofFormat, RefutedQuery};
    use ix_trace_rs::trace;

    use super::{map_proof_outcome, TemporalProofQuery};
    use crate::kani::run::process::{ProcessLimits, ProcessOutcome, ProcessRun};
    use crate::routed::smt::SolverStatus;

    fn query(kind: fsl_replay::SmtQueryKind, response: &[u8]) -> TemporalProofQuery {
        TemporalProofQuery::new(
            kind,
            NonZeroU64::MIN,
            format!("(query {})", kind.name()).into_bytes(),
            ProcessRun {
                executable: PathBuf::from("/usr/bin/cvc5"),
                arguments: vec![
                    OsString::from("--dump-proofs"),
                    OsString::from("--proof-format-mode=alethe"),
                ],
                limits: ProcessLimits {
                    input_bytes: NonZeroUsize::new(4096).expect("positive input limit"),
                    wall_clock: Duration::from_secs(1),
                    output_bytes: NonZeroUsize::new(4096).expect("positive output limit"),
                },
                started: true,
                stdout: Some(response.to_vec()),
                stderr: Some(Vec::new()),
                outcome: ProcessOutcome::Completed {
                    success: true,
                    exit_code: Some(0),
                },
            },
        )
    }

    #[trace("FSpec-FR-127-AC-14")]
    #[test]
    fn bounded_complete_outcome_keeps_fsl_query_and_captured_proof() {
        let kind = fsl_replay::SmtQueryKind::Unrolling { depth: 3 };
        let query_bytes = b"(query unrolling)";
        let proof_bytes = b"(step t0 (cl) :rule hole)";
        let run = query(kind, b"unsat\n((step t0 (cl) :rule hole))\n");
        let outcome = map_proof_outcome(
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
            &[run],
        )
        .expect("a completed Unsat proof maps to an FSL proof outcome");
        assert_eq!(
            outcome,
            fsl_replay::SmtProofOutcome {
                basis: fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
                certificate: Some(fsl_replay::SmtProofCertificate::BoundedComplete {
                    format: ProofFormat::Alethe,
                    unrolling: RefutedQuery {
                        query: query_bytes.to_vec(),
                        proof: proof_bytes.to_vec(),
                    },
                }),
            }
        );
    }

    #[trace("FSpec-FR-127-AC-14")]
    #[test]
    fn induction_requires_both_unsat_query_roles_and_preserves_order() {
        let basis = fsl_replay::SmtProofBasis::Inductive { depth: 1 };
        let base = query(
            fsl_replay::SmtQueryKind::InductionBase { depth: 1 },
            b"unsat\n((step base-proof (cl) :rule hole))\n",
        );
        let step = query(
            fsl_replay::SmtQueryKind::InductionStep { depth: 1 },
            b"unsat\n((step step-proof (cl) :rule hole))\n",
        );
        let outcome =
            map_proof_outcome(basis, &[base, step]).expect("the base and step proofs map together");
        let Some(fsl_replay::SmtProofCertificate::Inductive { base, step, .. }) =
            outcome.certificate
        else {
            panic!("induction must retain both proof parts");
        };
        assert!(base.query.ends_with(b"induction-base)"));
        assert!(base.proof.starts_with(b"(step base-proof"));
        assert!(step.query.ends_with(b"induction-step)"));
        assert!(step.proof.starts_with(b"(step step-proof"));

        let base = query(
            fsl_replay::SmtQueryKind::InductionBase { depth: 1 },
            b"unsat\n((step base-proof (cl) :rule hole))\n",
        );
        let step_sat = query(
            fsl_replay::SmtQueryKind::InductionStep { depth: 1 },
            b"sat\n",
        );
        assert!(map_proof_outcome(basis, &[base, step_sat]).is_none());
    }

    #[trace("FSpec-FR-127-AC-14")]
    #[test]
    fn proof_mode_without_certificate_maps_to_uncertified_outcome() {
        let kind = fsl_replay::SmtQueryKind::Unrolling { depth: 3 };
        let run = query(kind, b"unsat\n");
        let outcome = map_proof_outcome(
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
            &[run],
        )
        .expect("a completed Unsat response remains an FSL outcome");
        assert!(outcome.certificate.is_none());

        let malformed = query(kind, b"unsat\n((step t0 (cl) :rule hole))\n(extra)\n");
        assert_eq!(malformed.status(), SolverStatus::MalformedResponse);
        assert!(map_proof_outcome(
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
            &[malformed]
        )
        .is_none());
    }
}
