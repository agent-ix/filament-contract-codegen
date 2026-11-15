//! FR-018-AC-21..23 (TC-029 steps 13-15): the claim-map declaration closure of Boolean and
//! bounded-integer items, read against the environment the emitted constructor returns
//! (AC-21), run natively over the refinement domain (AC-22) and read independently through
//! Contract IR's reader (AC-23).
use super::composite_equality_agreement::{fixture, original_item_named};
use super::exact_scalar_agreement::{
    run_composite_agreement_workspace, stage_composite_agreement_case,
};
use filament_contract_codegen::{
    ClaimDisposition, CompositeEqualityItem, CompositeEqualityOracles, EqualityOperatorKind,
    GeneratedCompositeEqualityClaim, RecordedDeclaration, RecordedForm, RecordedPresence,
    RecordedType,
};
use ix_trace_rs::trace;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};

/// `(alias, function, operator)`: the corpus items, named as `closure_cases.rs` expects.
const ITEMS: [(&str, &str, EqualityOperatorKind); 6] = [
    ("flags_equal", "fr", EqualityOperatorKind::Equal),
    ("flags_not_equal", "fr", EqualityOperatorKind::NotEqual),
    ("duo_equal", "ft", EqualityOperatorKind::Equal),
    ("option_equal", "fo", EqualityOperatorKind::Equal),
    ("nested_option_equal", "fq", EqualityOperatorKind::Equal),
    ("shape_equal", "fu", EqualityOperatorKind::Equal),
];

struct Corpus {
    package: CheckedPackageV2,
    oracles: CompositeEqualityOracles,
    items: Vec<CompositeEqualityItem>,
}

fn corpus() -> Corpus {
    let compiled = fixture::compile(fixture::BOUNDED_CASE);
    let package = super::admitted_enum_equality::checked(&compiled, &[]);
    let items: Vec<_> = ITEMS
        .iter()
        .map(|(_, function, operator)| {
            let mut item = original_item_named(&package, function, false);
            item.operator = *operator;
            item
        })
        .collect();
    let oracles = super::admitted_enum_equality::generate_items(
        &package,
        Some(compiled.context()),
        &items,
        &[],
    )
    .expect("the bounded corpus generates");
    Corpus {
        package,
        oracles,
        items,
    }
}

/// The generated claim of the `index`th requested item; the claim map is ordered by
/// descriptor key, not by request.
fn generated(corpus: &Corpus, index: usize) -> &GeneratedCompositeEqualityClaim {
    let item = &corpus.items[index];
    corpus
        .oracles
        .claim_map
        .items
        .iter()
        .find_map(|entry| match &entry.result {
            ClaimDisposition::Generated(claim)
                if claim.node_id == item.node_id && claim.descriptor.operator == item.operator =>
            {
                Some(claim.as_ref())
            }
            _ => None,
        })
        .expect("a generated claim for the item")
}

/// Stage the corpus crate with `symbols.json` and aliases, optionally mutating the emitted
/// library, and return the staged native test names.
fn stage(
    workspace: &std::path::Path,
    label: &str,
    corpus: &Corpus,
    mutate_lib: Option<(&str, &str)>,
) -> (String, Vec<String>) {
    let mut names = String::new();
    let mut symbols = serde_json::Map::new();
    for (index, (alias, ..)) in ITEMS.iter().enumerate() {
        let claim = generated(corpus, index);
        names.push_str(&format!(
            "pub use self::{{{} as environment_{alias}, {} as oracle_{alias}}};\n",
            claim.environment_symbol, claim.oracle_symbol
        ));
        symbols.insert((*alias).to_owned(), claim.oracle_symbol.clone().into());
    }
    let symbols = serde_json::Value::Object(symbols).to_string();
    let artifacts: Vec<(String, String)> = corpus
        .oracles
        .artifacts
        .iter()
        .map(|artifact| {
            let mut contents = artifact.contents.clone();
            if let (Some((from, to)), "src/lib.rs") = (mutate_lib, artifact.path.as_str()) {
                assert!(contents.contains(from), "the mutation target is emitted");
                contents = contents.replace(from, to);
                assert_ne!(
                    contents, artifact.contents,
                    "the mutation changes the emitted source"
                );
            }
            (artifact.path.clone(), contents)
        })
        .collect();
    let tests = stage_composite_agreement_case(
        workspace,
        label,
        artifacts
            .iter()
            .map(|(path, contents)| (path.as_str(), contents.as_str())),
        &[
            ("names.rs", names.as_str()),
            ("symbols.json", symbols.as_str()),
            ("case-mode", fixture::BOUNDED_CASE),
        ],
        "tests/composite_equality_support/closure_cases.rs",
    );
    (label.to_owned(), tests)
}

