//! Admitted checked-package state-clause body access.

use std::collections::{BTreeMap, BTreeSet};

use super::checked_term::{application, reference};
use quire_contract_model::{
    ApplicationOperator, BoundedDomainForm, CheckedMemberType, CheckedNodeId, CheckedNodeKind,
    CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm, CompositeTypeForm, LiteralKind,
    ScalarTypeForm,
};

pub(crate) const MAX_CONDITION_TERMS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ClauseInputSide {
    Current,
    Pre,
    Parameter,
    Result,
}

/// Which physical draw of a declared operand an input is: the operand itself, or one leaf of an
/// option operand (FR-054).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ClauseLeaf {
    Whole,
    /// The presence draw of an option operand.
    Presence,
    /// A payload leaf of an option operand, by its path (empty for a scalar payload).
    Payload(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ClauseInput {
    pub(crate) name: String,
    pub(crate) leaf: ClauseLeaf,
    pub(crate) side: ClauseInputSide,
    pub(crate) declaring_node_id: Option<CheckedNodeId>,
    pub(crate) boolean: bool,
    pub(crate) lower: i128,
    pub(crate) upper: i128,
}

#[derive(Clone, Debug)]
pub(crate) enum ClauseTerm {
    Boolean(bool),
    Integer(String),
    Input(ClauseInput),
    Operation {
        identity: String,
        arguments: Vec<ClauseTerm>,
        result_bound: Option<(i128, i128)>,
    },
    /// `quire.op.option.value` of an option operand: the payload leaf, defined only where the
    /// presence draw is true.
    OptionValue {
        presence: ClauseInput,
        payload: ClauseInput,
    },
    /// A read of the innermost enclosing `let` binder, by the unique number the reader gave it.
    Binder {
        binder: usize,
    },
    /// `quire.op.control.let`: the value is evaluated once, outside the binder's scope, and the
    /// body reads it by `binder`.
    Let {
        binder: usize,
        value: Box<ClauseTerm>,
        body: Box<ClauseTerm>,
    },
    /// `quire.op.control.if`: only the selected branch is evaluated.
    If {
        condition: Box<ClauseTerm>,
        then: Box<ClauseTerm>,
        other: Box<ClauseTerm>,
    },
    /// `quire.op.control.case` over an arity-0 union operation parameter drawn as a member index:
    /// one arm body per member in declaration order, only the selected arm is evaluated.
    Case {
        scrutinee: ClauseInput,
        arms: Vec<ClauseTerm>,
    },
}

/// Option identities the clause readers lower (FR-054).
pub(crate) const OPTION_PRESENT: &str = "quire.op.option.present";
pub(crate) const OPTION_VALUE: &str = "quire.op.option.value";

/// The draws of one option operand: its presence and its payload leaves in ascending path order.
struct OptionOperand {
    presence: ClauseInput,
    payload: Vec<ClauseInput>,
}

/// One scalar leaf of an option payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OptionLeaf {
    /// The leaf's path in the payload: empty for a scalar payload, a field name for a record leaf.
    pub(crate) path: String,
    pub(crate) boolean: bool,
    pub(crate) lower: i128,
    pub(crate) upper: i128,
}

/// What an option type's payload is.
pub(crate) enum OptionShape {
    /// Boolean and bounded-integer leaves, in ascending path order.
    Leaves(Vec<OptionLeaf>),
    /// A leaf of a family other than Boolean or a bounded integer, by its semantic form.
    NotScalar(String),
    /// An option of an option, or a payload leaf that is an option.
    Nested,
    /// An integer leaf with no declared bound.
    Unbounded,
}

/// The payload shape of the option type `type_node`, or `None` when it is not an option type.
pub(crate) fn option_shape(
    nodes: &ClauseGraphView<'_>,
    type_node: &CheckedNodeId,
) -> Option<OptionShape> {
    if nodes.kind(type_node) != Some(CheckedNodeKind::CompositeType(CompositeTypeForm::Option)) {
        return None;
    }
    let CheckedTerm::Aggregate { members } = nodes.package().term(type_node)? else {
        return None;
    };
    let [payload] = members.as_slice() else {
        return None;
    };
    let payload = reference(payload)?;
    let mut leaves = Vec::new();
    let mut unbounded = false;
    Some(
        match payload_leaves(nodes, payload, "", &mut leaves, &mut unbounded, 0) {
            // A shape refusal decides which leaves exist, so it precedes a bound refusal.
            Err(refusal) => refusal,
            Ok(()) if unbounded => OptionShape::Unbounded,
            Ok(()) => {
                leaves.sort_by(|left, right| left.path.cmp(&right.path));
                OptionShape::Leaves(leaves)
            }
        },
    )
}

