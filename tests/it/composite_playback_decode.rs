//! FR-025-AC-9 and FR-033-AC-2 to FR-033-AC-5 (TC-048 steps 2 to 5): the leaf bindings of a
//! composite shadow identity, the playback decode through them and the rebuild of the operands.
//!
//! The playbacks here are the transcript shape Kani prints, synthesized from the identity's own
//! leaves; the installed-backend half, a real falsified run decoded the same way, is in
//! `composite_shadow_kani.rs`.

use filament_contract_codegen::{
    check_leaf_bindings, check_leaf_call_order, decode_composite_falsification,
    generate_composite_shadow, generate_composite_shadow_with_proof_body, parse_assertion_playback,
    rebuild_composite_operands, reconstruct_composite_operands, CompositeShadowRefusal,
    ConflictMember, KaniShadowHarness, LeafBindingRefusal, LeafKind, OperandSource, ShadowIdentity,
    ShadowOperand, WitnessDecodeLimits,
};
use fsl_replay::{WitnessField, WitnessSlot, WitnessValue};
use ix_trace_rs::trace;

use super::composite_equality_closure::corpus;
use super::composite_shadow::{
    entry, generate, item_of, DUO_EQUAL, FLAGS_EQUAL, FLAGS_NOT_EQUAL, NESTED_OPTION_EQUAL,
    OPTION_EQUAL,
};

#[derive(Clone, Copy)]
enum E {
    B(bool),
    I(i64),
}

fn entry_text(entry: E) -> String {
    let (comment, bytes) = match entry {
        E::B(value) => (value.to_string(), vec![u8::from(value)]),
        E::I(value) => (value.to_string(), value.to_le_bytes().to_vec()),
    };
    let bytes: Vec<String> = bytes.iter().map(u8::to_string).collect();
    format!(
        "        // {comment}\n        vec![{}],\n",
        bytes.join(", ")
    )
}

fn transcript_for(harness: &str, entries: &[E]) -> String {
    let body: String = entries.iter().copied().map(entry_text).collect();
    format!(
        "/// Test generated for harness `{harness}`\n\
         /// Check for `assertion`: \"assertion failed: verdict == expected_verdict\"\n\
         #[test]\n\
         fn kani_concrete_playback_check_1() {{\n\
         \x20   let concrete_vals: Vec<Vec<u8>> = vec![\n{body}    ];\n\
         \x20   kani::concrete_playback_run(concrete_vals, check);\n\
         }}\n"
    )
}

fn harness_path(identity: &ShadowIdentity) -> String {
    format!("{}::{}", identity.module_symbol, identity.harness_symbol)
}

fn limits() -> WitnessDecodeLimits {
    WitnessDecodeLimits {
        bytes: 1 << 20,
        values: 64,
    }
}

fn reconstruct(
    identity: &ShadowIdentity,
    entries: &[E],
) -> Result<[WitnessValue; 2], filament_contract_codegen::DecodeFailure> {
    let text = transcript_for(&harness_path(identity), entries);
    let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024)
        .expect("the synthesized transcript parses");
    reconstruct_composite_operands(identity, &playback, limits())
}

/// One `Flags { a, b, c? }` parameter's entries: `c` is drawn only when its state is 0.
fn flags(a: bool, b: i64, state: i64, c: i64) -> Vec<E> {
    let mut entries = vec![E::B(a), E::I(b), E::I(state)];
    if state == 0 {
        entries.push(E::I(c));
    }
    entries
}

fn fields(value: &WitnessValue) -> &[WitnessField] {
    match value {
        WitnessValue::Record { fields, .. } => fields,
        other => panic!("a record, not {other:?}"),
    }
}

fn slot<'a>(value: &'a WitnessValue, name: &str) -> &'a WitnessSlot {
    &fields(value)
        .iter()
        .find(|field| field.name == name)
        .expect("the declared field")
        .slot
}

fn is_present_int(slot: &WitnessSlot, expected: i64) -> bool {
    matches!(slot, WitnessSlot::Present(WitnessValue::ExactInteger(value))
        if *value == filament_exact::Integer::from(expected))
}

