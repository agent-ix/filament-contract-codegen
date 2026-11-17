//! The shadow machinery of FR-028-AC-13 to FR-028-AC-24 over a stand-in family: two bounded
//! integers compared for equality, with a stand-in production subject the tests mutate. The
//! composite family's own run is TC-025 steps 29 to 36.

use ix_trace_rs::trace;

use super::*;

fn ceilings(wall_seconds: u64) -> ProofCeilings {
    ProofCeilings {
        memory_bytes: std::num::NonZeroU64::new(16 << 30).unwrap(),
        wall_clock: std::time::Duration::from_secs(wall_seconds),
    }
}

fn claims() -> Vec<AbstractionClaim> {
    vec![
        AbstractionClaim {
            name: "representation".to_owned(),
            evidence: Some(AbstractionEvidence::RefinementComparison(
                "verdict".to_owned(),
            )),
        },
        AbstractionClaim {
            name: "integer_widening".to_owned(),
            evidence: Some(AbstractionEvidence::KaniCheck(
                "shadow_proof::prove_shadow arithmetic overflow check".to_owned(),
            )),
        },
    ]
}

fn helpers(read: ProductionRead) -> Vec<SharedHelperClaim> {
    vec![
        SharedHelperClaim {
            name: "pair_count".to_owned(),
            mark: HelperMark::ValueBearing,
            order_only_reason: None,
            production_read: read,
            independent_check: (read == ProductionRead::SameHelper)
                .then(|| "independent_helper_check against the declaration".to_owned()),
        },
        SharedHelperClaim {
            name: "leaf_order".to_owned(),
            mark: HelperMark::OrderOnly,
            order_only_reason: Some(
                "the verdict and the pair count do not read leaf order".to_owned(),
            ),
            production_read: ProductionRead::Independent,
            independent_check: None,
        },
    ]
}

fn leaf(name: &str, bound: Option<(i64, i64)>) -> ShadowLeaf {
    ShadowLeaf {
        node: format!("node-{name}"),
        name: name.to_owned(),
        declared: bound,
        narrowed: None,
        refinement: None,
    }
}

fn item() -> ShadowItem {
    item_over((-3, 3))
}

fn item_over(bound: (i64, i64)) -> ShadowItem {
    ShadowItem {
        node: "equality-node".to_owned(),
        shape: ShadowShape::TwoLeafEquality,
        leaves: vec![leaf("left", Some(bound)), leaf("right", Some(bound))],
        size: 1,
        subject: "stand_in".to_owned(),
        abstractions: claims(),
        unexercised: vec!["meter_limits".to_owned()],
        helpers: helpers(ProductionRead::Independent),
        ceilings: ceilings(600),
        unwind: 4,
    }
}

fn violation(hooks: &Hooks) -> String {
    match generate_with(&item(), hooks) {
        Err(ShadowRefusal::SourceCheck { violation }) => violation,
        other => panic!("expected a source-check refusal, got {other:?}"),
    }
}