fn payload_leaves(
    nodes: &ClauseGraphView<'_>,
    type_node: &CheckedNodeId,
    path: &str,
    out: &mut Vec<OptionLeaf>,
    unbounded: &mut bool,
    depth: usize,
) -> Result<(), OptionShape> {
    let package = nodes.package();
    let node = nodes.node(type_node).ok_or(OptionShape::Nested)?;
    let family = || OptionShape::NotScalar(node.semantic_form.to_string());
    match nodes.kind(type_node) {
        Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean)) => out.push(OptionLeaf {
            path: path.to_owned(),
            boolean: true,
            lower: 0,
            upper: 1,
        }),
        Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Integer)) => *unbounded = true,
        Some(CheckedNodeKind::BoundedDomain(BoundedDomainForm::IntegerRange)) => {
            let (lower, upper) = integer_bound(package, type_node).ok_or_else(family)?;
            out.push(OptionLeaf {
                path: path.to_owned(),
                boolean: false,
                lower,
                upper,
            });
        }
        Some(CheckedNodeKind::CompositeType(CompositeTypeForm::Option)) => {
            return Err(OptionShape::Nested)
        }
        Some(CheckedNodeKind::CompositeType(form @ (CompositeTypeForm::Record | CompositeTypeForm::Tuple)))
            if depth == 0 =>
        {
            let CheckedTerm::Aggregate { members } = package.term(type_node).ok_or_else(family)?
            else {
                return Err(family());
            };
            for (position, member) in members.iter().enumerate() {
                let (leaf_path, leaf_type) = match (form, member) {
                    (CompositeTypeForm::Record, CheckedTerm::Binding { name, value }) => {
                        (name.to_string(), reference(value).ok_or_else(family)?)
                    }
                    (CompositeTypeForm::Tuple, other) => {
                        (position.to_string(), reference(other).ok_or_else(family)?)
                    }
                    _ => return Err(family()),
                };
                payload_leaves(nodes, leaf_type, &leaf_path, out, unbounded, depth + 1)?;
            }
        }
        _ => return Err(family()),
    }
    Ok(())
}

/// The inclusive bounds of an `integer_range` node.
fn integer_bound(package: &CheckedPackageV2, id: &CheckedNodeId) -> Option<(i128, i128)> {
    let CheckedTerm::Aggregate { members } = package.term(id)? else {
        return None;
    };
    let number = |name| -> Option<i128> {
        let value = members.iter().find_map(|entry| match entry {
            CheckedTerm::Binding {
                name: actual,
                value,
            } if actual.as_ref() == name => Some(value.as_ref()),
            _ => None,
        })?;
        let CheckedTerm::Literal { value, .. } = value else {
            return None;
        };
        value.as_str()?.parse().ok()
    };
    Some((number("min")?, number("max")?))
}

/// Control identities the clause readers lower.
pub(crate) const CONTROL_LET: &str = "quire.op.control.let";
pub(crate) const CONTROL_IF: &str = "quire.op.control.if";
pub(crate) const CONTROL_CASE: &str = "quire.op.control.case";

pub(crate) struct ClauseCondition {
    pub(crate) term: ClauseTerm,
    pub(crate) inputs: Vec<ClauseInput>,
    pub(crate) condition_node_id: Option<CheckedNodeId>,
}

/// First occurrence of each checked graph node, retained for repeated clause lookups.
pub(crate) struct ClauseNodeIndex<'a> {
    nodes: BTreeMap<&'a CheckedNodeId, &'a CheckedSemanticNodeV2>,
}

impl<'a> ClauseNodeIndex<'a> {
    pub(crate) fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Keep the first node for a duplicate identity.
    pub(crate) fn insert(&mut self, node: &'a CheckedSemanticNodeV2) {
        self.nodes.entry(&node.node_id).or_insert(node);
    }

