//! The leaf bindings of a composite shadow identity (FR-025-AC-9).
//!
//! An identity records, for every drawn leaf, the parameter node it belongs to, its typed path
//! inside that parameter, its kind and its declared and drawn bounds; a literal operand's leaves
//! are recorded pinned. This module derives, from the identity's own declaration closure and
//! operand types alone, the leaves a shape must have and compares the recorded leaves with them.
//! It is the single schema both the generator and the playback decoder rely on, so a leaf the
//! generator would draw and a leaf the decoder would read cannot disagree.

use std::collections::BTreeSet;

use filament_exact::Integer;
use fsl_replay::{WitnessField, WitnessSlot, WitnessValue};

use serde::Serialize;

use crate::{
    core::ir::node_key::checked_node_key,
    kani::identity::shadow::{
        CompositeIdentity, LeafKind, LeafRecord, OperandSource, PinnedLeaf, ShadowIdentity,
    },
    oracle::equality::closure::{RecordedForm, RecordedPresence, RecordedType},
};

/// One leaf a composite operand of the identity's shape has, relative to the operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExpectedLeaf {
    /// The leaf's path inside the operand.
    pub(crate) path: String,
    /// What the leaf stands for.
    pub(crate) kind: LeafKind,
    /// The inclusive bound the leaf is declared and drawn in.
    pub(crate) bound: (i64, i64),
    /// The earlier leaves, by index into the operand's leaves, that must be present for this leaf
    /// to be drawn at all.
    pub(crate) guards: Vec<usize>,
}

/// Why the leaf bindings of an identity are refused (FR-025-AC-9). Each refusal yields no
/// harness at generation and no reconstruction at playback.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum LeafBindingRefusal {
    /// The identity is not a composite identity.
    NotComposite,
    /// The recorded declarations do not describe a shape of drawable leaves.
    Shape {
        /// What cannot be drawn.
        cause: String,
    },
    /// An operand's shape has a leaf the identity does not record.
    Missing {
        /// The operand's parameter node, or `left` or `right` for a literal.
        operand: String,
        /// The missing leaf's path.
        path: String,
    },
    /// A leaf name, or a node and path, is recorded twice.
    Duplicate {
        /// The leaf.
        name: String,
    },
    /// A recorded leaf names a node that is no operand, or a path the operand's shape lacks.
    Unknown {
        /// The leaf.
        name: String,
    },
    /// A recorded leaf's kind, bound or pinned value differs from its shape's.
    Conflict {
        /// The leaf.
        name: String,
        /// The member that differs.
        member: ConflictMember,
    },
    /// The recorded leaves are not in the shape's symbolic call order.
    OutOfOrder {
        /// The first leaf out of place.
        name: String,
    },
    /// The generated proof source does not parse or holds no proof function, so its draws cannot
    /// be compared with the identity's.
    SourceUnreadable,
    /// The generated source draws other leaves, in another order or of another primitive type,
    /// than the identity records.
    CallOrder {
        /// The identity's leaves, as `name:primitive`.
        recorded: Vec<String>,
        /// The source's draws, as `name:primitive`.
        drawn: Vec<String>,
    },
}

/// The member of a recorded leaf that differs from its shape's.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictMember {
    /// The leaf's kind.
    Kind,
    /// The declared bound.
    Declared,
    /// The drawn bound.
    Drawn,
    /// A pinned value outside its bound.
    Value,
}

impl std::fmt::Display for LeafBindingRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composite leaf binding refused: {self:?}")
    }
}

impl std::error::Error for LeafBindingRefusal {}

impl LeafBindingRefusal {
    /// The stable code the playback decoder reports this refusal under.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotComposite => "cg_composite_leaf_not_composite",
            Self::Shape { .. } => "cg_composite_leaf_shape",
            Self::Missing { .. } => "cg_composite_leaf_missing",
            Self::Duplicate { .. } => "cg_composite_leaf_duplicate",
            Self::Unknown { .. } => "cg_composite_leaf_unknown",
            Self::Conflict { .. } => "cg_composite_leaf_conflict",
            Self::OutOfOrder { .. } => "cg_composite_leaf_out_of_order",
            Self::CallOrder { .. } => "cg_composite_leaf_call_order",
            Self::SourceUnreadable => "cg_composite_leaf_source_unreadable",
        }
    }
}

