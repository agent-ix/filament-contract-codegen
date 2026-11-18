---
id: FR-033
title: "Bind composite equality parity to the original node and FSL settlement"
type: FR
relationships:
  - target: ix://agent-ix/filament-contract-codegen/StR-001
    type: satisfies
  - target: ix://agent-ix/filament-contract-codegen/FR-015
    type: depends_on
  - target: ix://agent-ix/filament-contract-codegen/FR-025
    type: depends_on
  - target: ix://agent-ix/filament-contract-codegen/FR-028
    type: depends_on
  - target: ix://agent-ix/filament-contract-codegen/FR-029
    type: depends_on
  - target: ix://agent-ix/filament-contract-codegen/AD-002
    type: references
  - target: ix://agent-ix/filament-contract-codegen/AD-003
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-070
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-357
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-358
    type: references
  - target: ix://agent-ix/filament-specification/FR-181
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-021
    type: references
---
# FR-033: Bind composite equality parity to the original node and FSL settlement

## Description

Imported contexts remain PLANNED: the Eq constructor returns typed `ImportedContextUnsupported`
until CG retains admitted dependency packages from the original compile.

When the driver submits a composite equality `bounded_shadow` item for settlement, CG shall
construct a FSL-owned node-selected equality-parity claim over the original proving package. The
proposition remains the independent equality verdict and admitted occurrence-pair count owned by
[FR-015](../../kani/functional/FR-015-bounded-kani-obligations.md) AC-71 and the native refinement
comparison owned by [FR-028](../../kani/functional/FR-028-bounded-proof-ceilings.md) AC-15. Equality
and inequality preserve the claimed operation; an inequality verdict is the negation of equality
with the same pair count. This is parity evidence about generated code, never a source-predicate
violation.

IR-666 delivers the public CG report converters and direct FSL facade tests of F-1 to F-7 and
V-1 to V-5. IR-635's verified constructor accepts Equal for admitted graph-child
`structural.eq` and NotEqual for admitted graph-child `structural.ne`. It adds the public
`OriginalCompositeEqContext` and immutable request/report types with retained source-lock checks,
original node/function/occurrence membership, exact recompiled package/context equality,
positional checked-claim retention and genuine public verified-shadow invocation through the converter.
The caller's retained original artifact, execution context and limits are pre-invocation binding inputs;
this slice does not independently authenticate a generated artifact or produce backend/refinement
evidence. The locked IR revision admits `structural.ne`. Falsified same-artifact
native execution, generated-family and proof-strength controls remain planned; playback decode and
operand rebuild through the leaf bindings are delivered for the families the composite shadow draws
(AC-2 to AC-5, PARTIAL). FSL's public composite witness, parity and verified-shadow interfaces are delivered.
The literal constructor controls use admitted one-field `Integer` records. A FSL-emitted
`Int[0, 9]` record literal inserts a `filament.op.numeric.narrow` conversion child (IR-691), and
`sequence[]` inserts an expression-constructor descendant. IR-651 refuses expression-bearing
literal graphs as `NonliteralGraphValue`; bounded-field and collection-constructor literals remain
gated and are not counted as positive coverage.
The builder shall use the owning `replay_composite_parity(wire, claim, falsified, replay_limits)`
and `settle_verified_shadow(wire, claim, verified, replay_limits)` interfaces, with public
`ReplayLimits` last. The claim retains original exact-evaluation `ScalarLimits`; transport
`ReplayLimits` remain a separate entry argument, not an invented `CompositeIdentity` member.
FSpec FR-322 owns the original checked-package artifact and node/occurrence membership. FSL FR-070
owns the canonical composite `WitnessValue` forms, including exact nested integer leaves; a
successful canonical decode does not establish source-type admission or lawful generated coverage.
Union text decode remains distinct from unsupported checker union admission. The completed CG
route requires actual original-artifact authentication, typed dependency integration and real
generated-family verification/falsification controls before CODE delivery.

## Inputs

- The actual composite claim node and operation, original checked/emitted package and original
  source/dependency byte provision needed for FSL recompilation.
- The generated harness identity, symbolic leaf bindings, operand declarations and literal values
  from the same proving context; harness bounds and retained original execution limits.
- The falsified run's selected assertion playback, or the verified run's SUCCESS count and
  independent native-refinement evidence. The retained shadow comparison outcome includes its
  equality/inequality result and occurrence-pair count.
- For falsified parity replay, the driver's actual typed native observation of the same proved
  generated artifact over the reconstructed operands and original limits, with its pair count.
  Verified settlement takes no native-observation field; it retains actual refinement evidence,
  including explicit absence for `not_run`, without an extra native probe or invented observation. A
  refusal, incomplete outcome or execution fault remains that observation, rather than a fabricated
  Boolean.
- The IR-reader structural `NodeKey`/`CheckedNodeId` for the claimed application, paired with
  the retained checked claim body: `arguments`, the application `node`, `obligation_kind` and
  authentic `occurrence_key` from the recompiled package.
  There is one argument per operand position in operand order, including repeated operands and
  literal operands. Parameter arguments use the actual `graph_child` node at that position and
  only its drawn `DomainKey::Node` harness bounds naming that parameter; repeated positions remain
  distinct argument entries, not a distinct node set. Population-keyed metadata remains in the
  full CG proving record, outside the admitted FSL composite claim and its positional arguments. A FSL
  request carrying a `DomainKey::Population` harness bound receives typed `HarnessUnknownKey`
  refusal during `prepare`, before structural identity and body comparison; its binding-checked `Refused` report maps
  to `Inconclusive(ReplayRefused)`. A CG precheck refusal still returns no report or terminal value. Composite literals use their own actual `graph_child` node with empty `bounds` because
  their value is bound by that content-addressed node. The owning operand projection refuses
  every top-level inline literal operand, and CG returns that refusal without a report.
  Within each Bounds argument, keys sort by canonical encoded key bytes, not `DomainKey` Ord;
  duplicate keys within that argument refuse. Repetition across positional arguments is preserved.