/// AC-21 and AC-22 run natively against the emitted crate: the closure equals the
/// constructed environment's declarations, and the oracle completes every pair of the domain
/// with the FR-149 verdict and schedule.
#[trace("TC-029", "FR-018-AC-21", "FR-018-AC-22")]
#[test]
fn tc_029_ac21_ac22_native_closure_and_totality_over_the_refinement_domain() {
    let corpus = corpus();
    let scratch = super::scratch_crate::TemporaryDirectory::new("composite-closure-workspace");
    let staged = stage(&scratch.path, "composite-closure-bounded", &corpus, None);
    run_composite_agreement_workspace(&scratch.path, &[staged]);
}

/// Control: the same native test fails on an emitted oracle that evaluates the other operator.
#[trace("TC-029", "FR-018-AC-22")]
#[test]
fn tc_029_ac22_native_run_refuses_an_oracle_of_the_other_operator() {
    let corpus = corpus();
    let scratch = super::scratch_crate::TemporaryDirectory::new("composite-closure-mutant");
    let staged = stage(
        &scratch.path,
        "composite-closure-mutant",
        &corpus,
        Some((
            "sv::EqualityOperator::Equal",
            "sv::EqualityOperator::NotEqual",
        )),
    );
    let outcome = std::panic::catch_unwind(|| {
        run_composite_agreement_workspace(&scratch.path, &[staged]);
    });
    let payload = outcome.expect_err("a flipped operator must fail the native domain run");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .expect("the workspace failure carries a message");
    assert!(
        message.contains("composite agreement workspace failed")
            && !message.contains("could not compile")
            && message.contains(
                "test tc_029_ac22_oracle_completes_every_domain_pair_with_the_fr149_verdict_and_schedule ... FAILED"
            )
            && message.contains("flags_equal")
            && message.contains("Completed("),
        "the failure must be the domain verdict assertion, not a build or setup error: {message}"
    );
}

// ---- FR-018-AC-23: the independent read ---------------------------------------------------

/// A declared type as the package says it, read only through the admitted V2 graph.
#[derive(Debug, Eq, PartialEq)]
enum Read {
    Boolean,
    Integer(Option<(i128, i128)>),
    Option(Box<Read>),
    Composite(CheckedNodeId),
    Payload(Vec<Read>),
}

fn body(package: &CheckedPackageV2, id: &CheckedNodeId) -> serde_json::Value {
    let node = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("node in the admitted graph");
    serde_json::to_value(&node.body).expect("a term serializes")
}

fn node_shape(package: &CheckedPackageV2, id: &CheckedNodeId) -> (String, String) {
    let node = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("node in the admitted graph");
    (node.node_tag.to_string(), node.semantic_form.to_string())
}

fn members(term: &serde_json::Value) -> Vec<serde_json::Value> {
    assert_eq!(term["term"], "aggregate", "{term}");
    term["members"]
        .as_array()
        .expect("aggregate members")
        .clone()
}

fn target(term: &serde_json::Value) -> CheckedNodeId {
    assert_eq!(term["term"], "reference", "{term}");
    serde_json::from_value(term["target"].clone()).expect("a node id")
}

fn read_type(package: &CheckedPackageV2, id: &CheckedNodeId) -> Read {
    match node_shape(package, id) {
        (tag, form) if tag == "scalar_type" && form == "boolean" => Read::Boolean,
        (tag, form) if tag == "scalar_type" && form == "integer" => Read::Integer(None),
        (tag, form) if tag == "bounded_domain" && form == "integer_range" => {
            let bound = |name: &str| -> i128 {
                let member = members(&body(package, id))
                    .into_iter()
                    .find(|member| member["name"] == name)
                    .expect("named bound");
                member["value"]["value"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| member["value"]["value"].to_string())
                    .parse()
                    .expect("integer bound")
            };
            Read::Integer(Some((bound("min"), bound("max"))))
        }
        (tag, form) if tag == "composite_type" && form == "option" => {
            let payload = members(&body(package, id));
            assert_eq!(payload.len(), 1, "an option has one payload reference");
            Read::Option(Box::new(read_type(package, &target(&payload[0]))))
        }
        (tag, form)
            if tag == "composite_type"
                && (form == "record" || form == "tuple" || form == "union") =>
        {
            Read::Composite(id.clone())
        }
        other => panic!("not a Boolean or bounded-integer shape: {other:?}"),
    }
}

