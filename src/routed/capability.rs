//! Capability settlement: the one point where a requested claim is settled.
//!
//! AD-016 places capability negotiation here so that language admission stays
//! language-only and the IR stays target-neutral. Every admitted registration,
//! whatever it describes, is settled by the generic arm in [`super::generic`]
//! (FSpec FR-711). A backend whose IR forms need their own check adds a
//! backend-specific arm after it; those arms are variants of a closed enum
//! dispatched with no catch-all, so a new one does not build until it is
//! handled.
//!
//! `candidates` is read, never computed. The `filament-spec-language` registry
//! computes it under FR-290's candidate-set rule; this module is the consumer
//! on the far side of the FR-331 envelope.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use filament_contract_model::{ApplicationOperator, CheckedTerm};
use filament_contract_model::{
    CheckedExtensionRefusalDetail, CheckedNodeId, CheckedPackageIncomplete, CheckedPackageLimit,
    CheckedPackageRefusal, CheckedPackageRefusalCause, CheckedPackageRefusalCode,
    CheckedPackageRefusalLocation, CheckedProtocolRefusalDetail,
    ContractPackageV1,
};
use serde::Serialize;

use crate::core::contract_package::{
    AdmittedItemAccounting, ContractRequestPosition, PackageItem, PackageItems, PackageRequestError,
};
use crate::routed::generic::negotiate_generic;
use crate::routed::vocabulary::{
    BackendDescriptor, Candidate, CapabilityKind, Cause, Disposition, ExtentClassification,
    ItemRequirements, KaniFormFamily, KaniFormIdentity, KaniFormName, KaniObligationRole,
    KaniStateFrameRole, ProviderOrigin,
};

#[cfg(test)]
thread_local! {
    static PACKAGE_CLASSIFIER_INDICES: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_package_classifier_indices() -> Vec<usize> {
    PACKAGE_CLASSIFIER_INDICES.with(|indices| std::mem::take(&mut *indices.borrow_mut()))
}

#[cfg(test)]
use crate::oracle::scalar as scalar_fixture_types;

#[cfg(test)]
#[allow(clippy::duplicate_mod)] // Reuse the admitted scalar corpus in this crate's unit test binary.
#[path = "../../tests/exact_scalar_support/package.rs"]
mod exact_scalar_fixture;

#[cfg(test)]
mod scalar_cache_tests {
    use super::{exact_scalar_fixture, CapabilityLookups, KaniFormFamily};
    use ix_trace_rs::trace;
    use std::collections::{BTreeMap, BTreeSet};

    /// Each distinct target keeps its own derived family, independent of lookup order.
    #[trace("TC-030", "FR-019-AC-25")]
    #[test]
    fn two_scalar_families_share_one_package_walk_without_swapping_identity() {
        use exact_scalar_fixture::{
            code_id, derivation_package, DERIVE_BOOLEAN_AND, DERIVE_INTEGER_GT,
        };

        let (package, ids) = derivation_package().admit_resolved();
        let boolean = ids.resolve(&code_id(DERIVE_BOOLEAN_AND));
        let integer = ids.resolve(&code_id(DERIVE_INTEGER_GT));
        let targets = BTreeSet::from([boolean.clone(), integer.clone()]);
        for order in [
            [
                (&boolean, KaniFormFamily::Boolean),
                (&integer, KaniFormFamily::Integer),
            ],
            [
                (&integer, KaniFormFamily::Integer),
                (&boolean, KaniFormFamily::Boolean),
            ],
        ] {
            let mut lookups = CapabilityLookups {
                scalar: crate::kani::generate::negotiate::ScalarFormLookups::new(),
                scalar_targets: BTreeMap::from([(std::ptr::from_ref(&package), targets.clone())]),
                scalar_families: BTreeMap::new(),
                functions: BTreeMap::new(),
            };
            let _ = crate::oracle::scalar::derive::take_scalar_derive_graph_visits();
            for (node_id, expected) in order {
                assert_eq!(lookups.scalar_family(&package, node_id), Some(expected));
            }
            assert_eq!(
                crate::oracle::scalar::derive::take_scalar_derive_graph_visits(),
                package.resolved_graph().nodes.len(),
                "both exact targets must share one scalar graph walk"
            );
        }
    }
}

struct CapabilityLookups<'a> {
    scalar: crate::kani::generate::negotiate::ScalarFormLookups<'a>,
    scalar_targets:
        BTreeMap<*const filament_contract_model::CheckedPackageV2, BTreeSet<CheckedNodeId>>,
    scalar_families: BTreeMap<
        *const filament_contract_model::CheckedPackageV2,
        BTreeMap<CheckedNodeId, Option<KaniFormFamily>>,
    >,
    functions: BTreeMap<
        *const filament_contract_model::CheckedPackageV2,
        crate::oracle::function::FunctionPreflightGraph<'a>,
    >,
}

impl<'a> CapabilityLookups<'a> {
    fn new(items: &[RequestItem]) -> Self {
        let mut scalar_targets = BTreeMap::<
            *const filament_contract_model::CheckedPackageV2,
            BTreeSet<CheckedNodeId>,
        >::new();
        for form in items.iter().filter_map(|item| item.kani_form.as_ref()) {
            match form {
                KaniForm::V2ScalarClaim {
                    package, node_id, ..
                }
                | KaniForm::OutsideSet { package, node_id } => {
                    scalar_targets
                        .entry(std::ptr::from_ref(package.as_ref()))
                        .or_default()
                        .insert(node_id.clone());
                }
                KaniForm::V2StateFrame { .. } | KaniForm::V2FunctionApplication { .. } => {}
            }
        }
        Self {
            scalar: crate::kani::generate::negotiate::ScalarFormLookups::new(),
            scalar_targets,
            scalar_families: BTreeMap::new(),
            functions: BTreeMap::new(),
        }
    }

    fn scalar_family(
        &mut self,
        package: &filament_contract_model::CheckedPackageV2,
        node_id: &CheckedNodeId,
    ) -> Option<KaniFormFamily> {
        let package_key = std::ptr::from_ref(package);
        let families = self.scalar_families.entry(package_key).or_insert_with(|| {
            let node_ids = self
                .scalar_targets
                .get(&package_key)
                .into_iter()
                .flat_map(|targets| targets.iter().cloned())
                .collect::<Vec<_>>();
            crate::oracle::scalar::derive::derive_exact_scalar_items_independent(package, &node_ids)
                .into_iter()
                .zip(node_ids)
                .map(|(result, id)| (id, result.ok().map(scalar_family_from_item)))
                .collect()
        });
        families.get(node_id).copied().flatten()
    }

