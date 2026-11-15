//! FR-015-AC-69 to FR-015-AC-76 (TC-025 steps 29 to 36): the composite-equality family of the
//! bounded shadow, generated from the claim map of the TC-029 bounded corpus.
//!
//! The generation checks here read the emitted source and identity. The native refinement runs
//! are in `composite_shadow_native.rs` and the installed-backend runs in `composite_shadow_kani.rs`.

use super::composite_equality_closure::{corpus, Corpus, ITEMS};
use filament_contract_codegen::{
    check_shadow_source, check_straight_line, generate_composite_shadow, BackingState,
    ClaimDisposition, CompositeEqualityClaim, CompositeShadowItem, CompositeShadowRefusal,
    KaniShadowHarness, LeafKind, ProofCeilings, ProofSubject, ShadowDisposition, ShadowOperand,
    SHADOW_PAIR_NODE_BUDGET,
};
use ix_trace_rs::trace;
use std::time::Duration;

pub(super) const FLAGS_EQUAL: usize = 0;
pub(super) const FLAGS_NOT_EQUAL: usize = 1;
pub(super) const DUO_EQUAL: usize = 2;
pub(super) const OPTION_EQUAL: usize = 3;
pub(super) const NESTED_OPTION_EQUAL: usize = 4;
pub(super) const SHAPE_EQUAL: usize = 5;

pub(super) fn ceilings() -> ProofCeilings {
    ProofCeilings {
        memory_bytes: std::num::NonZeroU64::new(16 << 30).unwrap(),
        wall_clock: Duration::from_secs(1800),
    }
}

/// The claim-map entry of the `index`th corpus item.
pub(super) fn entry(corpus: &Corpus, index: usize) -> CompositeEqualityClaim {
    let item = &corpus.items[index];
    corpus
        .oracles
        .claim_map
        .items
        .iter()
        .find(|entry| {
            entry.node_id == item.node_id
                && matches!(&entry.result, ClaimDisposition::Generated(claim)
                    if claim.descriptor.operator == item.operator)
        })
        .expect("the claim-map entry of the item")
        .clone()
}

/// The backing state of each covering criterion as the matrix lists it today.
pub(super) fn backing() -> Vec<(String, bool)> {
    [
        ("FR-018-AC-9", false),
        ("FR-018-AC-8", false),
        ("FR-018-AC-2", false),
        ("FR-018-AC-7", false),
        ("FR-018-AC-23", true),
    ]
    .into_iter()
    .map(|(criterion, backed)| (criterion.to_owned(), backed))
    .collect()
}

pub(super) fn parameter(entry: &CompositeEqualityClaim, second: bool) -> ShadowOperand {
    // A refused entry carries no semantic type: both operands stand on the entry's node.
    let second_node = match &entry.result {
        ClaimDisposition::Generated(claim) => claim.semantic_type.clone(),
        _ => entry.node_id.clone(),
    };
    if second {
        ShadowOperand::Parameter {
            node: second_node,
            name: "b".to_owned(),
        }
    } else {
        ShadowOperand::Parameter {
            node: entry.node_id.clone(),
            name: "a".to_owned(),
        }
    }
}

/// The two parameter nodes the equality application of the `index`th corpus item references,
/// read from the checked package: the real operand nodes, in operand order.
pub(super) fn parameter_nodes(
    corpus: &Corpus,
    index: usize,
) -> (
    quire_contract_model::CheckedNodeId,
    quire_contract_model::CheckedNodeId,
) {
    use quire_contract_model::CheckedTerm;
    let application = super::admitted_enum_equality::application(&corpus.package, ITEMS[index].1);
    let node = corpus
        .package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == application)
        .expect("the equality application");
    let CheckedTerm::Application { arguments, .. } = &node.body else {
        panic!("an application");
    };
    let target = |term: &CheckedTerm| match term {
        CheckedTerm::Reference { target } => target.clone(),
        other => panic!("an operand reference, not {other:?}"),
    };
    (target(&arguments[0]), target(&arguments[1]))
}

pub(super) fn item_of<'a>(
    entry: &'a CompositeEqualityClaim,
    subject: &str,
) -> CompositeShadowItem<'a> {
    CompositeShadowItem {
        entry,
        left: parameter(entry, false),
        right: parameter(entry, true),
        subject: subject.to_owned(),
        ceilings: ceilings(),
        unwind: 4,
        backing: backing(),
    }
}

pub(super) fn generate(corpus: &Corpus, index: usize) -> KaniShadowHarness {
    generate_for(corpus, index, "subject")
}

