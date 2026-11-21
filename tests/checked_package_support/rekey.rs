//! Reader admission for fixtures whose producers assign structural node IDs.
//!
//! The fixture builder keeps the same IDs in the authored wire and in expected
//! values. A body edit does not change a node's source position or identity.

use filament_contract_model::{
    CheckedNodeId, CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
    CheckedPackageV2ReadResult,
};
use serde_json::Value;

/// The exact wire submitted to the reader and its admission result.
pub struct ResolvedWire {
    pub wire: Value,
    pub outcome: CheckedPackageV2ReadResult,
    pub ids: FixtureIds,
}

/// Producer-assigned fixture IDs need no post-read substitution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FixtureIds;

impl FixtureIds {
    /// Keep an expected node ID identical to the producer's graph ID.
    pub fn resolve(&self, node: &CheckedNodeId) -> CheckedNodeId {
        node.clone()
    }

    /// Expected typed references already use the producer's graph IDs.
    pub fn resolve_value(&self, _value: &mut Value) {}
}

/// Structural package identity is its name and exact lock; body changes do not rekey it.
pub(super) fn refresh_identity(wire: &mut Value) {
    assert!(wire["name"].is_string(), "fixture package needs a name");
    assert!(wire["lock"].is_object(), "fixture package needs a lock");
}

/// Author empty operand rows for name-only fixture nodes, then let the reader admit the wire.
/// Existing operand rows and all producer identities remain unchanged.
pub fn resolve_wire(
    raw: &Value,
    evidence: &CheckedPackageEvidence,
    limits: CheckedPackageReadLimits,
) -> ResolvedWire {
    let mut wire = raw.clone();
    author_name_only_operand_rows(&mut wire);
    author_body_locations(&mut wire);
    let bytes = serde_json::to_vec(&wire).expect("fixture wire serializes");
    let outcome = CheckedPackageV2::read(&bytes, limits, evidence);
    ResolvedWire {
        wire,
        outcome,
        ids: FixtureIds,
    }
}

/// Supply the required empty rows for this producer's name-only fixture graph.
/// Existing rows, slot identities and document-index identities are never rewritten.
pub fn author_name_only_operand_rows(wire: &mut Value) {
    if let Some(graph) = wire
        .get_mut("semantic_graph")
        .and_then(Value::as_object_mut)
    {
        if !graph.contains_key("operand_rows") {
            if let Some(nodes) = graph.get("nodes").and_then(Value::as_array) {
                if nodes.iter().all(|node| {
                    node["node_id"]["path"]
                        .as_array()
                        .is_some_and(|path| path.iter().all(Value::is_string))
                }) {
                    let rows = vec![Vec::<u64>::new(); nodes.len()];
                    graph.insert("operand_rows".to_owned(), serde_json::json!(rows));
                }
            }
        }
    }
}

/// The control applications a body walk passes through without counting a term.
const WALKED_THROUGH: [&str; 3] = [
    "filament.op.control.let",
    "filament.op.control.if",
    "filament.op.control.case",
];

/// The node a function's `body` binding references, when the node carries one.
fn body_target(node: &Value) -> Option<&Value> {
    node["body"]["members"]
        .as_array()?
        .iter()
        .find(|member| member["term"] == "binding" && member["name"] == "body")
        .map(|member| &member["value"])
        .filter(|value| value["term"] == "reference")
        .map(|value| &value["target"])
}

/// The expression nodes a term references, in pre-order.
fn expression_references(term: &Value, wire: &Value, found: &mut Vec<usize>) {
    match term["term"].as_str() {
        Some("reference") => {
            let position = wire["semantic_graph"]["nodes"]
                .as_array()
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .position(|node| node["node_id"] == term["target"])
                });
            if let Some(position) = position {
                let node = &wire["semantic_graph"]["nodes"][position];
                if node["node_tag"] == "expression" && term["target"]["package"] == wire["name"] {
                    found.push(position);
                }
            }
        }
        Some("application") => {
            for argument in term["arguments"].as_array().into_iter().flatten() {
                expression_references(argument, wire, found);
            }
        }
        Some("aggregate") => {
            for member in term["members"].as_array().into_iter().flatten() {
                expression_references(member, wire, found);
            }
        }
        Some("binding") => expression_references(&term["value"], wire, found),
        _ => {}
    }
}