    fn checked_node(
        &mut self,
        package: &'a filament_contract_model::CheckedPackageV2,
        node_id: &CheckedNodeId,
    ) -> Option<&'a filament_contract_model::CheckedSemanticNodeV2> {
        self.function_graph(package).get(node_id)
    }

    fn function_graph(
        &mut self,
        package: &'a filament_contract_model::CheckedPackageV2,
    ) -> &crate::oracle::function::FunctionPreflightGraph<'a> {
        // Keep each admitted package's graph separate, including same-name packages with
        // different contents. The key is used only within this envelope's borrowed lifetime.
        self.functions
            .entry(std::ptr::from_ref(package))
            .or_insert_with(|| crate::oracle::function::FunctionPreflightGraph::new(package))
    }
}

/// The FR-331 envelope's contract version, the only one this module reads.
pub const BACKEND_PROVIDER_CONTRACT: &str = "filament.backend-provider/v1";

/// The FR-290 vocabulary identity, the only one this module reads.
pub const CAPABILITY_VOCABULARY: &str = "filament.capability-kind/v1";

/// The kind a request item carries, including the two ways it can fail to name one.
///
/// An absent kind and an unknown label settle differently and are reported with
/// different causes, so they are separate variants rather than one `Option`
/// that loses which happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestedKind {
    /// The item names a member of the vocabulary.
    Known(CapabilityKind),
    /// The item's kind is absent or `null`.
    Absent,
    /// The item names a label that is not byte-equal to any member.
    Unknown(String),
}

/// An item's `candidates` value.
///
/// FR-331 admits exactly two shapes, and the unknown-backend mark carries the
/// identity the request named so the refusal can name it back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Candidates {
    /// The candidate set the registry computed, ordered by identity.
    Set(Vec<Candidate>),
    /// The request named a backend that is not registered.
    UnknownBackend(String),
}

/// A typed identity for a Kani lowering form.
///
/// This is CG's in-process projection of the admitted IR form.
/// Cloning a form shares its immutable admitted package, so envelope lookups index
/// that instance once. Independently admitted packages retain separate allocations.
/// This is not a wire member; identity is never reconstructed from `Debug` or
/// display output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KaniForm {
    /// A checked claim and its exact admitted result occurrence.
    ///
    /// The operation identity, family, and provenance are read from the typed
    /// claim entry and checked graph; callers cannot assert them independently.
    V2ScalarClaim {
        /// The checked source package containing the source node.
        package: Arc<filament_contract_model::CheckedPackageV2>,
        /// The admitted lowering of the full driver request.
        contract: Box<ContractPackageV1>,
        /// The full typed lowering positions, including context-only roots.
        positions: Vec<ContractRequestPosition>,
        /// This claim's index among driver positions.
        request_index: usize,
        /// The requested checked node.
        node_id: filament_contract_model::CheckedNodeId,
    },
    /// A V2 state-clause/frame role handled by the state-frame path.
    V2StateFrame {
        /// The admitted package containing the state clause.
        package: Arc<filament_contract_model::CheckedPackageV2>,
        /// The checked state-clause node.
        clause: filament_contract_model::CheckedNodeId,
        /// The harness role requested from the checked state-clause node.
        role: KaniObligationRole,
        /// Rust path of the state ABI.
        state_path: String,
        /// State ABI fields in declaration order.
        state_fields: Vec<String>,
        /// Rust path of the operation subject.
        subject_path: String,
    },
    /// A checked V2 function declaration and its local checked call item.
    /// The argument list is a request descriptor and is accepted only when
    /// it is byte-for-byte the ordered reference list in the call node.
    V2FunctionApplication {
        /// The admitted package containing both nodes.
        package: Arc<filament_contract_model::CheckedPackageV2>,
        /// The checked `function/pure_function` declaration being applied.
        function_node_id: filament_contract_model::CheckedNodeId,
        /// The checked local `expression/call` node.
        call_node_id: filament_contract_model::CheckedNodeId,
        /// Source argument nodes, in the caller's claimed order.
        argument_node_ids: Vec<filament_contract_model::CheckedNodeId>,
    },
    /// A typed form outside the normative V1/V2 Kani set.
    OutsideSet {
        /// The admitted package containing the typed node.
        package: Arc<filament_contract_model::CheckedPackageV2>,
        /// The checked node outside Kani's normative form set.
        node_id: filament_contract_model::CheckedNodeId,
    },
}

fn obligation_role(role: &str) -> Option<KaniObligationRole> {
    match role {
        "precondition" => Some(KaniObligationRole::Precondition),
        "postcondition" => Some(KaniObligationRole::Postcondition),
        "invariant" => Some(KaniObligationRole::Invariant),
        _ => None,
    }
}

fn scalar_family_from_item(item: crate::oracle::scalar::ExactScalarItem) -> KaniFormFamily {
    use crate::oracle::scalar::{
        EqualityOperandKind, ExactScalarOperation as Op, OrderingOperandKind,
    };
    match item.operation {
        Op::Boolean { .. } => KaniFormFamily::Boolean,
        Op::Equality { operands, .. } => match operands {
            EqualityOperandKind::Boolean => KaniFormFamily::Boolean,
            EqualityOperandKind::Integer => KaniFormFamily::Integer,
        },
        Op::IntegerArithmetic { .. }
        | Op::IntegerDivision { .. }
        | Op::IntegerRemainder { .. }
        | Op::IntegerModulo { .. } => KaniFormFamily::Integer,
        Op::RationalArithmetic { .. } => KaniFormFamily::Rational,
        Op::Ordering { operands, .. } => match operands {
            OrderingOperandKind::Integer => KaniFormFamily::Integer,
            OrderingOperandKind::Rational => KaniFormFamily::Rational,
            OrderingOperandKind::Decimal => KaniFormFamily::Decimal,
        },
        Op::DecimalArithmetic { .. } => KaniFormFamily::Decimal,
        Op::IeeeArithmetic { .. } | Op::IeeeComparison { .. } | Op::IeeeWidthConversion { .. } => {
            KaniFormFamily::IeeeFloat
        }
        Op::TextAdmission { .. } | Op::TextComparison { .. } => KaniFormFamily::Text,
        Op::EnumComparison { .. } => KaniFormFamily::Enum,
        Op::QuantityConversion
        | Op::CarriedQuantityConversion { .. }
        | Op::QuantityArithmetic { .. }
        | Op::QuantityComparison { .. } => KaniFormFamily::Quantity,
    }
}