/// The harness of the `index`th corpus item whose obligation names `subject` as its production
/// subject.
pub(super) fn generate_for(corpus: &Corpus, index: usize, subject: &str) -> KaniShadowHarness {
    let entry = entry(corpus, index);
    generate_composite_shadow(&item_of(&entry, subject)).expect("the corpus item generates")
}

#[trace("TC-025", "FR-015-AC-69")]
#[test]
fn tc_025_each_supported_corpus_item_yields_one_bounded_shadow_harness_with_its_obligation() {
    let corpus = corpus();
    let supported = [
        FLAGS_EQUAL,
        FLAGS_NOT_EQUAL,
        DUO_EQUAL,
        OPTION_EQUAL,
        NESTED_OPTION_EQUAL,
    ];
    let harnesses: Vec<KaniShadowHarness> = supported
        .iter()
        .map(|index| generate(&corpus, *index))
        .collect();
    for (harness, index) in harnesses.iter().zip(supported) {
        assert_eq!(
            harness.identity.family, "composite_equality",
            "{}",
            ITEMS[index].0
        );
        assert_eq!(harness.identity.proof_subject, ProofSubject::BoundedShadow);
        assert_eq!(harness.obligation.subject, "subject");
        assert!(!harness.obligation.source.contents.is_empty());
    }
    // Both operators yield a harness, distinct from each other's.
    assert_ne!(harnesses[0].rust.contents, harnesses[1].rust.contents);
    assert_eq!(
        harnesses[0].identity.composite.as_ref().unwrap().operator,
        "equality.equal"
    );
    assert_eq!(
        harnesses[1].identity.composite.as_ref().unwrap().operator,
        "equality.not_equal"
    );
    assert!(harnesses[1].rust.contents.contains("(!equal, pairs)"));
    assert!(!harnesses[0].rust.contents.contains("(!equal, pairs)"));
    // The bounds are the closure's, not the descriptor's: every drawn integer leaf carries the
    // `Int[0, 2]` the declaration records, and the descriptor holds no bound to read.
    for leaf in harnesses[0]
        .identity
        .leaves
        .iter()
        .filter(|leaf| leaf.kind == LeafKind::Integer)
    {
        assert_eq!(leaf.declared, (0, 2), "{}", leaf.name);
    }
    // Every item's harness is its own.
    for (left, right) in harnesses.iter().zip(harnesses.iter().skip(1)) {
        assert_ne!(left.rust.contents, right.rust.contents);
    }
}

// ---- synthetic closures --------------------------------------------------------------------

use filament_contract_codegen::{RecordedForm, RecordedMember, RecordedPresence, RecordedType};
use syn::visit::{self, Visit};

pub(super) fn boolean(name: &str) -> RecordedMember {
    RecordedMember {
        name: name.to_owned(),
        presence: RecordedPresence::Required,
        value_type: RecordedType::Boolean,
    }
}

pub(super) fn int(name: &str, lower: &str, upper: &str) -> RecordedMember {
    RecordedMember {
        name: name.to_owned(),
        presence: RecordedPresence::Required,
        value_type: RecordedType::Int {
            lower: lower.to_owned(),
            upper: upper.to_owned(),
        },
    }
}

/// `base` with its closure replaced by one record of `members`.
pub(super) fn with_record(
    base: &CompositeEqualityClaim,
    members: Vec<RecordedMember>,
) -> CompositeEqualityClaim {
    let mut changed = base.clone();
    let ClaimDisposition::Generated(claim) = &mut changed.result else {
        panic!("generated entry");
    };
    let mut declaration = claim.declaration_closure[0].clone();
    declaration.key = "k0".to_owned();
    declaration.form = RecordedForm::Record;
    declaration.members = members;
    claim.declaration_closure = vec![declaration];
    let operand = RecordedType::Composite {
        key: "k0".to_owned(),
    };
    claim.operand_types.left = operand.clone();
    claim.operand_types.right = operand;
    changed
}

/// A record whose closure has exactly `nodes` pair nodes: `nodes - 1` Boolean fields.
pub(super) fn record_of_nodes(base: &CompositeEqualityClaim, nodes: u32) -> CompositeEqualityClaim {
    with_record(
        base,
        (0..nodes - 1)
            .map(|index| boolean(&format!("f{index}")))
            .collect(),
    )
}

struct Calls(Vec<String>);
impl<'a> Visit<'a> for Calls {
    fn visit_expr_call(&mut self, call: &'a syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func {
            self.0
                .push(path.path.segments.last().unwrap().ident.to_string());
        }
        visit::visit_expr_call(self, call);
    }
}

