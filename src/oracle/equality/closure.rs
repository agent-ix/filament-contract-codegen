//! The declaration closure a generated item's claim-map entry records
//! (FR-018-AC-21). It is read from the same reconstructed
//! [`CompositeDeclaration`]s the item's environment constructor is emitted from.

use super::admitted::CompositeBinding;
use filament_contract_model::CheckedNodeId;
use filament_exact::{CollectionKind, NodeKey, Presence, ValueType};
use filament_semantic_value::declaration::{CompositeDeclaration, CompositeShape};
use serde::Serialize;
use std::collections::BTreeMap;

/// A declared value type, reduced to what a comparison reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "family")]
pub enum RecordedType {
    /// `Boolean`.
    Boolean,
    /// The unbounded `Integer`.
    Integer,
    /// A bounded integer with inclusive decimal-spelled bounds.
    Int {
        /// Inclusive lower bound.
        lower: String,
        /// Inclusive upper bound.
        upper: String,
    },
    /// `Option<T>`.
    Option {
        /// The payload type.
        payload: Box<RecordedType>,
    },
    /// A record, tuple or union declaration, by its kernel key.
    Composite {
        /// The owner-issued kernel key, as `declaration_runtime_keys` spells it.
        key: String,
    },
    /// A union member's ordered payload types; empty for a nullary member.
    Payload {
        /// The payload position types in declaration order.
        positions: Vec<RecordedType>,
    },
    /// Any other family; its bounds are not recorded.
    Other {
        /// The family name.
        name: &'static str,
    },
}

/// A declaration's form.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedForm {
    /// A record.
    Record,
    /// A tuple.
    Tuple,
    /// A union.
    Union,
}

/// Whether a member is declared with `?`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedPresence {
    /// Always present.
    Required,
    /// May be absent or null.
    Optional,
}

/// One member in declaration order. A tuple position is named by its index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordedMember {
    /// Field name, tuple position index or union member identifier.
    pub name: String,
    /// Declared presence.
    pub presence: RecordedPresence,
    /// Declared value type; a union member records its payload positions.
    pub value_type: RecordedType,
}

/// One declaration of the closure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordedDeclaration {
    /// The V2 node id the declaration was read from.
    pub node_id: CheckedNodeId,
    /// The owner-issued kernel key.
    pub key: String,
    /// Record, tuple or union.
    pub form: RecordedForm,
    /// Members in declaration order.
    pub members: Vec<RecordedMember>,
}

/// The operand comparison types of the item (the conversion target when converted).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordedOperandTypes {
    /// Left comparison type.
    pub left: RecordedType,
    /// Right comparison type.
    pub right: RecordedType,
}

pub(super) fn recorded_type(value: &ValueType) -> RecordedType {
    match value {
        ValueType::Boolean => RecordedType::Boolean,
        ValueType::Integer => RecordedType::Integer,
        ValueType::Int(interval) => RecordedType::Int {
            lower: interval.lower().to_string(),
            upper: interval.upper().to_string(),
        },
        ValueType::Option(payload) => RecordedType::Option {
            payload: Box::new(recorded_type(payload)),
        },
        ValueType::Composite(key) => RecordedType::Composite {
            key: key.to_string(),
        },
        ValueType::Rational(_) => RecordedType::Other { name: "rational" },
        ValueType::Decimal(_) => RecordedType::Other { name: "decimal" },
        ValueType::Float(_) => RecordedType::Other { name: "float" },
        ValueType::Text(_) => RecordedType::Other { name: "text" },
        ValueType::Enum(_) => RecordedType::Other { name: "enum" },
        ValueType::Collection(collection) => RecordedType::Other {
            name: match collection.kind() {
                CollectionKind::Sequence => "sequence",
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                CollectionKind::OrderedSet => "ordered_set",
            },
        },
        ValueType::Quantity(_) | ValueType::CarriedQuantity(_) | ValueType::BoundedQuantity(_) => {
            RecordedType::Other { name: "quantity" }
        }
        ValueType::Reference(_) => RecordedType::Other { name: "reference" },
        _ => RecordedType::Other { name: "other" },
    }
}

/// Every declaration of the item, ascending by V2 node id (the order of
/// `declaration_keys`). `bindings` maps each kernel key to its V2 node.
pub(super) fn recorded_declarations(
    composites: &[CompositeDeclaration],
    bindings: &BTreeMap<NodeKey, CompositeBinding>,
    node_id: impl Fn(&CheckedNodeId) -> CheckedNodeId,
) -> Option<Vec<RecordedDeclaration>> {
    let mut declarations = composites
        .iter()
        .map(|declaration| {
            let binding = bindings.get(&declaration.key())?;
            let (form, members) = match declaration.shape() {
                CompositeShape::Record(fields) => (
                    RecordedForm::Record,
                    fields
                        .iter()
                        .map(|field| RecordedMember {
                            name: field.name().to_owned(),
                            presence: match field.presence() {
                                Presence::Optional => RecordedPresence::Optional,
                                _ => RecordedPresence::Required,
                            },
                            value_type: recorded_type(field.value_type()),
                        })
                        .collect(),
                ),
                CompositeShape::Tuple(positions) => (
                    RecordedForm::Tuple,
                    positions
                        .iter()
                        .enumerate()
                        .map(|(index, value)| RecordedMember {
                            name: index.to_string(),
                            presence: RecordedPresence::Required,
                            value_type: recorded_type(value),
                        })
                        .collect(),
                ),
                CompositeShape::Union(members) => (
                    RecordedForm::Union,
                    members
                        .iter()
                        .map(|member| RecordedMember {
                            name: member.identifier().to_owned(),
                            presence: RecordedPresence::Required,
                            value_type: RecordedType::Payload {
                                positions: member.positions().iter().map(recorded_type).collect(),
                            },
                        })
                        .collect(),
                ),
            };
            Some(RecordedDeclaration {
                node_id: node_id(&binding.node),
                key: declaration.key().to_string(),
                form,
                members,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    declarations.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    Some(declarations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{CardinalityBound, CollectionType};
    use ix_trace_rs::trace;

    /// Every collection kind is recorded under its own family name, so the shadow's
    /// unsupported-shape rows name the kind (FR-015-AC-73).
    #[trace("FR-018-AC-21", "TC-029")]
    #[test]
    fn tc_029_a_collection_is_recorded_under_the_name_of_its_kind() {
        let name = |kind| {
            let bound = CardinalityBound::new(0, 2).unwrap();
            let collection = CollectionType::new(kind, ValueType::Boolean, Some(bound));
            match recorded_type(&ValueType::collection(collection)) {
                RecordedType::Other { name } => name,
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(
            [
                CollectionKind::Sequence,
                CollectionKind::Set,
                CollectionKind::Bag,
                CollectionKind::OrderedSet
            ]
            .map(name),
            ["sequence", "set", "bag", "ordered_set"]
        );
    }
}