fn checked_operation<'a>(
    package: &'a filament_contract_model::CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Option<&'a filament_contract_model::CheckedApplicationOperation> {
    match package.term(node_id)? {
        CheckedTerm::Application { operation, .. } => Some(operation),
        _ => None,
    }
}

fn checked_operation_identity<'a>(
    package: &'a filament_contract_model::CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Option<&'a str> {
    checked_operation(package, node_id)?.identity()
}

fn checked_state_clause_role<'a>(
    package: &'a filament_contract_model::CheckedPackageV2,
    node_id: &CheckedNodeId,
) -> Option<&'a str> {
    Some(
        checked_operation(package, node_id)?
            .state_clause_kind()
            .ok()??
            .as_wire(),
    )
}

fn checked_node_family<'a>(
    package: &'a filament_contract_model::CheckedPackageV2,
    node_id: &CheckedNodeId,
    lookups: &mut CapabilityLookups<'a>,
) -> KaniFormFamily {
    if let Some(family) = lookups.scalar_family(package, node_id) {
        return family;
    }
    let identity = lookups
        .checked_node(package, node_id)
        .and_then(|_| checked_operation_identity(package, node_id));
    match identity {
        Some(
            "filament.op.boolean.and"
            | "filament.op.boolean.or"
            | "filament.op.boolean.not"
            | "filament.op.boolean.implies"
            | "filament.op.boolean.eq"
            | "filament.op.boolean.ne",
        ) => KaniFormFamily::Boolean,
        Some("filament.op.equality.equal" | "filament.op.equality.not_equal") => {
            KaniFormFamily::CompositeEquality
        }
        Some("filament.op.function.apply" | "filament.op.function.call") => {
            KaniFormFamily::FunctionApplication
        }
        Some(
            "filament.op.collection.for_all_non_negative" | "filament.op.collection.exists_equal",
        ) => KaniFormFamily::BoundedCollection,
        Some("filament.op.graph.reachability") => KaniFormFamily::BoundedGraph,
        _ => KaniFormFamily::OutsideSet,
    }
}

fn checked_function_call_matches(
    package: &filament_contract_model::CheckedPackageV2,
    graph: &crate::oracle::function::FunctionPreflightGraph<'_>,
    function_node_id: &filament_contract_model::CheckedNodeId,
    call_node_id: &filament_contract_model::CheckedNodeId,
    argument_node_ids: &[filament_contract_model::CheckedNodeId],
) -> bool {
    let Some(function) = graph.get(function_node_id) else {
        return false;
    };
    if &*function.node_tag != "function" || &*function.semantic_form != "pure_function" {
        return false;
    }
    let Some(CheckedTerm::Aggregate {
        members: function_members,
    }) = package.term(function_node_id)
    else {
        return false;
    };
    let binding = |name: &str| {
        function_members.iter().find_map(|member| match member {
            CheckedTerm::Binding {
                name: actual,
                value,
            } if actual.as_ref() == name => Some(value.as_ref()),
            _ => None,
        })
    };
    let Some(CheckedTerm::Aggregate {
        members: parameters,
    }) = binding("parameters")
    else {
        return false;
    };
    let valid_parameters = parameters.iter().all(|term| {
        resolve_checked_reference(graph, term)
            .is_some_and(|node| &*node.node_tag == "value" && &*node.semantic_form == "parameter")
    });
    let valid_body = binding("body")
        .and_then(|term| resolve_checked_reference(graph, term))
        .is_some_and(|node| &*node.node_tag == "expression");
    if !valid_parameters || !valid_body {
        return false;
    }
    let Some(call) = graph.get(call_node_id) else {
        return false;
    };
    if &*call.node_tag != "expression" || &*call.semantic_form != "call" {
        return false;
    }
    let Some(CheckedTerm::Application {
        operator: ApplicationOperator::Call,
        operation,
        arguments,
        ..
    }) = package.term(call_node_id)
    else {
        return false;
    };
    if operation.identity() != Some("filament.op.function.call") {
        return false;
    }
    if arguments.len() != argument_node_ids.len() + 1 || parameters.len() != argument_node_ids.len()
    {
        return false;
    }
    let expected = std::iter::once(function_node_id).chain(argument_node_ids.iter());
    arguments.iter().zip(expected).all(|(term, expected)| {
        let CheckedTerm::Reference { target } = term else {
            return false;
        };
        let actual = graph.get(target);
        actual.is_some_and(|node| node.node_id == *expected)
    })
}

fn resolve_checked_reference<'a>(
    graph: &crate::oracle::function::FunctionPreflightGraph<'a>,
    term: &CheckedTerm,
) -> Option<&'a filament_contract_model::CheckedSemanticNodeV2> {
    let CheckedTerm::Reference { target } = term else {
        return None;
    };
    graph.get(target)
}

