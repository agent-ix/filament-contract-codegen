//! Decode a Kani playback of a composite shadow harness through the identity's leaf bindings and
//! rebuild the operands it drew (FR-025-AC-9, FR-033-AC-2 to FR-033-AC-5).
//!
//! The playback holds one untyped entry per `kani::any()` call the counterexample executed, in
//! call order. A leaf drawn under a condition (the payload of a present option, the value of a
//! present optional slot) is not drawn when the condition fails, so the decoder walks the
//! identity's leaves, evaluates each leaf's condition over the values already read, and reads an
//! entry only for a leaf that is drawn. The identity's leaf bindings are checked first; no
//! playback is read through a schema that fails the check.

use filament_exact::Integer;
use fsl_replay::{WitnessField, WitnessSlot, WitnessValue};

use crate::{
    core::ir::node_key::checked_node_key,
    kani::{
        abi::KaniPrimitiveType,
        identity::{
            leaf_binding::{check_leaf_bindings, expected_leaves, leaves_of, ExpectedLeaf},
            shadow::{CompositeIdentity, LeafKind, OperandSource, ShadowIdentity},
        },
        output::playback::{Playback, WitnessDecodeLimits},
    },
    oracle::equality::closure::{RecordedForm, RecordedPresence, RecordedType},
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

fn primitive(kind: LeafKind) -> KaniPrimitiveType {
    match kind {
        LeafKind::Boolean | LeafKind::Presence => KaniPrimitiveType::Boolean,
        LeafKind::Integer | LeafKind::SlotState => KaniPrimitiveType::I64,
    }
}

/// Whether a guard leaf, read as `value`, lets the leaves under it be drawn.
fn admits(kind: LeafKind, value: Option<i64>) -> bool {
    matches!(
        (kind, value),
        (LeafKind::Presence, Some(1)) | (LeafKind::SlotState, Some(0))
    )
}

fn binding_failure(
    identity: &ShadowIdentity,
    refusal: &crate::LeafBindingRefusal,
) -> DecodeFailure {
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
        .ok_or_else(|| binding_failure(identity, &crate::LeafBindingRefusal::NotComposite))?;
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
                primitive(leaf.kind),
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

struct Builder<'a> {
    composite: &'a CompositeIdentity,
    values: &'a [Option<i64>],
    cursor: usize,
    harness: &'a str,
}

impl Builder<'_> {
    fn fail(&self, code: &str, context: &str) -> DecodeFailure {
        DecodeFailure::new(code, self.harness, context)
    }

    fn next(&mut self) -> Option<i64> {
        let value = self.values.get(self.cursor).copied().flatten();
        self.cursor += 1;
        value
    }

    /// The value of `ty`, consuming the leaves of `ty` in call order. A subtree whose condition
    /// failed is consumed too and yields a placeholder the caller discards.
    fn build(&mut self, ty: &RecordedType) -> Result<WitnessValue, DecodeFailure> {
        let placeholder = WitnessValue::Boolean(false);
        match ty {
            RecordedType::Boolean => Ok(self
                .next()
                .map_or(placeholder, |value| WitnessValue::Boolean(value == 1))),
            RecordedType::Int { .. } => Ok(self.next().map_or(placeholder, |value| {
                WitnessValue::ExactInteger(Integer::from(value))
            })),
            RecordedType::Option { payload } => {
                let present = self.next();
                let inner = self.build(payload)?;
                Ok(match present {
                    Some(1) => WitnessValue::Option(Some(Box::new(inner))),
                    Some(_) => WitnessValue::Option(None),
                    None => placeholder,
                })
            }
            RecordedType::Composite { key } => {
                let declaration = self
                    .composite
                    .declarations
                    .iter()
                    .find(|declaration| &declaration.key == key)
                    .ok_or_else(|| self.fail("cg_composite_declaration_unknown", key))?;
                let node = checked_node_key(&declaration.node_id)
                    .ok_or_else(|| self.fail("cg_composite_declaration_key", key))?;
                let mut slots = Vec::new();
                let mut live = true;
                for member in &declaration.members {
                    let slot = if member.presence == RecordedPresence::Optional {
                        let state = self.next();
                        let inner = self.build(&member.value_type)?;
                        match state {
                            Some(0) => WitnessSlot::Present(inner),
                            Some(1) => WitnessSlot::Absent,
                            Some(2) => WitnessSlot::Null,
                            Some(_) => {
                                return Err(self.fail("cg_witness_out_of_domain", &member.name))
                            }
                            None => {
                                live = false;
                                WitnessSlot::Absent
                            }
                        }
                    } else {
                        WitnessSlot::Present(self.build(&member.value_type)?)
                    };
                    slots.push((member.name.clone(), slot));
                }
                if !live {
                    return Ok(placeholder);
                }
                Ok(match declaration.form {
                    RecordedForm::Record => WitnessValue::Record {
                        declaration: node,
                        fields: slots
                            .into_iter()
                            .map(|(name, slot)| WitnessField { name, slot })
                            .collect(),
                    },
                    RecordedForm::Tuple => WitnessValue::Tuple {
                        declaration: node,
                        components: slots
                            .into_iter()
                            .filter_map(|(_, slot)| match slot {
                                WitnessSlot::Present(value) => Some(value),
                                WitnessSlot::Absent | WitnessSlot::Null => None,
                            })
                            .collect(),
                    },
                    RecordedForm::Union => {
                        return Err(self.fail("cg_composite_leaf_shape", key));
                    }
                })
            }
            RecordedType::Integer | RecordedType::Payload { .. } | RecordedType::Other { .. } => {
                Err(self.fail("cg_composite_leaf_shape", "an undrawable type"))
            }
        }
    }
}

fn pinned_values(
    composite: &CompositeIdentity,
    position: usize,
    expected: &[ExpectedLeaf],
) -> Vec<Option<i64>> {
    expected
        .iter()
        .map(|want| {
            composite
                .pinned
                .iter()
                .find(|pinned| pinned.operand == position && pinned.path == want.path)
                .map(|pinned| pinned.value)
        })
        .collect()
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
    check_leaf_bindings(identity).map_err(|refusal| binding_failure(identity, &refusal))?;
    let composite = identity
        .composite
        .as_ref()
        .ok_or_else(|| binding_failure(identity, &crate::LeafBindingRefusal::NotComposite))?;
    let expected =
        expected_leaves(composite).map_err(|refusal| binding_failure(identity, &refusal))?;
    let mut rebuilt = Vec::new();
    for (position, source) in composite.operands.iter().enumerate() {
        let values: Vec<Option<i64>> = match source {
            OperandSource::Parameter { node } => decoded
                .values_of(node)
                .ok_or_else(|| {
                    DecodeFailure::new("cg_composite_leaf_missing", &identity.harness_symbol, node)
                })?
                .to_vec(),
            OperandSource::Literal => pinned_values(composite, position, &expected),
        };
        if values.len() != expected.len() {
            return Err(DecodeFailure::new(
                "cg_composite_leaf_missing",
                &identity.harness_symbol,
                "the decoded leaves do not cover the shape",
            ));
        }
        let mut builder = Builder {
            composite,
            values: &values,
            cursor: 0,
            harness: &identity.harness_symbol,
        };
        rebuilt.push(builder.build(&composite.operand_types.left)?);
    }
    let [left, right]: [WitnessValue; 2] = rebuilt.try_into().map_err(|_| {
        DecodeFailure::new(
            "cg_composite_leaf_missing",
            &identity.harness_symbol,
            "operands",
        )
    })?;
    Ok([left, right])
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