fn refusal_of(entry: &CompositeEqualityClaim) -> CompositeShadowRefusal {
    generate_composite_shadow(&item_of(entry, "subject")).expect_err("the item is refused")
}

fn function_body<'a>(file: &'a syn::File, name: &str) -> Vec<&'a syn::Stmt> {
    struct Find<'a> {
        name: String,
        found: Vec<&'a syn::Stmt>,
    }
    impl<'a> Visit<'a> for Find<'a> {
        fn visit_item_fn(&mut self, function: &'a syn::ItemFn) {
            if function.sig.ident == self.name {
                self.found = function.block.stmts.iter().collect();
            }
            visit::visit_item_fn(self, function);
        }
    }
    let mut find = Find {
        name: name.to_owned(),
        found: Vec::new(),
    };
    find.visit_file(file);
    find.found
}

fn macro_names(statements: &[&syn::Stmt]) -> Vec<String> {
    statements
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Macro(mac) => mac.mac.path.segments.last().map(|s| s.ident.to_string()),
            syn::Stmt::Expr(syn::Expr::Macro(mac), _) => {
                mac.mac.path.segments.last().map(|s| s.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

#[trace("TC-025", "FR-015-AC-70")]
#[test]
fn tc_025_the_harness_draws_one_value_per_leaf_in_node_then_path_order_and_no_composite() {
    let corpus = corpus();
    let flags = generate(&corpus, FLAGS_EQUAL);
    let source = &flags.rust.contents;
    let leaves = &flags.identity.leaves;
    // Two parameters, each: a (bool), b (int), c's state (0..=2) and c's payload (int).
    assert_eq!(leaves.len(), 8);
    let kinds: Vec<LeafKind> = leaves.iter().take(4).map(|leaf| leaf.kind).collect();
    assert_eq!(
        kinds,
        [
            LeafKind::Boolean,
            LeafKind::Integer,
            LeafKind::SlotState,
            LeafKind::Integer
        ]
    );
    let paths: Vec<&str> = leaves
        .iter()
        .take(4)
        .map(|leaf| leaf.path.as_str())
        .collect();
    assert_eq!(paths, ["a", "b", "c.state", "c"]);
    // Parameters ascend by node id: the first four leaves share a node, the last four another,
    // and the first node is the smaller.
    assert!(leaves[..4].iter().all(|leaf| leaf.node == leaves[0].node));
    assert!(leaves[4..].iter().all(|leaf| leaf.node == leaves[4].node));
    assert!(leaves[0].node < leaves[4].node);
    // One `kani::any` per leaf; an assumption exactly for each integer-like leaf.
    assert_eq!(source.matches("kani::any").count(), 8);
    let assumed = check_shadow_source(source, Some("abstraction")).expect("source passes");
    assert_eq!(assumed.len(), 6);
    assert!(assumed
        .iter()
        .all(|bound| (bound.minimum, bound.maximum) == (0, 2)));
    // A payload is drawn only when present: the draw sits under its presence condition.
    assert!(source.contains("if a_c_state == 0 {"));
    assert!(source.contains("if b_c_state == 0 {"));
    // Nothing but scalars is an argument.
    let file = syn::parse_file(source).unwrap();
    for function in ["shadow", "expectation"] {
        struct Params(Vec<String>);
        impl<'a> Visit<'a> for Params {
            fn visit_item_fn(&mut self, function: &'a syn::ItemFn) {
                if function.sig.ident == "shadow" || function.sig.ident == "expectation" {
                    for input in &function.sig.inputs {
                        if let syn::FnArg::Typed(typed) = input {
                            if let syn::Type::Path(path) = &*typed.ty {
                                self.0
                                    .push(path.path.segments.last().unwrap().ident.to_string());
                            }
                        }
                    }
                }
            }
        }
        let mut params = Params(Vec::new());
        params.visit_file(&file);
        assert!(!params.0.is_empty(), "{function} takes arguments");
        assert!(
            params.0.iter().all(|ty| ty == "bool" || ty == "i64"),
            "{function}: {:?}",
            params.0
        );
    }

    // An option draws its presence and a payload only when present.
    let option = generate(&corpus, OPTION_EQUAL);
    assert!(option.rust.contents.contains("if a_some {"));
    let kinds: Vec<LeafKind> = option
        .identity
        .leaves
        .iter()
        .map(|leaf| leaf.kind)
        .collect();
    assert_eq!(&kinds[..2], &[LeafKind::Presence, LeafKind::Integer]);

    // Ascending order does not follow the operand order: the same claim with the operands'
    // nodes swapped draws the other parameter first.
    // The operand nodes are the two parameter nodes the checked equality application
    // references.
    let entry = entry(&corpus, FLAGS_EQUAL);
    let (first, second) = parameter_nodes(&corpus, FLAGS_EQUAL);
    assert_ne!(first, second);
    let (low, high) = if first < second {
        (first, second)
    } else {
        (second, first)
    };
    let operand =
        |node: &quire_contract_model::CheckedNodeId, name: &str| ShadowOperand::Parameter {
            node: node.clone(),
            name: name.to_owned(),
        };
    let mut ordered = item_of(&entry, "subject");
    ordered.left = operand(&low, "a");
    ordered.right = operand(&high, "b");
    let ascending = generate_composite_shadow(&ordered).unwrap();
    let mut swapped = item_of(&entry, "subject");
    swapped.left = operand(&high, "a");
    swapped.right = operand(&low, "b");
    let harness = generate_composite_shadow(&swapped).unwrap();
    assert_ne!(
        ascending.identity.leaves[0].node,
        ascending.identity.leaves[4].node
    );
    assert!(ascending.identity.leaves[0].name.starts_with('a'));
    // The operand drawn first is the one with the smaller node, whichever side it stands on.
    assert_eq!(
        harness.identity.leaves[0].node,
        ascending.identity.leaves[0].node
    );
    assert_eq!(
        harness.identity.leaves[4].node,
        ascending.identity.leaves[4].node
    );
    assert!(harness.identity.leaves[0].name.starts_with('b'));

    // A literal operand's leaves are pinned to exactly its value and are not drawn.
    let duo_entry = super::composite_shadow::entry(&corpus, DUO_EQUAL);
    let mut literal = item_of(&duo_entry, "subject");
    literal.right = ShadowOperand::Literal { leaves: vec![1, 2] };
    let pinned = generate_composite_shadow(&literal).unwrap();
    assert_eq!(pinned.identity.leaves.len(), 2);
    let composite = pinned.identity.composite.as_ref().unwrap();
    assert_eq!(
        composite
            .pinned
            .iter()
            .map(|p| (p.name.as_str(), p.value))
            .collect::<Vec<_>>(),
        [("lit_right_0", 1), ("lit_right_1", 2)]
    );
    assert!(pinned.rust.contents.contains("let lit_right_0 = true;"));
    assert!(pinned.rust.contents.contains("let lit_right_1 = 2_i64;"));
    assert_eq!(pinned.obligation.domain.len(), 2);
    let mut wrong = item_of(&duo_entry, "subject");
    wrong.right = ShadowOperand::Literal { leaves: vec![1] };
    assert!(matches!(
        generate_composite_shadow(&wrong),
        Err(CompositeShadowRefusal::LiteralLeaves {
            expected: 2,
            given: 1
        })
    ));
}

#[trace("TC-025", "FR-015-AC-71")]
#[test]
fn tc_025_the_harness_asserts_shadow_against_an_independent_expectation_and_ends_with_one_cover() {
    let corpus = corpus();
    for index in [
        FLAGS_EQUAL,
        FLAGS_NOT_EQUAL,
        DUO_EQUAL,
        OPTION_EQUAL,
        NESTED_OPTION_EQUAL,
    ] {
        let harness = generate(&corpus, index);
        let source = &harness.rust.contents;
        let file = syn::parse_file(source).unwrap();
        let names = macro_names(&function_body(&file, "prove_shadow"));
        assert_eq!(names, ["assert", "assert", "cover"], "{}", ITEMS[index].0);
        // The expectation reaches no function of the shadow.
        let mut calls = Calls(Vec::new());
        for statement in function_body(&file, "expectation") {
            calls.visit_stmt(statement);
        }
        assert!(
            calls.0.is_empty(),
            "{}: the expectation calls {:?}",
            ITEMS[index].0,
            calls.0
        );
    }
    // The verdict of `!=` is the negation of `=`'s with the same count, and its cover states
    // that the operands compare equal.
    let equal = generate(&corpus, FLAGS_EQUAL).rust.contents;
    let not_equal = generate(&corpus, FLAGS_NOT_EQUAL).rust.contents;
    assert!(equal.contains("kani::cover!(expected_verdict)"));
    assert!(not_equal.contains("kani::cover!(!expected_verdict)"));
    assert!(not_equal.contains("(!equal, pairs)"));
    // The tri-state rule: absent against null is unequal, absent against absent equal.
    assert!(equal.contains("a_c_state == 1 && b_c_state == 1"));
    assert!(equal.contains("a_c_state == 2 && b_c_state == 2"));
    // A cover placed before the assertions is a different shape, which the body check rejects.
    let moved = equal
        .replace("        kani::cover!(expected_verdict);\n", "")
        .replace("        assert!(verdict == expected_verdict);", "        kani::cover!(expected_verdict);\n        assert!(verdict == expected_verdict);");
    assert_ne!(moved, equal);
    let file = syn::parse_file(&moved).unwrap();
    assert_ne!(
        macro_names(&function_body(&file, "prove_shadow")),
        ["assert", "assert", "cover"]
    );
    // An expectation computed by calling the shadow is detected by the same call scan.
    let delegated = equal.replace(
        "let pairs = ",
        "let pairs = shadow(a_a, a_b, a_c_state, a_c, b_a, b_b, b_c_state, b_c).1 + 0 * ",
    );
    let file = syn::parse_file(&delegated).unwrap();
    let mut calls = Calls(Vec::new());
    for statement in function_body(&file, "expectation") {
        calls.visit_stmt(statement);
    }
    assert!(!calls.0.is_empty());
}

#[trace("TC-025", "FR-015-AC-72")]
#[test]
fn tc_025_the_shadow_is_straight_line_and_the_budget_refuses_one_node_over() {
    let corpus = corpus();
    for index in [FLAGS_EQUAL, DUO_EQUAL, OPTION_EQUAL, NESTED_OPTION_EQUAL] {
        let harness = generate(&corpus, index);
        assert_eq!(check_straight_line(&harness.rust.contents), Ok(()));
        assert!(!filament_contract_codegen::contains_loop(
            &harness.rust.contents
        ));
    }
    // Each excluded construct is found.
    let healthy = generate(&corpus, FLAGS_EQUAL).rust.contents;
    let mutants = [
        (
            "loop",
            healthy.replace("let left =", "for _ in 0..1 {}\n        let left ="),
        ),
        (
            "heap",
            healthy.replace(
                "let left =",
                "let _v: Vec<i64> = Vec::new();\n        let left =",
            ),
        ),
        (
            "wide",
            healthy.replace("let left =", "let _w: i128 = 0;\n        let left ="),
        ),
        (
            "recursion",
            healthy.replace(
                "pub fn walk_decl0(",
                "pub fn again(n: u64) -> u64 { again(n) }\n    pub fn walk_decl0(",
            ),
        ),
    ];
    for (label, source) in mutants {
        assert_ne!(source, healthy, "{label}");
        assert!(
            check_straight_line(&source).is_err(),
            "{label} must be found"
        );
    }
    // The budget: one node under, at and one over.
    let entry = entry(&corpus, FLAGS_EQUAL);
    let budget = SHADOW_PAIR_NODE_BUDGET;
    for nodes in [budget - 1, budget] {
        let claim = record_of_nodes(&entry, nodes);
        let harness = generate_composite_shadow(&item_of(&claim, "subject")).unwrap();
        let composite = harness.identity.composite.as_ref().unwrap();
        assert_eq!(
            (
                composite.pair_nodes,
                harness.identity.size,
                harness.identity.size_budget
            ),
            (nodes, nodes, budget)
        );
        // The identity is the record serialised with the harness; it carries both figures.
        let json = serde_json::to_value(&harness.identity).unwrap();
        assert_eq!(json["sizeBudget"], budget);
        assert_eq!(json["size"], nodes);
        assert_eq!(json["composite"]["pairNodes"], nodes);
    }
    let over = record_of_nodes(&entry, budget + 1);
    assert_eq!(
        refusal_of(&over),
        CompositeShadowRefusal::ShadowShapeOverBudget {
            count: budget + 1,
            budget
        }
    );
    assert_eq!(
        refusal_of(&over).disposition(),
        ShadowDisposition::Unsupported
    );
    // The identity binds the budget it was generated under.
    let at = record_of_nodes(&entry, budget);
    let under_a = filament_contract_codegen::generate_composite_shadow_with_budget(
        &item_of(&at, "subject"),
        budget,
    )
    .unwrap();
    let under_b = filament_contract_codegen::generate_composite_shadow_with_budget(
        &item_of(&at, "subject"),
        budget + 1,
    )
    .unwrap();
    assert_ne!(under_a.identity, under_b.identity);
}

#[trace("TC-025", "FR-015-AC-73")]
#[trace("TC-039", "FR-028-AC-20")]
#[test]
fn tc_025_each_unsupported_shape_settles_with_its_reason_and_no_harness() {
    let corpus = corpus();
    let base = entry(&corpus, FLAGS_EQUAL);
    let unbounded = with_record(
        &base,
        vec![RecordedMember {
            name: "n".to_owned(),
            presence: RecordedPresence::Required,
            value_type: RecordedType::Integer,
        }],
    );
    let r = refusal_of(&unbounded);
    assert!(
        matches!(&r, CompositeShadowRefusal::RequiresBound { .. }),
        "{r:?}"
    );
    assert_eq!(r.disposition(), ShadowDisposition::RequiresBound);

    let wide = with_record(&base, vec![int("n", "0", "9223372036854775808")]);
    assert!(matches!(
        refusal_of(&wide),
        CompositeShadowRefusal::DomainNotRepresentableInI64 { .. }
    ));
    let low = with_record(&base, vec![int("n", "-9223372036854775809", "0")]);
    assert!(matches!(
        refusal_of(&low),
        CompositeShadowRefusal::DomainNotRepresentableInI64 { .. }
    ));

    let recursive = with_record(
        &base,
        vec![RecordedMember {
            name: "next".to_owned(),
            presence: RecordedPresence::Optional,
            value_type: RecordedType::Composite {
                key: "k0".to_owned(),
            },
        }],
    );
    let r = refusal_of(&recursive);
    assert!(
        matches!(&r, CompositeShadowRefusal::RequiresBound { .. }),
        "{r:?}"
    );

    for family in ["text", "enum", "rational", "decimal", "float", "collection"] {
        let leaf = with_record(
            &base,
            vec![RecordedMember {
                name: "x".to_owned(),
                presence: RecordedPresence::Required,
                value_type: RecordedType::Other {
                    name: Box::leak(family.to_owned().into_boxed_str()),
                },
            }],
        );
        match refusal_of(&leaf) {
            CompositeShadowRefusal::ShadowFamilyNotBuilt { family: named, .. } => {
                assert_eq!(named, family)
            }
            other => panic!("{family}: {other:?}"),
        }
    }
    // The union of the bounded corpus.
    let union = entry(&corpus, SHAPE_EQUAL);
    assert!(matches!(
        refusal_of(&union),
        CompositeShadowRefusal::ShadowFamilyNotBuilt { family, .. } if family == "union"
    ));
    // A reference composite, or any item FR-018 refused, carries FR-018's refusal.
    let mut refused = base.clone();
    refused.result = ClaimDisposition::Refused {
        refusal: filament_contract_codegen::CompositeEqualityRefusal::UnknownComposite {
            declaration: "reference".to_owned(),
        },
    };
    assert!(matches!(
        refusal_of(&refused),
        CompositeShadowRefusal::Oracle { .. }
    ));
    // An operand that is not a parameter or literal.
    let mut computed = item_of(&base, "subject");
    computed.right = ShadowOperand::Computed {
        node: "call".to_owned(),
    };
    assert!(matches!(
        generate_composite_shadow(&computed),
        Err(CompositeShadowRefusal::OperandNotSupported { .. })
    ));
    // A `convert<T>` operand.
    let mut converted = base.clone();
    let ClaimDisposition::Generated(claim) = &mut converted.result else {
        panic!()
    };
    claim.descriptor.left_conversion_target = Some(claim.semantic_type.clone());
    assert!(matches!(
        refusal_of(&converted),
        CompositeShadowRefusal::OperandNotSupported { .. }
    ));
    // An over-budget closure.
    let over = record_of_nodes(&base, SHADOW_PAIR_NODE_BUDGET + 1);
    assert!(matches!(
        refusal_of(&over),
        CompositeShadowRefusal::ShadowShapeOverBudget { .. }
    ));

    // Siblings settle as they do alone: refused items drop no supported sibling.
    let supported = entry(&corpus, DUO_EQUAL);
    let alone = generate_composite_shadow(&item_of(&supported, "subject")).unwrap();
    let items = [&unbounded, &supported, &over, &refused, &supported];
    let settled: Vec<_> = items
        .iter()
        .map(|entry| generate_composite_shadow(&item_of(entry, "subject")))
        .collect();
    assert!(settled[0].is_err() && settled[2].is_err() && settled[3].is_err());
    for index in [1, 4] {
        let harness = settled[index]
            .as_ref()
            .expect("the supported sibling generates");
        assert_eq!(harness.rust.contents, alone.rust.contents);
        assert_eq!(harness.identity, alone.identity);
    }
}

#[trace("TC-025", "FR-015-AC-76")]
#[test]
fn tc_025_the_identity_records_the_closure_abstractions_and_the_backing_of_each_covering_criterion()
{
    let corpus = corpus();
    let harness = generate(&corpus, FLAGS_EQUAL);
    let identity = &harness.identity;
    let composite = identity.composite.as_ref().unwrap();
    assert_eq!(identity.family, "composite_equality");
    assert_eq!(identity.proof_subject, ProofSubject::BoundedShadow);
    assert_eq!(composite.operator, "equality.equal");
    assert_eq!(composite.declarations.len(), 1);
    assert_eq!(composite.declarations[0].members.len(), 3);
    assert_eq!(
        (composite.pair_nodes, identity.size_budget),
        (4, SHADOW_PAIR_NODE_BUDGET)
    );
    let names: Vec<&str> = identity
        .abstractions
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "representation",
            "integer_widening",
            "absent_payload",
            "unrolled_walk"
        ]
    );
    assert_eq!(
        identity.unexercised,
        [
            "meter_limits",
            "ill_typed_operands",
            "charge_amounts_other_than_pairs",
            "foreign_reference",
            "declaration_reconstruction"
        ]
    );
    // The backing state of each covering criterion is read from the matrix row, not assumed.
    let matrix = include_str!("../../spec/oracle/matrix/tests.md");
    for covered in &composite.coverage {
        let listed = matrix_covered(matrix, &covered.criterion);
        assert_eq!(
            covered.backing == BackingState::Backed,
            listed,
            "{}: the identity and the matrix disagree",
            covered.criterion
        );
    }
    let state = |behaviour: &str| {
        composite
            .coverage
            .iter()
            .find(|c| c.behaviour == behaviour)
            .unwrap()
            .backing
    };
    assert_eq!(state("meter_limits"), BackingState::Unbacked);
    assert_eq!(state("ill_typed_operands"), BackingState::Unbacked);
    assert_eq!(
        state("charge_amounts_other_than_pairs"),
        BackingState::Unbacked
    );
    assert_eq!(state("foreign_reference"), BackingState::Unbacked);
    assert_eq!(state("declaration_reconstruction"), BackingState::Backed);
    assert!(
        !composite.closes_family_row,
        "an unbacked covering criterion closes no family row"
    );

    // Changing the closure, a bound, the operator or the budget changes the identity;
    // regeneration from equal inputs is byte-identical.
    let again = generate(&corpus, FLAGS_EQUAL);
    assert_eq!(again.rust.contents, harness.rust.contents);
    assert_eq!(again.identity, harness.identity);
    let base = entry(&corpus, FLAGS_EQUAL);
    let closure = with_record(&base, vec![boolean("a"), int("b", "0", "2")]);
    let narrowed = with_record(&base, vec![boolean("a"), int("b", "0", "1")]);
    let widened = with_record(&base, vec![boolean("a"), int("b", "0", "2"), boolean("c")]);
    let identities: Vec<_> = [&closure, &narrowed, &widened]
        .iter()
        .map(|claim| {
            generate_composite_shadow(&item_of(claim, "subject"))
                .unwrap()
                .identity
        })
        .collect();
    assert_ne!(identities[0], identities[1], "a bound");
    assert_ne!(identities[0], identities[2], "the closure");
    assert_ne!(
        harness.identity,
        generate(&corpus, FLAGS_NOT_EQUAL).identity,
        "the operator"
    );
}

/// Whether the matrix lists `criterion` in a row marked covered.
pub(super) fn matrix_covered(matrix: &str, criterion: &str) -> bool {
    let number = |id: &str| id.rsplit('-').next().and_then(|n| n.parse::<u32>().ok());
    let wanted = number(criterion).expect("criterion number");
    let prefix = criterion.rsplit_once("-AC-").unwrap().0;
    matrix
        .lines()
        .filter(|line| line.starts_with('|'))
        .any(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 5 || !cells[2].contains(prefix) {
                return false;
            }
            let mut listed = Vec::new();
            let tokens: Vec<&str> = cells[2].split(',').map(str::trim).collect();
            for token in tokens {
                if let Some((from, to)) = token.split_once(" through ") {
                    if let (Some(a), Some(b)) = (number(from), number(to)) {
                        listed.extend(a..=b);
                    }
                } else if let Some(n) = number(token) {
                    listed.push(n);
                }
            }
            listed.contains(&wanted) && cells[4].starts_with('✅')
        })
}