/// FR-028-AC-13: the unmutated shadow harness names no crate but `core`, `alloc`, `std` and
/// `kani`, holds no stub and assumes only its drawn domain; each mutated generator fails the
/// check it breaks.
#[trace("TC-039", "FR-028-AC-13")]
#[test]
fn tc_039_the_shadow_source_has_no_runtime_path_no_stub_and_no_assumption_beyond_its_domain() {
    let harness = generate_shadow(&item()).unwrap();
    let drawn = source_check::check_shadow_source(&harness.rust.contents, None).unwrap();
    assert_eq!(
        drawn,
        vec![
            source_check::DrawnBound {
                argument: "left".to_owned(),
                minimum: -3,
                maximum: 3
            },
            source_check::DrawnBound {
                argument: "right".to_owned(),
                minimum: -3,
                maximum: 3
            },
        ]
    );
    assert!(!source_check::contains_loop(&harness.rust.contents));
    assert!(!harness.rust.contents.contains("kani::stub"));

    let runtime = violation(&Hooks {
        proof_extra: "    use filament_contract_runtime::Outcome;\n".to_owned(),
        ..Hooks::default()
    });
    assert!(runtime.contains("ForeignCrate") && runtime.contains("filament_contract_runtime"));
    let exact = violation(&Hooks {
        proof_extra: "    use filament_exact::Value;\n".to_owned(),
        ..Hooks::default()
    });
    assert!(exact.contains("filament_exact"));
    let inline = violation(&Hooks {
        body_extra: "        let _ = filament_exact::Value::default();\n".to_owned(),
        ..Hooks::default()
    });
    assert!(inline.contains("ForeignCrate"));
    let stub = violation(&Hooks {
        proof_extra: "    #[kani::stub(shadow, shadow)]\n".to_owned(),
        ..Hooks::default()
    });
    assert!(stub.contains("Stub"));
    let verified_stub = violation(&Hooks {
        proof_extra: "    #[kani::stub_verified(shadow)]\n".to_owned(),
        ..Hooks::default()
    });
    assert!(verified_stub.contains("Stub"));
    let relation = violation(&Hooks {
        body_extra: "        kani::assume(left == right);\n".to_owned(),
        ..Hooks::default()
    });
    assert!(relation.contains("Assumption"));
    let looping = violation(&Hooks {
        shadow_body: Some(
            "let mut n = 0; loop { n += 1; if n > 2 { break; } }\n        (true, pair_count())"
                .to_owned(),
        ),
        ..Hooks::default()
    });
    assert!(looping.contains("loop"));
    let one_sided = violation(&Hooks {
        body_extra: "        kani::assume(left >= 0_i64);\n".to_owned(),
        ..Hooks::default()
    });
    assert!(one_sided.contains("Assumption"));

    // Bypasses a path-only check misses. Each is spliced into the real rendering, so the source
    // differs from the unmutated one and the unmutated one passes.
    let healthy = generate_shadow(&item()).unwrap().rust.contents;
    assert!(source_check::check_shadow_source(&healthy, None).is_ok());
    let uses = "    use super::shadow_two_leaf::{expectation, shadow};\n";
    let body = "        let (verdict, pairs) = shadow(left, right);\n";
    let bypasses: [(&str, &str, &str, &str); 7] = [
        (
            "a bare assume imported by use",
            "    use kani::assume;\n",
            "        assume(left == right);\n",
            "Assumption",
        ),
        (
            "an assume through a renamed kani",
            "    use kani as k;\n",
            "        k::assume(left == right);\n",
            "Assumption",
        ),
        (
            "any_where",
            "",
            "        let _z: i64 = kani::any_where(|z: &i64| *z == left);\n",
            "ForeignCrate",
        ),
        (
            "a stub inside cfg_attr",
            "    #[cfg_attr(kani, kani::stub(shadow, shadow))]\n",
            "",
            "Stub",
        ),
        (
            "a local module named for a runtime crate",
            "    mod filament_exact {}\n",
            "        let _ = ::filament_exact::Value::default();\n",
            "ForeignCrate",
        ),
        (
            "an unresolved capitalised path",
            "",
            "        let _ = Value::default();\n",
            "ForeignCrate",
        ),
        (
            "a glob import of kani",
            "    use kani::*;\n",
            "",
            "ForeignCrate",
        ),
    ];
    for (name, before_proof, in_body, kind) in bypasses {
        let mutated = healthy
            .replace(uses, &format!("{uses}{before_proof}"))
            .replace(body, &format!("{in_body}{body}"));
        assert_ne!(mutated, healthy, "{name} changes the source");
        let refused = source_check::check_shadow_source(&mutated, None).expect_err(name);
        assert!(format!("{refused:?}").contains(kind), "{name}: {refused:?}");
    }
}