fn shape(cause: impl Into<String>) -> LeafBindingRefusal {
    LeafBindingRefusal::Shape {
        cause: cause.into(),
    }
}

fn walk(
    composite: &CompositeIdentity,
    ty: &RecordedType,
    path: &str,
    guards: &[usize],
    stack: &mut Vec<String>,
    out: &mut Vec<ExpectedLeaf>,
) -> Result<(), LeafBindingRefusal> {
    let leaf = |kind, bound, path: String, guards: &[usize]| ExpectedLeaf {
        path,
        kind,
        bound,
        guards: guards.to_vec(),
    };
    match ty {
        RecordedType::Boolean => {
            out.push(leaf(LeafKind::Boolean, (0, 1), path.to_owned(), guards));
            Ok(())
        }
        RecordedType::Integer => Err(shape(format!("{path}: an integer with no declared bound"))),
        RecordedType::Int { lower, upper } => match (lower.parse::<i64>(), upper.parse::<i64>()) {
            (Ok(lower), Ok(upper)) if lower <= upper => {
                out.push(leaf(
                    LeafKind::Integer,
                    (lower, upper),
                    path.to_owned(),
                    guards,
                ));
                Ok(())
            }
            _ => Err(shape(format!("{path}: a bound outside i64"))),
        },
        RecordedType::Option { payload } => {
            let presence = out.len();
            out.push(leaf(
                LeafKind::Presence,
                (0, 1),
                format!("{path}.some"),
                guards,
            ));
            let mut inner = guards.to_vec();
            inner.push(presence);
            walk(
                composite,
                payload,
                &format!("{path}.value"),
                &inner,
                stack,
                out,
            )
        }
        RecordedType::Composite { key } => {
            let declaration = composite
                .declarations
                .iter()
                .find(|declaration| &declaration.key == key)
                .ok_or_else(|| shape(format!("{path}: declaration {key} is not recorded")))?;
            if stack.contains(key) {
                return Err(shape(format!("{path}: declaration {key} reaches itself")));
            }
            if declaration.form == RecordedForm::Union {
                return Err(shape(format!("{path}: a union member has no drawn leaves")));
            }
            stack.push(key.clone());
            for member in &declaration.members {
                let member_path = format!("{path}.{}", member.name);
                if member.presence == RecordedPresence::Optional {
                    let state = out.len();
                    out.push(leaf(
                        LeafKind::SlotState,
                        (0, 2),
                        format!("{member_path}.state"),
                        guards,
                    ));
                    let mut inner = guards.to_vec();
                    inner.push(state);
                    walk(
                        composite,
                        &member.value_type,
                        &member_path,
                        &inner,
                        stack,
                        out,
                    )?;
                } else {
                    walk(
                        composite,
                        &member.value_type,
                        &member_path,
                        guards,
                        stack,
                        out,
                    )?;
                }
            }
            stack.pop();
            Ok(())
        }
        RecordedType::Payload { .. } | RecordedType::Other { .. } => {
            Err(shape(format!("{path}: a family no shadow draws")))
        }
    }
}