    /// Look up an admitted node without scanning the graph again.
    pub(crate) fn node(&self, id: &CheckedNodeId) -> Option<&'a CheckedSemanticNodeV2> {
        self.nodes.get(id).copied()
    }
}

/// One admitted package and its node lookup, shared by clause planning and body reading.
pub(crate) struct ClauseGraphView<'a> {
    package: &'a CheckedPackageV2,
    index: ClauseNodeIndex<'a>,
    kinds: BTreeMap<&'a CheckedNodeId, CheckedNodeKind>,
}

impl<'a> ClauseGraphView<'a> {
    pub(crate) fn new(package: &'a CheckedPackageV2) -> Self {
        let mut index = ClauseNodeIndex::new();
        let mut kinds = BTreeMap::new();
        for (node, kind) in package
            .resolved_graph()
            .nodes
            .iter()
            .zip(package.node_kinds())
        {
            index.insert(node);
            kinds.entry(&node.node_id).or_insert(*kind);
        }
        Self {
            package,
            index,
            kinds,
        }
    }

    pub(crate) fn package(&self) -> &'a CheckedPackageV2 {
        self.package
    }

    pub(crate) fn node(&self, id: &CheckedNodeId) -> Option<&'a CheckedSemanticNodeV2> {
        self.index.node(id)
    }

    pub(crate) fn kind(&self, id: &CheckedNodeId) -> Option<CheckedNodeKind> {
        self.kinds.get(id).copied()
    }
}

struct Reader<'a, 'view> {
    package: &'a CheckedPackageV2,
    nodes: &'view ClauseGraphView<'a>,
    self_id: CheckedNodeId,
    result_id: Option<CheckedNodeId>,
    allowed_parameters: BTreeSet<CheckedNodeId>,
    inputs: BTreeSet<ClauseInput>,
    visiting: BTreeSet<CheckedNodeId>,
    visits: usize,
    /// The `let` binders in force, innermost last: the declared name and the binder's number.
    scopes: Vec<(String, usize)>,
    next_binder: usize,
}

impl Reader<'_, '_> {
    fn node(&self, id: &CheckedNodeId) -> Option<&CheckedSemanticNodeV2> {
        self.nodes.node(id)
    }

    fn term(&mut self, value: &CheckedTerm, side: ClauseInputSide) -> Option<ClauseTerm> {
        self.visits = self.visits.checked_add(1)?;
        if self.visits > MAX_CONDITION_TERMS {
            return None;
        }
        match value {
            CheckedTerm::Reference { target } => {
                let id = target.clone();
                if !self.visiting.insert(id.clone()) {
                    return None;
                }
                let node = self.node(&id)?;
                if &*node.node_tag == "value" && &*node.semantic_form == "parameter" {
                    if let Some(binder) = self.binder(&id) {
                        self.visiting.remove(&id);
                        return Some(ClauseTerm::Binder { binder });
                    }
                    let boolean = self.nodes.kind(&node.semantic_type)
                        == Some(CheckedNodeKind::ScalarType(
                            quire_contract_model::ScalarTypeForm::Boolean,
                        ));
                    let bound = if boolean {
                        (0, 1)
                    } else {
                        self.bound(&node.semantic_type)?
                    };
                    let declaration = self.package.parameter_declaration(&id).ok()?;
                    if side != ClauseInputSide::Current
                        || (!self.allowed_parameters.contains(&id)
                            && self.result_id.as_ref() != Some(&id))
                    {
                        return None;
                    }
                    let input = ClauseInput {
                        name: declaration.name.to_string(),
                        leaf: ClauseLeaf::Whole,
                        side: if self.result_id.as_ref() == Some(&id) {
                            ClauseInputSide::Result
                        } else {
                            ClauseInputSide::Parameter
                        },
                        declaring_node_id: None,
                        boolean,
                        lower: bound.0,
                        upper: bound.1,
                    };
                    self.inputs.insert(input.clone());
                    self.visiting.remove(&id);
                    return Some(ClauseTerm::Input(input));
                }
                let body = self.package.term(&id)?.clone();
                let result_bound = self.bound(&node.semantic_type);
                let result = self.term_with_bound(&body, side, result_bound);
                self.visiting.remove(&id);
                result
            }
            _ => self.term_with_bound(value, side, None),
        }
    }