/// FR-028-AC-14: the result carries a refinement obligation naming subject, shadow, domain,
/// abstraction function and observables, with the abstraction in a module the shadow, the
/// expectation and the harness do not import.
#[trace("TC-039", "FR-028-AC-14")]
#[test]
fn tc_039_the_obligation_names_its_parts_and_the_abstraction_module_is_imported_by_none() {
    let harness = generate_shadow(&item()).unwrap();
    let obligation = &harness.obligation;
    assert_eq!(obligation.subject, "stand_in");
    assert_eq!(obligation.shadow, "shadow_two_leaf::shadow");
    assert_eq!(obligation.abstraction, "abstraction::lift");
    assert_eq!(obligation.observables, ["verdict", "pair_count"]);
    assert_eq!(
        obligation.domain,
        vec![
            DomainBound {
                argument: "left".to_owned(),
                minimum: -3,
                maximum: 3
            },
            DomainBound {
                argument: "right".to_owned(),
                minimum: -3,
                maximum: 3
            },
        ]
    );
    // The harness source, which holds the shadow, the expectation and the proof, does not name
    // the abstraction module; the obligation source, which holds it, does.
    assert!(source_check::check_shadow_source(&harness.rust.contents, Some("abstraction")).is_ok());
    assert!(!harness.rust.contents.contains("abstraction"));
    assert!(obligation.source.contents.contains("mod abstraction"));
    assert!(obligation.abstraction_source.contains("subject::build"));
    // A harness that imports the abstraction module fails the check.
    let imported = generate_with(
        &item(),
        &Hooks {
            proof_extra: "    use super::abstraction::lift;\n".to_owned(),
            ..Hooks::default()
        },
    );
    assert!(
        matches!(imported, Err(ShadowRefusal::SourceCheck { violation }) if violation.contains("ImportsAbstraction"))
    );
    // An obligation naming no production subject is not returned.
    let mut nameless = item();
    nameless.subject = String::new();
    assert_eq!(
        generate_shadow(&nameless).unwrap_err(),
        ShadowRefusal::NoProductionSubject
    );
}

/// FR-028-AC-18: the identity lists abstractions with evidence, unexercised behaviours and shared
/// helpers with their marks; an abstraction with no evidence is refused naming it.
#[trace("TC-039", "FR-028-AC-18")]
#[test]
fn tc_039_the_identity_lists_what_the_shadow_abstracts_and_an_undischarged_one_is_refused() {
    let harness = generate_shadow(&item()).unwrap();
    let identity = &harness.identity;
    assert_eq!(identity.family, TWO_LEAF_FAMILY);
    assert_eq!(identity.proof_subject, ProofSubject::BoundedShadow);
    assert_eq!(
        identity
            .abstractions
            .iter()
            .map(|record| record.name.as_str())
            .collect::<Vec<_>>(),
        ["representation", "integer_widening"]
    );
    assert_eq!(identity.unexercised, ["meter_limits"]);
    let marks: Vec<_> = identity
        .shared_helpers
        .iter()
        .map(|helper| (helper.name.as_str(), helper.mark))
        .collect();
    assert_eq!(
        marks,
        [
            ("pair_count", HelperMark::ValueBearing),
            ("leaf_order", HelperMark::OrderOnly)
        ]
    );
    assert!(identity.shared_helpers[1].order_only_reason.is_some());
    let json = serde_json::to_value(identity).unwrap();
    assert_eq!(json["proofSubject"], "bounded_shadow");

    let mut undischarged = item();
    undischarged.abstractions[1].evidence = None;
    assert_eq!(
        generate_shadow(&undischarged).unwrap_err(),
        ShadowRefusal::UndischargedAbstraction {
            name: "integer_widening".to_owned()
        }
    );
    let mut missing = item();
    missing.abstractions.remove(0);
    assert_eq!(
        generate_shadow(&missing).unwrap_err(),
        ShadowRefusal::UndischargedAbstraction {
            name: "representation".to_owned()
        }
    );
    // An order-only helper without a reason, and a same-helper value-bearing helper with no
    // independent check, are refused.
    let mut reasonless = item();
    reasonless.helpers[1].order_only_reason = None;
    assert_eq!(
        generate_shadow(&reasonless).unwrap_err(),
        ShadowRefusal::HelperClaim {
            name: "leaf_order".to_owned()
        }
    );
    let mut unchecked = item();
    unchecked.helpers[0].production_read = ProductionRead::SameHelper;
    assert_eq!(
        generate_shadow(&unchecked).unwrap_err(),
        ShadowRefusal::HelperClaim {
            name: "pair_count".to_owned()
        }
    );
    // Changing a bound changes the identity; regeneration is byte-identical.
    assert_ne!(
        generate_shadow(&item_over((-3, 4))).unwrap().identity,
        harness.identity
    );
    assert_eq!(generate_shadow(&item()).unwrap().rust, harness.rust);
}