fn positions(
    composite: &CompositeIdentity,
    ty: &RecordedType,
    path: &str,
    indexes: &[u32],
    stack: &mut Vec<String>,
    out: &mut Vec<(String, Vec<u32>)>,
) -> Result<(), LeafBindingRefusal> {
    match ty {
        RecordedType::Boolean => Ok(()),
        RecordedType::Int { .. } => {
            out.push((path.trim_start_matches('.').to_owned(), indexes.to_vec()));
            Ok(())
        }
        RecordedType::Option { payload } => {
            let mut inner = indexes.to_vec();
            inner.push(0);
            positions(
                composite,
                payload,
                &format!("{path}.value"),
                &inner,
                stack,
                out,
            )
        }
        RecordedType::Composite { key } => {
            let declaration = composite
                .declarations
                .iter()
                .find(|declaration| &declaration.key == key)
                .ok_or_else(|| shape(format!("{path}: declaration {key} is not recorded")))?;
            if stack.contains(key) || declaration.form == RecordedForm::Union {
                return Err(shape(format!(
                    "{path}: declaration {key} has no drawn leaves"
                )));
            }
            stack.push(key.clone());
            for (index, member) in declaration.members.iter().enumerate() {
                let index = u32::try_from(index)
                    .map_err(|_| shape(format!("{path}: more members than a child index holds")))?;
                let mut inner = indexes.to_vec();
                inner.push(index);
                positions(
                    composite,
                    &member.value_type,
                    &format!("{path}.{}", member.name),
                    &inner,
                    stack,
                    out,
                )?;
            }
            stack.pop();
            Ok(())
        }
        RecordedType::Integer | RecordedType::Payload { .. } | RecordedType::Other { .. } => {
            Err(shape(format!("{path}: a family no shadow draws")))
        }
    }
}

/// The bounded-integer leaves of one operand of the identity's shape, each with the leaf path the
/// identity records and the child-index path FSL keys the integer's declared position by: an
/// option's payload is child 0, a record or tuple member is its declaration index.
pub(crate) fn integer_positions(
    composite: &CompositeIdentity,
) -> Result<Vec<(String, Vec<u32>)>, LeafBindingRefusal> {
    if composite.operand_types.left != composite.operand_types.right {
        return Err(shape("the operands compare different types"));
    }
    let mut out = Vec::new();
    positions(
        composite,
        &composite.operand_types.left,
        "",
        &[],
        &mut Vec::new(),
        &mut out,
    )?;
    Ok(out)
}

/// The leaves one operand of the identity's shape has, in symbolic call order.
pub(crate) fn expected_leaves(
    composite: &CompositeIdentity,
) -> Result<Vec<ExpectedLeaf>, LeafBindingRefusal> {
    if composite.operand_types.left != composite.operand_types.right {
        return Err(shape("the operands compare different types"));
    }
    let mut out = Vec::new();
    walk(
        composite,
        &composite.operand_types.left,
        "",
        &[],
        &mut Vec::new(),
        &mut out,
    )?;
    for leaf in &mut out {
        leaf.path = leaf.path.trim_start_matches('.').to_owned();
    }
    Ok(out)
}

/// The distinct parameter nodes of the operands, in operand order.
pub(crate) fn parameter_nodes(composite: &CompositeIdentity) -> Vec<&str> {
    let mut nodes: Vec<&str> = Vec::new();
    for source in &composite.operands {
        if let OperandSource::Parameter { node } = source {
            if !nodes.contains(&node.as_str()) {
                nodes.push(node);
            }
        }
    }
    nodes
}

/// The recorded drawn leaves of `node`, in recorded order.
pub(crate) fn leaves_of<'a>(identity: &'a ShadowIdentity, node: &str) -> Vec<&'a LeafRecord> {
    identity
        .leaves
        .iter()
        .filter(|leaf| leaf.node == node)
        .collect()
}