    fn bound(&self, id: &CheckedNodeId) -> Option<(i128, i128)> {
        let node = self.node(id)?;
        if &*node.node_tag != "bounded_domain" || &*node.semantic_form != "integer_range" {
            return None;
        }
        integer_bound(self.package, id)
    }

    fn term_with_bound(
        &mut self,
        value: &CheckedTerm,
        side: ClauseInputSide,
        result_bound: Option<(i128, i128)>,
    ) -> Option<ClauseTerm> {
        match value {
            CheckedTerm::Literal {
                value_kind, value, ..
            } => match value_kind {
                LiteralKind::Boolean => Some(ClauseTerm::Boolean(value.as_bool()?)),
                LiteralKind::Integer => Some(ClauseTerm::Integer(value.as_str()?.to_owned())),
                _ => None,
            },
            CheckedTerm::Application {
                operation,
                arguments: args,
                ..
            } => {
                let identity = operation.identity()?;
                match identity {
                    "quire.op.state.pre" if args.len() == 1 => {
                        let read = self.term_body(&args[0])?;
                        let CheckedTerm::Application {
                            operation,
                            arguments,
                            ..
                        } = &read
                        else {
                            return None;
                        };
                        if operation.identity()? != "quire.op.record.project" {
                            return None;
                        }
                        let [deref] = arguments.as_slice() else {
                            return None;
                        };
                        self.field(&read, deref, ClauseInputSide::Pre)
                    }
                    "quire.op.record.project" if args.len() == 1 => {
                        if let Some(read) = self.option_leaf_read(value, &args[0], side) {
                            return read;
                        }
                        self.field(value, &args[0], side)
                    }
                    OPTION_PRESENT if args.len() == 1 => {
                        let option = self.option_operand(&args[0], side)?;
                        Some(ClauseTerm::Input(option.presence))
                    }
                    OPTION_VALUE if args.len() == 1 => {
                        let option = self.option_operand(&args[0], side)?;
                        let [payload] = option.payload.as_slice() else {
                            return None;
                        };
                        Some(ClauseTerm::OptionValue {
                            presence: option.presence,
                            payload: payload.clone(),
                        })
                    }
                    CONTROL_LET => self.let_term(args, side),
                    CONTROL_IF => {
                        let [condition, then, other] = args.as_slice() else {
                            return None;
                        };
                        Some(ClauseTerm::If {
                            condition: Box::new(self.term(condition, side)?),
                            then: Box::new(self.term(then, side)?),
                            other: Box::new(self.term(other, side)?),
                        })
                    }
                    CONTROL_CASE => self.case_term(args, side),
                    _ => {
                        let arguments = args
                            .iter()
                            .map(|arg| self.term(arg, side))
                            .collect::<Option<Vec<_>>>()?;
                        Some(ClauseTerm::Operation {
                            identity: identity.to_owned(),
                            arguments,
                            result_bound,
                        })
                    }
                }
            }
            _ => None,
        }
    }

    /// The binder a read of the parameter node `id` resolves to under the `let` scopes in force: a
    /// binder hides every parameter of the same declared name, and the innermost one wins.
    fn binder(&self, id: &CheckedNodeId) -> Option<usize> {
        let name = self
            .package
            .parameter_declaration(id)
            .ok()?
            .name
            .to_string();
        self.scopes
            .iter()
            .rev()
            .find_map(|(bound, binder)| (*bound == name).then_some(*binder))
    }

    /// The presence draw and payload leaves of the option parameter `operand` reads, recording
    /// every draw as a clause input.
    fn option_operand(
        &mut self,
        operand: &CheckedTerm,
        side: ClauseInputSide,
    ) -> Option<OptionOperand> {
        let id = reference(operand)?.clone();
        let node = self.node(&id)?;
        if &*node.node_tag != "value"
            || &*node.semantic_form != "parameter"
            || side != ClauseInputSide::Current
            || !self.allowed_parameters.contains(&id)
        {
            return None;
        }
        let OptionShape::Leaves(leaves) = option_shape(self.nodes, &node.semantic_type)? else {
            return None;
        };
        let declaration = self.package.parameter_declaration(&id).ok()?;
        let input = |leaf: ClauseLeaf, boolean: bool, lower: i128, upper: i128| ClauseInput {
            name: declaration.name.to_string(),
            leaf,
            side: ClauseInputSide::Parameter,
            declaring_node_id: None,
            boolean,
            lower,
            upper,
        };
        let presence = input(ClauseLeaf::Presence, true, 0, 1);
        let payload = leaves
            .iter()
            .map(|leaf| {
                input(
                    ClauseLeaf::Payload(leaf.path.clone()),
                    leaf.boolean,
                    leaf.lower,
                    leaf.upper,
                )
            })
            .collect::<Vec<_>>();
        self.inputs.insert(presence.clone());
        self.inputs.extend(payload.iter().cloned());
        Some(OptionOperand { presence, payload })
    }