impl KaniForm {
    fn identity<'a>(&'a self, lookups: &mut CapabilityLookups<'a>) -> KaniFormIdentity {
        match self {
            Self::V2ScalarClaim {
                package, node_id, ..
            } => {
                let operation_identity = lookups
                    .checked_node(package, node_id)
                    .and_then(|_| checked_operation_identity(package, node_id))
                    .map(str::to_owned);
                let state_clause_role = lookups
                    .checked_node(package, node_id)
                    .filter(|node| {
                        &*node.node_tag == "state" && &*node.semantic_form == "state_clause"
                    })
                    .and_then(|_| checked_state_clause_role(package, node_id));
                let identity_family = checked_node_family(package, node_id, lookups);
                KaniFormIdentity {
                    family: if state_clause_role.is_some() {
                        KaniFormFamily::StateFrame
                    } else {
                        identity_family
                    },
                    form: KaniFormName::V2ScalarClaim,
                    operation_identity,
                    role: state_clause_role.and_then(obligation_role),
                    state_frame_role: None,
                }
            }
            Self::V2StateFrame {
                package,
                clause,
                role,
                ..
            } => KaniFormIdentity {
                family: KaniFormFamily::StateFrame,
                form: KaniFormName::V2StateFrame,
                operation_identity: lookups
                    .checked_node(package, clause)
                    .and_then(|_| checked_operation_identity(package, clause))
                    .map(str::to_owned),
                role: Some(*role),
                state_frame_role: Some(match role {
                    KaniObligationRole::Frame => KaniStateFrameRole::Frame,
                    KaniObligationRole::Precondition
                    | KaniObligationRole::Postcondition
                    | KaniObligationRole::Invariant => KaniStateFrameRole::Contract,
                }),
            },
            Self::V2FunctionApplication {
                package,
                function_node_id: _,
                call_node_id,
                ..
            } => KaniFormIdentity {
                family: KaniFormFamily::FunctionApplication,
                form: KaniFormName::V2FunctionApplication,
                operation_identity: lookups
                    .function_graph(package)
                    .get(call_node_id)
                    .and_then(|_| checked_operation_identity(package, call_node_id))
                    .map(str::to_owned),
                role: None,
                state_frame_role: None,
            },
            Self::OutsideSet { package, node_id } => KaniFormIdentity {
                family: KaniFormFamily::OutsideSet,
                form: KaniFormName::OutsideSet,
                operation_identity: lookups
                    .checked_node(package, node_id)
                    .map(|node| node.semantic_form.to_string()),
                role: None,
                state_frame_role: None,
            },
        }
    }

    fn preflight<'a>(
        &'a self,
        backend: &BackendDescriptor,
        admitted: Option<(&ContractPackageV1, &[ContractRequestPosition], usize)>,
        lookups: &mut CapabilityLookups<'a>,
    ) -> Result<KaniFormPreflightOutcome, Box<Cause>> {
        #[cfg(test)]
        if let Some((_, _, request_index)) = admitted {
            PACKAGE_CLASSIFIER_INDICES.with(|indices| indices.borrow_mut().push(request_index));
        }
        let identity = self.identity(lookups);
        match self {
            Self::V2ScalarClaim {
                package,
                contract,
                positions,
                request_index,
                node_id,
            } => {
                use crate::kani::generate::negotiate::ScalarFormAssessment;
                match crate::kani::generate::negotiate::classify_scalar_form(
                    package,
                    contract,
                    positions,
                    *request_index,
                    node_id,
                    &mut lookups.scalar,
                ) {
                    ScalarFormAssessment::Supported => Ok(KaniFormPreflightOutcome::Ready),
                    ScalarFormAssessment::RequiresBound => {
                        Ok(KaniFormPreflightOutcome::RequiresBound)
                    }
                    ScalarFormAssessment::Unsupported(refusal) => {
                        Err(Box::new(Cause::KaniFormRefused {
                            form: identity,
                            refusal,
                            backend: backend.identity.clone(),
                        }))
                    }
                    ScalarFormAssessment::Invalid(refusal) => {
                        Err(Box::new(Cause::KaniFormInvalid {
                            form: identity,
                            refusal,
                            backend: backend.identity.clone(),
                        }))
                    }
                }
            }
            Self::V2StateFrame {
                package,
                clause,
                role,
                state_path,
                state_fields,
                subject_path,
            } => {
                use crate::kani::generate::outcome::StateFrameRole;
                let actual_clause = lookups
                    .checked_node(package, clause)
                    .and_then(|_| checked_state_clause_role(package, clause));
                let role = match role {
                    KaniObligationRole::Postcondition => StateFrameRole::Contract,
                    KaniObligationRole::Frame => StateFrameRole::Frame,
                    KaniObligationRole::Precondition | KaniObligationRole::Invariant => {
                        let expected = if *role == KaniObligationRole::Precondition {
                            "precondition"
                        } else {
                            "invariant"
                        };
                        if actual_clause != Some(expected) {
                            return Err(Box::new(Cause::KaniFormInvalid {
                                form: identity,
                                refusal: crate::kani::generate::outcome::InvalidObligationItem::UnknownNode,
                                backend: backend.identity.clone(),
                            }));
                        }
                        StateFrameRole::Contract
                    }
                };
                let state_fields = state_fields.iter().map(String::as_str).collect::<Vec<_>>();
                let request = crate::kani::generate::frame::StateFrameRequest {
                    ceilings: crate::kani::generate::frame::default_settlement_ceilings(),
                    package,
                    clause,
                    state_path,
                    state_fields: &state_fields,
                    subject_path,
                    unwind: 4,
                };
                match crate::kani::generate::frame::classify_state_frame_role(&request, role) {
                    Ok(()) => Ok(KaniFormPreflightOutcome::Ready),
                    Err(refusal) => Err(Box::new(Cause::KaniStateFrameRefused {
                        form: identity,
                        refusal,
                        backend: backend.identity.clone(),
                    })),
                }
            }
            Self::V2FunctionApplication {
                package,
                function_node_id,
                call_node_id,
                argument_node_ids,
            } => {
                let graph = lookups.function_graph(package);
                let source_matches = checked_function_call_matches(
                    package,
                    graph,
                    function_node_id,
                    call_node_id,
                    argument_node_ids,
                );
                if !source_matches {
                    return Err(Box::new(Cause::KaniFormInvalid {
                        form: identity,
                        refusal: crate::kani::generate::outcome::InvalidObligationItem::UnknownNode,
                        backend: backend.identity.clone(),
                    }));
                }
                let supported =
                    match crate::oracle::function::checked_integer_function_application_is_supported_with_graph(
                        package,
                        graph,
                        function_node_id,
                        call_node_id,
                        argument_node_ids,
                        admitted,
                    ) {
                        Ok(supported) => supported,
                        Err(crate::oracle::function::FunctionPackageReadFailure::CatalogUnavailable(cause)) => {
                            return Ok(KaniFormPreflightOutcome::EnvelopeRefusal(
                                EnvelopeRefusal::ContractPackageCatalogUnavailable {
                                    path: cause.path().to_owned(),
                                    message: cause.to_string(),
                                },
                            ));
                        }
                        Err(crate::oracle::function::FunctionPackageReadFailure::Refused(
                            refusal,
                        )) => {
                            return Ok(KaniFormPreflightOutcome::EnvelopeRefusal(
                                EnvelopeRefusal::ContractPackageReadRefused {
                                    refusal: project_contract_package_refusal(&refusal),
                                },
                            ));
                        }
                        Err(crate::oracle::function::FunctionPackageReadFailure::Incomplete(
                            incomplete,
                        )) => {
                            return Ok(KaniFormPreflightOutcome::EnvelopeRefusal(
                                EnvelopeRefusal::ContractPackageReadIncomplete {
                                    incomplete: project_contract_package_incomplete(&incomplete),
                                },
                            ));
                        }
                    };
                if supported {
                    Ok(KaniFormPreflightOutcome::Ready)
                } else {
                    Err(Box::new(Cause::KaniFormRefused {
                        form: identity,
                        refusal: crate::kani::generate::outcome::UnsupportedObligation::OperationNotRendered {
                            operation_identity: "filament.op.function.call".to_owned(),
                            derived_domains: Vec::new(),
                        },
                        backend: backend.identity.clone(),
                    }))
                }
            }
            Self::OutsideSet { package, node_id } => {
                let node = lookups.checked_node(package, node_id);
                let Some(node) = node else {
                    return Err(Box::new(Cause::KaniFormInvalid {
                        form: identity,
                        refusal: crate::kani::generate::outcome::InvalidObligationItem::UnknownNode,
                        backend: backend.identity.clone(),
                    }));
                };
                if lookups.scalar_family(package, node_id).is_some()
                    || &*node.semantic_form == "state_clause"
                {
                    Err(Box::new(Cause::KaniFormInvalid {
                        form: identity,
                        refusal: crate::kani::generate::outcome::InvalidObligationItem::UnknownNode,
                        backend: backend.identity.clone(),
                    }))
                } else {
                    Err(Box::new(Cause::UnsupportedKaniForm {
                        form: identity,
                        backend: backend.identity.clone(),
                    }))
                }
            }
        }
    }
}