- The full CG harness identity/evidence record owned by FR-015 AC-76, retaining its abstractions,
  size budget, static closure pair-node count and unexercised behaviours outside the checked parity claim. This record
  count is distinct from the runtime occurrence-pair count compared in F-7.
- The retained original generated artifact and execution context tying the native observation to
  the actual proved artifact and original limits. CG authenticates that relationship independently
  before invoking FSL; structural node identity and a matching report do not establish it.
  AD-002 R-6/R-7 and [AD-003](../../assurance/AD-003-evidence-chain.md) distinguish these roles.
- Configured public replay encoded-input-byte, value-occurrence and work limits admitted by the
  upstream contract. Capture limits for launcher output remain separate inputs and evidence.

## Outputs

- A typed FSL-owned parity request for this node and its operand values, or a typed claim/setup
  refusal before settlement. For an admitted same claim, retained Disagreed produces Failed/CgDefect
  before operand admission or missing/fault native-observation handling, under FR-029 AC-24. A
  required unavailable capability yields no invented settlement or observation.
- A binding-checked FSL settlement/record for
  [FR-029](../../kani/functional/FR-029-run-outcome-terminal-record.md). CG setup failure before
  invoking FSL, missing report or wrong-claim report returns a typed refusal without a terminal
  value. A FSL common-step `Refused` result is a report with a terminal value after its full claim
  binding passes. No fabricated FSL catalog code crosses an unavailable seam.

## Behavior

- The builder shall submit the claimed node inside the original recompiled FSpec FR-322 package to
  owning public FSL parity selector and preserve the original retained source/dependency bytes,
  package/context identity and claimed node/occurrence membership checks of AD-002 R-7.
- The builder shall take the application `NodeKey`/`CheckedNodeId` from the admitted IR reader
  and retain the checked claim body with positional arguments and their `Range` or `Bounds`.
- When the identity or required body metadata is unavailable, the builder shall return a typed
  setup refusal rather than fabricate an identity.
- The checked claim body shall gain no top-level operator, native observation,
  refinement or counter member; operation validation remains in node membership and the full claim.
- For each parameter argument, the builder shall include only drawn `DomainKey::Node` bounds naming
  that parameter, including its actual collection/depth bounds under declared-domain substitution.
- The builder shall retain Population-keyed metadata only in the full CG proving record, outside
  the admitted FSL composite claim and its positional arguments.
- When a FSL request carries a `DomainKey::Population` harness bound, the converter shall retain
  the binding-checked `HarnessUnknownKey` refusal report and its `Inconclusive(ReplayRefused)`
  terminal value. A CG pre-invocation refusal produces no report or terminal value.
- The builder shall sort keys within each Bounds argument by canonical encoded key bytes.
- When a Bounds argument contains duplicate keys, the builder shall refuse that argument.
- The builder shall preserve repeated positional arguments as separate entries.

Encoded-key ordering example: path `[0,0]` precedes `[0]`, and `[10]` precedes `[2]`.

- The builder shall retain the composite equality operation, each operand's actual declared domain
  and each literal operand's singleton domain, without substituting enclosing function parameters,
  changing source bytes or manufacturing a Boolean predicate/function.
- The builder shall reconstruct operands from the persisted original-node leaf bindings owned by
  [FR-025](../../kani/functional/FR-025-generated-subject-abi.md), preserving declaration
  identities, member identities, slots, presence, arity, order and multiplicity.
- The builder shall render FSL FR-070 witness values with the authoritative FSpec FR-181 canonical
  JCS encoding through its owning dependency, without copying schemas or other repositories'
  artifacts. Its transcript escaping replaces percent, semicolon, less-than and greater-than with
  `%25`, `%3B`, `%3C` and `%3E` respectively, using the upstream grammar.
- The canonical route shall preserve Boolean, arbitrary exact integer, exact IEEE float bits,
  rational, decimal, text, enum, quantity underlying exact scalar and reference values; it shall
  preserve option, record, tuple, union, sequence, set, bag and ordered-set structure. Population is
  not a value and no map family is added by this requirement.
- When the operand stage is reached, the builder shall validate canonical spelling, typed declared
  shape and admission through the owning FSL contract after same-valid-claim Disagreed precedence,
  including declared/unit/reference identities, enum/union members, required and optional slot
  presence, tuple arity, sequence order, set distinctness, bag multiplicity and
  ordered-set order/distinctness. Reference values require no invented object environment or
  dereferenced value; absent required environment retains FSL's typed no-value or refusal reading.
  Quantity conversion preserves its exact scalar and unit without rounding.
- When arbitrary public caller input arrives, the builder shall guard its encoded byte length
  against the configured replay-input limit, counting actual encoded bytes rather than a
  decoded-size estimate. It shall enforce checked occurrence/work accounting during traversal.
- The canonical value lifecycle shall use explicit heap stacks for decode, clone, equality, redacted
  Debug and drop, so admitted nesting consumes no proportional native stack. It shall refuse
  byte/count/work exhaustion with its typed resource reason, without an arbitrary nesting-depth cap.
  FR-017's stream capture ceiling is not this public-input guard.