    /// `quire.op.record.project` of `quire.op.option.value` of an option parameter: the payload
    /// leaf named by the projected field. `None` when `deref` is not such a read; `Some(None)`
    /// when it is and the read cannot be lowered.
    fn option_leaf_read(
        &mut self,
        project: &CheckedTerm,
        deref: &CheckedTerm,
        side: ClauseInputSide,
    ) -> Option<Option<ClauseTerm>> {
        let read = self.term_body(deref)?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &read
        else {
            return None;
        };
        if operation.identity()? != OPTION_VALUE {
            return None;
        }
        let CheckedTerm::Application { operation, .. } = project else {
            return Some(None);
        };
        let Some((_, name)) = operation.projection_field().ok().flatten() else {
            return Some(None);
        };
        let [argument] = arguments.as_slice() else {
            return Some(None);
        };
        let Some(option) = self.option_operand(argument, side) else {
            return Some(None);
        };
        let payload = option.payload.iter().find(|leaf| {
            matches!(&leaf.leaf, ClauseLeaf::Payload(path) if path == name)
        });
        Some(payload.map(|payload| ClauseTerm::OptionValue {
            presence: option.presence.clone(),
            payload: payload.clone(),
        }))
    }

    fn let_term(&mut self, args: &[CheckedTerm], side: ClauseInputSide) -> Option<ClauseTerm> {
        let [CheckedTerm::Binding { name, value }, body] = args else {
            return None;
        };
        // The value is outside the binder's scope; the body is inside it.
        let value = self.term(value, side)?;
        let binder = self.next_binder;
        self.next_binder = self.next_binder.checked_add(1)?;
        self.scopes.push((name.to_string(), binder));
        let body = self.term(body, side);
        self.scopes.pop();
        Some(ClauseTerm::Let {
            binder,
            value: Box::new(value),
            body: Box::new(body?),
        })
    }

    fn case_term(&mut self, args: &[CheckedTerm], side: ClauseInputSide) -> Option<ClauseTerm> {
        let (scrutinee, arms) = args.split_first()?;
        let id = reference(scrutinee)?.clone();
        let node = self.node(&id)?;
        if &*node.node_tag != "value"
            || &*node.semantic_form != "parameter"
            || side != ClauseInputSide::Current
            || !self.allowed_parameters.contains(&id)
        {
            return None;
        }
        let members = union_members(self.package, self.nodes, &node.semantic_type)?;
        if members.iter().any(|(_, arity)| *arity != 0) || members.len() != arms.len() {
            return None;
        }
        let declaration = self.package.parameter_declaration(&id).ok()?;
        let input = ClauseInput {
            name: declaration.name.to_string(),
            leaf: ClauseLeaf::Whole,
            side: ClauseInputSide::Parameter,
            declaring_node_id: None,
            boolean: false,
            lower: 0,
            upper: i128::try_from(members.len()).ok()?.checked_sub(1)?,
        };
        let mut ordered = vec![None; members.len()];
        for arm in arms {
            let CheckedTerm::Binding { name, value } = arm else {
                return None;
            };
            let position = members
                .iter()
                .position(|(member, _)| member == name.as_ref())?;
            let CheckedTerm::Aggregate { members: parts } = value.as_ref() else {
                return None;
            };
            let [CheckedTerm::Aggregate { members: binders }, body] = parts.as_slice() else {
                return None;
            };
            if !binders.is_empty() || ordered[position].is_some() {
                return None;
            }
            ordered[position] = Some(self.term(body, side)?);
        }
        self.inputs.insert(input.clone());
        Some(ClauseTerm::Case {
            scrutinee: input,
            arms: ordered.into_iter().collect::<Option<Vec<_>>>()?,
        })
    }