/// FR-028-AC-19: a refinement domain narrower than the drawn one is refused naming the argument;
/// the drawn domain equals the declared one or the narrowed bound.
#[trace("TC-039", "FR-028-AC-19")]
#[test]
fn tc_039_a_refinement_domain_narrower_than_the_drawn_domain_is_refused() {
    let mut narrower = item();
    narrower.leaves[1].refinement = Some((-3, 2));
    assert_eq!(
        generate_shadow(&narrower).unwrap_err(),
        ShadowRefusal::RefinementDomainNarrower {
            argument: "right".to_owned()
        }
    );
    let mut wider = item();
    wider.leaves[0].refinement = Some((-4, 4));
    let harness = generate_shadow(&wider).unwrap();
    assert_eq!(
        (
            harness.obligation.domain[0].minimum,
            harness.obligation.domain[0].maximum
        ),
        (-4, 4)
    );
    assert_eq!(harness.identity.leaves[0].drawn, (-3, 3));
    // A narrowed bound inside the declared one is what the harness draws from and what the
    // refinement covers; a narrowed bound outside it is refused.
    let mut narrowed = item();
    narrowed.leaves[0].narrowed = Some((-1, 2));
    let harness = generate_shadow(&narrowed).unwrap();
    let drawn = source_check::check_shadow_source(&harness.rust.contents, None).unwrap();
    assert_eq!((drawn[0].minimum, drawn[0].maximum), (-1, 2));
    assert_eq!(harness.identity.leaves[0].declared, (-3, 3));
    assert_eq!(
        (
            harness.obligation.domain[0].minimum,
            harness.obligation.domain[0].maximum
        ),
        (-1, 2)
    );
    let mut outside = item();
    outside.leaves[0].narrowed = Some((-4, 2));
    assert_eq!(
        generate_shadow(&outside).unwrap_err(),
        ShadowRefusal::NarrowedOutsideDeclared {
            argument: "left".to_owned()
        }
    );
}

/// FR-028-AC-20: an unbounded node, a shape no shadow covers and a shape over the size budget are
/// refused with their disposition and no harness; a supported sibling settles as it does alone.
#[trace("TC-039", "FR-028-AC-20")]
#[test]
fn tc_039_unsupported_shapes_are_refused_without_a_harness_or_a_production_fallback() {
    let mut unbounded = item();
    unbounded.leaves[1] = leaf("right", None);
    let mut no_shadow = item();
    no_shadow.shape = ShadowShape::NoShadow {
        family: "composite_ordering".to_owned(),
        capability: "ordered comparison".to_owned(),
    };
    let mut oversize = item();
    oversize.size = SHADOW_PAIR_NODE_BUDGET + 1;
    let mut at_budget = item();
    at_budget.size = SHADOW_PAIR_NODE_BUDGET;
    let results = generate_shadows(&[unbounded, no_shadow, oversize, item(), at_budget]);
    assert_eq!(
        results[0].as_ref().unwrap_err(),
        &ShadowRefusal::RequiresBound {
            node: "node-right".to_owned()
        }
    );
    assert_eq!(
        results[0].as_ref().unwrap_err().disposition(),
        ShadowDisposition::RequiresBound
    );
    assert_eq!(
        results[1].as_ref().unwrap_err(),
        &ShadowRefusal::Unsupported {
            family: "composite_ordering".to_owned(),
            capability: "ordered comparison".to_owned()
        }
    );
    assert_eq!(
        results[1].as_ref().unwrap_err().disposition(),
        ShadowDisposition::Unsupported
    );
    assert_eq!(
        results[2].as_ref().unwrap_err(),
        &ShadowRefusal::ShapeOverBudget {
            size: SHADOW_PAIR_NODE_BUDGET + 1,
            budget: SHADOW_PAIR_NODE_BUDGET
        }
    );
    assert_eq!(
        results[2].as_ref().unwrap_err().disposition(),
        ShadowDisposition::Unsupported
    );
    let alone = generate_shadow(&item()).unwrap();
    assert_eq!(results[3].as_ref().unwrap().rust, alone.rust);
    assert_eq!(results[3].as_ref().unwrap().identity, alone.identity);
    assert!(results[4].is_ok());
}

