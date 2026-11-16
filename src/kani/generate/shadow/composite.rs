//! The composite-equality family of the bounded shadow (FR-015-AC-69 to FR-015-AC-76).
//!
//! The shadow is the equality walk of FSpec FR-149's occurrence-pair plan over the fixed record,
//! tuple and option shape the item's declaration closure describes, written as straight-line Rust
//! over `bool`, `i64`, `Option` and plain structs. It imports nothing from the evaluator the
//! production oracle calls. The expectation is a flat expression over the drawn leaves; it shares
//! no function with the shadow. What the two share, and the production oracle shares with them,
//! is the closure the claim map records (FR-018-AC-21), which FR-018-AC-23 checks against an
//! independent read.

use crate::oracle::claim::ClaimDisposition;
use crate::{
    core::{artifact::Artifact, ir::node_key::render_checked_node_id},
    kani::{
        abi::{adapter_options, witness_options, KaniSolver},
        generate::render,
        identity::{
            leaf_binding::{check_leaf_bindings, LeafBindingRefusal},
            refinement::{DomainBound, ProofSubject},
            shadow::{
                AbstractionEvidence, AbstractionRecord, BackingState, CompositeIdentity,
                HelperMark, KaniShadowHarness, LeafKind, LeafRecord, OperandSource, PinnedLeaf,
                ProductionRead, RefinementObligation, ShadowIdentity, SharedHelperClaim,
                UnexercisedCoverage,
            },
            ProofCeilings,
        },
    },
    oracle::equality::{
        closure::{RecordedDeclaration, RecordedForm, RecordedPresence, RecordedType},
        CompositeEqualityClaim, CompositeEqualityRefusal, EqualityOperatorKind,
    },
};
use filament_contract_model::CheckedNodeId;
use serde::Serialize;

use super::{
    is_identifier, is_symbol, source_check, Hooks, ShadowDisposition, SHADOW_PAIR_NODE_BUDGET,
};

/// The family name the identity records.
pub const COMPOSITE_FAMILY: &str = "composite_equality";

/// The module the shadow and the expectation live in.
pub const COMPOSITE_SHADOW_MODULE: &str = "shadow_composite";

const ABSTRACTION_MODULE: &str = "abstraction";
const PROOF_MODULE: &str = "shadow_proof";
const PROOF_FUNCTION: &str = "prove_shadow";

/// The native criteria that cover the behaviours the proof does not exercise, in the order the
/// identity lists them.
pub const UNEXERCISED_COVERAGE: [(&str, &str); 5] = [
    ("meter_limits", "FR-018-AC-9"),
    ("ill_typed_operands", "FR-018-AC-8"),
    ("charge_amounts_other_than_pairs", "FR-018-AC-2"),
    ("foreign_reference", "FR-018-AC-7"),
    ("declaration_reconstruction", "FR-018-AC-23"),
];

/// One operand of the claim.
#[derive(Clone, Debug)]
pub enum ShadowOperand {
    /// A parameter of the claimed expression: its leaves are drawn.
    Parameter {
        /// The parameter's checked node id.
        node: CheckedNodeId,
        /// The parameter's name, the prefix of its leaves.
        name: String,
    },
    /// A literal: its leaves are constrained to exactly these values, one per leaf in leaf
    /// order, in the encoding of the refinement domain.
    Literal {
        /// The leaf values.
        leaves: Vec<i64>,
    },
    /// Anything else, named by its node.
    Computed {
        /// The operand's node.
        node: String,
    },
}

/// One composite-equality claim for the shadow generator.
#[derive(Clone, Debug)]
pub struct CompositeShadowItem<'a> {
    /// The claim-map entry FR-018 recorded for the item.
    pub entry: &'a CompositeEqualityClaim,
    /// The left operand.
    pub left: ShadowOperand,
    /// The right operand.
    pub right: ShadowOperand,
    /// The production oracle's symbol; the obligation names it.
    pub subject: String,
    /// The request ceilings.
    pub ceilings: ProofCeilings,
    /// The loop unwind bound.
    pub unwind: u32,
    /// For each covering criterion, whether the test matrix lists it covered.
    pub backing: Vec<(String, bool)>,
}

/// Why a composite-equality claim settles without a harness (FR-015-AC-73).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum CompositeShadowRefusal {
    /// FR-018 refused the item; its refusal, unchanged.
    Oracle {
        /// FR-018's refusal.
        refusal: CompositeEqualityRefusal,
    },
    /// An integer leaf has no declared bound, or a declaration reaches itself.
    RequiresBound {
        /// The unbounded node.
        node: String,
    },
    /// A bound has an endpoint outside `i64`.
    DomainNotRepresentableInI64 {
        /// The bound's node.
        node: String,
    },
    /// A leaf or member of a family no shadow is built for.
    ShadowFamilyNotBuilt {
        /// The family.
        family: String,
        /// The node.
        node: String,
    },
    /// An operand is not a parameter or a literal, is converted, or is not a composite.
    OperandNotSupported {
        /// The operand's node.
        node: String,
    },
    /// The closure is over the family's size budget.
    ShadowShapeOverBudget {
        /// The closure's pair-node count.
        count: u32,
        /// The budget.
        budget: u32,
    },
    /// A name is not a plain Rust identifier, or two leaves have the same name.
    LeafName {
        /// The name.
        name: String,
    },
    /// The obligation would name no production subject.
    NoProductionSubject,
    /// A literal operand does not give one value per leaf.
    LiteralLeaves {
        /// The leaves the type has.
        expected: usize,
        /// The values given.
        given: usize,
    },
    /// The identity's leaf bindings, or the draws of the generated source, do not match the
    /// shape (FR-025-AC-9).
    LeafBinding {
        /// The refusal.
        refusal: LeafBindingRefusal,
    },
    /// The rendered source fails a parsed-source check.
    SourceCheck {
        /// The violation.
        violation: String,
    },
}