/// FR-028-AC-13 for the composite family: the unmutated source names no crate but `core`,
/// `alloc`, `std` and `kani`, holds no stub and assumes only its drawn domain; each spliced
/// fault fails the check it breaks.
#[trace("TC-039", "FR-028-AC-13")]
#[test]
fn tc_039_the_composite_shadow_source_has_no_runtime_path_no_stub_and_no_assumption_beyond_its_domain(
) {
    let corpus = corpus();
    let healthy = generate(&corpus, FLAGS_EQUAL).rust.contents;
    let drawn = check_shadow_source(&healthy, Some("abstraction")).expect("the source passes");
    assert_eq!(drawn.len(), 6);
    assert!(!healthy.contains("kani::stub"));

    let uses = "    use super::shadow_composite::{expectation, shadow};\n";
    let body = "        let (verdict, pairs) = shadow(";
    assert!(healthy.contains(uses) && healthy.contains(body));
    let faults: [(&str, &str, &str, &str); 7] = [
        (
            "a runtime type imported",
            "    use filament_contract_runtime::Outcome;\n",
            "",
            "ForeignCrate",
        ),
        (
            "an exact value imported",
            "    use filament_exact::Value;\n",
            "",
            "ForeignCrate",
        ),
        (
            "a runtime path inline",
            "",
            "        let _ = filament_exact::Value::default();\n",
            "ForeignCrate",
        ),
        ("a stub", "    #[kani::stub(shadow, shadow)]\n", "", "Stub"),
        (
            "a verified stub",
            "    #[kani::stub_verified(shadow)]\n",
            "",
            "Stub",
        ),
        (
            "an assumption relating two drawn values",
            "",
            "        kani::assume(a_a == b_a);\n",
            "Assumption",
        ),
        (
            "a one-sided assumption",
            "",
            "        kani::assume(a_b >= 0_i64);\n",
            "Assumption",
        ),
    ];
    for (label, before_proof, in_body, kind) in faults {
        let mutated = healthy
            .replace(uses, &format!("{uses}{before_proof}"))
            .replace(body, &format!("{in_body}{body}"));
        assert_ne!(mutated, healthy, "{label} changes the source");
        let violation = check_shadow_source(&mutated, Some("abstraction")).expect_err(label);
        assert!(
            format!("{violation:?}").contains(kind),
            "{label}: {violation:?} must name {kind}"
        );
    }
}