#[derive(Clone)]
// FR-019 fixes the projection payload as a value in the public refusal.
#[allow(clippy::large_enum_variant)]
enum KaniFormPreflightOutcome {
    Ready,
    RequiresBound,
    EnvelopeRefusal(EnvelopeRefusal),
}

/// One request item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestItem {
    /// The one FR-290 kind this item requires.
    pub kind: RequestedKind,
    /// The item's extent classification, absent when the request carries none.
    pub extent: Option<ExtentClassification>,
    /// The backend the caller names, when it names one.
    pub named_backend: Option<String>,
    /// The candidates the registry computed for this item.
    pub candidates: Candidates,
    /// The typed IR form a Kani candidate's arm reads, absent when the item carries none.
    pub kani_form: Option<KaniForm>,
    /// The requirements the generic arm compares with the candidate's manifest entry.
    pub requirements: ItemRequirements,
}

/// The FR-331 backend provider envelope, as far as settlement reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendProviderEnvelope {
    /// Exactly [`BACKEND_PROVIDER_CONTRACT`].
    pub contract_version: String,
    /// Exactly [`CAPABILITY_VOCABULARY`].
    pub capability_vocabulary: String,
    /// One descriptor per backend identity in a valid FR-331 registry snapshot.
    /// A caller-supplied repeat violates the CG input precondition; FR-019-AC-45
    /// requires whole-envelope refusal even when both descriptors are identical.
    pub manifest: Vec<BackendDescriptor>,
    /// The request's items, in request order.
    pub items: Vec<RequestItem>,
}

/// A refusal of the whole carrier, before any item is settled.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "code", content = "cause")]
pub enum EnvelopeRefusal {
    /// The owner operation catalog could not be read; this is not a package defect.
    ContractPackageCatalogUnavailable {
        /// The catalog member that failed.
        path: String,
        /// The owner diagnostic.
        message: String,
    },
    /// The contract version or capability vocabulary is invalid.
    ///
    /// A foreign vocabulary makes every label in the carrier unreadable.
    InvalidCapability {
        /// Both carrier members refuse with `unsupported-version`.
        cause: &'static str,
        /// Which member of the carrier was wrong.
        ///
        /// Both members refuse with one cause, so without this a consumer that
        /// got both wrong is told only about the first and cannot tell which
        /// field to fix from the refusal it was handed.
        member: &'static str,
        /// The supplied carrier identity.
        received: String,
    },
    /// The manifest repeats a backend identity, so no item settles.
    ManifestRepeatsBackendIdentity {
        /// The bytewise-first identity repeated in the manifest.
        identity: String,
    },
    /// The function preflight's internal Contract IR read refused the package.
    ContractPackageReadRefused {
        /// The reader's public diagnostic fields.
        refusal: ContractPackageRefusalProjection,
    },
    /// The function preflight's internal Contract IR read exhausted a limit.
    ContractPackageReadIncomplete {
        /// The reader's public limit fields.
        incomplete: ContractPackageIncompleteProjection,
    },
}

/// Serializable CG view of the public fields of a Contract IR read refusal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContractPackageRefusalProjection {
    /// Stable reader refusal code.
    pub code: CheckedPackageRefusalCode,
    /// Nonempty reader locations, in declaration order.
    pub loci: Vec<CheckedPackageRefusalLocation>,
    /// Typed reason paired with the code, when available.
    pub cause: Option<CheckedPackageRefusalCause>,
    /// Offending semantic node, when available.
    pub locus: Option<CheckedNodeId>,
    /// Received version for a version refusal.
    pub contract_version: Option<String>,
    /// Pointer into a selected document, when relevant.
    pub document_pointer: Option<String>,
    /// Extension clause details retained by the reader.
    pub extension_detail: Option<Box<CheckedExtensionRefusalDetail>>,
    /// Protocol event, type or interval context retained by the reader.
    pub protocol_detail: Option<Box<CheckedProtocolRefusalDetail>>,
}

/// Copy the reader's public refusal fields into CG's wire projection.
#[must_use]
pub fn project_contract_package_refusal(
    refusal: &CheckedPackageRefusal,
) -> ContractPackageRefusalProjection {
    ContractPackageRefusalProjection {
        code: refusal.code,
        loci: refusal.loci.clone(),
        cause: refusal.cause,
        locus: refusal.locus.clone(),
        contract_version: refusal.contract_version.as_deref().map(str::to_owned),
        document_pointer: refusal
            .document_pointer
            .as_ref()
            .map(|path| path.as_str().to_owned()),
        extension_detail: refusal.extension_detail.clone(),
        protocol_detail: refusal.protocol_detail.clone(),
    }
}

/// Serializable CG view of the public fields of an incomplete Contract IR read.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContractPackageIncompleteProjection {
    /// The first exhausted resource.
    pub limit_kind: CheckedPackageLimit,
    /// Caller-selected ceiling.
    pub limit: u64,
    /// Counter value at the failed charge.
    pub consumed: u64,
    /// Pointer of the failed charge, absent for a byte ceiling.
    pub path: Option<String>,
}

/// Copy the reader's limit outcome into CG's wire projection.
#[must_use]
pub fn project_contract_package_incomplete(
    incomplete: &CheckedPackageIncomplete,
) -> ContractPackageIncompleteProjection {
    ContractPackageIncompleteProjection {
        limit_kind: incomplete.limit_kind,
        limit: incomplete.limit,
        consumed: incomplete.consumed,
        path: incomplete
            .path
            .as_ref()
            .map(|path| path.as_str().to_owned()),
    }
}

