---
id: AD-002
title: "CG to FSL replay seam: call_site, replay, replay_frame, replay_state_clause and the counterexample envelope"
type: ArchitectureDescription
status: accepted
owner: codegen-maintainers
system: filament-contract-codegen replay adapter (`src/replay/function.rs`, `frame.rs`, `witness.rs`) and the fsl-replay facade it calls
relationships:
  - target: ix://agent-ix/filament-contract-codegen/AD-001
    type: references
  - target: ix://agent-ix/filament-contract-codegen/AD-003
    type: references
  - target: ix://agent-ix/filament-contract-codegen/FR-016
    type: references
  - target: ix://agent-ix/filament-contract-codegen/FR-024
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
---
# CG to FSL replay seam

The Replay subsystem row of the Subsystem Registry lists this AD beside AD-001. It is a view of
one boundary of AD-001's "CG to FSL" seam rows, not a second architecture, and it is one of the
seam descriptions of IR-324. The evidence chain that joins this seam to IR and to the terminal
value is [AD-003](AD-003-evidence-chain.md).

## System Boundary

A Kani falsification becomes evidence only if FSL's native evaluation reproduces it. This AD
states what CG hands FSL for that, what FSL returns, and who is responsible for each part. It
covers `fsl_replay::call_site`, `replay`, `replay_frame`, `replay_state_clause`, the backend-witness
transcript, the counterexample envelope, the replay request and the invocation and snapshot
documents CG builds for a state-clause replay.