- For falsified replay, the binding shall require the driver's measured native observation of the
  same proved artifact and the retained shadow result/count, tied to the node, operands, original
  limits and retained original artifact. Playback alone and a freshly regenerated correct
  artifact supply no native observation of mutated proved content. This required observation for
  replay continuing beyond Disagreed does not mask retained same-valid-claim disagreement; the
  converter preserves FR-029 AC-24 before checking missing or failed native observation.
- The converter shall retain the actual sent `CompositeIdentity::new` over the request obligation,
  `CompositeParityClaim` and complete `CompositeEvidence` before calling the owning entry.
- The converter shall validate the result/run-to-claim binding before reading FSL's closed typed
  discriminants and catalog codes. It shall compare `report.claim()` with
  `CompositeIdentity::new(obligation, &claim, evidence)` over exactly what CG sent: obligation
  identity; node; occurrence; operator; obligation kind; harness bounds; exact-evaluation limits;
  and the full observation. Falsified observation includes both
  operands, retained shadow verdict and pair count, actual native observation and cause, and
  refinement. Verified observation includes SUCCESS count and refinement. A missing report or a
  change to any one member yields a typed CG refusal with no terminal value. CG's own failed
  source/package, harness, playback or same-artifact prechecks likewise return no report or terminal
  value. Once the full report binds, any FSL `Refused` result is a typed report with a terminal
  value: non-fault refusals retain their FSL code in
  `Inconclusive(ReplayRefused)`, while `ReplayRefusal::Fault` and
  `ReplayRefusal::Admission(AdmissionFailure::Fault)` yield `Failed`. FSL's `prepare` refusals
  precede F-1. A refusal raised during operand admission follows F-1 and F-2; a refusal raised
  during exact comparison follows admission. Retained Disagreed therefore wins over those later
  refusals. CG shall not synthesize an invariant fault or a FSL report to exercise this mapping.
- The converter shall expose `composite_parity_terminal_value` for a falsified
  `CompositeParityReport` and `verified_shadow_terminal_value` for a `VerifiedShadowReport`.
  Each operation shall take the retained sent `CompositeIdentity` and an optional genuine report
  and return a binding-checked settlement with private result storage, a `result()` accessor and
  a `terminal_value()` derived from that FSL result.
- When the report is absent or `report.claim()` differs from the retained sent identity, the
  converter shall return `CompositeReportError::MissingReport` or
  `CompositeReportError::ClaimMismatch`, respectively, with no terminal value. IR-635's
  builder owns the delivered pre-invocation source, package and context refusals; IR-806 owns the
  planned artifact, harness and playback refusals.
- The comparison shall use the retained sent claim, not a later-mutated request.
  When a valid request changes before sending, the converter shall retain the identity of the
  changed request actually sent and compare the genuine returned report against that identity.
- When the genuine report equals the actual sent identity, the converter shall pass this binding
  check regardless of differences from an earlier unsent request. Independent source/artifact
  prechecks still apply.
  Echo equality does not authenticate the proved artifact: independent retained
  AD-002 R-6/R-7 source/package/context, same-artifact authentication and original-limit ties shall
  still establish that the measured native observation belongs to the actual proved generated
  artifact. Structural claim identity, same-artifact authentication and refinement certificate identity
  remain separate roles; none shall substitute for another.
- The driver shall call FSL's exact equality parity arm between CG's public builder and converter.
  CG supplies no local exact evaluator, synthetic predicate or canned replay verdict.
- The builder shall project actual FR-028 refinement evidence into one shared `Refinement` for both
  falsified and verified requests: `Exhausted`, `NotExhausted`, `CeilingReached` or `Disagreed`.
  Exhaustive maps to `Exhausted`, sampled/not_run to `NotExhausted`, actual refinement ceiling to
  `CeilingReached`, and retained refinement_failed to `Disagreed`. Verified settlement carries no
  native observation, including no extra probe for `not_run`.
- The converter shall preserve the claim-validity refusals before applying any parity settlement
  row, including `Disagreed`; another package/node/occurrence, malformed bounds or structural identity/body mismatch
  cannot settle the actual claim.
- For a valid falsified claim, the converter shall consume FSL's first applicable outcome in the
  Falsified Settlement table below, retaining the actual native cause and resource stage.
- For a valid verified claim, the converter shall consume FSL's disagreement, refinement ceiling,
  zero/vacuous, exhausted-and-covered, then otherwise-Tested order. The ordinary CG backend's
  vacuous/inconclusive classification remains owned by FR-029; it is never relabelled a verified
  backend run to create a zero-check request.
- The builder shall submit the same node-selected claim and actual harness bounds for verified
  coverage without a CG-authored declared-bound list.
- The converter shall consume FSL-derived positions and source coverage.
- The converter shall retain FSL's uncovered reading for missing keys.
- The converter shall preserve FSL's typed key-naming refusal for unknown, duplicate or
  kind-mismatched bounds, an enum Variants bound with an undeclared variant, or a request
  DeclaredDomain over an enum position.
- The builder shall preserve literal singleton domains without deriving parameter positions.
- The converter shall consume full enum coverage only when FSL establishes that every
  source-declared variant is included.
- The converter shall retain uncovered/Tested for other non-Boolean leaf families (text, rational,
  decimal, IEEE, quantity and reference) until FSL has an authoritative bound kind describing the
  whole declared domain.