fn check_drawn(
    identity: &ShadowIdentity,
    node: &str,
    expected: &[ExpectedLeaf],
) -> Result<(), LeafBindingRefusal> {
    let recorded = leaves_of(identity, node);
    let mut paths = BTreeSet::new();
    for leaf in &recorded {
        if !paths.insert(leaf.path.as_str()) {
            return Err(LeafBindingRefusal::Duplicate {
                name: leaf.name.clone(),
            });
        }
        if !expected.iter().any(|want| want.path == leaf.path) {
            return Err(LeafBindingRefusal::Unknown {
                name: leaf.name.clone(),
            });
        }
    }
    for want in expected {
        if !paths.contains(want.path.as_str()) {
            return Err(LeafBindingRefusal::Missing {
                operand: node.to_owned(),
                path: want.path.clone(),
            });
        }
    }
    for leaf in &recorded {
        let want = expected
            .iter()
            .find(|want| want.path == leaf.path)
            .ok_or_else(|| LeafBindingRefusal::Unknown {
                name: leaf.name.clone(),
            })?;
        let conflict = |member: ConflictMember| LeafBindingRefusal::Conflict {
            name: leaf.name.clone(),
            member,
        };
        if leaf.kind != want.kind {
            return Err(conflict(ConflictMember::Kind));
        }
        if leaf.declared != want.bound {
            return Err(conflict(ConflictMember::Declared));
        }
        if leaf.drawn != want.bound {
            return Err(conflict(ConflictMember::Drawn));
        }
    }
    for (leaf, want) in recorded.iter().zip(expected) {
        if leaf.path != want.path {
            return Err(LeafBindingRefusal::OutOfOrder {
                name: leaf.name.clone(),
            });
        }
    }
    Ok(())
}

fn check_pinned(
    composite: &CompositeIdentity,
    position: usize,
    expected: &[ExpectedLeaf],
) -> Result<(), LeafBindingRefusal> {
    let label = if position == 0 { "left" } else { "right" };
    let recorded: Vec<&PinnedLeaf> = composite
        .pinned
        .iter()
        .filter(|pinned| pinned.operand == position)
        .collect();
    let mut paths = BTreeSet::new();
    for pinned in &recorded {
        if !paths.insert(pinned.path.as_str()) {
            return Err(LeafBindingRefusal::Duplicate {
                name: pinned.name.clone(),
            });
        }
        if !expected.iter().any(|want| want.path == pinned.path) {
            return Err(LeafBindingRefusal::Unknown {
                name: pinned.name.clone(),
            });
        }
    }
    for want in expected {
        if !paths.contains(want.path.as_str()) {
            return Err(LeafBindingRefusal::Missing {
                operand: label.to_owned(),
                path: want.path.clone(),
            });
        }
    }
    for pinned in &recorded {
        let want = expected
            .iter()
            .find(|want| want.path == pinned.path)
            .ok_or_else(|| LeafBindingRefusal::Unknown {
                name: pinned.name.clone(),
            })?;
        let conflict = |member: ConflictMember| LeafBindingRefusal::Conflict {
            name: pinned.name.clone(),
            member,
        };
        if pinned.kind != want.kind {
            return Err(conflict(ConflictMember::Kind));
        }
        if pinned.value < want.bound.0 || pinned.value > want.bound.1 {
            return Err(conflict(ConflictMember::Value));
        }
    }
    for (pinned, want) in recorded.iter().zip(expected) {
        if pinned.path != want.path {
            return Err(LeafBindingRefusal::OutOfOrder {
                name: pinned.name.clone(),
            });
        }
    }
    Ok(())
}

/// Checks the identity's recorded leaves against the leaves its declaration closure and operand
/// types give (FR-025-AC-9): every leaf of every parameter operand is recorded once, in symbolic
/// call order, with its kind and bounds; no recorded leaf is unknown; every literal operand's
/// leaves are pinned once to a value inside their bound; and a literal leaf is never drawn.
///
/// # Errors
///
/// The first [`LeafBindingRefusal`] found.
pub fn check_leaf_bindings(identity: &ShadowIdentity) -> Result<(), LeafBindingRefusal> {
    let composite = identity
        .composite
        .as_ref()
        .ok_or(LeafBindingRefusal::NotComposite)?;
    let expected = expected_leaves(composite)?;
    let mut names = BTreeSet::new();
    let recorded_names = identity
        .leaves
        .iter()
        .map(|leaf| &leaf.name)
        .chain(composite.pinned.iter().map(|pinned| &pinned.name));
    for name in recorded_names {
        if !names.insert(name.as_str()) {
            return Err(LeafBindingRefusal::Duplicate { name: name.clone() });
        }
    }
    let nodes = parameter_nodes(composite);
    if let Some(leaf) = identity
        .leaves
        .iter()
        .find(|leaf| !nodes.contains(&leaf.node.as_str()))
    {
        return Err(LeafBindingRefusal::Unknown {
            name: leaf.name.clone(),
        });
    }
    // The leaves of one parameter are contiguous: a draw of another parameter between them would
    // change the call order the playback is read in.
    let mut closed: Vec<&str> = Vec::new();
    let mut open: Option<&str> = None;
    for leaf in &identity.leaves {
        if open != Some(leaf.node.as_str()) {
            if closed.contains(&leaf.node.as_str()) {
                return Err(LeafBindingRefusal::OutOfOrder {
                    name: leaf.name.clone(),
                });
            }
            if let Some(previous) = open {
                closed.push(previous);
            }
            open = Some(leaf.node.as_str());
        }
    }
    for node in &nodes {
        check_drawn(identity, node, &expected)?;
    }
    for (position, source) in composite.operands.iter().enumerate() {
        if matches!(source, OperandSource::Literal) {
            check_pinned(composite, position, &expected)?;
        }
    }
    if let Some(pinned) = composite.pinned.iter().find(|pinned| {
        composite
            .operands
            .get(pinned.operand)
            .is_none_or(|source| !matches!(source, OperandSource::Literal))
    }) {
        return Err(LeafBindingRefusal::Unknown {
            name: pinned.name.clone(),
        });
    }
    Ok(())
}