    fn field(
        &mut self,
        project: &CheckedTerm,
        deref: &CheckedTerm,
        side: ClauseInputSide,
    ) -> Option<ClauseTerm> {
        let CheckedTerm::Application { operation, .. } = project else {
            return None;
        };
        let (declaration, name) = operation.projection_field().ok()??;
        if !matches!(side, ClauseInputSide::Current | ClauseInputSide::Pre) {
            return None;
        }
        let deref = self.term_body(deref)?;
        let CheckedTerm::Application { result_type, .. } = &deref else {
            return None;
        };
        if result_type != declaration {
            return None;
        }
        let [subject] = application(&deref, ApplicationOperator::Deref, "quire.op.model.deref")?
        else {
            return None;
        };
        if reference(subject)? != &self.self_id {
            return None;
        }
        let fields = self.package.model_object_fields(declaration).ok()?;
        let field = fields.field(name)?;
        let (boolean, lower, upper) = match field.member_type()? {
            CheckedMemberType::Boolean => (true, 0, 1),
            CheckedMemberType::IntRange { lower, upper } => (false, *lower, *upper),
            _ => return None,
        };
        let input = ClauseInput {
            name: name.to_owned(),
            leaf: ClauseLeaf::Whole,
            side,
            declaring_node_id: Some(field.declaring_node_id().clone()),
            boolean,
            lower,
            upper,
        };
        self.inputs.insert(input.clone());
        Some(ClauseTerm::Input(input))
    }

    fn term_body(&self, value: &CheckedTerm) -> Option<CheckedTerm> {
        match reference(value) {
            Some(id) => Some(self.package.term(id)?.clone()),
            None => Some(value.clone()),
        }
    }
}

/// The ordered members of the union declared by `type_node`, each with its payload arity, or `None`
/// when the node is not a union declaration.
pub(crate) fn union_members(
    package: &CheckedPackageV2,
    nodes: &ClauseGraphView<'_>,
    type_node: &CheckedNodeId,
) -> Option<Vec<(String, usize)>> {
    let declaration = nodes.node(type_node)?;
    if &*declaration.node_tag != "composite_type" || &*declaration.semantic_form != "union" {
        return None;
    }
    let CheckedTerm::Aggregate { members } = package.term(type_node)? else {
        return None;
    };
    members
        .iter()
        .map(|member| {
            let CheckedTerm::Binding { name, value } = member else {
                return None;
            };
            let CheckedTerm::Aggregate { members: payload } = value.as_ref() else {
                return None;
            };
            Some((name.to_string(), payload.len()))
        })
        .collect()
}

pub(crate) fn read_clause(
    id: &CheckedNodeId,
    nodes: &ClauseGraphView<'_>,
) -> Option<ClauseCondition> {
    let package = nodes.package();
    let clause = nodes.node(id)?;
    if &*clause.node_tag != "state" || &*clause.semantic_form != "state_clause" {
        return None;
    }
    let [parameters, _anchor, condition] = application(
        package.term(id)?,
        ApplicationOperator::StateClause,
        "quire.op.state.clause",
    )?
    else {
        return None;
    };
    let CheckedTerm::Aggregate { members: declared } = parameters else {
        return None;
    };
    let self_id = declared.first().and_then(reference)?.clone();
    if package.parameter_declaration(&self_id).ok()?.name.as_ref() != "self" {
        return None;
    }
    let result_id = declared.get(1).and_then(reference).and_then(|id| {
        (package.parameter_declaration(id).ok()?.name.as_ref() == "result").then(|| id.clone())
    });
    let allowed_parameters = declared
        .iter()
        .skip(if result_id.is_some() { 2 } else { 1 })
        .map(|term| reference(term).cloned())
        .collect::<Option<_>>()?;
    let mut reader = Reader {
        package,
        nodes,
        self_id,
        result_id,
        allowed_parameters,
        inputs: BTreeSet::new(),
        visiting: BTreeSet::new(),
        visits: 0,
        scopes: Vec::new(),
        next_binder: 0,
    };
    let term = reader.term(condition, ClauseInputSide::Current)?;
    Some(ClauseCondition {
        term,
        inputs: reader.inputs.into_iter().collect(),
        condition_node_id: reference(condition).cloned(),
    })
}

#[cfg(test)]
#[path = "clause_control_fixture.rs"]
pub(crate) mod control_fixture;
#[cfg(test)]
#[path = "clause_tests.rs"]
mod tests;