/// Author the `checked_term_data.body_locations` rows the producer writes for every term of every
/// function body (FSpec FR-322): one row per application term the body walk visits, keyed by
/// application, occurrence and a contiguous term ordinal. A wire with no function body, or one
/// that already carries the array, is left unchanged.
pub fn author_body_locations(wire: &mut Value) {
    let Some(nodes) = wire["semantic_graph"]["nodes"].as_array() else {
        return;
    };
    let package = wire["name"].clone();
    let mut functions = Vec::new();
    for (position, node) in nodes.iter().enumerate() {
        if node["node_tag"] == "function"
            && node["node_id"]["package"] == package
            && body_target(node).is_some()
        {
            functions.push(position);
        }
    }
    if functions.is_empty()
        || wire
            .get("checked_term_data")
            .is_some_and(|data| data.get("body_locations").is_some())
    {
        return;
    }
    // (application, occurrence ordinal) -> rows in function order.
    let mut rows: Vec<(Value, Value, Value)> = Vec::new();
    for (index, function) in functions.iter().enumerate() {
        let function_node = &nodes[*function];
        let name = function_node["node_id"]["path"]
            .as_array()
            .and_then(|path| path.last())
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));
        let origin = serde_json::json!({
            "kind": "function_body",
            "function": function_node["node_id"],
            "name": name,
            "index": index,
        });
        let Some(start) = body_target(function_node)
            .and_then(|target| nodes.iter().position(|node| node["node_id"] == *target))
        else {
            continue;
        };
        let mut pending = vec![start];
        let mut visit = 0_u64;
        while let Some(position) = pending.pop() {
            let node = &nodes[position];
            let term = &node["body"];
            let walked_through = term["operation"]["identity"]
                .as_str()
                .is_some_and(|identity| WALKED_THROUGH.contains(&identity));
            if term["term"] == "application"
                && node["semantic_form"] != "reference"
                && !walked_through
            {
                let occurrence = node["occurrences"][0].clone();
                rows.push((
                    node["node_id"].clone(),
                    occurrence,
                    serde_json::json!({"origin": origin, "path": [visit]}),
                ));
            }
            visit += 1;
            let mut next = Vec::new();
            expression_references(term, wire, &mut next);
            next.reverse();
            pending.extend(next);
        }
    }
    let order = |id: &Value| {
        (
            id["package"].as_str().unwrap_or_default().to_owned(),
            id["path"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|segment| segment.as_str().unwrap_or_default().to_owned())
                .collect::<Vec<_>>(),
            id["ordinal"].as_u64().unwrap_or_default(),
        )
    };
    rows.sort_by_key(|(application, occurrence, _)| {
        (
            order(application),
            occurrence["role"].as_str().unwrap_or_default().to_owned(),
            occurrence["ordinal"].as_u64().unwrap_or_default(),
        )
    });
    let mut written: Vec<Value> = Vec::new();
    let mut ordinal = 0_u64;
    for (index, (application, occurrence, location)) in rows.iter().enumerate() {
        let continues =
            index > 0 && rows[index - 1].0 == *application && rows[index - 1].1 == *occurrence;
        ordinal = if continues { ordinal + 1 } else { 0 };
        written.push(serde_json::json!({
            "application": application,
            "occurrence": occurrence,
            "term_ordinal": ordinal,
            "location": location,
        }));
    }
    let data = wire
        .as_object_mut()
        .expect("fixture wire is an object")
        .entry("checked_term_data")
        .or_insert_with(|| {
            serde_json::json!({
                "equality_decisions": [],
                "protocol_locations": [],
                "dispatch_tables": [],
                "probabilistic_claims": [],
            })
        });
    data["body_locations"] = Value::Array(written);
}