fn boolean_operand_identity() -> ShadowIdentity {
    use crate::{
        kani::identity::shadow::{CompositeIdentity, LeafKind, LeafRecord, OperandSource},
        oracle::equality::closure::{RecordedOperandTypes, RecordedType},
    };
    let leaf = |name: &str, node: &str| LeafRecord {
        name: name.to_owned(),
        node: node.to_owned(),
        path: String::new(),
        kind: LeafKind::Boolean,
        declared: (0, 1),
        drawn: (0, 1),
    };
    ShadowIdentity {
        family: composite::COMPOSITE_FAMILY.to_owned(),
        proof_subject: ProofSubject::BoundedShadow,
        node: "n".to_owned(),
        ceilings: ceilings(60),
        module_symbol: PROOF_MODULE.to_owned(),
        harness_symbol: PROOF_FUNCTION.to_owned(),
        leaves: vec![leaf("a", "n"), leaf("b", "m")],
        size_budget: 1,
        size: 1,
        abstractions: Vec::new(),
        unexercised: Vec::new(),
        shared_helpers: Vec::new(),
        solver: KaniSolver::Cadical,
        unwind: 1,
        options: Vec::new(),
        witness_options: Vec::new(),
        composite: Some(CompositeIdentity {
            operator: "equality.equal".to_owned(),
            declarations: Vec::new(),
            operand_types: RecordedOperandTypes {
                left: RecordedType::Boolean,
                right: RecordedType::Boolean,
            },
            pair_nodes: 1,
            operands: [
                OperandSource::Parameter {
                    node: "n".to_owned(),
                },
                OperandSource::Parameter {
                    node: "m".to_owned(),
                },
            ],
            pinned: Vec::new(),
            coverage: Vec::new(),
            closes_family_row: false,
        }),
    }
}

#[trace("TC-048", "FR-025-AC-9")]
#[test]
fn tc_048_generation_refuses_a_source_whose_draws_disagree_with_the_identity_in_call_order() {
    use crate::kani::identity::leaf_binding::LeafBindingRefusal;
    use composite::{check_generated_leaves, CompositeShadowRefusal};
    let identity = boolean_operand_identity();
    let proof = |body: &str| format!("pub fn {PROOF_FUNCTION}() {{\n{body}}}\n");
    let agreeing = proof("    let a: bool = kani::any();\n    let b: bool = kani::any();\n");
    assert_eq!(check_generated_leaves(&identity, &agreeing), Ok(()));
    let reordered = proof("    let b: bool = kani::any();\n    let a: bool = kani::any();\n");
    let extra = proof(
        "    let a: bool = kani::any();\n    let b: bool = kani::any();\n    let c: bool = kani::any();\n",
    );
    for source in [reordered, extra] {
        assert!(
            matches!(
                check_generated_leaves(&identity, &source),
                Err(CompositeShadowRefusal::LeafBinding {
                    refusal: LeafBindingRefusal::CallOrder { .. }
                })
            ),
            "{source}"
        );
    }
    // The leaf bindings are checked first: a binding fault is reported before the source is read.
    let mut broken = identity;
    broken.leaves.pop();
    assert!(matches!(
        check_generated_leaves(&broken, &agreeing),
        Err(CompositeShadowRefusal::LeafBinding {
            refusal: LeafBindingRefusal::Missing { .. }
        })
    ));
}
