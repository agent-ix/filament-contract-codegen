//! Admitted checked-package state-clause body access.

use std::collections::{BTreeMap, BTreeSet};

use super::checked_term::{application, reference};
use quire_contract_model::{
    ApplicationOperator, CheckedMemberType, CheckedNodeId, CheckedNodeKind, CheckedPackageV2,
    CheckedSemanticNodeV2, CheckedTerm, LiteralKind,
};

pub(crate) const MAX_CONDITION_TERMS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ClauseInputSide {
    Current,
    Pre,
    Parameter,
    Result,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ClauseInput {
    pub(crate) name: String,
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
        let CheckedTerm::Aggregate { members } = self.package.term(id)? else {
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
                        self.field(value, &args[0], side)
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

/// Packages compiled from FSL functions whose bodies hold the control applications.
#[cfg(test)]
pub(crate) mod control_fixture {
    use quire_contract_model::{
        CheckedNodeId, CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
        CheckedPackageV2ReadResult, CheckedTerm,
    };

    const SOURCE: &str = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "union Kind { A, B, C }\n",
        "union Shape { Empty, Circle(Integer) }\n",
        "function pick using v(k: Kind): Integer pure { case k { C: 3; A: 1; B: 2; } }\n",
        "function words using v(k: Kind, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { case k { A: t; B: t; C: t; } }\n",
        "function area using v(s: Shape): Integer pure { case s { Empty: 0; Circle(r): r; } }\n",
        "function texts using v(c: Boolean, t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { if c then t else t }\n",
        "function plain using v(c: Boolean): Integer pure { if c then 1 else 2 }\n",
    );

    pub(crate) fn package() -> CheckedPackageV2 {
        let compiled = qsl_replay::compile_admitted(
            qsl_replay::CompileUnit {
                source: qsl_replay::SourceIdentity::new("test", "controls.native"),
                package_name: "controls".to_owned(),
                path: "controls.native",
                bytes: SOURCE.as_bytes(),
            },
            [],
            &qsl_replay::DependencyInput::default(),
            qsl_replay::StageLimits::default(),
            qsl_replay::ReplayLimits::default(),
        )
        .expect("the control functions compile");
        let mut evidence = CheckedPackageEvidence::new();
        evidence.support_feature("quire.value.complete/v1");
        let read = CheckedPackageV2::read(
            compiled.package().bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        let CheckedPackageV2ReadResult::Admitted(package) = read else {
            panic!("the control package is admitted: {read:?}")
        };
        *package
    }

    /// The arguments of every application of `identity`, in node order.
    pub(crate) fn applications<'a>(
        package: &'a CheckedPackageV2,
        identity: &str,
    ) -> Vec<&'a [CheckedTerm]> {
        package
            .resolved_graph()
            .nodes
            .iter()
            .filter_map(|node| package.term(&node.node_id))
            .filter_map(|term| match term {
                CheckedTerm::Application {
                    operation,
                    arguments,
                    ..
                } if operation.identity() == Some(identity) => Some(arguments.as_slice()),
                _ => None,
            })
            .collect()
    }

    /// The parameter node a control's scrutinee or condition reads.
    pub(crate) fn read_node(term: &CheckedTerm) -> &CheckedNodeId {
        match term {
            CheckedTerm::Reference { target } => target,
            other => panic!("expected a reference, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::control_fixture::{applications, package, read_node};
    use super::*;
    use ix_trace_rs::trace;

    fn reader<'a, 'view>(
        package: &'a CheckedPackageV2,
        nodes: &'view ClauseGraphView<'a>,
        allowed: &[&CheckedNodeId],
        scopes: Vec<(String, usize)>,
    ) -> Reader<'a, 'view> {
        Reader {
            package,
            nodes,
            self_id: allowed[0].clone(),
            result_id: None,
            allowed_parameters: allowed.iter().map(|id| (*id).clone()).collect(),
            inputs: BTreeSet::new(),
            visiting: BTreeSet::new(),
            visits: 0,
            scopes,
            next_binder: 0,
        }
    }

    /// The `case` applications are in source order: `pick`, `words`, `area`.
    fn case_arguments(package: &CheckedPackageV2, position: usize) -> &[CheckedTerm] {
        applications(package, CONTROL_CASE)[position]
    }

    #[trace("FR-015-AC-154", "TC-025")]
    #[test]
    fn union_members_lists_each_member_with_its_payload_arity_in_declaration_order() {
        let package = package();
        let view = ClauseGraphView::new(&package);
        let kind = view
            .node(read_node(&case_arguments(&package, 0)[0]))
            .unwrap()
            .semantic_type
            .clone();
        assert_eq!(
            union_members(&package, &view, &kind),
            Some(vec![
                ("A".to_owned(), 0),
                ("B".to_owned(), 0),
                ("C".to_owned(), 0)
            ])
        );
        let shape = view
            .node(read_node(&case_arguments(&package, 2)[0]))
            .unwrap()
            .semantic_type
            .clone();
        assert_eq!(
            union_members(&package, &view, &shape),
            Some(vec![("Empty".to_owned(), 0), ("Circle".to_owned(), 1)])
        );
        // A scalar type node is not a union declaration.
        let scalar = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &*node.node_tag == "scalar_type")
            .unwrap()
            .node_id
            .clone();
        assert_eq!(union_members(&package, &view, &scalar), None);
    }

    #[trace("FR-015-AC-154", "TC-025")]
    #[test]
    fn a_case_reads_one_arm_per_member_in_declaration_order_whatever_the_source_order() {
        let package = package();
        let view = ClauseGraphView::new(&package);
        let arguments = case_arguments(&package, 0);
        let parameter = read_node(&arguments[0]);
        let mut reader = reader(&package, &view, &[parameter], Vec::new());
        // The source writes the arms `C`, `A`, `B`.
        let term = reader
            .case_term(arguments, ClauseInputSide::Current)
            .expect("an arity-0 union case reads");
        let ClauseTerm::Case { scrutinee, arms } = term else {
            panic!("expected a case")
        };
        assert_eq!((scrutinee.lower, scrutinee.upper), (0, 2));
        assert_eq!(scrutinee.name, "k");
        let values: Vec<_> = arms
            .iter()
            .map(|arm| match arm {
                ClauseTerm::Integer(value) => value.as_str(),
                other => panic!("expected an integer arm, got {other:?}"),
            })
            .collect();
        assert_eq!(
            values,
            ["1", "2", "3"],
            "arms follow the declared members A, B, C"
        );
    }

    #[trace("FR-015-AC-154", "FR-015-AC-171", "TC-025")]
    #[test]
    fn a_case_is_refused_for_a_payload_a_duplicate_arm_a_wrong_side_or_a_foreign_scrutinee() {
        let package = package();
        let view = ClauseGraphView::new(&package);
        let pick = case_arguments(&package, 0);
        let parameter = read_node(&pick[0]);
        let current = ClauseInputSide::Current;
        // A payload-carrying member.
        let area = case_arguments(&package, 2);
        let mut payload = reader(&package, &view, &[read_node(&area[0])], Vec::new());
        assert!(payload.case_term(area, current).is_none());
        // A duplicate arm: three arms, one member twice.
        let mut duplicated = pick.to_vec();
        duplicated[3] = duplicated[1].clone();
        let mut reader_dup = reader(&package, &view, &[parameter], Vec::new());
        assert!(reader_dup.case_term(&duplicated, current).is_none());
        // A missing arm.
        let mut reader_short = reader(&package, &view, &[parameter], Vec::new());
        assert!(reader_short.case_term(&pick[..3], current).is_none());
        // The scrutinee read on another side.
        let mut reader_pre = reader(&package, &view, &[parameter], Vec::new());
        assert!(reader_pre.case_term(pick, ClauseInputSide::Pre).is_none());
        // A parameter the clause does not declare.
        let other = read_node(&area[0]);
        let mut foreign = reader(&package, &view, &[other], Vec::new());
        assert!(foreign.case_term(pick, current).is_none());
    }

    #[trace("FR-015-AC-170", "FR-015-AC-173", "TC-025")]
    #[test]
    fn a_parameter_read_resolves_to_the_innermost_binder_of_its_declared_name() {
        let package = package();
        let view = ClauseGraphView::new(&package);
        let pick = case_arguments(&package, 0);
        let parameter = read_node(&pick[0]);
        let resolve = |scopes: Vec<(String, usize)>| {
            reader(&package, &view, &[parameter], scopes).binder(parameter)
        };
        assert_eq!(
            resolve(Vec::new()),
            None,
            "no enclosing let: the parameter itself"
        );
        assert_eq!(resolve(vec![("k".to_owned(), 5)]), Some(5));
        assert_eq!(
            resolve(vec![("k".to_owned(), 5), ("k".to_owned(), 7)]),
            Some(7),
            "the innermost binder wins"
        );
        assert_eq!(
            resolve(vec![("k".to_owned(), 5), ("other".to_owned(), 7)]),
            Some(5),
            "a binder of another name does not hide it"
        );
        assert_eq!(resolve(vec![("other".to_owned(), 1)]), None);
    }
}