fn flags_identity() -> KaniShadowHarness {
    generate(&corpus(), FLAGS_EQUAL)
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_every_supported_identity_records_node_path_kind_and_bounds_that_pass_the_binding_check() {
    let corpus = corpus();
    for index in [
        FLAGS_EQUAL,
        FLAGS_NOT_EQUAL,
        DUO_EQUAL,
        OPTION_EQUAL,
        NESTED_OPTION_EQUAL,
    ] {
        let harness = generate(&corpus, index);
        check_leaf_bindings(&harness.identity).expect("the generated identity binds");
        check_leaf_call_order(&harness.identity, &harness.rust.contents)
            .expect("the generated source draws the identity's leaves in its order");
        for leaf in &harness.identity.leaves {
            assert!(!leaf.node.is_empty(), "{}", leaf.name);
            assert_eq!(leaf.declared, leaf.drawn, "{}", leaf.name);
        }
    }
    // Option presence and optional-slot state are distinct leaf kinds, and the two operands of a
    // two-parameter claim name two different parameter nodes.
    let option = generate(&corpus, OPTION_EQUAL).identity;
    assert!(option
        .leaves
        .iter()
        .any(|leaf| leaf.kind == LeafKind::Presence));
    assert!(option
        .leaves
        .iter()
        .all(|leaf| leaf.kind != LeafKind::SlotState));
    let flags = flags_identity().identity;
    assert!(flags
        .leaves
        .iter()
        .any(|leaf| leaf.kind == LeafKind::SlotState));
    assert!(flags
        .leaves
        .iter()
        .all(|leaf| leaf.kind != LeafKind::Presence));
    let composite = flags.composite.as_ref().unwrap();
    let [OperandSource::Parameter { node: left }, OperandSource::Parameter { node: right }] =
        &composite.operands
    else {
        panic!("two parameter operands");
    };
    assert_ne!(left, right);
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_a_missing_duplicate_unknown_or_conflicting_leaf_refuses_with_a_typed_reason() {
    let healthy = flags_identity().identity;
    let leaf_count = healthy.leaves.len();

    let mut missing = healthy.clone();
    let removed = missing.leaves.remove(1);
    assert!(matches!(
        check_leaf_bindings(&missing),
        Err(LeafBindingRefusal::Missing { path, .. }) if path == removed.path
    ));

    let mut duplicate_name = healthy.clone();
    duplicate_name.leaves[1].name = duplicate_name.leaves[0].name.clone();
    assert_eq!(
        check_leaf_bindings(&duplicate_name),
        Err(LeafBindingRefusal::Duplicate {
            name: healthy.leaves[0].name.clone()
        })
    );

    let mut duplicate_path = healthy.clone();
    duplicate_path.leaves[1].path = duplicate_path.leaves[0].path.clone();
    assert_eq!(
        check_leaf_bindings(&duplicate_path),
        Err(LeafBindingRefusal::Duplicate {
            name: healthy.leaves[1].name.clone()
        })
    );

    let mut unknown_path = healthy.clone();
    unknown_path.leaves[1].path = "zz".to_owned();
    assert_eq!(
        check_leaf_bindings(&unknown_path),
        Err(LeafBindingRefusal::Unknown {
            name: healthy.leaves[1].name.clone()
        })
    );

    let mut unknown_node = healthy.clone();
    unknown_node.leaves[0].node = "not-an-operand".to_owned();
    assert_eq!(
        check_leaf_bindings(&unknown_node),
        Err(LeafBindingRefusal::Unknown {
            name: healthy.leaves[0].name.clone()
        })
    );

    for (what, edit) in [
        (
            ConflictMember::Kind,
            (|identity: &mut ShadowIdentity| identity.leaves[1].kind = LeafKind::Boolean)
                as fn(&mut ShadowIdentity),
        ),
        (ConflictMember::Declared, |identity| {
            identity.leaves[1].declared = (0, 9);
        }),
        (ConflictMember::Drawn, |identity| {
            identity.leaves[1].drawn = (0, 1);
        }),
    ] {
        let mut conflicting = healthy.clone();
        edit(&mut conflicting);
        assert_eq!(
            check_leaf_bindings(&conflicting),
            Err(LeafBindingRefusal::Conflict {
                name: healthy.leaves[1].name.clone(),
                member: what
            }),
            "{what:?}"
        );
    }

    let mut swapped = healthy.clone();
    swapped.leaves.swap(0, 1);
    assert_eq!(
        check_leaf_bindings(&swapped),
        Err(LeafBindingRefusal::OutOfOrder {
            name: healthy.leaves[1].name.clone()
        })
    );

    // The two parameters' leaves alternate, each keeping its own order: only the contiguity of a
    // parameter's leaves can refuse it, and it names the first leaf that reopens a closed node.
    let half = leaf_count / 2;
    assert!(healthy.leaves[..half]
        .iter()
        .all(|leaf| leaf.node == healthy.leaves[0].node));
    assert!(healthy.leaves[half..]
        .iter()
        .all(|leaf| leaf.node == healthy.leaves[half].node));
    let mut interleaved = healthy.clone();
    interleaved.leaves = (0..half)
        .flat_map(|index| {
            [
                healthy.leaves[index].clone(),
                healthy.leaves[half + index].clone(),
            ]
        })
        .collect();
    assert_eq!(
        check_leaf_bindings(&interleaved),
        Err(LeafBindingRefusal::OutOfOrder {
            name: healthy.leaves[1].name.clone()
        })
    );

    // A shadow identity of no composite family has no composite leaves.
    let mut plain = healthy.clone();
    plain.composite = None;
    assert_eq!(
        check_leaf_bindings(&plain),
        Err(LeafBindingRefusal::NotComposite)
    );
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_the_generated_source_draws_the_identity_leaves_in_order_and_a_reordered_draw_refuses() {
    let harness = flags_identity();
    let source = harness.rust.contents.clone();
    // Swap the first two draws of the proof: the identity no longer lists them in call order.
    let first = "let a_a: bool = kani::any();";
    let second = "let a_b: i64 = kani::any();";
    let swapped = source
        .replacen(first, "@@FIRST@@", 1)
        .replacen(second, first, 1)
        .replacen("@@FIRST@@", second, 1);
    assert_ne!(swapped, source);
    assert!(matches!(
        check_leaf_call_order(&harness.identity, &swapped),
        Err(LeafBindingRefusal::CallOrder { .. })
    ));
    // A draw of another primitive type is not the identity's.
    let retyped = source.replacen(first, "let a_a: i64 = kani::any();", 1);
    assert!(matches!(
        check_leaf_call_order(&harness.identity, &retyped),
        Err(LeafBindingRefusal::CallOrder { .. })
    ));
    // A source with no proof function, or that does not parse, is unreadable, not "no draws".
    assert_eq!(
        check_leaf_call_order(&harness.identity, "fn other() {}"),
        Err(LeafBindingRefusal::SourceUnreadable)
    );
    assert_eq!(
        check_leaf_call_order(&harness.identity, "fn ("),
        Err(LeafBindingRefusal::SourceUnreadable)
    );
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_a_source_that_disagrees_with_the_identity_in_call_order_refuses_generation_with_no_harness(
) {
    let corpus = corpus();
    let entry = entry(&corpus, FLAGS_EQUAL);
    let item = item_of(&entry, "subject");
    assert!(generate_composite_shadow_with_proof_body(&item, "").is_ok());
    let refusal = generate_composite_shadow_with_proof_body(
        &item,
        "    let zz_extra: bool = kani::any();\n    let _ = zz_extra;\n",
    )
    .map(|_| ())
    .unwrap_err();
    let CompositeShadowRefusal::LeafBinding {
        refusal: LeafBindingRefusal::CallOrder { recorded, drawn },
    } = refusal
    else {
        panic!("a call order refusal: {refusal:?}");
    };
    assert_eq!(drawn.len(), recorded.len() + 1);
    assert_eq!(drawn.last().map(String::as_str), Some("zz_extra:bool"));
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_an_identity_with_two_literal_operands_draws_nothing_and_matches_only_a_source_that_draws_nothing(
) {
    // Generation needs a parameter operand, so the literal/literal identity is the generated
    // literal/parameter one with its drawn leaves pinned instead.
    let corpus = corpus();
    let entry = entry(&corpus, DUO_EQUAL);
    let mut item = item_of(&entry, "subject");
    item.right = ShadowOperand::Literal { leaves: vec![1, 2] };
    let mut identity = generate_composite_shadow(&item).unwrap().identity;
    let composite = identity.composite.as_mut().unwrap();
    composite.operands[0] = OperandSource::Literal;
    for leaf in identity.leaves.drain(..) {
        composite
            .pinned
            .push(filament_contract_codegen::PinnedLeaf {
                name: leaf.name,
                operand: 0,
                path: leaf.path,
                kind: leaf.kind,
                value: 0,
            });
    }
    check_leaf_bindings(&identity).expect("two literal operands bind");
    let drawless = format!("fn {}() {{}}", identity.harness_symbol);
    check_leaf_call_order(&identity, &drawless).expect("a proof that draws nothing matches");
    let drawing = format!(
        "fn {}() {{ let zz_extra: bool = kani::any(); }}",
        identity.harness_symbol
    );
    assert!(matches!(
        check_leaf_call_order(&identity, &drawing),
        Err(LeafBindingRefusal::CallOrder { .. })
    ));
    assert_eq!(
        check_leaf_call_order(&identity, "not rust"),
        Err(LeafBindingRefusal::SourceUnreadable)
    );
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_two_parameter_names_for_one_node_refuse_generation_with_no_harness() {
    let corpus = corpus();
    let entry = entry(&corpus, FLAGS_EQUAL);
    let mut item = item_of(&entry, "subject");
    item.right = ShadowOperand::Parameter {
        node: entry.node_id.clone(),
        name: "other".to_owned(),
    };
    assert!(matches!(
        generate_composite_shadow(&item),
        Err(CompositeShadowRefusal::LeafBinding {
            refusal: LeafBindingRefusal::Duplicate { .. }
        })
    ));
}

#[trace("TC-048", "FR-025-AC-9")]
#[trace("TC-048", "FR-033-AC-5")]
#[test]
fn tc_048_literal_operands_keep_their_pinned_identity_and_are_never_drawn() {
    let corpus = corpus();
    let entry = entry(&corpus, DUO_EQUAL);
    let mut item = item_of(&entry, "subject");
    item.right = ShadowOperand::Literal { leaves: vec![1, 2] };
    let harness = generate_composite_shadow(&item).unwrap();
    let identity = &harness.identity;
    check_leaf_bindings(identity).expect("a literal operand binds");
    let composite = identity.composite.as_ref().unwrap();
    assert!(matches!(composite.operands[1], OperandSource::Literal));
    assert_eq!(
        composite
            .pinned
            .iter()
            .map(|pinned| (pinned.operand, pinned.path.as_str(), pinned.value))
            .collect::<Vec<_>>(),
        [(1, "0", 1), (1, "1", 2)]
    );
    // The literal side is rebuilt from the pinned values; the playback holds only the drawn side.
    let operands = reconstruct(identity, &[E::B(false), E::I(2)]).unwrap();
    let WitnessValue::Tuple { components, .. } = &operands[1] else {
        panic!("a tuple");
    };
    assert!(matches!(&components[0], WitnessValue::Boolean(true)));
    assert!(matches!(&components[1], WitnessValue::ExactInteger(value)
        if *value == filament_exact::Integer::from(2_i64)));
    let WitnessValue::Tuple { components, .. } = &operands[0] else {
        panic!("a tuple");
    };
    assert!(matches!(&components[0], WitnessValue::Boolean(false)));

    // A playback entry for the literal side is surplus: a literal is never drawn.
    let surplus = reconstruct(identity, &[E::B(false), E::I(2), E::B(true), E::I(2)]);
    assert_eq!(surplus.unwrap_err().code, "kani_witness_arity_mismatch");

    // Removing a pinned leaf, changing its value out of bound, or pinning a drawn operand refuses.
    let mut removed = identity.clone();
    removed.composite.as_mut().unwrap().pinned.remove(0);
    assert!(matches!(
        check_leaf_bindings(&removed),
        Err(LeafBindingRefusal::Missing { operand, .. }) if operand == "right"
    ));
    let mut out_of_bound = identity.clone();
    out_of_bound.composite.as_mut().unwrap().pinned[1].value = 3;
    assert!(matches!(
        check_leaf_bindings(&out_of_bound),
        Err(LeafBindingRefusal::Conflict {
            member: ConflictMember::Value,
            ..
        })
    ));
    // A complete literal operand plus one more pinned leaf on a parameter operand: only the
    // check that a parameter is never pinned can refuse it, and an operand index beyond the two
    // operands is refused the same way.
    for operand in [0, 7] {
        let mut on_a_parameter = identity.clone();
        let mut extra = on_a_parameter.composite.as_ref().unwrap().pinned[0].clone();
        extra.operand = operand;
        extra.name = "zz_pinned".to_owned();
        on_a_parameter
            .composite
            .as_mut()
            .unwrap()
            .pinned
            .push(extra);
        assert_eq!(
            check_leaf_bindings(&on_a_parameter),
            Err(LeafBindingRefusal::Unknown {
                name: "zz_pinned".to_owned()
            }),
            "operand {operand}"
        );
    }
    // The same pinned path recorded twice under another name for one literal operand.
    let mut duplicated = identity.clone();
    let mut again = duplicated.composite.as_ref().unwrap().pinned[0].clone();
    again.name = "zz_again".to_owned();
    duplicated.composite.as_mut().unwrap().pinned.push(again);
    assert_eq!(
        check_leaf_bindings(&duplicated),
        Err(LeafBindingRefusal::Duplicate {
            name: "zz_again".to_owned()
        })
    );
}

#[trace("TC-048", "FR-033-AC-5")]
#[trace("TC-048", "FR-033-AC-2")]
#[test]
fn tc_048_absent_null_and_present_slots_rebuild_as_distinct_values() {
    let harness = flags_identity();
    let identity = &harness.identity;
    let build = |state: i64| {
        let mut entries = flags(true, 1, state, 2);
        entries.extend(flags(true, 1, 0, 2));
        reconstruct(identity, &entries).unwrap()
    };
    let present = build(0);
    let absent = build(1);
    let null = build(2);
    assert!(matches!(slot(&present[0], "c"), WitnessSlot::Present(_)));
    assert!(is_present_int(slot(&present[0], "c"), 2));
    assert!(matches!(slot(&absent[0], "c"), WitnessSlot::Absent));
    assert!(matches!(slot(&null[0], "c"), WitnessSlot::Null));
    assert!(present[0] != absent[0] && absent[0] != null[0] && present[0] != null[0]);
    // The required fields keep their declared order and value.
    let names: Vec<&str> = fields(&present[0])
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "c"]);
    assert!(matches!(
        slot(&present[0], "a"),
        WitnessSlot::Present(WitnessValue::Boolean(true))
    ));
    assert!(is_present_int(slot(&present[0], "b"), 1));
    // The right operand is the second parameter's leaves.
    assert_eq!(present[1], build(0)[1]);
    assert!(is_present_int(slot(&present[1], "c"), 2));
}

#[trace("TC-048", "FR-033-AC-5")]
#[trace("TC-048", "FR-033-AC-2")]
#[test]
fn tc_048_nested_option_presence_rebuilds_each_level_and_skips_the_payload_of_an_absent_one() {
    let corpus = corpus();
    let nested = generate(&corpus, NESTED_OPTION_EQUAL).identity;
    // Option<Option<Boolean>>: outer presence, then (when present) inner presence, then (when
    // present) the Boolean.
    let none = [E::B(false)];
    let some_none = [E::B(true), E::B(false)];
    let some_some = [E::B(true), E::B(true), E::B(true)];
    let build = |left: &[E], right: &[E]| {
        let mut entries = left.to_vec();
        entries.extend_from_slice(right);
        reconstruct(&nested, &entries).unwrap()
    };
    let operands = build(&none, &some_none);
    assert!(matches!(&operands[0], WitnessValue::Option(None)));
    let WitnessValue::Option(Some(inner)) = &operands[1] else {
        panic!("outer present");
    };
    assert!(matches!(**inner, WitnessValue::Option(None)));
    let operands = build(&some_some, &none);
    let WitnessValue::Option(Some(inner)) = &operands[0] else {
        panic!("outer present");
    };
    let WitnessValue::Option(Some(flag)) = &**inner else {
        panic!("inner present");
    };
    assert!(matches!(**flag, WitnessValue::Boolean(true)));
    // None, Some(None) and Some(Some(_)) are three different values.
    assert!(build(&none, &none)[0] != build(&some_none, &none)[0]);
    assert!(build(&some_none, &none)[0] != build(&some_some, &none)[0]);
}

#[trace("TC-048", "FR-033-AC-2")]
#[test]
fn tc_048_rebuilt_operands_round_trip_through_the_canonical_value_text() {
    let harness = flags_identity();
    for state in [0, 1, 2] {
        // The widest values: the integer leaves are exact integers inside the composite.
        let mut entries = flags(false, 2, state, 0);
        entries.extend(flags(true, 0, 0, 2));
        let operands = reconstruct(&harness.identity, &entries).unwrap();
        for operand in &operands {
            let text = operand.to_value_text().expect("a canonical text");
            let back = WitnessValue::from_wire_bytes(text.as_bytes()).expect("the text reads back");
            assert!(&back == operand, "{text}");
        }
    }
    let texts: Vec<String> = [1, 2]
        .iter()
        .map(|state| {
            let mut entries = flags(true, 1, *state, 0);
            entries.extend(flags(true, 1, 0, 2));
            reconstruct(&harness.identity, &entries).unwrap()[0]
                .to_value_text()
                .unwrap()
        })
        .collect();
    assert_ne!(texts[0], texts[1], "absent and null are different texts");

    // A wide integer leaf is the whole i64 range, exact.
    let wide = super::composite_equality_closure::wide_corpus();
    let identity = generate(&wide, 0).identity;
    let operands = reconstruct(
        &identity,
        &[E::I(i64::MIN), E::I(i64::MAX), E::I(-1), E::I(0)],
    )
    .unwrap();
    for operand in &operands {
        let text = operand.to_value_text().unwrap();
        assert!(&WitnessValue::from_wire_bytes(text.as_bytes()).unwrap() == operand);
    }
    let WitnessValue::Record { fields, .. } = &operands[0] else {
        panic!("a record");
    };
    assert!(
        matches!(&fields[0].slot, WitnessSlot::Present(WitnessValue::ExactInteger(value))
        if *value == filament_exact::Integer::from(i64::MIN))
    );
}

#[trace("TC-048", "FR-033-AC-3")]
#[trace("TC-048", "FR-033-AC-5")]
#[test]
fn tc_048_a_playback_that_does_not_fit_the_leaf_schema_refuses_at_its_typed_boundary() {
    let identity = flags_identity().identity;
    let good = {
        let mut entries = flags(true, 1, 0, 2);
        entries.extend(flags(true, 1, 0, 2));
        entries
    };
    assert!(reconstruct(&identity, &good).is_ok());
    let code = |entries: &[E]| reconstruct(&identity, entries).unwrap_err().code;

    // Missing and surplus leaves.
    assert_eq!(code(&good[..good.len() - 1]), "kani_witness_arity_mismatch");
    let mut surplus = good.clone();
    surplus.push(E::I(0));
    assert_eq!(code(&surplus), "kani_witness_arity_mismatch");
    // A payload of an absent slot is not drawn, so an entry for it shifts every later leaf onto
    // another primitive.
    let mut undrawn = flags(true, 1, 1, 2);
    undrawn.push(E::I(2));
    undrawn.extend(flags(true, 1, 0, 2));
    assert_eq!(code(&undrawn), "kani_witness_width_mismatch");
    // Wrong primitive width in place of a Boolean, and in place of an integer.
    let mut wide_bool = good.clone();
    wide_bool[0] = E::I(1);
    assert_eq!(code(&wide_bool), "kani_witness_width_mismatch");
    let mut narrow_int = good.clone();
    narrow_int[1] = E::B(true);
    assert_eq!(code(&narrow_int), "kani_witness_width_mismatch");
    // Swapped leaf order: the Boolean and the first integer trade places.
    let mut swapped = good.clone();
    swapped.swap(0, 1);
    assert_eq!(code(&swapped), "kani_witness_width_mismatch");
    // Out-of-domain values: an integer above its bound, a slot state above 2, below the bound.
    let mut high = good.clone();
    high[1] = E::I(3);
    assert_eq!(code(&high), "cg_witness_out_of_domain");
    let mut low = good.clone();
    low[1] = E::I(-1);
    assert_eq!(code(&low), "cg_witness_out_of_domain");
    let mut state = good.clone();
    state[2] = E::I(3);
    assert_eq!(code(&state), "cg_witness_out_of_domain");
    // Swapped integers of one bound change the rebuilt operand rather than decode as another.
    let a = reconstruct(&identity, &good).unwrap();
    let mut changed = good.clone();
    changed[1] = E::I(2);
    assert!(reconstruct(&identity, &changed).unwrap()[0] != a[0]);

    // A transcript of another harness is refused, not decoded under this identity.
    let text = transcript_for("shadow_proof::other", &good);
    let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).unwrap();
    assert_eq!(
        reconstruct_composite_operands(&identity, &playback, limits())
            .unwrap_err()
            .code,
        "cg_witness_harness_identity_mismatch"
    );

    // A decoded value that disagrees with Kani's own rendering is refused.
    let text = transcript_for(&harness_path(&identity), &good).replacen("// 1\n", "// 2\n", 1);
    let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).unwrap();
    assert_eq!(
        reconstruct_composite_operands(&identity, &playback, limits())
            .unwrap_err()
            .code,
        "kani_witness_comment_mismatch"
    );

    // A schema that fails its binding check reads no playback at all.
    let mut broken = identity.clone();
    broken.leaves.remove(1);
    let text = transcript_for(&harness_path(&broken), &good);
    let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).unwrap();
    assert_eq!(
        decode_composite_falsification(&broken, &playback, limits())
            .unwrap_err()
            .code,
        "cg_composite_leaf_missing"
    );
    let decoded = {
        let text = transcript_for(&harness_path(&identity), &good);
        let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).unwrap();
        decode_composite_falsification(&identity, &playback, limits()).unwrap()
    };
    assert_eq!(
        rebuild_composite_operands(&broken, &decoded)
            .unwrap_err()
            .code,
        "cg_composite_leaf_missing"
    );
}