- The converter shall not promote one covered length/range/scale dimension to full coverage.
- The converter shall consume FSL's tightened-harness-bound coverage under ADR-021 TX-3 without
  promoting it to full declared-domain coverage.

The actual generated-family and source coverage remains a CODE gate. Real Text-leaf profile
selection and nested set/bag single-encoding performance remain open upstream work; API availability
and rational-leaf substitution do not close those checks. Unbounded collection/recursive coverage
exercised only through bound derivation/coverage seams does not establish source-function coverage
where source grammar requires explicit maxima. Families lacking lawful generated shadow or owning
admission shall remain explicitly unsupported/gated, with no synthetic proof or claim of closure.

## Falsified Settlement

When source/package, claimed-node membership, bound and structural identity/body admission succeeds, CG shall
preserve the following FSL first-match order for that valid claim. FSL FR-358's F-1 to F-7 rows
and `fsl-replay` facade were merged in FSL #645. IR-666's public converter preserves their
bound results; IR-806's original-artifact builder and production invocation remain planned.

| Row | Condition | Consumed settlement and retained evidence |
|---|---|---|
| F-1 | `Refinement::Disagreed` | Failed with CG-owned CgDefect, even if operand admission, native observation or exact comparison would otherwise agree/refuse/stop |
| F-2 | actual native observation is Incomplete or ExecutionFault | `GeneratedFault { native }`/Failed retaining the actual native outcome and NativeCause; FSL owns the no-admission/no-exact-evaluation ordering |
| F-3 | operand admission refusal | `RefusedInput`, Inconclusive(ReplayRefused), retaining the actual FSL catalog code and operand index |
| F-4 | request accounting limit reached during operand admission | `Incomplete { stage: Admission }`/Incomplete(ResourceExhausted), retaining the counter, configured limit and count reached; FSL owns the no-exact-evaluation ordering |
| F-5 | FSL exact evaluation reaches its accounting limit | `Incomplete { stage: ExactEvaluation }`/Incomplete(ResourceExhausted), retaining FSL's counter, configured limit and count reached |
| F-6 | `Refinement::CeilingReached` | `Incomplete { stage: RefinementCeiling }`/Incomplete(ResourceExhausted), retaining actual refinement ceiling evidence |
| F-7 | otherwise, exact versus retained shadow equality/inequality result and pair count | divergence Failed/CgDefect; agreement Inconclusive(ScalarAgrees) retaining the CompositeEquality claim and outcome; never Refuted |

A native `Completed` or `Refused` observation remains measured same-artifact evidence but is not the
settlement oracle: the exact-versus-shadow comparison owns F-7. `Equal` and `NotEqual` preserve the
actual claimed operator and the same admitted pair count. A retained independent refinement
disagreement cannot disappear merely because the replayed case agrees. The common identity refusal
is earlier because a mismatched request is not the same claim; FR-029 AC-24 remains intact for the
admitted claim.

The single-observation native failure in F-2 retains its actual cause and precedes operand refusal
and both limit stages. Request accounting exhaustion during admission is F-4 and never claims exact
evaluation ran; FSL exact-evaluation exhaustion is F-5 and precedes the independent refinement
ceiling at F-6. Backend Kani
wall/memory ceilings are outside these replay inputs: FR-028 AC-2/3 classify that backend run as
`KaniRunOutcome::Inconclusive`, and FR-029's ordinary map returns final `Incomplete(TimedOut)` or
`Incomplete(ResourceExhausted)`. FR-028 AC-24 governs the separate native refinement run. No stage
is collapsed into completed `NotExhausted` evidence or `Tested`; FR-029 AC-23 remains intact.

The terminal decision and its precedence remain owned by FR-029 AC-17 and AC-19 to AC-28; the source
strength and resource classification remain owned by FR-028 AC-17/24. Canonical decoding all value
families does not expand a generator's supported shadow shapes or machine ABI. A shape with no
lawful harness or upstream conversion remains explicitly typed unsupported/requires-bound under
FR-028 AC-20, with no fake harness, native value or compatibility fallback.

## Setup Refusal Precedence

Claim/setup checks and operand/native handling are separate stages:

1. Public transport's encoded byte limit guards input before parsing or canonical unescaping;
   checked occurrence/work limits guard every required decode. No valid claim or evidence is
   invented from malformed transport.
2. CG's original source/package, claimed-node membership/operation/occurrence, declared operand schema,
   harness-bound keys/kinds and structural identity/body, original limits and same-artifact/context authentication
   must be coherent. Adapter assertion/harness selection and the persisted leaf-binding schema
   must name that actual claim; another result/run cannot settle it. These claim-identity refusals
   precede Disagreed and return no report or terminal value. A bound-schema refusal is not an
   operand value's admission failure. If CG sends a request instead, FSL's `prepare` may return
   `CompositeParityResult::Refused`; after full identity binding this report has a terminal value
   and precedes F-1. A later operand-admission or exact-comparison fault may also produce
   `Refused`, but only after F-1 (and F-2); Disagreed suppresses it. The terminal value of any
   binding-valid `Refused` report is ReplayRefused with FSL's code for a non-fault, or Failed for
   `Fault` and `Admission(Fault)`.
3. FSL #645 has delivered the node-parity/value/settlement facade, and IR-666 exposes the
   public CG report converters. IR-806 still owns the original-artifact builder and production
   invocation. A future pre-invocation refusal from that builder carries no FSL report or
   terminal value; the converter never fabricates a FSL catalog code.
