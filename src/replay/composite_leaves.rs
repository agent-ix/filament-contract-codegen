//! Decode a Kani playback of a composite shadow harness through the identity's leaf bindings and
//! rebuild the operands it drew (FR-025-AC-9, FR-033-AC-2 to FR-033-AC-5).
//!
//! The playback holds one untyped entry per `kani::any()` call the counterexample executed, in
//! call order. A leaf drawn under a condition (the payload of a present option, the value of a
//! present optional slot) is not drawn when the condition fails, so the decoder walks the
//! identity's leaves, evaluates each leaf's condition over the values already read, and reads an
//! entry only for a leaf that is drawn. The identity's leaf bindings are checked first; no
//! playback is read through a schema that fails the check.

use fsl_replay::WitnessValue;

use crate::{
    kani::{
        identity::{
            leaf_binding::{
                check_leaf_bindings, expected_leaves, leaves_of, rebuild_operands,
                LeafBindingRefusal,
            },
            shadow::{LeafKind, ShadowIdentity},
        },
        output::playback::{Playback, WitnessDecodeLimits},
    },
    replay::witness::{check_bytes, check_values_and_harness, read_entry, DecodeFailure},
};

/// The leaf values one playback drew, per parameter node, in the identity's leaf order. A leaf
/// whose condition failed is `None`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedCompositeLeaves {
    parameters: Vec<(String, Vec<Option<i64>>)>,
}

impl DecodedCompositeLeaves {
    /// The values drawn for the parameter at `node`, one per leaf of its shape.
    #[must_use]
    pub fn values_of(&self, node: &str) -> Option<&[Option<i64>]> {
        self.parameters
            .iter()
            .find(|(found, _)| found == node)
            .map(|(_, values)| values.as_slice())
    }
}

/// Whether a guard leaf, read as `value`, lets the leaves under it be drawn.
fn admits(kind: LeafKind, value: Option<i64>) -> bool {
    matches!(
        (kind, value),
        (LeafKind::Presence, Some(1)) | (LeafKind::SlotState, Some(0))
    )
}

fn binding_failure(identity: &ShadowIdentity, refusal: &LeafBindingRefusal) -> DecodeFailure {
    DecodeFailure::new(
        refusal.code(),
        &identity.harness_symbol,
        &format!("{refusal:?}"),
    )
}

/// The nodes of the identity's drawn parameters, in the order their leaves are drawn.
fn draw_order(identity: &ShadowIdentity) -> Vec<&str> {
    let mut nodes: Vec<&str> = Vec::new();
    for leaf in &identity.leaves {
        if !nodes.contains(&leaf.node.as_str()) {
            nodes.push(&leaf.node);
        }
    }
    nodes
}

/// Types the playback of a composite shadow harness with the identity's leaf bindings.
///
/// # Errors
///
/// A [`DecodeFailure`] with the code of the failed leaf-binding check (`cg_composite_leaf_*`);
/// `cg_witness_decode_bytes_limit` or `cg_witness_decode_values_limit` beyond the limits;
/// `cg_witness_harness_identity_mismatch` for another harness's playback;
/// `kani_witness_arity_mismatch` for a missing or surplus entry, with the leaf's name for a
/// missing one; `kani_witness_width_mismatch`, `kani_witness_boolean_byte_invalid` and
/// `kani_witness_comment_mismatch` for an entry that is not its leaf's primitive; and
/// `cg_witness_out_of_domain` for a value outside its leaf's bound.
pub fn decode_composite_falsification(
    identity: &ShadowIdentity,
    playback: &Playback<'_>,
    limits: WitnessDecodeLimits,
) -> Result<DecodedCompositeLeaves, DecodeFailure> {
    let (harness, module) = (&identity.harness_symbol, &identity.module_symbol);
    check_bytes(harness, module, playback, limits)?;
    check_leaf_bindings(identity).map_err(|refusal| binding_failure(identity, &refusal))?;
    check_values_and_harness(harness, module, playback, limits)?;
    let composite = identity
        .composite
        .as_ref()
        .ok_or_else(|| binding_failure(identity, &LeafBindingRefusal::NotComposite))?;
    let expected =
        expected_leaves(composite).map_err(|refusal| binding_failure(identity, &refusal))?;
    let mut entries = playback.entries.iter();
    let mut parameters = Vec::new();
    for node in draw_order(identity) {
        let recorded = leaves_of(identity, node);
        let mut values: Vec<Option<i64>> = Vec::with_capacity(expected.len());
        for (want, leaf) in expected.iter().zip(recorded) {
            let drawn = want
                .guards
                .iter()
                .all(|guard| admits(expected[*guard].kind, values[*guard]));
            if !drawn {
                values.push(None);
                continue;
            }
            let (comment, bytes) = entries.next().ok_or_else(|| {
                DecodeFailure::new("kani_witness_arity_mismatch", playback.harness, &leaf.name)
            })?;
            let read = read_entry(
                playback.harness,
                &leaf.name,
                leaf.kind.primitive(),
                comment,
                bytes,
            )?;
            let value = match read {
                WitnessValue::Boolean(flag) => i64::from(flag),
                WitnessValue::Integer(integer) => i64::try_from(integer).map_err(|_| {
                    DecodeFailure::new("cg_witness_out_of_domain", playback.harness, &leaf.name)
                })?,
                _ => {
                    return Err(DecodeFailure::new(
                        "cg_witness_type_mismatch",
                        playback.harness,
                        &leaf.name,
                    ))
                }
            };
            if value < want.bound.0 || value > want.bound.1 {
                return Err(DecodeFailure::new(
                    "cg_witness_out_of_domain",
                    playback.harness,
                    &leaf.name,
                ));
            }
            values.push(Some(value));
        }
        parameters.push((node.to_owned(), values));
    }
    if entries.next().is_some() {
        return Err(DecodeFailure::new(
            "kani_witness_arity_mismatch",
            playback.harness,
            playback.check_text,
        ));
    }
    Ok(DecodedCompositeLeaves { parameters })
}

/// Rebuilds the two operands of the claim from decoded leaves: a parameter operand from the
/// leaves drawn for its node (both positions naming one node get equal operands), a literal
/// operand from its pinned leaves. Absent, null and present slots, and absent and present options,
/// are rebuilt as distinct values; a nested integer leaf is an exact integer.
///
/// # Errors
///
/// A [`DecodeFailure`] when the identity's leaf bindings fail their check, a leaf has no value, or
/// a declaration has no structural key.
pub fn rebuild_composite_operands(
    identity: &ShadowIdentity,
    decoded: &DecodedCompositeLeaves,
) -> Result<[WitnessValue; 2], DecodeFailure> {
    rebuild_operands(identity, &decoded.parameters).map_err(|failure| {
        DecodeFailure::new(failure.code, &identity.harness_symbol, &failure.context)
    })
}

/// Decodes `playback` through the identity's leaf bindings and rebuilds the two operands it drew.
///
/// # Errors
///
/// The [`DecodeFailure`] of [`decode_composite_falsification`] or [`rebuild_composite_operands`].
pub fn reconstruct_composite_operands(
    identity: &ShadowIdentity,
    playback: &Playback<'_>,
    limits: WitnessDecodeLimits,
) -> Result<[WitnessValue; 2], DecodeFailure> {
    let decoded = decode_composite_falsification(identity, playback, limits)?;
    rebuild_composite_operands(identity, &decoded)
}