/// Why the operands of an identity cannot be rebuilt from decoded leaves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RebuildFailure {
    /// Stable machine-readable cause code.
    pub(crate) code: &'static str,
    /// What the failure concerns.
    pub(crate) context: String,
}

impl From<LeafBindingRefusal> for RebuildFailure {
    fn from(refusal: LeafBindingRefusal) -> Self {
        Self {
            code: refusal.code(),
            context: format!("{refusal:?}"),
        }
    }
}

struct Builder<'a> {
    composite: &'a CompositeIdentity,
    values: &'a [Option<i64>],
    cursor: usize,
}

impl Builder<'_> {
    fn fail(&self, code: &'static str, context: &str) -> RebuildFailure {
        RebuildFailure {
            code,
            context: context.to_owned(),
        }
    }

    fn next(&mut self) -> Option<i64> {
        let value = self.values.get(self.cursor).copied().flatten();
        self.cursor += 1;
        value
    }

    /// The value of `ty`, consuming the leaves of `ty` in call order. A subtree whose condition
    /// failed is consumed too and yields a placeholder the caller discards.
    fn build(&mut self, ty: &RecordedType) -> Result<WitnessValue, RebuildFailure> {
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

/// Rebuilds the two operands of the claim from the leaves drawn per parameter node: a parameter
/// operand from its node's leaves, a literal operand from its pinned leaves.
pub(crate) fn rebuild_operands(
    identity: &ShadowIdentity,
    drawn: &[(String, Vec<Option<i64>>)],
) -> Result<[WitnessValue; 2], RebuildFailure> {
    check_leaf_bindings(identity)?;
    let composite = identity
        .composite
        .as_ref()
        .ok_or(LeafBindingRefusal::NotComposite)?;
    let expected = expected_leaves(composite)?;
    let missing = |context: &str| RebuildFailure {
        code: "cg_composite_leaf_missing",
        context: context.to_owned(),
    };
    let mut rebuilt = Vec::new();
    for (position, source) in composite.operands.iter().enumerate() {
        let values: Vec<Option<i64>> = match source {
            OperandSource::Parameter { node } => drawn
                .iter()
                .find(|(found, _)| found == node)
                .map(|(_, values)| values.clone())
                .ok_or_else(|| missing(node))?,
            OperandSource::Literal => pinned_values(composite, position, &expected),
        };
        if values.len() != expected.len() {
            return Err(missing("the decoded leaves do not cover the shape"));
        }
        let mut builder = Builder {
            composite,
            values: &values,
            cursor: 0,
        };
        rebuilt.push(builder.build(&composite.operand_types.left)?);
    }
    let [left, right]: [WitnessValue; 2] = rebuilt.try_into().map_err(|_| missing("operands"))?;
    Ok([left, right])
}