impl CompositeShadowRefusal {
    /// The disposition of the refusal.
    #[must_use]
    pub fn disposition(&self) -> ShadowDisposition {
        match self {
            Self::RequiresBound { .. } => ShadowDisposition::RequiresBound,
            Self::DomainNotRepresentableInI64 { .. }
            | Self::ShadowFamilyNotBuilt { .. }
            | Self::OperandNotSupported { .. }
            | Self::ShadowShapeOverBudget { .. } => ShadowDisposition::Unsupported,
            _ => ShadowDisposition::Refused,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Ty {
    Bool,
    Int(i64, i64),
    Option(Box<Ty>),
    Decl(usize),
}

#[derive(Debug)]
struct Member {
    name: String,
    optional: bool,
    ty: Ty,
}

#[derive(Debug)]
struct Decl {
    key: String,
    tuple: bool,
    members: Vec<Member>,
}

struct Model<'a> {
    closure: &'a [RecordedDeclaration],
    decls: Vec<Decl>,
    keys: Vec<usize>,
    stack: Vec<usize>,
}

fn node_text(node: &CheckedNodeId) -> String {
    render_checked_node_id(node).unwrap_or_else(|| format!("{node:?}"))
}

impl<'a> Model<'a> {
    fn resolve(&mut self, recorded: &RecordedType, at: &str) -> Result<Ty, CompositeShadowRefusal> {
        match recorded {
            RecordedType::Boolean => Ok(Ty::Bool),
            RecordedType::Integer => Err(CompositeShadowRefusal::RequiresBound {
                node: at.to_owned(),
            }),
            RecordedType::Int { lower, upper } => {
                let parse = |text: &str| text.parse::<i64>().ok();
                match (parse(lower), parse(upper)) {
                    (Some(lower), Some(upper)) if lower <= upper => Ok(Ty::Int(lower, upper)),
                    _ => Err(CompositeShadowRefusal::DomainNotRepresentableInI64 {
                        node: at.to_owned(),
                    }),
                }
            }
            RecordedType::Option { payload } => {
                Ok(Ty::Option(Box::new(self.resolve(payload, at)?)))
            }
            RecordedType::Composite { key } => self.declaration(key, at),
            RecordedType::Payload { .. } => Err(CompositeShadowRefusal::ShadowFamilyNotBuilt {
                family: "union".to_owned(),
                node: at.to_owned(),
            }),
            RecordedType::Other { name } => Err(CompositeShadowRefusal::ShadowFamilyNotBuilt {
                family: (*name).to_owned(),
                node: at.to_owned(),
            }),
        }
    }

    fn declaration(&mut self, key: &str, at: &str) -> Result<Ty, CompositeShadowRefusal> {
        let position = self
            .closure
            .iter()
            .position(|declaration| declaration.key == key)
            .ok_or_else(|| CompositeShadowRefusal::OperandNotSupported {
                node: at.to_owned(),
            })?;
        let declaration = &self.closure[position];
        if self.stack.contains(&position) {
            return Err(CompositeShadowRefusal::RequiresBound {
                node: node_text(&declaration.node_id),
            });
        }
        if let Some(index) = self.keys.iter().position(|seen| *seen == position) {
            return Ok(Ty::Decl(index));
        }
        let tuple = match declaration.form {
            RecordedForm::Record => false,
            RecordedForm::Tuple => true,
            RecordedForm::Union => {
                return Err(CompositeShadowRefusal::ShadowFamilyNotBuilt {
                    family: "union".to_owned(),
                    node: node_text(&declaration.node_id),
                })
            }
        };
        let node = node_text(&declaration.node_id);
        let index = self.decls.len();
        self.keys.push(position);
        self.decls.push(Decl {
            key: key.to_owned(),
            tuple,
            members: Vec::new(),
        });
        self.stack.push(position);
        let mut members = Vec::new();
        for member in &declaration.members {
            let ty = self.resolve(&member.value_type, &format!("{node}.{}", member.name))?;
            members.push(Member {
                name: member.name.clone(),
                optional: member.presence == RecordedPresence::Optional,
                ty,
            });
        }
        self.stack.pop();
        self.decls[index].members = members;
        Ok(Ty::Decl(index))
    }
}

fn pair_nodes(model: &Model<'_>, ty: &Ty) -> u32 {
    match ty {
        Ty::Bool | Ty::Int(..) => 1,
        Ty::Option(payload) => 1 + pair_nodes(model, payload),
        Ty::Decl(index) => {
            1 + model.decls[*index]
                .members
                .iter()
                .map(|member| pair_nodes(model, &member.ty))
                .sum::<u32>()
        }
    }
}

/// The pair-node count of the largest occurrence-pair tree the type of a closure admits.
fn closure_pair_nodes(model: &Model<'_>, root: &Ty) -> u32 {
    pair_nodes(model, root)
}

#[derive(Clone, Debug)]
struct Leaf {
    name: String,
    path: String,
    kind: LeafKind,
    bound: (i64, i64),
    when: Option<String>,
}

impl Leaf {
    fn is_bool(&self) -> bool {
        matches!(self.kind, LeafKind::Boolean | LeafKind::Presence)
    }
}

fn conjoin(when: &Option<String>, condition: &str) -> String {
    match when {
        Some(earlier) => format!("{earlier} && {condition}"),
        None => condition.to_owned(),
    }
}

fn enumerate(
    model: &Model<'_>,
    ty: &Ty,
    name: &str,
    path: &str,
    when: &Option<String>,
    out: &mut Vec<Leaf>,
) {
    let leaf = |kind, bound, name: String, path: String| Leaf {
        name,
        path,
        kind,
        bound,
        when: when.clone(),
    };
    match ty {
        Ty::Bool => out.push(leaf(
            LeafKind::Boolean,
            (0, 1),
            name.to_owned(),
            path.to_owned(),
        )),
        Ty::Int(lower, upper) => out.push(leaf(
            LeafKind::Integer,
            (*lower, *upper),
            name.to_owned(),
            path.to_owned(),
        )),
        Ty::Option(payload) => {
            let some = format!("{name}_some");
            out.push(leaf(
                LeafKind::Presence,
                (0, 1),
                some.clone(),
                format!("{path}.some"),
            ));
            let inner = Some(conjoin(when, &some));
            enumerate(
                model,
                payload,
                &format!("{name}_value"),
                &format!("{path}.value"),
                &inner,
                out,
            );
        }
        Ty::Decl(index) => {
            for member in &model.decls[*index].members {
                let member_name = format!("{name}_{}", member.name);
                let member_path = format!("{path}.{}", member.name);
                if member.optional {
                    let state = format!("{member_name}_state");
                    out.push(leaf(
                        LeafKind::SlotState,
                        (0, 2),
                        state.clone(),
                        format!("{member_path}.state"),
                    ));
                    let inner = Some(conjoin(when, &format!("{state} == 0")));
                    enumerate(model, &member.ty, &member_name, &member_path, &inner, out);
                } else {
                    enumerate(model, &member.ty, &member_name, &member_path, when, out);
                }
            }
        }
    }
}

fn rust_type(ty: &Ty) -> String {
    match ty {
        Ty::Bool => "bool".to_owned(),
        Ty::Int(..) => "i64".to_owned(),
        Ty::Option(payload) => format!("Option<{}>", rust_type(payload)),
        Ty::Decl(index) => format!("Decl{index}"),
    }
}

fn is_leafy(ty: &Ty) -> bool {
    matches!(ty, Ty::Bool | Ty::Int(..))
}

fn slug(ty: &Ty) -> String {
    rust_type(ty)
        .to_ascii_lowercase()
        .replace(['<', '>'], "_")
        .trim_end_matches('_')
        .to_owned()
}

fn walk_name(ty: &Ty) -> String {
    match ty {
        Ty::Decl(index) => format!("walk_decl{index}"),
        Ty::Option(_) => format!("walk_{}", slug(ty)),
        Ty::Bool | Ty::Int(..) => "compare_leaf".to_string(),
    }
}

fn pair_expression(ty: &Ty, left: &str, right: &str) -> String {
    if is_leafy(ty) {
        format!("({left} == {right}, 1)")
    } else {
        format!("{}({left}, {right})", walk_name(ty))
    }
}

fn collect_options(ty: &Ty, out: &mut Vec<Ty>) {
    if let Ty::Option(payload) = ty {
        if !out.contains(ty) {
            out.push(ty.clone());
        }
        collect_options(payload, out);
    }
}

fn render_walks(model: &Model<'_>, root: &Ty) -> String {
    let mut options: Vec<Ty> = Vec::new();
    collect_options(root, &mut options);
    for decl in &model.decls {
        for member in &decl.members {
            collect_options(&member.ty, &mut options);
        }
    }
    let mut out = String::new();
    let mut functions = String::new();
    for (index, decl) in model.decls.iter().enumerate() {
        out.push_str(&format!(
            "    #[derive(Clone, Copy)]\n    pub struct Decl{index} {{\n"
        ));
        for (position, member) in decl.members.iter().enumerate() {
            let ty = rust_type(&member.ty);
            let ty = if member.optional {
                format!("Slot<{ty}>")
            } else {
                ty
            };
            out.push_str(&format!("        pub f{position}: {ty},\n"));
        }
        out.push_str("    }\n\n");
        functions.push_str(&format!(
            "    pub fn walk_decl{index}(l: Decl{index}, r: Decl{index}) -> (bool, u64) {{\n"
        ));
        let mut verdicts = Vec::new();
        let mut counts = Vec::new();
        for (position, member) in decl.members.iter().enumerate() {
            let (l, r) = (format!("l.f{position}"), format!("r.f{position}"));
            if member.optional {
                let inner = pair_expression(&member.ty, "x", "y");
                functions.push_str(&format!(
                    "        let p{position} = match ({l}, {r}) {{\n            (Slot::Absent, Slot::Absent) | (Slot::Null, Slot::Null) => (true, 1),\n            (Slot::Present(x), Slot::Present(y)) => {inner},\n            _ => (false, 1),\n        }};\n"
                ));
            } else {
                let inner = pair_expression(&member.ty, &l, &r);
                functions.push_str(&format!("        let p{position} = {inner};\n"));
            }
            verdicts.push(format!("p{position}.0"));
            counts.push(format!("p{position}.1"));
        }
        let verdict = if verdicts.is_empty() {
            "true".to_owned()
        } else {
            verdicts.join(" && ")
        };
        let count = if counts.is_empty() {
            "1".to_owned()
        } else {
            format!("1 + {}", counts.join(" + "))
        };
        functions.push_str(&format!("        ({verdict}, {count})\n    }}\n\n"));
    }
    for ty in &options {
        let Ty::Option(payload) = ty else { continue };
        let rust = rust_type(ty);
        let name = walk_name(ty);
        let inner = pair_expression(payload, "x", "y");
        functions.push_str(&format!(
            "    pub fn {name}(l: {rust}, r: {rust}) -> (bool, u64) {{\n        match (l, r) {{\n            (Some(x), Some(y)) => {{\n                let inner = {inner};\n                (inner.0, 1 + inner.1)\n            }}\n            (None, None) => (true, 1),\n            _ => (false, 1),\n        }}\n    }}\n\n"
        ));
    }
    format!("{out}{functions}")
}

fn build_expression(model: &Model<'_>, ty: &Ty, name: &str) -> String {
    match ty {
        Ty::Bool | Ty::Int(..) => name.to_owned(),
        Ty::Option(payload) => format!(
            "if {name}_some {{ Some({}) }} else {{ None }}",
            build_expression(model, payload, &format!("{name}_value"))
        ),
        Ty::Decl(index) => {
            let fields: Vec<String> = model.decls[*index]
                .members
                .iter()
                .enumerate()
                .map(|(position, member)| {
                    let member_name = format!("{name}_{}", member.name);
                    let value = build_expression(model, &member.ty, &member_name);
                    if member.optional {
                        format!(
                            "f{position}: match {member_name}_state {{ 0 => Slot::Present({value}), 1 => Slot::Absent, _ => Slot::Null }}"
                        )
                    } else {
                        format!("f{position}: {value}")
                    }
                })
                .collect();
            format!("Decl{index} {{ {} }}", fields.join(", "))
        }
    }
}

fn expectation_terms(model: &Model<'_>, ty: &Ty, left: &str, right: &str) -> (String, String) {
    match ty {
        Ty::Bool | Ty::Int(..) => (format!("({left} == {right})"), "1".to_owned()),
        Ty::Option(payload) => {
            let (verdict, count) = expectation_terms(
                model,
                payload,
                &format!("{left}_value"),
                &format!("{right}_value"),
            );
            (
                format!(
                    "(({left}_some && {right}_some && {verdict}) || (!{left}_some && !{right}_some))"
                ),
                format!("(1 + if {left}_some && {right}_some {{ {count} }} else {{ 0 }})"),
            )
        }
        Ty::Decl(index) => {
            let mut verdicts = Vec::new();
            let mut counts = Vec::new();
            for member in &model.decls[*index].members {
                let l = format!("{left}_{}", member.name);
                let r = format!("{right}_{}", member.name);
                let (verdict, count) = expectation_terms(model, &member.ty, &l, &r);
                if member.optional {
                    verdicts.push(format!(
                        "(({l}_state == 0 && {r}_state == 0 && {verdict}) || ({l}_state == 1 && {r}_state == 1) || ({l}_state == 2 && {r}_state == 2))"
                    ));
                    counts.push(format!(
                        "(if {l}_state == 0 && {r}_state == 0 {{ {count} }} else {{ 1 }})"
                    ));
                } else {
                    verdicts.push(verdict);
                    counts.push(count);
                }
            }
            let verdict = if verdicts.is_empty() {
                "true".to_owned()
            } else {
                verdicts.join(" && ")
            };
            let count = if counts.is_empty() {
                "1".to_owned()
            } else {
                format!("1 + {}", counts.join(" + "))
            };
            (format!("({verdict})"), format!("({count})"))
        }
    }
}

fn value_type_expression(model: &Model<'_>, ty: &Ty) -> String {
    match ty {
        Ty::Bool => "exact::ValueType::Boolean".to_owned(),
        Ty::Int(lower, upper) => format!(
            "exact::ValueType::Int(exact::IntegerInterval::new(exact::Integer::from({lower}_i64), exact::Integer::from({upper}_i64)).map_err(|error| format!(\"{{error:?}}\"))?)"
        ),
        Ty::Option(payload) => format!(
            "exact::ValueType::option({})",
            value_type_expression(model, payload)
        ),
        Ty::Decl(index) => format!(
            "exact::ValueType::Composite(subject::declaration_key(environment, {:?})?)",
            model.decls[*index].key
        ),
    }
}

fn lift_expression(model: &Model<'_>, ty: &Ty, name: &str) -> String {
    match ty {
        Ty::Bool => format!("exact::Value::Boolean({name})"),
        Ty::Int(..) => format!("exact::Value::Integer(exact::Integer::from({name}))"),
        Ty::Option(payload) => {
            let payload_type = value_type_expression(model, payload);
            let inner = lift_expression(model, payload, &format!("{name}_value"));
            format!(
                "if {name}_some {{ exact::OptionValue::present({payload_type}, {inner}).map_err(|error| format!(\"{{error:?}}\"))? }} else {{ exact::OptionValue::none({payload_type}) }}"
            )
        }
        Ty::Decl(index) => {
            let decl = &model.decls[*index];
            let key = format!("subject::declaration_key(environment, {:?})?", decl.key);
            let fields: Vec<String> = decl
                .members
                .iter()
                .map(|member| {
                    let member_name = format!("{name}_{}", member.name);
                    let value = lift_expression(model, &member.ty, &member_name);
                    let slot = if member.optional {
                        format!(
                            "match {member_name}_state {{ 0 => exact::FieldValue::Present({value}), 1 => exact::FieldValue::Absent, _ => exact::FieldValue::Null }}"
                        )
                    } else {
                        format!("exact::FieldValue::Present({value})")
                    };
                    if decl.tuple {
                        slot
                    } else {
                        format!("({:?}, {slot})", member.name)
                    }
                })
                .collect();
            if decl.tuple {
                let positions: Vec<String> = decl
                    .members
                    .iter()
                    .map(|member| {
                        let member_name = format!("{name}_{}", member.name);
                        lift_expression(model, &member.ty, &member_name)
                    })
                    .collect();
                format!(
                    "subject::type_environment(environment).tuple({key}, vec![{}]).map_err(|error| format!(\"{{error:?}}\"))?",
                    positions.join(", ")
                )
            } else {
                format!(
                    "subject::type_environment(environment).record({key}, vec![{}]).map_err(|error| format!(\"{{error:?}}\"))?",
                    fields.join(", ")
                )
            }
        }
    }
}

fn parameter_list(leaves: &[&Leaf]) -> String {
    leaves
        .iter()
        .map(|leaf| {
            format!(
                "{}: {}",
                leaf.name,
                if leaf.is_bool() { "bool" } else { "i64" }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct Operand<'a> {
    prefix: String,
    leaves: Vec<Leaf>,
    pinned: Option<&'a [i64]>,
    node: Option<&'a CheckedNodeId>,
}

fn operand_plan<'a>(
    model: &Model<'_>,
    root: &Ty,
    operand: &'a ShadowOperand,
    literal_prefix: &str,
    operand_node: &str,
) -> Result<Operand<'a>, CompositeShadowRefusal> {
    let (prefix, pinned, node) = match operand {
        ShadowOperand::Parameter { node, name } => {
            if !is_identifier(name) {
                return Err(CompositeShadowRefusal::LeafName { name: name.clone() });
            }
            (name.clone(), None, Some(node))
        }
        ShadowOperand::Literal { leaves } => {
            (literal_prefix.to_owned(), Some(leaves.as_slice()), None)
        }
        ShadowOperand::Computed { node } => {
            return Err(CompositeShadowRefusal::OperandNotSupported { node: node.clone() })
        }
    };
    let mut leaves = Vec::new();
    enumerate(model, root, &prefix, "", &None, &mut leaves);
    if let Some(values) = pinned {
        if values.len() != leaves.len() {
            return Err(CompositeShadowRefusal::LiteralLeaves {
                expected: leaves.len(),
                given: values.len(),
            });
        }
        for (leaf, value) in leaves.iter().zip(values) {
            if *value < leaf.bound.0 || *value > leaf.bound.1 {
                return Err(CompositeShadowRefusal::OperandNotSupported {
                    node: operand_node.to_owned(),
                });
            }
        }
    }
    Ok(Operand {
        prefix,
        leaves,
        pinned,
        node,
    })
}

fn render_abstraction(
    model: &Model<'_>,
    root: &Ty,
    left: &Operand<'_>,
    right: &Operand<'_>,
    shared: bool,
) -> String {
    let mut out = format!(
        "mod {ABSTRACTION_MODULE} {{
    use super::subject::{{self, exact}};

    /// Builds the production values of both operands through the production oracle's own
    /// environment and its declaration-checked constructors; a refusal is a construction
    /// refusal.
"
    );
    let left_leaves: Vec<&Leaf> = left.leaves.iter().collect();
    let right_leaves: Vec<&Leaf> = right.leaves.iter().collect();
    let mut parameters = parameter_list(&left_leaves);
    if !shared {
        if !parameters.is_empty() {
            parameters.push_str(", ");
        }
        parameters.push_str(&parameter_list(&right_leaves));
    }
    let left_value = lift_expression(model, root, &left.prefix);
    let right_value = lift_expression(model, root, &right.prefix);
    out.push_str(&format!(
        "    #[allow(clippy::too_many_arguments)]
    pub fn lift(
        environment: &subject::Environment,
        {parameters},
    ) -> Result<(exact::Value, exact::Value), String> {{
        let left = {left_value};
        let right = {right_value};
        Ok((left, right))
    }}
}}
"
    ));
    out
}

const DRIVER: &str =
    "// Generated native refinement program: the production subject against the shadow.

use @SUBJECT@ as subject;

@SHADOW@
@ABSTRACTION@
fn main() {
    let path = std::env::args().nth(1).expect(\"cases file\");
    let text = std::fs::read_to_string(path).expect(\"cases\");
    let environment = subject::environment();
    let (mut run, mut disagreements) = (0_u64, 0_u64);
    for line in text.lines() {
        let case: Vec<i64> = line
            .split(' ')
            .map(|value| value.parse().expect(\"case value\"))
            .collect();
@BINDINGS@
        let (verdict, pairs) = @SHADOW_MODULE@::shadow(@ARGUMENTS@);
        let shadow_result = format!(\"Completed(verdict={verdict},pairs={pairs})\");
        let disagreement = match abstraction::lift(&environment, @ARGUMENTS@) {
            Ok((left, right)) => match subject::compare(&environment, &left, &right) {
                subject::Outcome::Completed { verdict: v, pairs: p } => {
                    if (v, p) == (verdict, pairs) {
                        None
                    } else {
                        Some(format!(\"Completed(verdict={v},pairs={p})\"))
                    }
                }
                subject::Outcome::Refused(message) => Some(format!(\"Refused({message})\")),
                subject::Outcome::Incomplete(message) => Some(format!(\"Incomplete({message})\")),
            },
            Err(message) => Some(format!(\"ConstructionRefused({message})\")),
        };
        run += 1;
        if let Some(production) = disagreement {
            disagreements += 1;
            if disagreements <= 16 {
                let shown: Vec<String> = case.iter().map(ToString::to_string).collect();
                println!(\"DISAGREE\\t{}\\t{production}\\t{shadow_result}\", shown.join(\" \"));
            }
        }
    }
    println!(\"DONE\\t{run}\\t{disagreements}\");
}
";

/// Generates the shadow harness and its refinement obligation for one composite-equality claim,
/// or says why not.
pub fn generate_composite_shadow(
    item: &CompositeShadowItem<'_>,
) -> Result<KaniShadowHarness, CompositeShadowRefusal> {
    generate_composite_with(item, &Hooks::default(), SHADOW_PAIR_NODE_BUDGET)
}

/// Generates as [`generate_composite_shadow`] does under an explicit pair-node budget.
///
/// The family's budget is [`SHADOW_PAIR_NODE_BUDGET`], set by measurement; this entry exists for
/// that measurement and to show that the identity binds the budget it was generated under. A
/// caller never raises the budget to fit a closure.
pub fn generate_composite_shadow_with_budget(
    item: &CompositeShadowItem<'_>,
    budget: u32,
) -> Result<KaniShadowHarness, CompositeShadowRefusal> {
    generate_composite_with(item, &Hooks::default(), budget)
}

pub(crate) fn generate_composite_with(
    item: &CompositeShadowItem<'_>,
    hooks: &Hooks,
    budget: u32,
) -> Result<KaniShadowHarness, CompositeShadowRefusal> {
    let claim = match &item.entry.result {
        ClaimDisposition::Generated(claim) => claim,
        ClaimDisposition::Refused { refusal } => {
            return Err(CompositeShadowRefusal::Oracle {
                refusal: refusal.clone(),
            })
        }
    };
    let operand_node = node_text(&item.entry.node_id);
    let descriptor = &claim.descriptor;
    if descriptor.left_conversion_target.is_some() || descriptor.right_conversion_target.is_some() {
        return Err(CompositeShadowRefusal::OperandNotSupported { node: operand_node });
    }
    if claim.operand_types.left != claim.operand_types.right {
        return Err(CompositeShadowRefusal::OperandNotSupported { node: operand_node });
    }
    for operand in [&item.left, &item.right] {
        if let ShadowOperand::Computed { node } = operand {
            return Err(CompositeShadowRefusal::OperandNotSupported { node: node.clone() });
        }
    }
    let mut model = Model {
        closure: &claim.declaration_closure,
        decls: Vec::new(),
        keys: Vec::new(),
        stack: Vec::new(),
    };
    let root = model.resolve(&claim.operand_types.left, &operand_node)?;
    if is_leafy(&root) {
        return Err(CompositeShadowRefusal::OperandNotSupported { node: operand_node });
    }
    let count = closure_pair_nodes(&model, &root);
    if count > budget {
        return Err(CompositeShadowRefusal::ShadowShapeOverBudget { count, budget });
    }
    if !is_symbol(&item.subject) {
        return Err(CompositeShadowRefusal::NoProductionSubject);
    }
    let shared = matches!(
        (&item.left, &item.right),
        (
            ShadowOperand::Parameter { node: a, name: x },
            ShadowOperand::Parameter { node: b, name: y },
        ) if a == b && x == y
    );
    let left = operand_plan(&model, &root, &item.left, "lit_left", &operand_node)?;
    let right = operand_plan(&model, &root, &item.right, "lit_right", &operand_node)?;
    if !shared && left.prefix == right.prefix {
        return Err(CompositeShadowRefusal::LeafName {
            name: left.prefix.clone(),
        });
    }

    // Distinct leaf names.
    let mut all: Vec<&Leaf> = left.leaves.iter().collect();
    if !shared {
        all.extend(right.leaves.iter());
    }
    let mut names: Vec<&str> = all.iter().map(|leaf| leaf.name.as_str()).collect();
    names.sort_unstable();
    if let Some(pair) = names.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(CompositeShadowRefusal::LeafName {
            name: pair[0].to_owned(),
        });
    }
    if let Some(leaf) = all.iter().find(|leaf| !is_identifier(&leaf.name)) {
        return Err(CompositeShadowRefusal::LeafName {
            name: leaf.name.clone(),
        });
    }

    // Drawn operands ascend by parameter node id, then by leaf path (depth-first declaration
    // order); literal operands draw nothing.
    let mut drawn_operands: Vec<&Operand<'_>> = [&left, &right]
        .into_iter()
        .filter(|operand| operand.node.is_some())
        .collect();
    if shared {
        drawn_operands.truncate(1);
    }
    drawn_operands.sort_by(|a, b| a.node.cmp(&b.node));
    let drawn: Vec<(&Operand<'_>, &Leaf)> = drawn_operands
        .iter()
        .flat_map(|operand| operand.leaves.iter().map(move |leaf| (*operand, leaf)))
        .collect();
    if drawn.is_empty() {
        return Err(CompositeShadowRefusal::OperandNotSupported { node: operand_node });
    }
    let pinned: Vec<(String, i64, bool)> = [&left, &right]
        .into_iter()
        .filter_map(|operand| operand.pinned.map(|values| (operand, values)))
        .flat_map(|(operand, values)| {
            operand
                .leaves
                .iter()
                .zip(values)
                .map(|(leaf, value)| (leaf.name.clone(), *value, leaf.is_bool()))
        })
        .collect();
    let pinned_records: Vec<PinnedLeaf> = [&left, &right]
        .into_iter()
        .enumerate()
        .filter_map(|(position, operand)| operand.pinned.map(|values| (position, operand, values)))
        .flat_map(|(position, operand, values)| {
            operand
                .leaves
                .iter()
                .zip(values)
                .map(move |(leaf, value)| PinnedLeaf {
                    name: leaf.name.clone(),
                    operand: position,
                    path: leaf.path.trim_start_matches('.').to_owned(),
                    kind: leaf.kind,
                    value: *value,
                })
        })
        .collect();
    let operand_sources = [&left, &right].map(|operand| match operand.node {
        Some(node) => OperandSource::Parameter {
            node: node_text(node),
        },
        None => OperandSource::Literal,
    });

    let negate = descriptor.operator == EqualityOperatorKind::NotEqual;
    let arguments: Vec<&str> = all.iter().map(|leaf| leaf.name.as_str()).collect();

    // The shadow module.
    let walks = render_walks_for(&model, &root);
    let left_build = build_expression(&model, &root, &left.prefix);
    let right_build = build_expression(&model, &root, &right.prefix);
    let (verdict, pair_count) = expectation_terms(&model, &root, &left.prefix, &right.prefix);
    let signature = parameter_list(&all);
    let order: Vec<String> = model
        .decls
        .iter()
        .enumerate()
        .flat_map(|(index, decl)| {
            decl.members
                .iter()
                .map(move |member| format!("decl{index}.{}", member.name))
        })
        .collect();
    let member_order = hooks.leaf_order_body.clone().unwrap_or_else(|| {
        format!(
            "[{}]",
            order
                .iter()
                .map(|name| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    });
    let operator_apply = if negate { "!equal" } else { "equal" };
    let expected_apply = if negate { "!equal" } else { "equal" };
    let shadow_module = format!(
        "pub mod {COMPOSITE_SHADOW_MODULE} {{
    /// The state of an optional slot of a record.
    #[derive(Clone, Copy)]
    pub enum Slot<T> {{
        Absent,
        Null,
        Present(T),
    }}

    /// The order the members are walked and reported in.
    pub fn member_order() -> [&'static str; {order_len}] {{
        {member_order}
    }}

{walks}    /// The shadow: the occurrence-pair walk over the closure's shape, with its pair count.
    #[allow(clippy::too_many_arguments)]
    pub fn shadow({signature}) -> (bool, u64) {{
        let left = {left_build};
        let right = {right_build};
        let (equal, pairs) = {root_walk};
        ({operator_apply}, pairs)
    }}

    /// The expectation: FR-149's verdict and node count computed from the drawn leaves.
    #[allow(clippy::too_many_arguments)]
    pub fn expectation({signature}) -> (bool, u64) {{
        let equal = {verdict};
        let pairs = {pair_count};
        ({expected_apply}, pairs)
    }}
{extra}}}
",
        order_len = order.len(),
        root_walk = root_walk_call(&root),
        extra = hooks.shadow_extra,
    );

    // The proof module.
    let draws: Vec<render::shadow::Draw<'_>> = drawn
        .iter()
        .map(|(_, leaf)| render::shadow::Draw {
            name: leaf.name.as_str(),
            domain: if leaf.is_bool() {
                render::shadow::DrawDomain::Bool
            } else {
                render::shadow::DrawDomain::Int(leaf.bound.0, leaf.bound.1)
            },
            when: leaf.when.as_deref(),
        })
        .collect();
    let pinned_lines: Vec<(&str, String)> = pinned
        .iter()
        .map(|(name, value, is_bool)| {
            (
                name.as_str(),
                if *is_bool {
                    format!("{}", *value != 0)
                } else {
                    format!("{value}_i64")
                },
            )
        })
        .collect();
    let cover = if negate {
        "!expected_verdict"
    } else {
        "expected_verdict"
    };
    let proof_module = render::shadow::proof_module(&render::shadow::ProofModule {
        module: PROOF_MODULE,
        function: PROOF_FUNCTION,
        shadow_module: COMPOSITE_SHADOW_MODULE,
        draws: &draws,
        pinned: &pinned_lines,
        arguments: &arguments,
        cover,
        before_proof: &hooks.proof_extra,
        before_assertions: &hooks.body_extra,
    });
    let rust = Artifact::new(
        "shadow_harness.rs",
        format!("{shadow_module}\n{proof_module}"),
    );

    // Parsed-source checks (FR-028-AC-13, FR-015-AC-72).
    let assumed = source_check::check_shadow_source(&rust.contents, Some(ABSTRACTION_MODULE))
        .map_err(|violation| CompositeShadowRefusal::SourceCheck {
            violation: format!("{violation:?}"),
        })?;
    source_check::check_straight_line(&rust.contents)
        .map_err(|violation| CompositeShadowRefusal::SourceCheck { violation })?;
    let declared: Vec<(String, i128, i128)> = drawn
        .iter()
        .filter(|(_, leaf)| !leaf.is_bool())
        .map(|(_, leaf)| {
            (
                leaf.name.clone(),
                i128::from(leaf.bound.0),
                i128::from(leaf.bound.1),
            )
        })
        .collect();
    let in_source: Vec<(String, i128, i128)> = assumed
        .into_iter()
        .map(|bound| (bound.argument, bound.minimum, bound.maximum))
        .collect();
    if declared != in_source {
        return Err(CompositeShadowRefusal::SourceCheck {
            violation: "the harness draws from a domain other than the identity's".to_owned(),
        });
    }

    // The refinement obligation.
    let abstraction_source = render_abstraction(&model, &root, &left, &right, shared);
    let mut bindings = String::new();
    for (position, (_, leaf)) in drawn.iter().enumerate() {
        if leaf.is_bool() {
            bindings.push_str(&format!(
                "        let {} = case[{position}] != 0;\n",
                leaf.name
            ));
        } else {
            bindings.push_str(&format!("        let {} = case[{position}];\n", leaf.name));
        }
    }
    for (name, value, is_bool) in &pinned {
        if *is_bool {
            bindings.push_str(&format!("        let {name} = {};\n", *value != 0));
        } else {
            bindings.push_str(&format!("        let {name} = {value}_i64;\n"));
        }
    }
    let argument_list = arguments.join(", ");
    let source = Artifact::new(
        "refinement.rs",
        DRIVER
            .replace("@SUBJECT@", &item.subject)
            .replace("@SHADOW@", &shadow_module)
            .replace("@ABSTRACTION@", &abstraction_source)
            .replace("@BINDINGS@", bindings.trim_end_matches('\n'))
            .replace("@SHADOW_MODULE@", COMPOSITE_SHADOW_MODULE)
            .replace("@ARGUMENTS@", &argument_list),
    );
    let domain: Vec<DomainBound> = drawn
        .iter()
        .map(|(_, leaf)| DomainBound {
            argument: leaf.name.clone(),
            minimum: leaf.bound.0,
            maximum: leaf.bound.1,
        })
        .collect();

    // The identity.
    let harness_path = format!("{PROOF_MODULE}::{PROOF_FUNCTION}");
    let solver = KaniSolver::Cadical;
    let leaves: Vec<LeafRecord> = drawn
        .iter()
        .map(|(operand, leaf)| LeafRecord {
            name: leaf.name.clone(),
            node: operand.node.map(node_text).unwrap_or_default(),
            path: leaf.path.trim_start_matches('.').to_owned(),
            kind: leaf.kind,
            declared: leaf.bound,
            drawn: leaf.bound,
        })
        .collect();
    let abstractions = vec![
        AbstractionRecord {
            name: "representation".to_owned(),
            evidence: AbstractionEvidence::RefinementComparison("verdict".to_owned()),
        },
        AbstractionRecord {
            name: "integer_widening".to_owned(),
            evidence: AbstractionEvidence::RefinementComparison("verdict".to_owned()),
        },
        AbstractionRecord {
            name: "absent_payload".to_owned(),
            evidence: AbstractionEvidence::KaniCheck(harness_path.clone()),
        },
        AbstractionRecord {
            name: "unrolled_walk".to_owned(),
            evidence: AbstractionEvidence::RefinementComparison("pair_count".to_owned()),
        },
    ];
    let coverage: Vec<UnexercisedCoverage> = UNEXERCISED_COVERAGE
        .iter()
        .map(|(behaviour, criterion)| UnexercisedCoverage {
            behaviour: (*behaviour).to_owned(),
            criterion: (*criterion).to_owned(),
            backing: if item
                .backing
                .iter()
                .any(|(name, backed)| name == criterion && *backed)
            {
                BackingState::Backed
            } else {
                BackingState::Unbacked
            },
        })
        .collect();
    let closes_family_row = coverage
        .iter()
        .all(|covered| covered.backing == BackingState::Backed);
    let shared_helpers = vec![
        SharedHelperClaim {
            name: "closure_reader".to_owned(),
            mark: HelperMark::ValueBearing,
            order_only_reason: None,
            production_read: ProductionRead::SameHelper,
            independent_check: Some(
                "FR-018-AC-23: the closure against an independent read of the checked package"
                    .to_owned(),
            ),
        },
        SharedHelperClaim {
            name: "member_order".to_owned(),
            mark: HelperMark::OrderOnly,
            order_only_reason: Some(
                "the verdict is a conjunction over members and the pair count a sum with no early exit, so neither reads their order"
                    .to_owned(),
            ),
            production_read: ProductionRead::Independent,
            independent_check: None,
        },
    ];
    let identity = ShadowIdentity {
        family: COMPOSITE_FAMILY.to_owned(),
        proof_subject: ProofSubject::BoundedShadow,
        node: operand_node,
        ceilings: item.ceilings,
        module_symbol: PROOF_MODULE.to_owned(),
        harness_symbol: PROOF_FUNCTION.to_owned(),
        leaves,
        size_budget: budget,
        size: count,
        abstractions,
        unexercised: UNEXERCISED_COVERAGE
            .iter()
            .map(|(behaviour, _)| (*behaviour).to_owned())
            .collect(),
        shared_helpers,
        solver,
        unwind: item.unwind,
        options: adapter_options(&harness_path, item.unwind, solver, false),
        witness_options: witness_options(&harness_path, item.unwind, solver, false),
        composite: Some(CompositeIdentity {
            operator: match descriptor.operator {
                EqualityOperatorKind::Equal => "equality.equal",
                EqualityOperatorKind::NotEqual => "equality.not_equal",
            }
            .to_owned(),
            declarations: claim.declaration_closure.clone(),
            operand_types: claim.operand_types.clone(),
            pair_nodes: count,
            operands: operand_sources,
            pinned: pinned_records,
            coverage,
            closes_family_row,
        }),
    };
    check_leaf_bindings(&identity)
        .map_err(|refusal| CompositeShadowRefusal::LeafBinding { refusal })?;
    check_leaf_call_order(&identity, &rust.contents)
        .map_err(|refusal| CompositeShadowRefusal::LeafBinding { refusal })?;
    let obligation = RefinementObligation {
        subject: item.subject.clone(),
        shadow: format!("{COMPOSITE_SHADOW_MODULE}::shadow"),
        domain,
        abstraction: format!("{ABSTRACTION_MODULE}::lift"),
        observables: vec!["verdict".to_owned(), "pair_count".to_owned()],
        source,
        abstraction_source,
    };
    Ok(KaniShadowHarness {
        identity,
        rust,
        obligation,
    })
}

fn root_walk_call(root: &Ty) -> String {
    format!("{}(left, right)", walk_name(root))
}

fn render_walks_for(model: &Model<'_>, root: &Ty) -> String {
    render_walks(model, root)
}

/// The primitive type a leaf is drawn as.
fn primitive_of(kind: LeafKind) -> &'static str {
    match kind {
        LeafKind::Boolean | LeafKind::Presence => "bool",
        LeafKind::Integer | LeafKind::SlotState => "i64",
    }
}

/// Checks that the generated proof `source` draws exactly the identity's leaves, in the
/// identity's order, each as its primitive type: the order a counterexample's playback is read in
/// (FR-025-AC-9).
///
/// # Errors
///
/// [`LeafBindingRefusal::CallOrder`] naming both sequences as `name:primitive`.
pub fn check_leaf_call_order(
    identity: &ShadowIdentity,
    source: &str,
) -> Result<(), LeafBindingRefusal> {
    let recorded: Vec<String> = identity
        .leaves
        .iter()
        .map(|leaf| format!("{}:{}", leaf.name, primitive_of(leaf.kind)))
        .collect();
    let drawn: Vec<String> = source_check::drawn_order(source, PROOF_FUNCTION)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, ty)| format!("{name}:{ty}"))
        .collect();
    if recorded == drawn {
        Ok(())
    } else {
        Err(LeafBindingRefusal::CallOrder { recorded, drawn })
    }
}