/// `(form, [(name, optional, type)])` of a record or tuple node.
fn read_declaration(
    package: &CheckedPackageV2,
    id: &CheckedNodeId,
) -> (String, Vec<(String, bool, Read)>) {
    let (_, form) = node_shape(package, id);
    let terms = members(&body(package, id));
    let read = if form == "record" {
        terms
            .iter()
            .map(|member| {
                assert_eq!(member["term"], "binding", "{member}");
                let name = member["name"].as_str().expect("field name").to_owned();
                let value = &member["value"];
                if value["term"] == "reference" {
                    (name, false, read_type(package, &target(value)))
                } else {
                    let wrapper = members(value);
                    assert_eq!(wrapper.len(), 1);
                    assert_eq!(wrapper[0]["name"], "optional");
                    let payload = read_type(package, &target(&wrapper[0]["value"]));
                    // The wrapper names an option node; the declared member is its payload.
                    match payload {
                        Read::Option(inner) => (name, true, *inner),
                        other => panic!("optional wrapper over {other:?}"),
                    }
                }
            })
            .collect()
    } else if form == "union" {
        terms
            .iter()
            .map(|member| {
                assert_eq!(member["term"], "binding", "{member}");
                let name = member["name"].as_str().expect("member name").to_owned();
                let payload = members(&member["value"])
                    .iter()
                    .map(|position| read_type(package, &target(position)))
                    .collect();
                (name, false, Read::Payload(payload))
            })
            .collect()
    } else {
        terms
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    index.to_string(),
                    false,
                    read_type(package, &target(member)),
                )
            })
            .collect()
    };
    (form, read)
}

fn describe(value: &RecordedType, closure: &[RecordedDeclaration]) -> Read {
    match value {
        RecordedType::Boolean => Read::Boolean,
        RecordedType::Integer => Read::Integer(None),
        RecordedType::Int { lower, upper } => {
            Read::Integer(Some((lower.parse().unwrap(), upper.parse().unwrap())))
        }
        RecordedType::Option { payload } => Read::Option(Box::new(describe(payload, closure))),
        RecordedType::Composite { key } => Read::Composite(
            closure
                .iter()
                .find(|declaration| &declaration.key == key)
                .expect("a recorded composite is in the closure")
                .node_id
                .clone(),
        ),
        RecordedType::Payload { positions } => Read::Payload(
            positions
                .iter()
                .map(|position| describe(position, closure))
                .collect(),
        ),
        RecordedType::Other { name } => panic!("family {name} is outside this corpus"),
    }
}

/// Compare a claim's closure and operand types with the independent read of the package:
/// member names, presence, integer bounds and option nesting.
fn check_against_package(
    package: &CheckedPackageV2,
    claim: &GeneratedCompositeEqualityClaim,
) -> Result<(), String> {
    for declaration in &claim.declaration_closure {
        let (form, read) = read_declaration(package, &declaration.node_id);
        let recorded_form = match declaration.form {
            RecordedForm::Record => "record",
            RecordedForm::Tuple => "tuple",
            RecordedForm::Union => "union",
        };
        if form != recorded_form {
            return Err(format!(
                "{:?}: form {recorded_form} != {form}",
                declaration.node_id
            ));
        }
        if declaration.members.len() != read.len() {
            return Err(format!("{:?}: member count", declaration.node_id));
        }
        for (member, (name, optional, value_type)) in declaration.members.iter().zip(read) {
            if member.name != name {
                return Err(format!("member name {} != {name}", member.name));
            }
            if (member.presence == RecordedPresence::Optional) != optional {
                return Err(format!("{name}: presence"));
            }
            let recorded = describe(&member.value_type, &claim.declaration_closure);
            if recorded != value_type {
                return Err(format!("{name}: {recorded:?} != {value_type:?}"));
            }
        }
    }
    for (recorded, source) in [
        (
            &claim.operand_types.left,
            &claim.descriptor.left_source_type,
        ),
        (
            &claim.operand_types.right,
            &claim.descriptor.right_source_type,
        ),
    ] {
        let read = read_type(package, source);
        let described = describe(recorded, &claim.declaration_closure);
        if described != read {
            return Err(format!("operand {described:?} != {read:?}"));
        }
    }
    Ok(())
}