/// FR-015-AC-75, the closure reader: a fault of the closure that the shadow, its expectation and
/// the production oracle all share verifies and agrees with itself, so only the independent read
/// of FR-018-AC-23 catches it. The shadow generated from each faulty closure records that
/// closure in its identity, and the read of the package fails on exactly what the identity holds.
#[trace("TC-025", "FR-015-AC-75")]
#[trace("TC-039", "FR-028-AC-18")]
#[test]
fn tc_025_a_closure_reader_mutant_is_caught_by_the_independent_read_not_by_the_shadow() {
    let corpus = corpus();
    let healthy = generate(&corpus, FLAGS_EQUAL);
    let package_claim = super::composite_equality_closure::generated(&corpus, FLAGS_EQUAL);
    let read_of = |harness: &KaniShadowHarness| {
        let composite = harness.identity.composite.as_ref().unwrap();
        let mut claim = package_claim.clone();
        claim.declaration_closure = composite.declarations.clone();
        claim.operand_types = composite.operand_types.clone();
        super::composite_equality_closure::check_against_package(&corpus.package, &claim)
    };
    assert_eq!(
        read_of(&healthy),
        Ok(()),
        "the unmutated closure passes the read"
    );

    type Fault = (&'static str, fn(&mut Vec<RecordedMember>));
    let faults: [Fault; 3] = [
        ("dropped member", |members| {
            members.pop();
        }),
        ("bound narrowed by one", |members| {
            members[1].value_type = RecordedType::Int {
                lower: "0".to_owned(),
                upper: "1".to_owned(),
            };
        }),
        ("optional read as required", |members| {
            members[2].presence = RecordedPresence::Required;
        }),
    ];
    let base = entry(&corpus, FLAGS_EQUAL);
    let ClaimDisposition::Generated(original) = &base.result else {
        panic!("generated entry");
    };
    for (label, fault) in faults {
        let mut members = original.declaration_closure[0].members.clone();
        fault(&mut members);
        assert_ne!(members, original.declaration_closure[0].members, "{label}");
        let faulty = with_record(&base, members);
        let harness =
            generate_composite_shadow(&item_of(&faulty, "subject")).unwrap_or_else(|error| {
                panic!("{label}: the faulty closure still generates: {error:?}")
            });
        assert_ne!(
            harness.identity, healthy.identity,
            "{label}: the identity records it"
        );
        assert!(
            read_of(&harness).is_err(),
            "{label} must fail the independent read"
        );
    }
}