4. For the same admitted claim after CG consumer delivery, retained Disagreed takes F-1/FR-029 AC-24 precedence
   before operand decode/admission and native-observation handling. A composite operand field `x`
   declared `Int[0, 9]` but supplied as `x: 12`, wrong operand shape/member/presence, missing native
   observation or native fault cannot mask that
   retained disagreement as a no-settlement setup refusal. The converter consumes Failed/CgDefect
   without fabricating an operand or native observation; CG code must use FSL #645's delivered
   owning representation.
5. Without Disagreed, playback leaf arity/width/order, canonical syntax, typed value
   shape/member/presence and declared-domain validation occurs in the operand stage before exact
   evaluation. Native Incomplete/ExecutionFault is F-2 before operand admission refusal at F-3;
   request admission exhaustion is F-4. Missing required native observation remains a typed
   refusal with no settlement for a replay continuing beyond F-1, while actual Completed/Refused
   remains evidence. The same-artifact observation tie is required. Exact and refinement limits
   retain F-5/F-6 order. Verified settlement instead consumes coherent actual refinement evidence,
   with `not_run` explicitly absent and no native probe.

The public converter binds reports from direct FSL facade calls and the Eq/Ne verified
constructor. Falsified production construction and same-artifact authentication remain IR-806
work. On a valid direct claim, operand/native
failures do not erase same-valid-claim Disagreed. FSL non-fault refusals retain their own catalog
codes; executor faults
and CG-raised malformed playback/assumption-domain defects retain FR-029 failure ownership after
the disagreement row. A real backend or refinement stop retains its original FR-028 classification
and FR-029 resource terminal; it cannot become completed NotExhausted evidence or Tested.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-033-AC-1 | PARTIAL (IR-635 verified Eq graph-child constructor controls and Ne parameter/parameter and parameter/literal controls, including operator mismatch refusal; Ne literal/literal is AC-14 and inline operands are AC-16 and AC-17, all PLANNED under IR-814; falsified and generated-artifact routes planned, IR-806). A public consumer builds the equality-parity request directly from the composite node and original proving context. CG submits the original source/package and node selector to FSL and accepts only its admitted same-package/node result under FSpec FR-322. CG validates admitted node/occurrence presence and exact recompiled context before invocation. The verified request carries the original application node's equality operator: `structural.eq` is `Equal` and `structural.ne` is `NotEqual`. If the requested operator differs from the node's, CG refuses before any FSL invocation with typed `OperatorMismatch` carrying the requested and actual operators and returns no report or terminal value; an admitted claim carries the node's operator. FSL owns selected-function body membership and occurrence origin through its composite-site locator; CG has no graph-closure or source-region membership rule. A mismatch caught by CG before invocation returns no report or terminal value; a FSL common-step membership mismatch returns a binding-checked `Refused` report with its terminal value. Parameter/parameter, parameter/literal and literal/literal claims preserve their operands and literal singleton domains without an enclosing-function predicate or synthetic source. Original recompiled node/occurrence and each positional graph child or inline literal must belong to the actual retained proving context; no enclosing-function substitution satisfies this. | Test |
| FR-033-AC-2 | PARTIAL (IR-806 PR-2; refs `tc_048_rebuilt_operands_round_trip_through_the_canonical_value_text`, `tc_048_absent_null_and_present_slots_rebuild_as_distinct_values` and `tc_048_nested_option_presence_rebuilds_each_level_and_skips_the_payload_of_an_absent_one` of `tests/it/composite_playback_decode.rs`). Covered: the composite shadow's leaf families, a Boolean and a bounded integer (an `ExactInteger` leaf inside a composite, the whole `i64` range included, while a top-level Integer stays `i64`), an option, a record with an optional slot and a tuple, each rebuilt from a playback by the decoder and round-tripped through FSL's canonical value text; absent, null and present slots and `none`, `some(none)` and `some(some(_))` are distinct values, and a record keeps its declared field order. Not covered, with no CG decoder to exercise because the shadow generator draws no leaf of these families: IEEE NaN and signed-zero bits, rational, decimal, text, enum, quantity and unit, union members, sequence/set/bag/ordered-set order and multiplicity. Reference identity additionally needs FSL to export the admitted model object types: `AdmittedContext` exposes enums, composites, units and dimensions but no object types, so CG cannot admit a `Reference<T>` operand. That FSL gap is out of scope here and stays the honest remaining part of this criterion. | Test |
| FR-033-AC-3 | PARTIAL (IR-806 PR-2; refs `tc_048_a_playback_that_does_not_fit_the_leaf_schema_refuses_at_its_typed_boundary` of `tests/it/composite_playback_decode.rs`). CG-owned refusals are covered, each before any exact evaluation: a missing or surplus playback leaf, a payload entry for a leaf whose presence or state does not admit it, a wrong primitive width, a swap of leaves of different primitive types, an out-of-domain integer or slot state, a value that disagrees with Kani's own rendering, another harness's playback, and a leaf schema that fails its binding check. A swap of two leaves of one primitive type is not a refusal: it decodes, and the rebuilt operand differs (FR-033-AC-5). Not covered here: noncanonical JCS/tag/scalar/escape spelling, unknown unit/reference identity, wrong enum/union member, wrong tuple arity, excess cardinality and duplicate set elements belong to FSL's canonical value reader (FR-070) and to leaf families the shadow generator does not draw; the same-claim Disagreed precedence over an operand refusal (an operand field x declared Int[0, 9] but supplied as x: 12) needs the production route and is IR-806 PR-3. | Test |
| FR-033-AC-4 | PARTIAL (IR-806 PR-2; refs `tc_048_the_encoded_byte_and_value_limits_admit_the_limit_and_refuse_one_over` and `tc_048_a_100000_link_value_clones_compares_formats_and_drops_on_a_512_kib_stack` of `tests/it/composite_playback_decode.rs`). Covered: a playback exactly at the configured encoded-byte limit is admitted and one byte over refuses before parsing, in the reader and in the decoder; value-count exhaustion refuses with the limit named; a 100,000-link nested value built through FSL's public value type clones, compares, formats a redacted Debug rendering (asserted to hold none of the value's leaf content) and drops on a 512 KiB native stack. Not covered: decoding a 100,000-deep value through FSL's canonical reader, which no test here runs (the production route, IR-806 PR-3); the recursive value is not an admitted recursive declaration, because the composite shadow refuses a declaration that reaches itself with `RequiresBound`, so no CG-built operand is recursive and the stack test pins FSL's value type only; the one-byte-too-small budget for such a value and occurrence/work exhaustion of FSL's replay limits belong to the production route (IR-806 PR-3). | Test |
| FR-033-AC-5 | PARTIAL (IR-806 PR-2; refs `tc_048_literal_operands_keep_their_pinned_identity_and_are_never_drawn`, `tc_048_absent_null_and_present_slots_rebuild_as_distinct_values`, `tc_048_a_playback_that_does_not_fit_the_leaf_schema_refuses_at_its_typed_boundary` and the installed-backend `tc_048_installed_kani_playback_of_a_falsified_mutant_decodes_and_rebuilds_both_operands`). The actual assertion playback reconstructs both operands through the FR-025 original-node/leaf bindings (`reconstruct_composite_operands`): a parameter operand from the leaves of its node (two positions naming one node get equal operands), a literal operand from its pinned singleton value, absent, null and present states distinct. A swapped, missing, extra or wrongly typed playback leaf, another harness's playback and a changed or removed leaf binding refuse or change the rebuilt operand instead of decoding another one. A real falsified Kani mutant (field `b` left out of the comparison) was decoded and rebuilt: the operands agree on `a` and `c`, differ on `b` and are unequal as values. Not covered: refusal ordering after same-claim Disagreed precedence (retained Disagreed stays Failed/CgDefect) needs the production route (IR-806 PR-3). | Test |
| FR-033-AC-6 | ~~Retired content-identity claim criterion.~~ FR-033-AC-19 carries forward actual same-artifact native observation, original limits, mutation-preserving authentication and Disagreed precedence as an independent CG pre-invocation check. The removed `CompositeParityClaim.content_identity` is not replaced by another digest or claim member. | Inspection |
| FR-033-AC-7 | PARTIAL (IR-666 direct public F-7 converter controls; IR-806 original-artifact path planned). For an identity-valid claim reaching row F-7, actual FSL exact equality/count divergence from a falsified shadow maps to CG-defect `Failed`; an assertion-only mutation with otherwise agreeing equality/count maps to `Inconclusive(ScalarAgrees)` retaining the CompositeEquality claim/outcome. Neither maps to `Refuted`. Independently mutating equality result and occurrence-pair count is detected, including no early exit after an unequal pair. | Test |
| FR-033-AC-8 | PARTIAL (IR-806 PR-3). CG submits the original claim's operand declarations and literal singleton domains and consumes FSL-derived bound keys without a CG-authored declared-bound list. Omitting a bounded position or using an empty list with declared/unbounded/recursive keys leaves it uncovered; unknown/duplicate/kind-mismatched keys refuse. Exact declared coverage and a wider legal harness range cover; a tightened range/cardinality/depth does not silently cover under ADR-021 TX3. Enum Variants bounds cover only when every source-declared variant is included; a Variants bound naming an undeclared variant and a request DeclaredDomain over an enum position each preserve FSL's typed refusal naming the key; text/rational/decimal/IEEE/quantity/reference positions remain uncovered/Tested until an authoritative whole-domain bound kind exists, rather than promoting one dimension as full coverage. CG cannot supply an invented declared-bound list to promote the result. Composite literal graph children have empty Bounds and no free position; this cannot remove declared/drawn bounds of a parameter position or promote source-unbounded derivation-only coverage. Covered: CG submits no declared-bound list; the harness bounds are the harness identity's drawn integer ranges keyed by FSL's position paths (an option payload is child 0, a record or tuple member its declaration index) and FSL decides coverage: the exact drawn range is `Proved`, a tightened drawn range is `Tested` (`tc_048_a_tightened_drawn_range_does_not_cover_the_declared_one`, `tc_048_a_verified_shadow_settles_through_fsl_by_refinement_and_coverage` of `tests/it/composite_shadow_route.rs`). PLANNED (IR-1567): a wider legal range, an omitted position, an empty list with declared keys, unknown, duplicate and kind-mismatched keys, enum Variants and DeclaredDomain refusals and the whole-domain families have no route-level test; the shadow generator draws only Boolean and bounded-integer leaves. | Test |
| FR-033-AC-9 | ~~Retired content-identity report-binding criterion.~~ FR-033-AC-20 carries forward full actual-sent FSL claim/report binding, every remaining claim and evidence member, refusal mapping and outcome precedence. FR-033-AC-19 separately carries the same-artifact check; the retired field is not reused. | Inspection |
| FR-033-AC-10 | PARTIAL (IR-806 PR-3). Each lawful generated family exercised by the completed route has a real verify and falsify control with retained backend evidence and mutation-killed same-artifact replay; all canonical families have the AC-2/3 decode checks. Families awaiting shadow generation or upstream admission remain explicitly unsupported/gated and are reported as gaps, rather than counted as passing end-to-end coverage. Open real Text-profile, nested set/bag performance and source-unbounded coverage limitations remain explicit; API presence or rational substitution supplies no generated-family acceptance. Covered for the Boolean, bounded-integer and optional-slot record family only (`Flags`): a real installed-Kani verified control and a real falsified mutant (field `b` left out), each settled through the production route (`tc_048_installed_kani_runs_settle_through_the_production_route`, ignored kani lane). The other lawful families (tuple, option, nested option) have generation and decode tests but no route-level real-Kani control; union, text, rational, decimal, IEEE, enum, quantity and collection families stay unsupported, reported gaps. | Test |
| FR-033-AC-11 | ~~Retired O-09 parity-preimage criterion.~~ FR-033-AC-18 carries forward positional graph-child arguments, repeated and literal operands, key ordering, membership refusal, complete retained-body comparison, separate proving-record metadata and independent artifact authentication. The old hash oracle is not reused. | Inspection |
| FR-033-AC-12 | PARTIAL (IR-666 direct FSL F-1 precedence controls; IR-635 Eq-only verified constructor and FSL membership/unknown-bound refusals before supplied Disagreed; falsified and cross-outcome integration planned, IR-806). Both paths carry the same four-state Refinement. Claim/source/node/occurrence/bounds/structural-identity-or-body refusal precedes Disagreed; for a valid claim Disagreed wins over operand refusal (including an operand field x declared Int[0, 9] but supplied as x: 12), missing native observation, native fault, exact limit, refinement ceiling and agreeing replay. No early operand/native setup refusal erases that disagreement and no missing evidence is fabricated. Completed/Refused native evidence does not replace exact-versus-shadow verdict/pair-count comparison, and the actual Eq/Ne operation survives. | Test |
| FR-033-AC-13 | PARTIAL (IR-666 direct public F-2 to F-6 controls; IR-806 must reach the same rows from a CG-built request over the original proving context). On a valid falsified claim without Disagreed, a native Incomplete or ExecutionFault wins over an invalid operand, too-small admission limit, too-small exact limit and CeilingReached: CG observes GeneratedFault/Failed retaining its NativeCause. With a Completed native, an operand outside its declared domain yields RefusedInput/ReplayRefused even with CeilingReached; a request limit reached during admission yields Incomplete/Admission with its counter, and an exact limit reached after admission yields Incomplete/ExactEvaluation even with CeilingReached; only with sufficient limits does CeilingReached yield Incomplete/RefinementCeiling. CG asserts these report results and terminal values; FSL FR-358 owns the internal rule that F-2 skips admission and exact evaluation and F-4 skips exact evaluation. All three limits are ResourceExhausted but keep distinct stages. Backend Kani timeout/memory Inconclusive outcomes never enter this facade. No resource stage becomes Tested. | Test |
| FR-033-AC-14 | DELIVERED (IR-814; literal operands are admitted one-field `Integer` records, with bounded-field narrowing literals (IR-691) and collection-constructor literals gated). When the original application node is a `structural.ne` whose two operands are both composite literals and the requested operator is NotEqual, the verified request carries NotEqual and a retained checked claim body with two positional `graph_child` arguments, each the literal's own node with empty Bounds and no parameter position. | Test |
| FR-033-AC-15 | DELIVERED (IR-814). When the original application node is a `structural.ne` comparing one parameter with itself and the requested operator is NotEqual, the verified request carries NotEqual and a retained checked claim body with two positional `graph_child` arguments with the same node identity, each retaining the drawn Node-keyed bounds naming that parameter and neither deduplicated. | Test |
| FR-033-AC-16 | DELIVERED (IR-814; verified by Inspection of the single `map_err` on `composite_application_operands` in `selected_claim`, `src/replay/composite_builder.rs`, which converts every projection cause to `CompositeBuildError::Operands` without inspecting it and runs before any FSL invocation). If the owning operand projection refuses an inline integer literal operand (`InlineInteger`), then the builder returns that refusal through `CompositeBuildError::Operands` before any FSL invocation, with no report or terminal value and no built argument. The owning contract treats this branch as defensive because reader admission already refuses an integer operand of `structural.eq` and `structural.ne`, so no admitted fixture can reach it. Both inline refusals pass through the same `CompositeBuildError::Operands` conversion, which AC-17 exercises. | Inspection |
| FR-033-AC-17 | DELIVERED (IR-814). If the owning operand projection refuses an inline non-integer literal operand (`InlineNonInteger`), then the builder returns that refusal through `CompositeBuildError::Operands` before any FSL invocation, with no report or terminal value and no new CG error variant. The test re-reads a package whose operand is an inline `none` literal of an Option type, which IR's reader admits. | Test |
| FR-033-AC-18 | PLANNED / CODE-GATED. CG obtains the original equality/inequality application structural identity from IR admission and retains the authentic occurrence, kind and one argument per operand position in original order. A self-comparison keeps two graph-child entries with each drawn Node-keyed bound; distinct parameters and composite literals keep their own graph-child entries, with empty Bounds for literals. Population-keyed metadata stays in the full proving record; an admitted FSL claim containing it refuses as HarnessUnknownKey before identity/body comparison. Bounds key ordering and duplicate-key refusal remain authoritative. A package, path or producer-ordinal change changes the structural identity; changing an included body member under the same identity refuses by complete body comparison. Changes only to abstractions, size budget, static closure pair-node count or unexercised behaviours do not change the structural node identity, but the full proving record and same-artifact/context authentication remain required. No copied parity encoder or hash participates. | Test |
| FR-033-AC-19 | PARTIAL (IR-806 PR-3). Before falsified parity invocation, CG authenticates the retained proved generated artifact and original source/package/context, reconstructed operands and original execution limits against the driver's actual native observation; it preserves the owning same-valid-claim Disagreed precedence before treating a missing or failed observation as a refusal. Without a same-valid-claim Disagreed, a different artifact, changed retained bytes or context, altered operands or limits, missing observation, or fresh regeneration that removes a mutation refuses with a typed CG pre-invocation error and no report or terminal value; a Kani transcript or matching report claim alone does not authenticate it. A valid same-claim Disagreed remains Failed/CgDefect even when the native observation is missing or failed; no fabricated observation is supplied. Verified settlement requires no native observation and no extra probe. A healthy same-artifact run and a body-only mutant from the same original proving context exercise both sides of the check. No digest, content-identity field or independent claim member is introduced. Covered: before the falsified invocation the entry ties the authenticated artifact (IR-1184 binding of the emitted bytes, source, package and limits) to the claim identity, request context and rebuilt operands, requires a native observation consistent with the artifact's own run, and refuses each failure with no report or terminal value; a retained disagreement is settled first and is not lost to a missing observation (`tc_048_the_native_observation_is_tied_to_the_authenticated_artifact`). PLANNED (IR-1567): the native observation is supplied by the driver, not executed through the route, and carries no measured occurrence-pair count (the generated oracle returns a Boolean); the shadow comparison that alone decides F-7 is likewise a driver measurement and a named trust assumption; the route tests construct the authenticated capture without running the emitted oracle, so the consistency check compares two supplied values; a changed retained byte or source is refused by `AuthenticatedCompositeArtifact::complete` (IR-1184, `composite_artifact_binding.rs`), not through the route; a body-only mutant produced from the same context and exercised on both sides, with the sentinel for each of its members, is not built. Covered through the route: an artifact of other operands, bounds, accounting limits, stage limit or replay limits, and each pairing of artifact outcome with native observation (`tc_048_an_artifact_of_other_bounds_limits_or_context_refuses_with_no_value`, `tc_048_each_native_observation_pairs_only_with_its_own_artifact_outcome`). | Test |
| FR-033-AC-20 | PLANNED / CODE-GATED for the current FSL FR-358 claim shape; earlier IR-666 converter and IR-635 verified-constructor controls remain partial evidence only for unchanged members. Before projecting any FSL outcome, CG compares a genuine `report.claim()` against `CompositeIdentity::new` over exactly the actual sent obligation, all remaining `CompositeParityClaim` members (node, occurrence, operator, obligation kind, harness bounds and limits) and complete `CompositeEvidence` (falsified operands, shadow verdict and pair count, native observation and cause, refinement; or verified SUCCESS count and refinement). With the genuine report fixed, independently changing each retained member or omitting the report returns a typed CG refusal with no terminal value; a valid changed request retained before send binds its own genuine report, despite differing from an earlier unsent request. A binding-valid FSL `Refused` report preserves its code and maps non-fault to Inconclusive(ReplayRefused), or `Fault`/`Admission(Fault)` to Failed; `prepare` refusal precedes Disagreed while admission and exact-comparison refusal follow it. CG invents no report, catalog code or predicate route. Public ReplayLimits is the separate final facade argument, not a claim member. Matching report identity cannot satisfy FR-033-AC-19. | Test |