Out of scope: how Kani output is read (CG's transcript parser), FSL's evaluation, the terminal
value of a run (AD-003) and the driver.

## Views

The seam is described as what crosses it, how identity is asserted, which way dependencies point
and who reports each failure.

### What crosses the seam

| Item | Direction | Owner of the type | Who builds / reads |
| --- | --- | --- | --- |
| `call_site::<FunctionSite \| OperationSite \| ClauseSite>` result: `package_id`, lowered `quire.checked-package/v2` bytes, parameter, anchor, frame and clause node ids and occurrence keys | FSL to CG | FSL (`CallSiteSelection` is sealed) | FSL computes every node id; CG never derives one |
| Source bytes, dependency libraries (`SuppliedLibrary`, `DependencyInput`), domain package documents | CG to FSL | FSL types; bytes are the proving run's | CG supplies them from the proved lock |
| Backend-witness transcript, admitted by `Witness::parse` | CG to FSL | FSL grammar | CG renders it from decoded values, never from Kani text |
| `ReplayRequestWire` (and `ReplaySource::{Witness, Input}`) | CG to FSL | FSL | CG fills it; `replay` reads it |
| `WitnessEnvelope` / `WitnessPacket` (function and frame counterexamples) | CG to FSL | FSL | CG admits each packet through `WitnessEnvelope::reconstruct` before replay |
| `WitnessEnvelope<StateClauseCounterexample>` (postcondition state clause with `Invocation` observation, FR-024-AC-11, IR-460; object-type invariant with `Current` observation, FR-024-AC-57, IR-1505) and `replay_state_clause` | CG to FSL | FSL | CG builds the payload, with the clause node and occurrence from `call_site`'s `ClauseSite`, and the envelope through `WitnessEnvelope::reconstruct`; FSL replays |
| `quire.state.invocation/v1` and `quire.state.snapshot/v1` documents (pre from the Kani playback, post from the caller's native run of the subject), as raw entries of `ReplayRequestWire.documents` (FR-024-AC-52/53, IR-1133 planned) | CG to FSL | FSL grammar (FR-106) | CG builds them from its own inputs and encodes them only through `core::canonical`; each document declares the two labels (authority and identity) FSL reads as its identity. The separate two-label source `byte_provision` carries no state document. FSL admits their typed values |
| `StateClauseReplayResult` | FSL to CG | FSL | CG reads; the terminal map reads it as FR-029-AC-16 states |
| Structural `ObligationIdentity`, in the envelope and the request's obligation-identity slot | FSL to CG to FSL | FSL type and producer-issued value | CG retains the selected site's value and checks separate harness facts before replay; see AD-003 |
| `ReplayResult`, `FrameReplayResult`, `WitnessSettlement`, `Category` (FSL's `ProofCategory`, folded into one `Category` and re-exported by `fsl-replay`) | FSL to CG | FSL | CG reads; CG defines its own verdict only as a partition of FSL's (`ReplayVerdict`) |
| `DeclaredDomain(ProofBound)` with a `DomainKey`, `Option<DomainKind>` and `FiniteBound` | CG to FSL (in the envelope) | FSL (re-exported by `fsl-replay`) | CG calls `ProofBound::new`; the domain kind must pair with the finite bound. State-clause field ranges use `Some(DomainKind::Integer)` |

`ProofBound::new` requires `Some(DomainKind)` for cardinality, integer-range and depth bounds;
`None` is admitted only for harness-drawn `Variants`. CG's sequence bounds use
`DomainKind::Collection` for cardinality and `DomainKind::Integer` for element ranges.

Nothing else crosses. CG holds no copy of an envelope, witness, replay source, terminal record or
obligation identity type, and neither does IR. CG's only FSL dependency is the `fsl-replay` crate
(the one FSL entry in `Cargo.toml`).

### Versioned contracts and how identity is asserted

| Contract | Spelled by | Asserted by |
| --- | --- | --- |
| `fsl-replay` Rust API | FSL | The compiler. CG's `Cargo.toml` names FSL's `main` branch and `Cargo.lock` selects the revision it builds against; the seam asserts nothing about that revision beyond compilation, and no digest or version check is needed: a changed type or removed function fails CG's build. |
| Request version and vocabulary members | none | FSL deleted `contract_version`, `capability_vocabulary` and `package_contract_version` from `ReplayRequestWire` and `WitnessPacket` (R-Q5, done in FSL), so the request carries no contract spelling and CG writes none. |
| `package_id` (`quire.package.semantic/v2`) | FSL | FSL recompiles the provided source and requires the recompiled `package_id` to equal the request's (`execute.rs`, "Rule 5"; `execute/frame.rs` for the frame path). This is the one canonical content-identity digest the whole replay seam rests on: it binds a replay to the content that was proved. |
| Source bytes | CG computes the digest with FSL's `ByteDigest` | FSL checks each provided byte against its digest (`stale_dependency/byte-digest-mismatch`). |
| Request `obligation_identity` slot (formerly `originating_counterexample_identity`) | CG carries the structural `ObligationIdentity` issued by FSL's selected `FunctionSite` or `OperationSite`. `function_contract_identity` and `frame_identity` in `src/replay/obligation.rs` retain that value; kind, occurrence and argument domains are checked as separate claim facts. The function packet is admitted through `WitnessEnvelope::reconstruct` before `ReplayPackage::request_from_envelope` fills the request. | FSL admits the envelope and uses the carried identity for witness/replay binding; CG does not derive a digest from the harness record. |
| Backend identity | request `backend` member | carried; FSL records it, does not interpret it |

The obligation identity is a producer-issued structural claim identity, distinct from content
digests over package and source bytes. CG carries it through replay and does not mint a second
obligation digest. No tool or version pin is asserted on this seam.

### Dependency direction

- IR never depends on FSL and defines no replay type (IR FR-037, IR AD-001). IR has no `replay`
  or `witness` module and no FSL dependency.
- CG depends on `fsl-replay` and on IR; `fsl-replay` depends on the model crate through
  `fsl-package`. No cycle.
- CG calls `fsl_replay::replay`, `replay_frame` and `replay_state_clause` only. The function path
  (`replay_counterexample_through`) and the state-clause path (`StateClauseReplay::replay_through`,
  IR-460) take a caller-supplied executor, which ends in the matching FSL function so a
  caller can observe the request FSL receives; the frame path takes none.
- The replay verdict is FSL's evaluation. CG supplies no value that decides it.

### Failure outcomes and who reports them

| Condition | Reporter | Outcome |
| --- | --- | --- |
| A harness argument has no bound parameter, delimiter in harness or check text, transcript not admitted | CG (`SpineReplayError`) | typed refusal, no replay |
| Dependency selections not admitted, call site not located (`CallSiteRefusal`, a fault among them) | CG wrapping FSL (`ReplayPackageError`, `FrameReplayError`) | typed refusal, no replay; a wrapped fault is still a fault (AD-003, link 7) |
| A qualified name built from the operation's identifiers is not admitted (`FrameReplayError::Name`) | CG | typed refusal, no replay |
| The frame witness transcript or the envelope CG built is not admitted (`FrameReplayError::Transcript`, `FrameReplayError::Envelope`, from `WitnessEnvelope::reconstruct`) | CG | typed refusal, no replay; CG built a value its own contract says FSL admits, so AD-003 maps it to a failure |
| A state-clause integer range cannot be paired with its domain kind by `ProofBound::new` (`StateClauseReplayError::ProofBound(ProofBoundRefusal)`) | CG wrapping FSL's typed constructor refusal | typed error before replay; it carries no FSL replay code and maps to `Failed` as a CG defect under FR-029-AC-16 |
| `replay` / `replay_frame` refuses the request or envelope | FSL (`ReplayRefusal`) | CG carries it unchanged in `Refused`; it is not a verdict on the evidence |
| State-clause replay (IR-460): a playback or post state binds no value for a declared state field (`StateClauseReplayError::MissingField`), binds a name that is no state field or one field twice (`UndeclaredField`, `DuplicateField`), a playback value lies outside its field's declared range (`OutOfDomain`), the operation declares a parameter or a result (`UnsupportedOperationShape`), a document is not encodable or a fact it is built from is not readable from the supplied package (`Document`), a name, transcript or envelope is not admitted (`Name`, `Transcript`, `Envelope`) | CG | typed error before any replay and no executor call; each carries no FSL code, so FR-029-AC-16 maps each to `Failed`, never `Incomplete` and never `Inconclusive(ReplayRefused)` |
| State-clause replay (IR-460): dependency selections not admitted or the clause not located (`StateClauseReplayError::Dependencies`, `CallSite`), or `replay_state_clause` refuses (`Refused`) | CG wrapping FSL | typed error, no result; read as the function and frame paths read the same refusals (FR-029-AC-13, FR-029-AC-16) |
| Decoded value outside its declared domain (function path) | CG (`EvidenceFailureCause::Domain`) before any request is built | evidence failure; AD-003 link 7 maps it to a failure value, since it is CG's own defect and not a `ReplayRefusal` |
| Replay ran and did not settle `ReproducedWithEvaluatedWitness` in category `violation` | FSL settles; CG partitions (`verdict_of`) | evidence failure with FSL's settlement and category |
| A witness-sourced request settles on the input arm | CG (`WrongArm`) | typed refusal |

A counterexample that is not reproduced is never a clause success or a clause failure: it is
evidence failure whatever the cause. An input-arm settlement (`reproduced-without-witness`) is
never backend evidence.

## Decisions

CG builds what FSL reads and FSL decides the verdict; each statement below is one a test can
check.

### Invariants a test can check

Candidate statements (local labels; the repository assigns requirement ids when one is
authored).

- R-1. CG imports no FSL item except from `fsl-replay`, and no CG or IR source defines a
  `Witness`, `ReplaySource`, `WitnessEnvelope`, `TerminalValue` or `ObligationIdentity`.
  Test: cargo metadata on CG's normal dependencies, and a grep gate on type definitions.
- R-2. Every parameter, anchor, frame and clause node id CG puts in a request or envelope is the
  value `call_site` returned. Test: a mutation that rewrites an id turns the replay red.
- R-3. A transcript is built by one function and admitted only through `Witness::parse`.
- R-4. A value outside its declared domain yields evidence failure before `replay` is called
  (test: a stand-in that panics if called).
- R-5. Only a `ReproducedWithEvaluatedWitness` settlement in category `violation` reproduces;
  every other settlement, including `Inconclusive` in any category, is evidence failure.
- R-6. Every function and frame counterexample envelope and replay request carries the
  producer-issued structural `ObligationIdentity` from FSL's selected site; CG checks the
  harness kind and argument-domain facts separately (AD-003 E-1, FR-024-AC-48 through AC-51).
- R-7. A request names the `package_id` returned by `call_site` for the same source, and the
  replay is refused (not reproduced) when one source byte changes.
- R-8. A request carries no contract spelling CG wrote itself. Met: FSL deleted the version and
  vocabulary members (R-Q5), so there is nothing to spell.

## Risks

The current seam is measured against CG's checked-in source and the `fsl-replay` crate selected
by its lock. The routed rows below retain historical decisions where they explain an owner seam.

### Current state and gaps

- Function path: `src/replay/function.rs` selects FSL's `FunctionSite`, checks the retained
  proving-run identity and parameter bindings, and admits the backend witness packet through
  `WitnessEnvelope::reconstruct` in `request_from_packet`. It checks the reconstructed target,
  occurrence, subject, limits, domains, source arm and obligation identity before
  `request_from_envelope` builds the replay request. The envelope and request carry the
  structural `ObligationIdentity` FSL issued for the selected site; CG does not hash a preimage
  from the function id, kind or argument domains. CG still decodes and domain-checks playback
  before handing the request to FSL for replay.
- Frame path: `src/replay/frame.rs` selects FSL's `OperationSite`, reconstructs its
  `WitnessEnvelope`, and calls `replay_frame`. `frame_identity` carries the selected site's
  producer-issued identity into the envelope and request; it does not mint a digest from the
  harness. Playback is decoded and its pre-state tied to the invocation (FR-024-AC-25 to
  AC-28). The frame inputs carry no separate counterexample identity.
- No domain check precedes the frame path, and the frame envelope carries
  `declared_domains: Some(Vec::new())` (`frame.rs`): the proof bound is not in the envelope.
  At the FSL revision CG's lock selects, replay checks only that the member is present. The frame
  harness's only symbolic quantities are fields of its one state struct, so the domains that will
  matter are field domains. They wait on QSL-345's key shape and on its refusal of an empty
  declaration, and CG builds no key before then (FR-024-AC-29); the state-clause path builds
  its own domains in the shape that revision offers, and the frame path does not copy it.
- FSL removed the three version and vocabulary members from the request (R-Q5), and CG dropped
  its copies of their spellings from the function path's request. The `filament.capability-kind/v1`
  constant at `capability.rs:22` belongs to the capability envelope (FR-019), not to the request.
- The former function-path envelope gap is resolved: `request_from_packet` reconstructs and
  validates the function envelope before constructing a request. The producer-issued O-09
  identity replaces the old transcript digest in the request and envelope. The old
  digest-preimage and missing-envelope descriptions were historical migration findings, not
  open current-state gaps.
- `first_out_of_domain` is crate-private; `decode_falsification` is public
  (`src/replay/witness.rs:116`) and decodes to `fsl_replay::WitnessValue`.
- FSL's `call_site` accepts a `ClauseSite` selection and FSL has a state-clause replay entry
  (`replay_state_clause`); CG's clause-replay entry is `StateClauseReplay` in
  `src/replay/state_clause.rs` (FR-024-AC-11 to FR-024-AC-19, IR-460). The IR-412 postcondition
  harness is its consumer.
- AD-001's Replay view and Current state describe the same function-path facts. AD-001 is not
  edited here.

### Open questions

| Question | Owner | Recommendation | Cost of the alternative |
| --- | --- | --- | --- |
| The domain check before replay exists on the function path only | CG with FSL | CG keeps its own pre-check: a playback outside the harness's proof bound is a CG harness defect, and a FSL-side check of admitted values against `DeclaredDomain`, if FSL adds one, would report it as an invalid input and hide the defect. Do not drop the CG check on FSL's account. | Without CG's check a FSL refusal would hide a CG defect. |

No compatibility layer is proposed or needed. If FSL changes the facade, CG changes its calls.

### Routed gaps

Needs stated to owners, not decisions. Ids are the routing ids of IR-324; they are not
requirement ids.

To FSL (FSL reviews these rows):

| Id | Stated need |
| --- | --- |
| R-Q5 | Done in FSL: the three version and vocabulary members are gone from the request, and CG dropped its copies. |
| R-Q7 | Resolved: the request's `obligation_identity` slot and the function envelope carry the FSL producer-issued structural O-09 value. `request_from_packet` reconstructs the function envelope; no transcript digest or CG obligation preimage supplies that slot. The former missing-envelope and digest claims are historical. |
| R-Q8 | CG's frame envelope sends an empty declared-domain list, which QSL-345 part 2 will refuse; CG must supply the bounds. CG keeps its own pre-check regardless. |
