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
        collections::BTreeMap,
        ffi::OsString,
        num::{NonZeroU64, NonZeroUsize},
        path::PathBuf,
        time::Duration,
    };

    use filament_exact::ScalarLimits;
    use fsl_eval::{
        model_subject::ModelSubject,
        simulation::model::{
            ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem,
        },
    };
    use fsl_replay::spine::{self, DependencyInput, ParseRequest};
    use fsl_replay::{
        CertificateCheckLimits, CertificateRequest, DomainKey, DomainKind, FiniteBound,
        ModelCheckItem, ProofBound, ProofFormat, RefutedQuery, SmtEncodingLimits, SpineLimits,
        TerminalValue,
    };
    use ix_trace_rs::trace;

    use super::{
        map_proof_outcome, run_and_settle_temporal_smt_proof, TemporalProofQuery,
        TemporalSmtProofResult,
    };
    use crate::kani::run::process::{ProcessLimits, ProcessOutcome, ProcessRun};
    use crate::routed::smt::SolverStatus;

    struct NoAtomLetters;

    impl fsl_eval::model_subject::ModelLetters for NoAtomLetters {
        fn atom(
            &self,
            _atom: fsl_semantics::check::FormulaNodeId,
            _instance: &fsl_eval::model_subject::ModelInstance,
            _state: &fsl_eval::simulation::model::ModelSimState,
            _meter: &mut filament_exact::Meter,
        ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
            panic!("the fixture clause has no holds atoms")
        }

        fn activation(
            &self,
            _instance: &fsl_eval::model_subject::ModelInstance,
            _state: &fsl_eval::simulation::model::ModelSimState,
            _meter: &mut filament_exact::Meter,
        ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
            panic!("the fixture clause has no guarded activation")
        }

        fn activation_reads(
            &self,
        ) -> Result<
            std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
            fsl_semantics::check::footprint::FootprintRefusal,
        > {
            Ok(std::collections::BTreeSet::new())
        }

        fn reads(
            &self,
            _atom: fsl_semantics::check::FormulaNodeId,
        ) -> Result<
            std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
            fsl_semantics::check::footprint::FootprintRefusal,
        > {
            Ok(std::collections::BTreeSet::new())
        }
    }

    static NO_ATOM_LETTERS: NoAtomLetters = NoAtomLetters;

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

    #[trace("TC-041", "FR-030-AC-19", "FSpec-FR-127-AC-14")]
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

    #[trace("TC-041", "FR-030-AC-19", "FSpec-FR-127-AC-14")]
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

    #[trace("TC-041", "FR-030-AC-19", "FSpec-FR-127-AC-14")]
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

    struct TemporalRequestFixture {
        checked: spine::CheckedUnit,
        packages: BTreeMap<String, Vec<u8>>,
        snapshots: BTreeMap<fsl_replay::DocumentRef, Vec<u8>>,
        initial: Vec<fsl_replay::DocumentRef>,
    }

    impl TemporalRequestFixture {
        fn new() -> Self {
            let package_name = "test/temporal-smt";
            let spec_identity = format!("ix://{package_name}/spec");
            let gadget_identity = format!("ix://{package_name}/Gadget");
            let population_identity = format!("ix://{package_name}/gadgets");
            let model = serde_json::json!({
                "contractVersion": "2.0.0",
                "canonicalization": {"algorithm": "rfc8785-v1"},
                "source": {
                    "identity": spec_identity,
                    "version": "1.0.0",
                    "dialect": "spec-bundle"
                },
                "package": {
                    "identity": package_name,
                    "version": "1.0.0",
                    "mappingVersions": [],
                    "profileVersions": []
                },
                "occurrences": [],
                "extensions": [],
                "constructs": [
                    {
                        "kind": {"module": package_name, "name": "entity"},
                        "moduleVersion": "1.0.0",
                        "construct": {
                            "identity": "none",
                            "shape": "record",
                            "members": {},
                            "meaning": "filament.meaning.model.object-type/v1"
                        }
                    },
                    {
                        "kind": {"module": package_name, "name": "population"},
                        "moduleVersion": "1.0.0",
                        "construct": {
                            "identity": "none",
                            "shape": "record",
                            "members": {},
                            "meaning": "filament.meaning.model.population/v1"
                        }
                    }
                ],
                "types": [{
                    "identity": gadget_identity,
                    "displayName": "Gadget",
                    "kind": {"module": package_name, "name": "entity"},
                    "roles": [],
                    "origin": {"generated": {
                        "generatorIdentity": spec_identity,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [gadget_identity]
                    }},
                    "constraints": [],
                    "extensions": [],
                    "unknownPolicy": "reject",
                    "supertypes": [],
                    "fields": [],
                    "operations": [],
                    "relationships": []
                }],
                "populations": [{
                    "identity": population_identity,
                    "displayName": "gadgets",
                    "kind": {"module": package_name, "name": "population"},
                    "members": [gadget_identity],
                    "extent": "closed",
                    "origin": {"generated": {
                        "generatorIdentity": spec_identity,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [population_identity]
                    }}
                }]
            })
            .to_string()
            .into_bytes();
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n\
                 profile v = \"filament.value.complete/v1\";\n\
                 profile ep = \"filament.temporal.event-position.false-extension/v1\";\n\
                 model M = \"{package_name}\";\n\
                 temporal Bounded using ep over (g: M::Gadget) clock \"model-steps\" on origin {{\n\
                   always[0,3] holds(true)\n\
                 }}\n"
            );
            let source_identity = fsl_replay::SourceIdentity::new("test", "temporal-smt");
            let cancel = fsl_replay::Cancel::new();
            let limits = SpineLimits::default();
            let parsed = spine::parse(
                &ParseRequest {
                    source: &source_identity,
                    path: "temporal-smt.native",
                    bytes: source.as_bytes(),
                },
                limits.source,
                &cancel,
            )
            .expect("the FSL fixture source parses")
            .into_value();
            let packages = BTreeMap::from([(package_name.to_owned(), model)]);
            let admitted = spine::select(&parsed, &packages, limits.model, &cancel)
                .expect("the FSL model package admits")
                .into_value();
            let checked = spine::check(
                &parsed,
                "temporal-smt-fixture".to_owned(),
                &admitted,
                &DependencyInput::default(),
                limits,
                &cancel,
            )
            .expect("the FSL temporal item checks")
            .into_value();
            let initial_ref = fsl_replay::DocumentRef {
                authority: "test".to_owned(),
                identity: "s0".to_owned(),
            };
            let snapshot = serde_json::json!({
                "format": "filament.state.snapshot/v1",
                "identity": {"authority": "test", "identity": "s0"},
                "observation": "current",
                "model": {"identity": package_name},
                "anchor": {"kind": "initialization", "name": "start"},
                "populations": [{
                    "population": population_identity,
                    "complete": true,
                    "objects": [{
                        "key": "g",
                        "type": gadget_identity,
                        "fields": {}
                    }]
                }]
            })
            .to_string()
            .into_bytes();
            Self {
                checked,
                packages,
                snapshots: BTreeMap::from([(initial_ref.clone(), snapshot)]),
                initial: vec![initial_ref],
            }
        }

        fn subject(&self) -> ModelSubject<'_> {
            ModelSubject {
                simulation: ModelSimulationRequest {
                    input: ModelCheckInput::source(self.checked.package()),
                    packages: &self.packages,
                    normalization: Default::default(),
                    snapshots: &self.snapshots,
                    initial: self.initial.clone(),
                    universes: vec![fsl_eval::simulation::model::PopulationUniverse {
                        population: fsl_replay::DeclarationKey {
                            package: "test/temporal-smt".to_owned(),
                            node: "ix://test/temporal-smt/gadgets".to_owned(),
                        },
                        keys: vec!["g".to_owned()],
                    }],
                    observation: Default::default(),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000,
                    },
                    meter: ScalarLimits {
                        integer_bits: 1 << 20,
                        decimal_digits: 1 << 20,
                        scale_expansion: 1 << 20,
                        text_input_bytes: 1 << 20,
                        text_scalars: 1 << 20,
                        normalized_scalars: 1 << 20,
                        unit_edges: 1 << 20,
                        value_occurrences: 1 << 20,
                        work_units: 1 << 20,
                        result_units: 1 << 20,
                    },
                    emit: fsl_package::EmitLimits::new(1 << 20),
                },
                proof_bounds: vec![ProofBound::new(
                    DomainKey::Node {
                        node: fsl_replay::NodeKey::new(
                            "test/temporal-smt",
                            vec![
                                fsl_replay::PathSegment::Text("relation".to_owned()),
                                fsl_replay::PathSegment::Text("population".to_owned()),
                                fsl_replay::PathSegment::Text(
                                    "ix://test/temporal-smt/gadgets".to_owned(),
                                ),
                            ],
                            0,
                        )
                        .expect("the population identity is a valid node key"),
                        path: Vec::new(),
                    },
                    Some(DomainKind::Population),
                    FiniteBound::Cardinality { maximum: 1 },
                )
                .expect("the proof subject has a finite population bound")],
            }
        }

        fn request_with_item<'a>(
            &'a self,
            subject: ModelSubject<'a>,
            item: ModelCheckItem<'a>,
        ) -> CertificateRequest<'a> {
            CertificateRequest {
                subject,
                temporal_clock_selections: &[],
                item,
                smt_encoding: SmtEncodingLimits::default(),
            }
        }

        fn request(&self) -> CertificateRequest<'_> {
            let [clause] = self.checked.package().graph().temporal_clauses() else {
                panic!("the fixture contains one temporal clause")
            };
            self.request_with_item(
                self.subject(),
                ModelCheckItem::Clause {
                    clause: clause.clause(),
                    letters: &NO_ATOM_LETTERS,
                },
            )
        }
    }

    fn fake_solver_script(stdout: &'static str) -> Vec<OsString> {
        vec![
            OsString::from("-c"),
            OsString::from(format!("cat >/dev/null; printf '%b' '{stdout}'")),
            OsString::from("fake-cvc5"),
            OsString::from("--dump-proofs"),
            OsString::from("--proof-format-mode=alethe"),
        ]
    }

    fn process_limits() -> ProcessLimits {
        ProcessLimits {
            input_bytes: NonZeroUsize::new(1 << 20).expect("positive input limit"),
            wall_clock: Duration::from_secs(2),
            output_bytes: NonZeroUsize::new(1 << 20).expect("positive output limit"),
        }
    }

    #[trace("TC-041", "FR-030-AC-19", "FSpec-FR-127-AC-14")]
    #[test]
    fn public_temporal_route_encodes_the_checked_clause_and_settles_uncertified_proof() {
        let fixture = TemporalRequestFixture::new();
        let request = fixture.request();
        let expected_query = fsl_replay::encode_smt_query(
            &request.subject,
            request.item,
            fsl_replay::SmtQueryKind::Unrolling { depth: 3 },
            &request.smt_encoding,
        )
        .expect("the FSL clause has a canonical temporal query");
        let result = run_and_settle_temporal_smt_proof(
            &request,
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
            PathBuf::from("/bin/sh").as_path(),
            &fake_solver_script("unsat\\n"),
            process_limits(),
            &CertificateCheckLimits::default(),
            &fsl_replay::Cancel::new(),
        )
        .expect("the bounded solver invocation returns typed evidence");
        let TemporalSmtProofResult::Settled(settlement) = result else {
            panic!("an Unsat bounded run settles: {result:?}");
        };
        assert_eq!(settlement.queries.len(), 1);
        assert_eq!(
            settlement.queries[0].kind(),
            fsl_replay::SmtQueryKind::Unrolling { depth: 3 }
        );
        assert!(settlement
            .queries
            .iter()
            .all(|query| { query.status() == SolverStatus::Unsat && query.process().started }));
        assert_eq!(settlement.queries[0].query_bytes(), expected_query);
        assert!(settlement.outcome.certificate.is_none());
        assert_eq!(
            settlement.terminal,
            TerminalValue::Proved {
                basis: fsl_replay::ProofBasis::BoundedComplete { depth: 3 },
                certification: fsl_replay::Certification::Uncertified,
            }
        );
    }

    #[trace("TC-041", "FR-030-AC-19", "FSpec-FR-127-AC-14")]
    #[test]
    fn public_deadlock_induction_route_settles_without_a_certificate_as_uncertified() {
        let fixture = TemporalRequestFixture::new();
        let request = fixture.request_with_item(fixture.subject(), ModelCheckItem::DeadlockFreedom);
        let expected_query = fsl_replay::encode_smt_query(
            &request.subject,
            request.item,
            fsl_replay::SmtQueryKind::InductionBase { depth: 1 },
            &request.smt_encoding,
        )
        .expect("the bounded FSL clause has a canonical query");
        let result = run_and_settle_temporal_smt_proof(
            &request,
            fsl_replay::SmtProofBasis::Inductive { depth: 1 },
            PathBuf::from("/bin/sh").as_path(),
            &fake_solver_script("unsat\\n"),
            process_limits(),
            &CertificateCheckLimits::default(),
            &fsl_replay::Cancel::new(),
        )
        .expect("the bounded solver invocation returns typed evidence");
        let TemporalSmtProofResult::Settled(settlement) = result else {
            panic!("an Unsat bounded run settles: {result:?}");
        };
        assert_eq!(settlement.queries.len(), 2);
        assert_eq!(settlement.queries[0].query_bytes(), expected_query);
        assert!(settlement.queries[0].process().started);
        assert!(settlement.outcome.certificate.is_none());
        assert_eq!(
            settlement.terminal,
            TerminalValue::Proved {
                basis: fsl_replay::ProofBasis::Inductive { depth: 1 },
                certification: fsl_replay::Certification::Uncertified,
            }
        );
    }

    #[trace("TC-041", "FR-030-AC-20", "FSpec-FR-127-AC-14")]
    #[test]
    fn public_temporal_route_preserves_fsl_certificate_rejection() {
        let fixture = TemporalRequestFixture::new();
        let subject = fixture.subject();
        let system = ModelSystem::new(subject.simulation.clone()).expect("the model admits");
        let [clause] = fixture.checked.package().graph().temporal_clauses() else {
            panic!("the fixture contains one temporal clause")
        };
        let letters = system
            .clause_letters(clause)
            .expect("the checked clause has model letters");
        let request = fixture.request_with_item(
            subject,
            ModelCheckItem::Clause {
                clause: clause.clause(),
                letters: &letters,
            },
        );
        let result = run_and_settle_temporal_smt_proof(
            &request,
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 },
            PathBuf::from("/bin/sh").as_path(),
            &fake_solver_script("unsat\\n((step t0 (cl (= x x)) :rule poly_simp))\\n"),
            process_limits(),
            &CertificateCheckLimits::default(),
            &fsl_replay::Cancel::new(),
        )
        .expect("the bounded solver invocation returns typed evidence");
        let TemporalSmtProofResult::Settled(settlement) = result else {
            panic!("an Unsat run with a captured proof reaches FSL: {result:?}");
        };
        assert_eq!(settlement.queries.len(), 1);
        assert!(settlement
            .queries
            .iter()
            .all(|query| query.proof_bytes().is_some()));
        assert!(matches!(
            settlement.terminal,
            TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::CertificateRejected {
                rule: fsl_replay::CertificateRule::NotRefutation,
                ..
            })
        ));
        assert_eq!(
            settlement.outcome.basis,
            fsl_replay::SmtProofBasis::BoundedComplete { depth: 3 }
        );
    }
}