## Dependencies

[TC-048](../matrix/TC-048-composite-parity-replay-binding.md) records the direct public-converter
controls and the remaining IR-806 production scenarios. FR-025 owns emitted binding
semantics; FR-028 owns shadow independence, refinement class, domain containment and execution
ceilings; FR-029 owns the exhaustive strength projection, terminal decision and record category. FSL
owns canonical value types/admission, original-package node selection, exact evaluation,
declared-bound completeness and the legal settlement/cause vocabulary. FSpec FR-181 remains the
encoding authority. No schema, binary or source file is vendored from those owners.

Implementation shall consume the delivered owning public interfaces and retain every required
canonical conversion/lifecycle, refinement, same-claim observation/content and legal cause check.
The Eq constructor currently projects operands and retains the genuine checked claim before authenticating
recompiled context, because FSL's public request decoder requires that wire to yield typed
`StageLimits`. Its standalone stage-limit decoder is private, and CG has no original-compile
caller retaining typed limits. An invalid operand or encoder input can therefore precede a
context refusal; no request is invoked until exact context validation succeeds. Moving
context authentication earlier awaits an owning public API or established retained typed limits.

The Eq verified constructor controls use supplied verified evidence and establish no generated
backend proof or native observation. Actual CG driver authentication, generated-family proofs and
tests remain PLANNED/UNRUN CODE gates;
public API availability and specification publication establish none of that evidence.

## Native Publication-Boundary Evidence

`tests/it/composite_artifact_binding.rs` executes the original generated native
oracle and refuses independent body, context and artifact mutations.
`tests/it/composite_record_residuals.rs` additionally refuses a change to each
published native file and to typed claim-map fields while the node identity
stays equal. These are native publication/authentication controls. They do not
supply the full proved-artifact record, backend success, falsified conversion
or shadow pair-count evidence still Planned under IR-806. AC-18 retains its
complete proving-record and same-artifact requirements.