/// One item's settlement, in request order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemSettlement {
    /// The item's index in the request.
    pub request_index: usize,
    /// The kind that settled, absent when the item named none this module reads.
    pub kind: Option<CapabilityKind>,
    /// The disposition.
    pub disposition: Disposition,
}

impl ItemSettlement {
    /// The warning an `unsupported` settlement carries, if any.
    ///
    /// `None` means this settlement has no warning to name: it is not
    /// `unsupported`, or its cause is not an `unsupported_projection` one. CG
    /// never builds an `unsupported` settlement with an `invalid_capability`
    /// cause, so FR-019's rule that an `unsupported` settlement carries a
    /// warning holds for every settlement CG builds; a caller that assembles
    /// one by hand gets `None`.
    ///
    /// FR-290 requires the warning to name the item's kind, and the named
    /// backend when the request names one. It is derived from the cause rather
    /// than stored beside it, so the two cannot disagree.
    #[must_use]
    pub fn warning(&self) -> Option<String> {
        match &self.disposition {
            Disposition::Unsupported { cause } => match cause {
                Cause::UnsupportedRequestedCapability { kind, backend } => Some(match backend {
                    Some(backend) => format!(
                        "no registered backend advertises {} for named backend {backend}",
                        kind.label()
                    ),
                    None => format!("no registered backend advertises {}", kind.label()),
                }),
                Cause::UnboundedExtent { kind, backend } => Some(format!(
                    "{backend} advertises {} bounded only and no finite bound is available",
                    kind.label()
                )),
                Cause::UnsupportedDomain {
                    kind,
                    domain,
                    backend,
                } => Some(format!(
                    "{backend} does not advertise domain {} for {}",
                    domain.to_wire(),
                    kind.label()
                )),
                Cause::UnsupportedRequirement {
                    kind,
                    requirement,
                    backend,
                } => Some(format!(
                    "{backend} does not advertise {} for {}",
                    requirement.describe(),
                    kind.label()
                )),
                Cause::UnsupportedKaniForm { form, backend } => Some(format!(
                    "{backend} does not support Kani form {}",
                    form.form.label()
                )),
                Cause::KaniFormRefused {
                    form,
                    refusal,
                    backend,
                } => Some(format!(
                    "{backend} refused Kani form {}: {refusal:?}",
                    form.form.label()
                )),
                Cause::KaniFormInvalid { .. } => None,
                Cause::KaniStateFrameRefused {
                    form,
                    refusal,
                    backend,
                } => Some(format!(
                    "{backend} refused Kani state-frame form {}: {refusal}",
                    form.form.label()
                )),
                // Exhaustive on purpose. A catch-all here would give the next
                // cause a warning that names neither the kind nor the backend
                // FR-290 requires it to name, with nothing failing to say so.
                // An `invalid_capability` cause settles invalid-request, so this
                // generator never builds it under `Unsupported`; the fields are
                // public, so a caller can, and this settlement has no warning.
                Cause::AbsentKind
                | Cause::UnknownKind { .. }
                | Cause::AbsentExtent
                | Cause::AbsentEvidenceKind
                | Cause::UnknownEvidenceKind { .. }
                | Cause::UnknownBackend { .. }
                | Cause::InconsistentCandidates { .. }
                | Cause::AmbiguousBackend { .. } => None,
            },
            Disposition::Supported { .. }
            | Disposition::RequiresBound { .. }
            | Disposition::InvalidRequest { .. } => None,
        }
    }
}

/// The closed set of backend kinds this generator settles for.
///
/// One arm per variant, no catch-all. Adding a variant without an arm is a
/// compile error at the dispatch, which is the whole point of the kind
/// being closed: an open set of hand-written negotiate functions gives the same
/// behaviour at run time and none of the enforcement.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    /// The Kani backend descriptor, implemented here and in `filament-contract-ir`.
    Kani,
    /// One process provider, identified by its exact registry identity.
    Process(BackendId),
}

/// A CG-owned backend identity, independent of FSL's Rust identity type.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BackendId(pub String);

impl BackendKind {
    /// The finite built-in kinds; process identities come from descriptors.
    pub const ALL: [Self; 1] = [Self::Kani];

    /// Category sort key: built-in Kani precedes process identities.
    #[must_use]
    pub const fn index(&self) -> usize {
        match self {
            Self::Kani => 0,
            Self::Process(_) => 1,
        }
    }

    /// The backend identity this kind registers under.
    #[must_use]
    pub fn identity(&self) -> &str {
        match self {
            Self::Kani => "kani",
            Self::Process(id) => &id.0,
        }
    }

    /// The built-in kind a linked identity names, if any.
    ///
    /// Generation dispatches on this; negotiation does not, because every
    /// admitted registration is settled by the generic arm.
    #[must_use]
    pub fn from_identity(identity: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.identity() == identity)
    }

    /// Classify a descriptor by its registry origin before reading its identity.
    #[must_use]
    pub fn from_descriptor(descriptor: &BackendDescriptor) -> Option<Self> {
        match descriptor.origin {
            ProviderOrigin::Linked => Self::from_identity(&descriptor.identity),
            ProviderOrigin::Process => Some(Self::Process(BackendId(descriptor.identity.clone()))),
        }
    }
}

/// Settle every item of an envelope.
///
/// Trace: TC-030
// Implements: FR-019
// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
pub fn negotiate_backend_provider(
    envelope: &BackendProviderEnvelope,
) -> Result<Vec<ItemSettlement>, EnvelopeRefusal> {
    negotiate_backend_provider_with_package(envelope, None)
}

// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
fn negotiate_backend_provider_with_package(
    envelope: &BackendProviderEnvelope,
    admitted: Option<(&ContractPackageV1, &[ContractRequestPosition], &[usize])>,
) -> Result<Vec<ItemSettlement>, EnvelopeRefusal> {
    validate_backend_provider_envelope(envelope)?;
    let mut lookups = CapabilityLookups::new(&envelope.items);
    let request_indices = match admitted {
        Some((_, _, indices)) => indices.to_vec(),
        None => (0..envelope.items.len()).collect(),
    };
    envelope
        .items
        .iter()
        .zip(request_indices)
        .map(|(item, request_index)| {
            Ok(ItemSettlement {
                request_index,
                kind: match item.kind {
                    RequestedKind::Known(kind) => Some(kind),
                    RequestedKind::Absent | RequestedKind::Unknown(_) => None,
                },
                disposition: negotiate_item_in(
                    &envelope.manifest,
                    item,
                    admitted.map(|(package, positions, _)| (package, positions, request_index)),
                    &mut lookups,
                )?,
            })
        })
        .collect()
}

