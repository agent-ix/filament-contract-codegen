// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native FR-018-AC-21 and AC-22 over Boolean and bounded-integer shapes: the claim-map
//! closure against the declarations the emitted environment constructor returns, and the
//! emitted oracle over the whole refinement domain against FSpec FR-149's verdict and
//! occurrence-pair schedule, both computed here from the values.
#[macro_use]
#[path = "agreement.rs"]
mod support;
#[path = "codes.rs"]
mod package;
#[path = "authentic_native.rs"]
mod fixture;
mod generated {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/names.rs"));

    pub(super) fn declarations(prepared: &PreparedEqualityEnvironment<'_>) -> Vec<sv::CompositeDeclaration> {
        prepared.environment.composites().cloned().collect()
    }
    pub(super) fn environment<'a>(prepared: &'a PreparedEqualityEnvironment<'_>) -> &'a sv::TypeEnvironment {
        &prepared.environment
    }
    pub(super) fn operand_types(prepared: &PreparedEqualityEnvironment<'_>) -> (exact::ValueType, exact::ValueType) {
        (prepared.left_source.clone(), prepared.right_source.clone())
    }
}
use ix_trace_rs::trace;
use serde_json::Value as Json;
use support::rt_side::*;

const ALIASES: [&str; 6] = [
    "flags_equal",
    "flags_not_equal",
    "duo_equal",
    "option_equal",
    "nested_option_equal",
    "shape_equal",
];

/// The union item has no value-level domain here: its closure is checked (AC-21), its
/// oracle's totality is not (AC-22).
const DOMAIN_ALIASES: [&str; 5] = [
    "flags_equal",
    "flags_not_equal",
    "duo_equal",
    "option_equal",
    "nested_option_equal",
];

