//! FR-015-AC-73 (TC-025 step 33): the unsupported-shape rows that need a package of their own,
//! and a request that mixes refused and supported items. The families are read from genuine
//! checked packages through FR-018's closure reader, not fed to the generator as type names.

use filament_contract_codegen::{
    generate_composite_shadow, ClaimDisposition, CompositeEqualityClaim, CompositeEqualityItem,
    CompositeEqualityRefusal, CompositeShadowRefusal,
};
use filament_contract_model::ContractFeatureRecordV1;
use ix_trace_rs::trace;

use super::composite_shadow::item_of;

const FUNCTIONS: [&str; 6] = [
    "f_seq",
    "f_set",
    "f_bag",
    "f_ordered",
    "f_quantity",
    "f_flags",
];

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ndimension Length;\nunit m : Length = rational(1, 1);\nrecord WithSeq { s: Sequence<Int[0, 2]>[0, 2]; }\nrecord WithSet { s: Set<Int[0, 2]>[0, 3]; }\nrecord WithBag { s: Bag<Int[0, 2]>[0, 4]; }\nrecord WithOrdered { s: OrderedSet<Int[0, 2]>[0, 5]; }\nrecord WithQuantity { q: Quantity<m>; }\nrecord Flags { a: Boolean; b: Int[0, 2]; }\nfunction f_seq using v(a: WithSeq, b: WithSeq): Boolean pure { a = b }\nfunction f_set using v(a: WithSet, b: WithSet): Boolean pure { a = b }\nfunction f_bag using v(a: WithBag, b: WithBag): Boolean pure { a = b }\nfunction f_ordered using v(a: WithOrdered, b: WithOrdered): Boolean pure { a = b }\nfunction f_quantity using v(a: WithQuantity, b: WithQuantity): Boolean pure { a = b }\nfunction f_flags using v(a: Flags, b: Flags): Boolean pure { a = b }\n";

fn entries() -> Vec<CompositeEqualityClaim> {
    let compiled = super::composite_equality_agreement::fixture::compile_source(SOURCE);
    let package = super::admitted_enum_equality::checked(&compiled, &[]);
    let items: Vec<CompositeEqualityItem> = FUNCTIONS
        .iter()
        .map(|function| {
            super::composite_equality_agreement::original_item_named(&package, function, false)
        })
        .collect();
    let oracles = super::admitted_enum_equality::generate_items(
        &package,
        Some(compiled.context()),
        &items,
        &[],
    )
    .expect("the family package generates");
    items
        .iter()
        .map(|item| {
            oracles
                .claim_map
                .items
                .iter()
                .find(|entry| entry.node_id == item.node_id)
                .expect("a claim-map entry per requested item")
                .clone()
        })
        .collect()
}

fn refusal(entry: &CompositeEqualityClaim) -> CompositeShadowRefusal {
    generate_composite_shadow(&item_of(entry, "subject")).expect_err("the item is refused")
}

#[trace("TC-025", "FR-015-AC-73")]
#[trace("TC-039", "FR-028-AC-20")]
#[test]
fn tc_025_a_sequence_a_set_a_bag_an_ordered_set_and_a_quantity_settle_each_with_its_reason_from_a_checked_package(
) {
    let entries = entries();
    // A sequence is read from the package under its own name.
    match refusal(&entries[0]) {
        CompositeShadowRefusal::ShadowFamilyNotBuilt { family, .. } => {
            assert_eq!(family, "sequence")
        }
        other => panic!("{other:?}"),
    }
    // FR-018 does not yet read the bound of a set, a bag or an ordered set, and its refusal is
    // what settles those items.
    for (position, entry) in entries.iter().enumerate().take(4).skip(1) {
        let ClaimDisposition::Refused { refusal: oracle } = &entry.result else {
            panic!("FR-018 refuses item {position}: {:?}", entry.result);
        };
        assert!(
            matches!(
                oracle,
                CompositeEqualityRefusal::Unsupported {
                    node_tag: "bounded_domain",
                    ..
                }
            ),
            "{oracle:?}"
        );
        assert_eq!(
            refusal(entry),
            CompositeShadowRefusal::Oracle {
                refusal: oracle.clone()
            }
        );
    }
    // FR-018 refuses a quantity before the shadow sees it, and its refusal is what settles.
    let ClaimDisposition::Refused { refusal: oracle } = &entries[4].result else {
        panic!("FR-018 refuses the quantity item: {:?}", entries[4].result);
    };
    assert_eq!(
        refusal(&entries[4]),
        CompositeShadowRefusal::Oracle {
            refusal: oracle.clone()
        }
    );
    assert!(
        matches!(
            oracle,
            CompositeEqualityRefusal::Unsupported {
                node_tag: "quantity",
                ..
            }
        ),
        "{oracle:?}"
    );
}

/// FR-015-AC-73 `reference` row: a record that carries a `Reference` field settles with FR-018's
/// typed refusal, the FR-330 `Unsupported` accounting record of the reference's `model` target
/// (FR-018-AC-7), carried unchanged, and the shadow generator builds no harness for it.
#[trace("TC-025", "FR-015-AC-73")]
#[test]
fn tc_025_a_reference_composite_settles_with_fr_018s_refusal_and_no_harness() {
    let entry = super::composite_equality_generation::reference_record_entry();
    let ClaimDisposition::Refused { refusal: oracle } = &entry.result else {
        panic!(
            "FR-018 refuses the reference-bearing record: {:?}",
            entry.result
        );
    };
    let CompositeEqualityRefusal::PackageAccounting { record, .. } = oracle else {
        panic!("expected FR-330 accounting, got {oracle:?}");
    };
    assert!(
        matches!(
            record.as_ref(),
            ContractFeatureRecordV1::Unsupported { node_tag, .. } if node_tag.as_ref() == "model"
        ),
        "{record:?}"
    );
    let settled = generate_composite_shadow(&item_of(&entry, "subject"));
    assert_eq!(
        settled.as_ref().err(),
        Some(&CompositeShadowRefusal::Oracle {
            refusal: oracle.clone()
        }),
        "the shadow generator carries the refusal unchanged and returns no harness"
    );
}