// FR-019 fixes the typed projection payload by value in this refusal.
#[allow(clippy::result_large_err)]
fn validate_backend_provider_envelope(
    envelope: &BackendProviderEnvelope,
) -> Result<(), EnvelopeRefusal> {
    if envelope.contract_version != BACKEND_PROVIDER_CONTRACT {
        return Err(EnvelopeRefusal::InvalidCapability {
            cause: "unsupported-version",
            member: "contract_version",
            received: envelope.contract_version.clone(),
        });
    }
    if envelope.capability_vocabulary != CAPABILITY_VOCABULARY {
        return Err(EnvelopeRefusal::InvalidCapability {
            cause: "unsupported-version",
            member: "capability_vocabulary",
            received: envelope.capability_vocabulary.clone(),
        });
    }
    let mut identities = BTreeSet::new();
    if let Some(identity) = envelope
        .manifest
        .iter()
        .map(|backend| backend.identity.as_str())
        .filter(|identity| !identities.insert(*identity))
        .min()
    {
        return Err(EnvelopeRefusal::ManifestRepeatsBackendIdentity {
            identity: identity.to_owned(),
        });
    }
    Ok(())
}

/// The decision for one exact FR-330 request position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractItemSettlement {
    /// Original driver index; excluded positions count and context roots do not.
    pub request_index: usize,
    /// Exact requested key, including repeated positions.
    pub node_id: CheckedNodeId,
    /// The exact admitted accounting record and source occurrences.
    pub admitted: AdmittedItemAccounting,
    /// The backend settlement after `Lowered`; absent for an IR-refused item.
    pub backend: Option<ItemSettlement>,
}

/// A whole-request mismatch before any item can be settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractNegotiationError {
    /// The FR-331 envelope is invalid.
    Envelope(EnvelopeRefusal),
    /// Request positions and exact keys disagree with the admitted package.
    Package(PackageRequestError),
    /// The FR-331 envelope and the bound request positions have different counts.
    EnvelopeItemCount {
        /// Number of bound driver positions.
        requested: usize,
        /// Number of FR-331 item records supplied.
        supplied: usize,
    },
}

/// Settle every request position from one admitted FR-330 package.
///
/// FR-019 still owns backend negotiation for a lowered item. A non-lowered
/// item retains its exact FR-330 record and cannot become backend-supported.
// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
pub fn negotiate_contract_package(
    source: &filament_contract_model::CheckedPackageV2,
    package: &ContractPackageV1,
    positions: &[ContractRequestPosition],
    envelope: &BackendProviderEnvelope,
) -> Result<Vec<ContractItemSettlement>, ContractNegotiationError> {
    let items =
        PackageItems::new(source, package, positions).map_err(ContractNegotiationError::Package)?;
    validate_backend_provider_envelope(envelope).map_err(ContractNegotiationError::Envelope)?;
    if envelope.items.len() != items.bound_count() {
        return Err(ContractNegotiationError::EnvelopeItemCount {
            requested: items.bound_count(),
            supplied: envelope.items.len(),
        });
    }
    let backend = negotiate_backend_provider_with_package(
        envelope,
        Some((package, positions, items.bound_driver_indices())),
    )
    .map_err(ContractNegotiationError::Envelope)?;
    backend
        .into_iter()
        .enumerate()
        .map(|(envelope_index, settlement)| {
            let request_index = items.bound_driver_index(envelope_index).ok_or(
                ContractNegotiationError::Package(PackageRequestError::Position {
                    request_index: envelope_index,
                }),
            )?;
            let node_id =
                items
                    .driver_node(request_index)
                    .ok_or(ContractNegotiationError::Package(
                        PackageRequestError::Position { request_index },
                    ))?;
            let item = items
                .at(request_index, node_id)
                .map_err(ContractNegotiationError::Package)?;
            let admitted = item.accounting();
            let backend = match item {
                PackageItem::Lowered { .. } => Some(settlement),
                PackageItem::Refused { .. } => None,
            };
            Ok(ContractItemSettlement {
                request_index,
                node_id: node_id.clone(),
                admitted,
                backend,
            })
        })
        .collect()
}

/// Settle one request item over the request's manifest (FSpec FR-711).
///
/// The disposition is a function of the manifest and the item alone: the same
/// item over the same manifest settles the same whether its candidate is a
/// linked engine, a process provider or a Kani backend, except for what a
/// backend-specific arm adds. The item needs no backend-specific form unless
/// its single candidate's arm reads one. The manifest must hold unique
/// identities, as `negotiate_backend_provider` requires of an envelope.
///
/// Trace: TC-1741
// Implements: FR-711
// The refusal is boxed: it carries FR-019's typed projection by value.
pub fn negotiate_item(
    manifest: &[BackendDescriptor],
    item: &RequestItem,
) -> Result<Disposition, Box<EnvelopeRefusal>> {
    let mut lookups = CapabilityLookups::new(std::slice::from_ref(item));
    negotiate_item_in(manifest, item, None, &mut lookups).map_err(Box::new)
}

