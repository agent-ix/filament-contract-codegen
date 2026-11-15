use super::control_fixture::{applications, package, read_node, scalar_type_node};
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
    let scalar = scalar_type_node(&package);
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