fn symbols() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/symbols.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The generated claim of the item whose oracle is `alias`, read from `claim-map.json`.
fn claim(alias: &str) -> Json {
    let symbol = symbols()[alias].as_str().expect("alias symbol").to_owned();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/claim-map.json");
    let map: Json = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    map["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["result"].clone())
        .find(|result| result["oracle_symbol"] == symbol)
        .expect("claim for the alias")
}

fn render_json_type(value: &Json) -> String {
    match value["family"].as_str().expect("family") {
        "boolean" => "bool".to_owned(),
        "int" => format!("int[{},{}]", value["lower"].as_str().unwrap(), value["upper"].as_str().unwrap()),
        "option" => format!("opt<{}>", render_json_type(&value["payload"])),
        "composite" => format!("comp:{}", value["key"].as_str().unwrap()),
        "payload" => format!(
            "payload({})",
            value["positions"]
                .as_array()
                .unwrap()
                .iter()
                .map(render_json_type)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => format!("other:{other}"),
    }
}

fn render_type(value: &ValueType) -> String {
    match value {
        ValueType::Boolean => "bool".to_owned(),
        ValueType::Int(interval) => format!("int[{},{}]", interval.lower(), interval.upper()),
        ValueType::Option(payload) => format!("opt<{}>", render_type(payload)),
        ValueType::Composite(key) => format!("comp:{key}"),
        other => panic!("not a Boolean or bounded-integer shape: {other:?}"),
    }
}

/// `(form, [(name, optional, type)])` per declaration key, from the environment the emitted
/// constructor returned.
fn environment_closure(
    declarations: &[CompositeDeclaration],
) -> std::collections::BTreeMap<String, (String, Vec<(String, bool, String)>)> {
    declarations
        .iter()
        .map(|declaration| {
            let (form, members) = match declaration.shape() {
                CompositeShape::Record(fields) => (
                    "record",
                    fields
                        .iter()
                        .map(|field| {
                            (
                                field.name().to_owned(),
                                field.presence() == Presence::Optional,
                                render_type(field.value_type()),
                            )
                        })
                        .collect(),
                ),
                CompositeShape::Tuple(positions) => (
                    "tuple",
                    positions
                        .iter()
                        .enumerate()
                        .map(|(index, value)| (index.to_string(), false, render_type(value)))
                        .collect(),
                ),
                CompositeShape::Union(members) => (
                    "union",
                    members
                        .iter()
                        .map(|member| {
                            let positions: Vec<String> =
                                member.positions().iter().map(render_type).collect();
                            (
                                member.identifier().to_owned(),
                                false,
                                format!("payload({})", positions.join(",")),
                            )
                        })
                        .collect(),
                ),
            };
            (declaration.key().to_string(), (form.to_owned(), members))
        })
        .collect()
}

/// Member-by-member comparison (name, presence, value type, order) of a claim's closure with
/// the environment's declarations.
fn compare(claim: &Json, declarations: &[CompositeDeclaration]) -> Result<(), String> {
    let expected = environment_closure(declarations);
    let recorded = claim["declaration_closure"].as_array().ok_or("no closure")?;
    if recorded.len() != expected.len() {
        return Err(format!("declaration count {} != {}", recorded.len(), expected.len()));
    }
    for declaration in recorded {
        let key = declaration["key"].as_str().ok_or("key")?;
        let (form, members) = expected.get(key).ok_or_else(|| format!("unexpected {key}"))?;
        if declaration["form"] != *form {
            return Err(format!("{key}: form {} != {form}", declaration["form"]));
        }
        let recorded_members = declaration["members"].as_array().ok_or("members")?;
        if recorded_members.len() != members.len() {
            return Err(format!("{key}: member count"));
        }
        for (recorded, (name, optional, value_type)) in recorded_members.iter().zip(members) {
            if recorded["name"] != *name {
                return Err(format!("{key}: member name {} != {name}", recorded["name"]));
            }
            let presence = if *optional { "optional" } else { "required" };
            if recorded["presence"] != presence {
                return Err(format!("{key}.{name}: presence {} != {presence}", recorded["presence"]));
            }
            let rendered = render_json_type(&recorded["value_type"]);
            if rendered != *value_type {
                return Err(format!("{key}.{name}: type {rendered} != {value_type}"));
            }
        }
    }
    Ok(())
}

fn compare_operands(claim: &Json, prepared: &generated::PreparedEqualityEnvironment<'_>) -> Result<(), String> {
    let (left, right) = generated::operand_types(prepared);
    for (side, actual) in [("left", left), ("right", right)] {
        let recorded = render_json_type(&claim["operand_types"][side]);
        if recorded != render_type(&actual) {
            return Err(format!("{side} operand {recorded} != {}", render_type(&actual)));
        }
    }
    Ok(())
}

#[trace("TC-029", "FR-018-AC-21")]
#[test]
fn tc_029_ac21_closure_equals_the_constructed_environment_declarations() {
    let mut with_declarations = 0;
    for alias in ALIASES {
        let claim = claim(alias);
        let prepared = environment_for(alias);
        let declarations = generated::declarations(&prepared);
        compare(&claim, &declarations).unwrap_or_else(|error| panic!("{alias}: {error}"));
        compare_operands(&claim, &prepared).unwrap_or_else(|error| panic!("{alias}: {error}"));
        with_declarations += usize::from(!declarations.is_empty());
    }
    assert_eq!(with_declarations, 4, "record x2, tuple and union carry declarations; options carry none");

    // Controls: a closure that dropped a member, narrowed a bound, read an optional member as
    // required or swapped a type fails the same comparison.
    let claim = claim("flags_equal");
    let declarations = generated::declarations(&environment_for("flags_equal"));
    assert_eq!(compare(&claim, &declarations), Ok(()));
    let mutants: [(&str, fn(&mut Json)); 4] = [
        ("dropped member", |c| {
            c["declaration_closure"][0]["members"].as_array_mut().unwrap().pop();
        }),
        ("narrowed bound", |c| {
            c["declaration_closure"][0]["members"][1]["value_type"]["upper"] = "1".into();
        }),
        ("optional as required", |c| {
            c["declaration_closure"][0]["members"][2]["presence"] = "required".into();
        }),
        ("reordered members", |c| {
            c["declaration_closure"][0]["members"].as_array_mut().unwrap().swap(0, 1);
        }),
    ];
    for (label, mutate) in mutants {
        let mut changed = claim.clone();
        mutate(&mut changed);
        assert_ne!(changed, claim, "{label}: the mutation changes the closure");
        assert!(compare(&changed, &declarations).is_err(), "{label} must be refused");
    }

    // The union keeps one member per declared member, a nullary one included.
    let shape = self::claim("shape_equal");
    let shape_declarations = generated::declarations(&environment_for("shape_equal"));
    assert_eq!(compare(&shape, &shape_declarations), Ok(()));
    assert_eq!(shape["declaration_closure"][0]["members"].as_array().unwrap().len(), 3);
    let union_mutants: [(&str, fn(&mut Json)); 3] = [
        ("dropped nullary member", |c| {
            c["declaration_closure"][0]["members"].as_array_mut().unwrap().remove(0);
        }),
        ("dropped payload position", |c| {
            c["declaration_closure"][0]["members"][2]["value_type"]["positions"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
        ("narrowed payload bound", |c| {
            c["declaration_closure"][0]["members"][2]["value_type"]["positions"][1]["upper"] =
                "1".into();
        }),
    ];
    for (label, mutate) in union_mutants {
        let mut changed = shape.clone();
        mutate(&mut changed);
        assert_ne!(changed, shape, "{label}: the mutation changes the closure");
        assert!(compare(&changed, &shape_declarations).is_err(), "{label} must be refused");
    }
}

fn environment_for(alias: &str) -> generated::PreparedEqualityEnvironment<'static> {
    let context = support::context();
    match alias {
        "flags_equal" => generated::environment_flags_equal(Some(context)),
        "flags_not_equal" => generated::environment_flags_not_equal(Some(context)),
        "duo_equal" => generated::environment_duo_equal(Some(context)),
        "option_equal" => generated::environment_option_equal(Some(context)),
        "nested_option_equal" => generated::environment_nested_option_equal(Some(context)),
        "shape_equal" => generated::environment_shape_equal(Some(context)),
        other => panic!("unknown alias {other}"),
    }
    .expect("same original admitted context")
}

// ---- FR-018-AC-22 ---------------------------------------------------------------------

/// A value of the shapes under test, written independently of the runtime's `Value`.
#[derive(Clone, Debug)]
enum M {
    B(bool),
    I(i64),
    Opt(Option<Box<M>>),
    /// A record's slots or a tuple's positions, in declaration order.
    Rec(Vec<Slot>),
}

#[derive(Clone, Debug)]
enum Slot {
    Absent,
    Null,
    Present(M),
}

/// FR-149: same shape and pairwise-equal payloads; two `none` equal, `none` against present
/// unequal; two absent slots equal, two null slots equal, absent against null unequal.
fn model_equal(left: &M, right: &M) -> bool {
    match (left, right) {
        (M::B(l), M::B(r)) => l == r,
        (M::I(l), M::I(r)) => l == r,
        (M::Opt(Some(l)), M::Opt(Some(r))) => model_equal(l, r),
        (M::Opt(None), M::Opt(None)) => true,
        (M::Opt(_), M::Opt(_)) => false,
        (M::Rec(l), M::Rec(r)) => l.iter().zip(r).all(|(l, r)| slot_equal(l, r)),
        _ => panic!("operands of one static type"),
    }
}

fn slot_equal(left: &Slot, right: &Slot) -> bool {
    match (left, right) {
        (Slot::Absent, Slot::Absent) | (Slot::Null, Slot::Null) => true,
        (Slot::Present(l), Slot::Present(r)) => model_equal(l, r),
        _ => false,
    }
}

/// The node count of the occurrence-pair tree: one node per compared pair, a `some`/`some`
/// option adds its payload pair, a composite adds one node per slot pair, and an unequal pair
/// stops nothing (no early exit).
fn model_nodes(left: &M, right: &M) -> u64 {
    match (left, right) {
        (M::Opt(Some(l)), M::Opt(Some(r))) => 1 + model_nodes(l, r),
        (M::Rec(l), M::Rec(r)) => 1 + l.iter().zip(r).map(|(l, r)| slot_nodes(l, r)).sum::<u64>(),
        _ => 1,
    }
}

fn slot_nodes(left: &Slot, right: &Slot) -> u64 {
    match (left, right) {
        (Slot::Present(l), Slot::Present(r)) => model_nodes(l, r),
        _ => 1,
    }
}

/// The static shape a model value is built against.
#[derive(Clone, Debug)]
enum Shape {
    Bool,
    Int(i64, i64),
    Opt(Box<Shape>),
}

fn value_type(shape: &Shape) -> ValueType {
    match shape {
        Shape::Bool => ValueType::Boolean,
        Shape::Int(lo, hi) => ValueType::Int(IntegerInterval::new(Integer::from(*lo), Integer::from(*hi)).unwrap()),
        Shape::Opt(payload) => ValueType::option(value_type(payload)),
    }
}

fn build(environment: &TypeEnvironment, key: &NodeKey, names: &[&str], shape: &[Shape], model: &M) -> Value {
    let leaf = |shape: &Shape, model: &M| build_leaf(shape, model);
    match model {
        M::Rec(slots) => {
            let fields = |slots: &[Slot]| -> Vec<(String, FieldValue)> {
                slots
                    .iter()
                    .zip(names)
                    .zip(shape)
                    .map(|((slot, name), shape)| {
                        let value = match slot {
                            Slot::Absent => FieldValue::Absent,
                            Slot::Null => FieldValue::Null,
                            Slot::Present(m) => FieldValue::Present(leaf(shape, m)),
                        };
                        ((*name).to_owned(), value)
                    })
                    .collect()
            };
            if names.iter().all(|name| name.chars().all(|c| c.is_ascii_digit())) {
                let positions = slots
                    .iter()
                    .zip(shape)
                    .map(|(slot, shape)| match slot {
                        Slot::Present(m) => leaf(shape, m),
                        _ => panic!("tuple positions are always present"),
                    })
                    .collect();
                environment.tuple(key.clone(), positions).unwrap()
            } else {
                let owned = fields(slots);
                environment
                    .record(key.clone(), owned.iter().map(|(n, v)| (n.as_str(), v.clone())).collect())
                    .unwrap()
            }
        }
        other => build_leaf(&shape[0], other),
    }
}

fn build_leaf(shape: &Shape, model: &M) -> Value {
    match (shape, model) {
        (Shape::Bool, M::B(b)) => Value::Boolean(*b),
        (Shape::Int(..), M::I(n)) => Value::Integer(Integer::from(*n)),
        (Shape::Opt(payload), M::Opt(None)) => OptionValue::none(value_type(payload)),
        (Shape::Opt(payload), M::Opt(Some(inner))) => {
            OptionValue::present(value_type(payload), build_leaf(payload, inner)).unwrap()
        }
        other => panic!("model does not fit its shape: {other:?}"),
    }
}

fn present(m: M) -> Slot {
    Slot::Present(m)
}

fn domain(alias: &str) -> (Vec<M>, Vec<Shape>, Vec<&'static str>) {
    match alias {
        "flags_equal" | "flags_not_equal" => {
            let mut values = Vec::new();
            for a in [false, true] {
                for b in 0..=2 {
                    let mut third = vec![Slot::Absent, Slot::Null];
                    third.extend((0..=2).map(|c| present(M::I(c))));
                    for c in third {
                        values.push(M::Rec(vec![present(M::B(a)), present(M::I(b)), c]));
                    }
                }
            }
            (values, vec![Shape::Bool, Shape::Int(0, 2), Shape::Int(0, 2)], vec!["a", "b", "c"])
        }
        "duo_equal" => {
            let mut values = Vec::new();
            for a in [false, true] {
                for b in 0..=2 {
                    values.push(M::Rec(vec![present(M::B(a)), present(M::I(b))]));
                }
            }
            (values, vec![Shape::Bool, Shape::Int(0, 2)], vec!["0", "1"])
        }
        "option_equal" => {
            let mut values = vec![M::Opt(None)];
            values.extend((0..=2).map(|n| M::Opt(Some(Box::new(M::I(n))))));
            (values, vec![Shape::Opt(Box::new(Shape::Int(0, 2)))], vec![])
        }
        "nested_option_equal" => {
            let mut values = vec![M::Opt(None), M::Opt(Some(Box::new(M::Opt(None))))];
            values.extend([false, true].map(|b| M::Opt(Some(Box::new(M::Opt(Some(Box::new(M::B(b)))))))));
            (
                values,
                vec![Shape::Opt(Box::new(Shape::Opt(Box::new(Shape::Bool))))],
                vec![],
            )
        }
        other => panic!("unknown alias {other}"),
    }
}

fn run(alias: &str, prepared: &generated::PreparedEqualityEnvironment<'_>, left: &Value, right: &Value, meter: &mut Meter) -> Outcome<bool> {
    match alias {
        "flags_equal" => generated::oracle_flags_equal(prepared, left, right, meter),
        "flags_not_equal" => generated::oracle_flags_not_equal(prepared, left, right, meter),
        "duo_equal" => generated::oracle_duo_equal(prepared, left, right, meter),
        "option_equal" => generated::oracle_option_equal(prepared, left, right, meter),
        "nested_option_equal" => generated::oracle_nested_option_equal(prepared, left, right, meter),
        other => panic!("unknown alias {other}"),
    }
}

#[trace("TC-029", "FR-018-AC-22")]
#[test]
fn tc_029_ac22_oracle_completes_every_domain_pair_with_the_fr149_verdict_and_schedule() {
    let mut pairs = 0_u64;
    let mut equal_seen = false;
    let mut unequal_seen = false;
    for alias in DOMAIN_ALIASES {
        let prepared = environment_for(alias);
        let environment = generated::environment(&prepared);
        let (models, shapes, names) = domain(alias);
        let key = generated::declarations(&prepared).first().map(CompositeDeclaration::key);
        let negate = alias == "flags_not_equal";
        let values: Vec<Value> = models
            .iter()
            .map(|model| match &key {
                Some(key) => build(environment, key, &names, &shapes, model),
                None => build_leaf(&shapes[0], model),
            })
            .collect();
        for (left_model, left) in models.iter().zip(&values) {
            for (right_model, right) in models.iter().zip(&values) {
                let mut meter = Meter::new(UNLIMITED);
                let outcome = run(alias, &prepared, left, right, &mut meter);
                let expected = model_equal(left_model, right_model) != negate;
                assert_eq!(outcome, Outcome::Completed(expected), "{alias}: {left_model:?} vs {right_model:?}");
                let nodes = model_nodes(left_model, right_model);
                let mut schedule = vec![ChargePoint::EqualityPlanForm, ChargePoint::EqualityPlan];
                schedule.extend((0..nodes).map(|_| ChargePoint::EqualityPair));
                schedule.push(ChargePoint::EqualityResultRetain);
                let admitted: Vec<ChargePoint> = meter.admitted_charges().iter().map(|charge| charge.point).collect();
                assert_eq!(admitted, schedule, "{alias}: {left_model:?} vs {right_model:?}");
                pairs += 1;
                equal_seen |= expected;
                unequal_seen |= !expected;
            }
        }
    }
    // 30^2 twice (both operators), 6^2, 4^2 and 4^2.
    assert_eq!(pairs, 900 * 2 + 36 + 16 + 16);
    assert!(equal_seen && unequal_seen, "the domain holds both verdicts");
}