/// Settle one item, evaluating FR-290's rules in order; the first match settles.
// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
fn negotiate_item_in<'a>(
    manifest: &[BackendDescriptor],
    item: &'a RequestItem,
    admitted: Option<(&ContractPackageV1, &[ContractRequestPosition], usize)>,
    lookups: &mut CapabilityLookups<'a>,
) -> Result<Disposition, EnvelopeRefusal> {
    let kind = match &item.kind {
        RequestedKind::Absent => {
            return Ok(Disposition::InvalidRequest {
                cause: Cause::AbsentKind,
            })
        }
        RequestedKind::Unknown(received) => {
            return Ok(Disposition::InvalidRequest {
                cause: Cause::UnknownKind {
                    received: received.clone(),
                },
            })
        }
        RequestedKind::Known(kind) => *kind,
    };
    let Some(extent) = item.extent.as_ref() else {
        return Ok(Disposition::InvalidRequest {
            cause: Cause::AbsentExtent,
        });
    };
    let candidates = match &item.candidates {
        Candidates::UnknownBackend(backend) => {
            return Ok(Disposition::InvalidRequest {
                cause: Cause::UnknownBackend {
                    backend: backend.clone(),
                },
            })
        }
        Candidates::Set(candidates) => candidates.as_slice(),
    };
    if let Some(backend) = unroutable_named_backend(manifest, item.named_backend.as_deref()) {
        return Ok(Disposition::InvalidRequest {
            cause: Cause::UnknownBackend { backend },
        });
    }
    let offending = inconsistent_candidates(manifest, item, kind, candidates);
    if !offending.is_empty() {
        return Ok(Disposition::InvalidRequest {
            cause: Cause::InconsistentCandidates {
                candidates: offending,
            },
        });
    }
    match candidates {
        [] => Ok(Disposition::Unsupported {
            cause: Cause::UnsupportedRequestedCapability {
                kind,
                backend: item.named_backend.clone(),
            },
        }),
        [single] => {
            negotiate_single_candidate(manifest, single, kind, extent, item, admitted, lookups)
        }
        many if item.named_backend.is_none() => Ok(Disposition::InvalidRequest {
            cause: Cause::AmbiguousBackend {
                candidates: many.to_vec(),
            },
        }),
        // FR-290's candidate table has no row for "more than one candidate and
        // the request names one": a repeated candidate entry reaches this arm
        // even with a unique manifest. This fallback classifies the unmatched
        // many-candidate case as inconsistent candidates.
        many => Ok(Disposition::InvalidRequest {
            cause: Cause::InconsistentCandidates {
                candidates: many.to_vec(),
            },
        }),
    }
}

/// The named backend, when the manifest does not hold it.
///
/// FR-290's first candidate row pairs the unknown-backend mark with the
/// identity the request *named*, so the row is read as being about the named
/// backend, not about any member of the set: the alternative reading makes the
/// row swallow the ambiguous-backend row below it. Every admitted registration
/// has a negotiation arm (FSpec FR-711), so only an unregistered name reaches
/// this cause.
fn unroutable_named_backend(manifest: &[BackendDescriptor], named: Option<&str>) -> Option<String> {
    named
        .filter(|named| !manifest.iter().any(|backend| backend.identity == *named))
        .map(str::to_owned)
}

/// Every candidate absent from the manifest, not advertising the kind, or
/// disagreeing with the named backend, in candidate order.
fn inconsistent_candidates(
    manifest: &[BackendDescriptor],
    item: &RequestItem,
    kind: CapabilityKind,
    candidates: &[Candidate],
) -> Vec<Candidate> {
    candidates
        .iter()
        .filter(|candidate| {
            let registered = descriptor(manifest, candidate);
            let disagrees = item
                .named_backend
                .as_ref()
                .is_some_and(|named| *named != candidate.identity);
            disagrees || registered.is_none_or(|backend| !backend.advertises(kind))
        })
        .cloned()
        .collect()
}

fn descriptor<'a>(
    manifest: &'a [BackendDescriptor],
    candidate: &Candidate,
) -> Option<&'a BackendDescriptor> {
    manifest
        .iter()
        .find(|backend| backend.identity == candidate.identity)
}

/// The backend-specific arms that run after the generic arm passes.
///
/// One variant per arm, matched without a catch-all, so a new backend-specific
/// arm does not build until it is dispatched.
enum SpecificArm {
    /// The Kani arm reads a typed IR form.
    Kani,
    /// The backend has no check beyond the generic arm.
    None,
}

impl SpecificArm {
    fn of(backend: &BackendDescriptor) -> Self {
        match BackendKind::from_descriptor(backend) {
            Some(BackendKind::Kani) => Self::Kani,
            Some(BackendKind::Process(_)) | None => Self::None,
        }
    }
}

/// Settle the single candidate: the generic arm, then its backend-specific arm.
///
/// Every function that constructs a [`Disposition`] is named `negotiate_*`.
// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
fn negotiate_single_candidate<'a>(
    manifest: &[BackendDescriptor],
    candidate: &Candidate,
    kind: CapabilityKind,
    extent: &ExtentClassification,
    item: &'a RequestItem,
    admitted: Option<(&ContractPackageV1, &[ContractRequestPosition], usize)>,
    lookups: &mut CapabilityLookups<'a>,
) -> Result<Disposition, EnvelopeRefusal> {
    let Some(backend) = descriptor(manifest, candidate) else {
        return Ok(Disposition::InvalidRequest {
            cause: Cause::InconsistentCandidates {
                candidates: vec![candidate.clone()],
            },
        });
    };
    let generic = negotiate_generic(backend, kind, extent, &item.requirements);
    if !matches!(generic, Disposition::Supported { .. }) {
        return Ok(generic);
    }
    match SpecificArm::of(backend) {
        SpecificArm::Kani => {
            negotiate_kani(backend, generic, item.kani_form.as_ref(), admitted, lookups)
        }
        SpecificArm::None => Ok(generic),
    }
}

/// The Kani arm, run after the generic arm passed (`generic`).
///
/// It reads the typed IR form. An item whose Kani candidate needs the form
/// and lacks it is `invalid-request` naming the candidate. The arm adds a
/// refusal of the form or a requirement for a bound; otherwise the generic
/// disposition stands.
// FR-019 requires the typed projection by value in the public refusal.
#[allow(clippy::result_large_err)]
fn negotiate_kani<'a>(
    backend: &BackendDescriptor,
    generic: Disposition,
    kani_form: Option<&'a KaniForm>,
    admitted: Option<(&ContractPackageV1, &[ContractRequestPosition], usize)>,
    lookups: &mut CapabilityLookups<'a>,
) -> Result<Disposition, EnvelopeRefusal> {
    let Some(kani_form) = kani_form else {
        return Ok(Disposition::InvalidRequest {
            cause: Cause::InconsistentCandidates {
                candidates: vec![Candidate {
                    identity: backend.identity.clone(),
                }],
            },
        });
    };
    let readiness = match kani_form.preflight(backend, admitted, lookups) {
        Ok(readiness) => readiness,
        Err(cause) => match *cause {
            cause @ Cause::KaniFormInvalid { .. } => {
                return Ok(Disposition::InvalidRequest { cause })
            }
            cause => return Ok(Disposition::Unsupported { cause }),
        },
    };
    match readiness {
        KaniFormPreflightOutcome::EnvelopeRefusal(refusal) => Err(refusal),
        KaniFormPreflightOutcome::RequiresBound => Ok(Disposition::RequiresBound {
            backend: backend.identity.clone(),
        }),
        KaniFormPreflightOutcome::Ready => Ok(generic),
    }
}