#[trace("TC-048", "FR-033-AC-4")]
#[test]
fn tc_048_the_encoded_byte_and_value_limits_admit_the_limit_and_refuse_one_over() {
    let identity = flags_identity().identity;
    let mut entries = flags(true, 1, 0, 2);
    entries.extend(flags(true, 1, 0, 2));
    let text = transcript_for(&harness_path(&identity), &entries);
    let try_with = |bytes: usize, values: usize| {
        let playback = parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).unwrap();
        decode_composite_falsification(&identity, &playback, WitnessDecodeLimits { bytes, values })
    };
    assert!(try_with(text.len(), entries.len()).is_ok());
    assert_eq!(
        try_with(text.len() - 1, entries.len()).unwrap_err().code,
        "cg_witness_decode_bytes_limit"
    );
    assert_eq!(
        try_with(text.len(), entries.len() - 1).unwrap_err().code,
        "cg_witness_decode_values_limit"
    );
    // The reader refuses a transcript one byte over its limit before parsing it.
    assert!(parse_assertion_playback(&text, "t", "ctx", text.len(), 1024).is_ok());
    let refusal = parse_assertion_playback(&text, "t", "ctx", text.len() - 1, 1024).unwrap_err();
    assert_eq!(refusal.code.as_str(), "cg_witness_decode_bytes_limit");
    let refusal =
        parse_assertion_playback(&text, "t", "ctx", text.len(), entries.len() - 1).unwrap_err();
    assert_eq!(refusal.code.as_str(), "cg_witness_decode_values_limit");
}

#[trace("TC-048", "FR-033-AC-4")]
#[test]
fn tc_048_a_100000_link_value_clones_compares_formats_and_drops_on_a_512_kib_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let mut value =
                WitnessValue::ExactInteger(filament_exact::Integer::from(7_654_321_i64));
            for _ in 0..100_000 {
                value = WitnessValue::Option(Some(Box::new(value)));
            }
            let copy = value.clone();
            assert!(copy == value);
            let rendered = format!("{value:?}");
            assert!(!rendered.is_empty());
            assert!(
                !rendered.contains("7654321"),
                "the Debug rendering is redacted and holds no leaf content"
            );
            drop(copy);
            drop(value);
        })
        .unwrap();
    handle
        .join()
        .expect("no stack overflow and no depth refusal");
}
