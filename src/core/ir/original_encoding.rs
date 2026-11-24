// SPDX-License-Identifier: AGPL-3.0-or-later
//! The canonical bytes of an admitted checked package, as FSL's admitted closure carries them.

use filament_contract_model::{
    encode_checked_package_v2_with_path_condition_frames, CheckedPackageSidecarsRef,
    CheckedPackageSourceCorrespondenceRef, CheckedPackageV2, ProtocolCorrelationProvenance,
};

/// The package could not be re-encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EncodingFailed;

/// Re-encode `original` with every sidecar its admitted closure carries: the checked-term data,
/// claim names, temporal rows, `requirement_items`, the protocol correlation rows and the shared path-condition frames. Leaving one
/// out makes the bytes differ from the owner's whenever the package has such records.
pub(crate) fn encode_original_package_bytes(
    original: &CheckedPackageV2,
) -> Result<Vec<u8>, EncodingFailed> {
    let correlation_rows = match original.protocol_correlation_provenance() {
        ProtocolCorrelationProvenance::Unavailable(_) => None,
        ProtocolCorrelationProvenance::Complete(table) => Some(
            table
                .groups()
                .flat_map(|group| group.clauses().iter().cloned())
                .collect::<Vec<_>>(),
        ),
    };
    let requirement_items = original.requirement_items();
    let path_condition_frames = original.path_condition_frames();
    encode_checked_package_v2_with_path_condition_frames(
        original.name(),
        original.lock(),
        original.graph(),
        CheckedPackageSourceCorrespondenceRef {
            source_map: original.source_map(),
            refinement_step_rows: original.refinement_step_rows(),
        },
        CheckedPackageSidecarsRef {
            checked_term_data: original.checked_term_data(),
            claim_names: original.claim_names(),
            temporal_bindings: original.temporal_bindings(),
            temporal_clause_names: original.temporal_clause_names(),
            schedulability_claims: original.schedulability_claim_rows(),
            operation_effects: original.operation_effect_rows(),
            operation_bodies: original.operation_body_rows(),
        },
        (!requirement_items.is_empty()).then_some(requirement_items),
        correlation_rows.as_deref(),
        (!path_condition_frames.is_empty()).then_some(path_condition_frames),
        original.capability_report(),
        original.diagnostics(),
        quire_canonical::Limits::new(u64::MAX),
    )
    .map_err(|_| EncodingFailed)
}