/// A mutation of the claim at `item` (an index into [`ITEMS`]).
type ClaimMutation = (
    &'static str,
    usize,
    fn(&mut GeneratedCompositeEqualityClaim),
);

#[trace("TC-029", "FR-018-AC-23")]
#[test]
fn tc_029_ac23_closure_equals_an_independent_read_of_the_checked_package() {
    let corpus = corpus();
    let mut members_checked = 0;
    for (index, _) in corpus.items.iter().enumerate() {
        let claim = generated(&corpus, index);
        check_against_package(&corpus.package, claim)
            .unwrap_or_else(|error| panic!("{}: {error}", ITEMS[index].0));
        members_checked += claim
            .declaration_closure
            .iter()
            .map(|declaration| declaration.members.len())
            .sum::<usize>();
    }
    assert_eq!(
        members_checked,
        3 * 2 + 2 + 3,
        "two Flags items, one Duo item and one three-member union"
    );

    // Controls: each fault of the closure is caught by the independent read, which the
    // generator's own reconstruction could not see (it would carry the same fault).
    let mutants: [ClaimMutation; 9] = [
        ("dropped member", 0, |claim| {
            claim.declaration_closure[0].members.pop();
        }),
        ("narrowed bound", 0, |claim| {
            claim.declaration_closure[0].members[1].value_type = RecordedType::Int {
                lower: "0".into(),
                upper: "1".into(),
            };
        }),
        ("optional read as required", 0, |claim| {
            claim.declaration_closure[0].members[2].presence = RecordedPresence::Required;
        }),
        ("record operand replaced by Boolean", 0, |claim| {
            claim.operand_types.left = RecordedType::Boolean;
        }),
        ("option payload bound narrowed", 3, |claim| {
            claim.operand_types.left = RecordedType::Option {
                payload: Box::new(RecordedType::Int {
                    lower: "0".into(),
                    upper: "1".into(),
                }),
            };
        }),
        ("nested option layer dropped", 4, |claim| {
            claim.operand_types.left = RecordedType::Option {
                payload: Box::new(RecordedType::Boolean),
            };
        }),
        ("option read as its payload", 3, |claim| {
            claim.operand_types.right = RecordedType::Int {
                lower: "0".into(),
                upper: "2".into(),
            };
        }),
        ("nullary union member dropped", 5, |claim| {
            claim.declaration_closure[0].members.remove(0);
        }),
        ("union payload position dropped", 5, |claim| {
            let RecordedType::Payload { positions } =
                &mut claim.declaration_closure[0].members[2].value_type
            else {
                panic!("the Both member records its payload");
            };
            positions.pop();
        }),
    ];
    for (label, item, mutate) in mutants {
        let claim = generated(&corpus, item).clone();
        assert_eq!(check_against_package(&corpus.package, &claim), Ok(()));
        let mut changed = claim.clone();
        mutate(&mut changed);
        assert_ne!(changed, claim, "{label}: the mutation changes the claim");
        assert!(
            check_against_package(&corpus.package, &changed).is_err(),
            "{label} must fail the independent read"
        );
    }
}

/// The retained TC-029 `option` case (`Option<Int[-100, 100]>` on both sides) also has only
/// bounded-integer leaves, so its claim is read against its own package too.
#[trace("TC-029", "FR-018-AC-23")]
#[test]
fn tc_029_ac23_retained_option_case_equals_an_independent_read() {
    let compiled = fixture::compile("option");
    let package = super::admitted_enum_equality::checked(&compiled, &[]);
    let item = original_item_named(&package, "f", false);
    let oracles = super::admitted_enum_equality::generate_items(
        &package,
        Some(compiled.context()),
        std::slice::from_ref(&item),
        &[],
    )
    .expect("the retained option case generates");
    let ClaimDisposition::Generated(claim) = &oracles.claim_map.items[0].result else {
        panic!("retained option case refused");
    };
    assert_eq!(check_against_package(&package, claim), Ok(()));
    let bounded = RecordedType::Option {
        payload: Box::new(RecordedType::Int {
            lower: "-100".into(),
            upper: "100".into(),
        }),
    };
    assert_eq!(claim.operand_types.left, bounded);
    let mut narrowed = claim.as_ref().clone();
    narrowed.operand_types.left = RecordedType::Option {
        payload: Box::new(RecordedType::Int {
            lower: "-100".into(),
            upper: "99".into(),
        }),
    };
    assert!(check_against_package(&package, &narrowed).is_err());
}
