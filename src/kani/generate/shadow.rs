//! The bounded-shadow generator and its refinement obligation (FR-028-AC-13 to FR-028-AC-20,
//! ADR-003).
//!
//! A shadow harness proves a claim about a smaller program that shares no code with the
//! production subject. The refinement obligation generated beside it is the separate claim that
//! the production subject behaves as that program does. This module holds the machinery that is
//! the same for every family and one stand-in family (two bounded integers compared for
//! equality) that exercises it; the composite-equality family builds on the same items.

use serde::Serialize;

use crate::{
    core::artifact::Artifact,
    kani::{
        abi::{adapter_options, witness_options, KaniSolver},
        identity::{
            refinement::{DomainBound, ProofSubject},
            shadow::{
                AbstractionEvidence, AbstractionRecord, HelperMark, KaniShadowHarness, LeafKind,
                LeafRecord, ProductionRead, RefinementObligation, ShadowIdentity,
                SharedHelperClaim,
            },
            ProofCeilings,
        },
    },
};

use super::render;

pub mod composite;
pub mod source_check;
#[cfg(test)]
mod tests;

/// The family name a stand-in shadow records.
pub const TWO_LEAF_FAMILY: &str = "two_leaf_equality";

/// The shadow families' size budget: the pair nodes a closure may have.
///
/// Set by measurement (TC-025): a mixed closure of 128 pair nodes verifies in under a minute and
/// stays well inside the 16 GiB memory ceiling the default identity records, while 192 nodes
/// needs most of it, so the next size is not offered.
pub const SHADOW_PAIR_NODE_BUDGET: u32 = 128;

/// The module the shadow, the expectation and their shared helpers live in.
pub const SHADOW_MODULE: &str = "shadow_two_leaf";

/// The module of the abstraction function. Neither the shadow, the expectation nor the Kani
/// harness may name it (FR-028-AC-14).
pub const ABSTRACTION_MODULE: &str = "abstraction";

const PROOF_MODULE: &str = "shadow_proof";
const PROOF_FUNCTION: &str = "prove_shadow";

/// What the shadow covers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShadowShape {
    /// Two bounded integers compared for equality.
    TwoLeafEquality,
    /// A shape no shadow covers.
    NoShadow {
        /// The family the item belongs to.
        family: String,
        /// The capability the shadow lacks.
        capability: String,
    },
}

/// One drawn argument of a shadow item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowLeaf {
    /// The node the leaf belongs to, named by a refusal.
    pub node: String,
    /// The leaf's Rust identifier.
    pub name: String,
    /// The declared inclusive bound, or none for an unbounded integer.
    pub declared: Option<(i64, i64)>,
    /// A narrowed inclusive bound inside the declared one (FR-028-AC-8).
    pub narrowed: Option<(i64, i64)>,
    /// The refinement domain of this argument; the drawn bound when absent.
    pub refinement: Option<(i64, i64)>,
}

/// One abstraction the shadow makes, with the evidence the item supplies for it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AbstractionClaim {
    /// The abstraction's name.
    pub name: String,
    /// What discharges it; none leaves it undischarged and the item is refused.
    pub evidence: Option<AbstractionEvidence>,
}

/// One item for the shadow generator.
#[derive(Clone, Debug)]
pub struct ShadowItem {
    /// The node the item claims.
    pub node: String,
    /// The shape of the item.
    pub shape: ShadowShape,
    /// The drawn arguments, in order.
    pub leaves: Vec<ShadowLeaf>,
    /// The item's pair-node count, compared with [`SHADOW_PAIR_NODE_BUDGET`].
    pub size: u32,
    /// The production subject's symbol; the obligation names it.
    pub subject: String,
    /// The abstractions the shadow makes, each with its evidence.
    pub abstractions: Vec<AbstractionClaim>,
    /// Behaviours of the production subject the proof does not exercise.
    pub unexercised: Vec<String>,
    /// The helpers the shadow shares with its expectation.
    pub helpers: Vec<SharedHelperClaim>,
    /// The request ceilings.
    pub ceilings: ProofCeilings,
    /// The loop unwind bound.
    pub unwind: u32,
}

/// Why an item settles without a harness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ShadowRefusal {
    /// A leaf has no declared bound.
    RequiresBound {
        /// The unbounded node.
        node: String,
    },
    /// No shadow covers the shape.
    Unsupported {
        /// The family.
        family: String,
        /// The missing capability.
        capability: String,
    },
    /// The shape is over the family's size budget.
    ShapeOverBudget {
        /// The item's size.
        size: u32,
        /// The budget.
        budget: u32,
    },
    /// An abstraction has no evidence that discharges it.
    UndischargedAbstraction {
        /// The abstraction.
        name: String,
    },
    /// A helper claim is missing, repeated, unknown or incomplete.
    HelperClaim {
        /// The helper.
        name: String,
    },
    /// The obligation would name no production subject.
    NoProductionSubject,
    /// The refinement domain does not contain every case the shadow harness admits.
    RefinementDomainNarrower {
        /// The argument.
        argument: String,
    },
    /// A narrowed bound is not inside the declared one.
    NarrowedOutsideDeclared {
        /// The argument.
        argument: String,
    },
    /// The rendered source fails the parsed-source check of FR-028-AC-13 or FR-028-AC-14, or
    /// draws from a domain other than the identity's.
    SourceCheck {
        /// The violation.
        violation: String,
    },
    /// A leaf name is not a plain Rust identifier.
    LeafName {
        /// The name.
        name: String,
    },
}

/// How a refusal is reported (FR-028-AC-20).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowDisposition {
    /// The item needs a bound it does not have.
    RequiresBound,
    /// The family does not support the item.
    Unsupported,
    /// The item's own claims are refused.
    Refused,
}

impl ShadowRefusal {
    /// The disposition of the refusal.
    #[must_use]
    pub fn disposition(&self) -> ShadowDisposition {
        match self {
            Self::RequiresBound { .. } => ShadowDisposition::RequiresBound,
            Self::Unsupported { .. } | Self::ShapeOverBudget { .. } => {
                ShadowDisposition::Unsupported
            }
            _ => ShadowDisposition::Refused,
        }
    }
}

/// Text substituted into a rendering by the tests that mutate the generator.
#[derive(Clone, Debug, Default)]
pub(crate) struct Hooks {
    /// Extra lines inside the shadow module.
    pub(crate) shadow_extra: String,
    /// Extra lines at the top of the proof module.
    pub(crate) proof_extra: String,
    /// Extra lines in the proof body, before the assertions.
    pub(crate) body_extra: String,
    /// Replaces the shadow function's body.
    pub(crate) shadow_body: Option<String>,
    /// Replaces the shared `pair_count` helper's body.
    pub(crate) pair_count_body: Option<String>,
    /// Replaces the shared `leaf_order` helper's body.
    pub(crate) leaf_order_body: Option<String>,
}

/// Generates the shadow harness and its refinement obligation for `item`, or says why not.
///
/// Each item settles alone: nothing here reads another item.
pub fn generate_shadow(item: &ShadowItem) -> Result<KaniShadowHarness, ShadowRefusal> {
    generate_with(item, &Hooks::default())
}

/// Generates every item of a request; each result is what the item yields alone.
#[must_use]
pub fn generate_shadows(items: &[ShadowItem]) -> Vec<Result<KaniShadowHarness, ShadowRefusal>> {
    items.iter().map(generate_shadow).collect()
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !matches!(
            name,
            "as" | "fn" | "let" | "mod" | "use" | "self" | "super" | "type" | "match"
        )
}

fn is_symbol(name: &str) -> bool {
    !name.is_empty()
        && name
            .split("::")
            .all(|part| is_identifier(part) || part == "crate")
}

pub(crate) fn generate_with(
    item: &ShadowItem,
    hooks: &Hooks,
) -> Result<KaniShadowHarness, ShadowRefusal> {
    match &item.shape {
        ShadowShape::TwoLeafEquality => {}
        ShadowShape::NoShadow { family, capability } => {
            return Err(ShadowRefusal::Unsupported {
                family: family.clone(),
                capability: capability.clone(),
            })
        }
    }
    if item.leaves.len() != 2 {
        return Err(ShadowRefusal::Unsupported {
            family: TWO_LEAF_FAMILY.to_owned(),
            capability: format!("{} drawn arguments", item.leaves.len()),
        });
    }
    if let Some(leaf) = item.leaves.iter().find(|leaf| leaf.declared.is_none()) {
        return Err(ShadowRefusal::RequiresBound {
            node: leaf.node.clone(),
        });
    }
    if item.size > SHADOW_PAIR_NODE_BUDGET {
        return Err(ShadowRefusal::ShapeOverBudget {
            size: item.size,
            budget: SHADOW_PAIR_NODE_BUDGET,
        });
    }
    if let Some(leaf) = item.leaves.iter().find(|leaf| !is_identifier(&leaf.name)) {
        return Err(ShadowRefusal::LeafName {
            name: leaf.name.clone(),
        });
    }
    if !is_symbol(&item.subject) {
        return Err(ShadowRefusal::NoProductionSubject);
    }
    let abstractions = check_abstractions(&item.abstractions)?;
    check_helpers(&item.helpers)?;
    let leaves = leaf_records(&item.leaves)?;
    let domain = refinement_domain(&item.leaves, &leaves)?;

    let shadow_module = render_shadow_module(&item.leaves, hooks);
    let draws: Vec<render::shadow::Draw<'_>> = leaves
        .iter()
        .map(|leaf| render::shadow::Draw {
            name: leaf.name.as_str(),
            domain: render::shadow::DrawDomain::Int(leaf.drawn.0, leaf.drawn.1),
            when: None,
        })
        .collect();
    let arguments: Vec<&str> = leaves.iter().map(|leaf| leaf.name.as_str()).collect();
    let proof_module = render::shadow::proof_module(&render::shadow::ProofModule {
        module: PROOF_MODULE,
        function: PROOF_FUNCTION,
        shadow_module: SHADOW_MODULE,
        draws: &draws,
        pinned: &[],
        arguments: &arguments,
        cover: "expected_verdict",
        before_proof: &hooks.proof_extra,
        before_assertions: &hooks.body_extra,
    });
    let rust = Artifact::new(
        "shadow_harness.rs",
        format!("{shadow_module}\n{proof_module}"),
    );
    let drawn = source_check::check_shadow_source(&rust.contents, Some(ABSTRACTION_MODULE))
        .map_err(|violation| ShadowRefusal::SourceCheck {
            violation: format!("{violation:?}"),
        })?;
    if source_check::contains_loop(&rust.contents) {
        return Err(ShadowRefusal::SourceCheck {
            violation: "the shadow holds a loop construct".to_owned(),
        });
    }
    let declared_draws: Vec<(String, i128, i128)> = leaves
        .iter()
        .map(|leaf| {
            (
                leaf.name.clone(),
                i128::from(leaf.drawn.0),
                i128::from(leaf.drawn.1),
            )
        })
        .collect();
    let source_draws: Vec<(String, i128, i128)> = drawn
        .into_iter()
        .map(|bound| (bound.argument, bound.minimum, bound.maximum))
        .collect();
    if declared_draws != source_draws {
        return Err(ShadowRefusal::SourceCheck {
            violation: "the harness draws from a domain other than the identity's".to_owned(),
        });
    }
    let abstraction_source = render_abstraction_module();
    let source = Artifact::new(
        "refinement.rs",
        render_driver(item, &shadow_module, &abstraction_source),
    );
    let harness = format!("{PROOF_MODULE}::{PROOF_FUNCTION}");
    let solver = KaniSolver::Cadical;
    let identity = ShadowIdentity {
        family: TWO_LEAF_FAMILY.to_owned(),
        proof_subject: ProofSubject::BoundedShadow,
        node: item.node.clone(),
        ceilings: item.ceilings,
        module_symbol: PROOF_MODULE.to_owned(),
        harness_symbol: PROOF_FUNCTION.to_owned(),
        leaves,
        size_budget: SHADOW_PAIR_NODE_BUDGET,
        size: item.size,
        abstractions,
        unexercised: item.unexercised.clone(),
        shared_helpers: item.helpers.clone(),
        solver,
        unwind: item.unwind,
        options: adapter_options(&harness, item.unwind, solver, false),
        witness_options: witness_options(&harness, item.unwind, solver, false),
        composite: None,
    };
    let obligation = RefinementObligation {
        subject: item.subject.clone(),
        shadow: format!("{SHADOW_MODULE}::shadow"),
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

fn check_abstractions(
    claims: &[AbstractionClaim],
) -> Result<Vec<AbstractionRecord>, ShadowRefusal> {
    let mut records = Vec::new();
    for name in ["representation", "integer_widening"] {
        let evidence = claims
            .iter()
            .find(|claim| claim.name == name)
            .and_then(|claim| claim.evidence.clone())
            .ok_or_else(|| ShadowRefusal::UndischargedAbstraction {
                name: name.to_owned(),
            })?;
        records.push(AbstractionRecord {
            name: name.to_owned(),
            evidence,
        });
    }
    if let Some(extra) = claims
        .iter()
        .find(|claim| !["representation", "integer_widening"].contains(&claim.name.as_str()))
        .filter(|claim| claim.evidence.is_none())
    {
        return Err(ShadowRefusal::UndischargedAbstraction {
            name: extra.name.clone(),
        });
    }
    Ok(records)
}

fn check_helpers(claims: &[SharedHelperClaim]) -> Result<(), ShadowRefusal> {
    let refuse = |name: &str| ShadowRefusal::HelperClaim {
        name: name.to_owned(),
    };
    for name in ["pair_count", "leaf_order"] {
        let mut named = claims.iter().filter(|claim| claim.name == name);
        let claim = named.next().ok_or_else(|| refuse(name))?;
        if named.next().is_some() {
            return Err(refuse(name));
        }
        let reason_ok = match claim.mark {
            HelperMark::OrderOnly => claim
                .order_only_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty()),
            HelperMark::ValueBearing => claim.order_only_reason.is_none(),
        };
        let checked = claim.mark == HelperMark::OrderOnly
            || claim.production_read == ProductionRead::Independent
            || claim.independent_check.is_some();
        if !reason_ok || !checked {
            return Err(refuse(name));
        }
    }
    if let Some(unknown) = claims
        .iter()
        .find(|claim| !["pair_count", "leaf_order"].contains(&claim.name.as_str()))
    {
        return Err(refuse(&unknown.name));
    }
    Ok(())
}

fn leaf_records(leaves: &[ShadowLeaf]) -> Result<Vec<LeafRecord>, ShadowRefusal> {
    leaves
        .iter()
        .map(|leaf| {
            let declared = leaf.declared.unwrap_or((i64::MIN, i64::MAX));
            let drawn = leaf.narrowed.unwrap_or(declared);
            if drawn.0 > drawn.1 || drawn.0 < declared.0 || drawn.1 > declared.1 {
                return Err(ShadowRefusal::NarrowedOutsideDeclared {
                    argument: leaf.name.clone(),
                });
            }
            Ok(LeafRecord {
                name: leaf.name.clone(),
                node: leaf.node.clone(),
                path: String::new(),
                kind: LeafKind::Integer,
                declared,
                drawn,
            })
        })
        .collect()
}

fn refinement_domain(
    leaves: &[ShadowLeaf],
    records: &[LeafRecord],
) -> Result<Vec<DomainBound>, ShadowRefusal> {
    leaves
        .iter()
        .zip(records)
        .map(|(leaf, record)| {
            let bound = leaf.refinement.unwrap_or(record.drawn);
            if bound.0 > record.drawn.0 || bound.1 < record.drawn.1 {
                return Err(ShadowRefusal::RefinementDomainNarrower {
                    argument: leaf.name.clone(),
                });
            }
            Ok(DomainBound {
                argument: leaf.name.clone(),
                minimum: bound.0,
                maximum: bound.1,
            })
        })
        .collect()
}

fn render_shadow_module(leaves: &[ShadowLeaf], hooks: &Hooks) -> String {
    let (left, right) = (&leaves[0].name, &leaves[1].name);
    let pair_count = hooks.pair_count_body.as_deref().unwrap_or("1");
    let leaf_order = hooks
        .leaf_order_body
        .clone()
        .unwrap_or_else(|| format!("[\"{left}\", \"{right}\"]"));
    let shadow_body = hooks.shadow_body.clone().unwrap_or_else(|| {
        format!(
            "let difference = i128::from({left}) - i128::from({right});\n        (difference == 0, pair_count())"
        )
    });
    format!(
        "pub mod {SHADOW_MODULE} {{
    /// The pairs one comparison of the two leaves charges.
    pub fn pair_count() -> u64 {{
        {pair_count}
    }}

    /// The order the leaves are drawn and reported in.
    pub fn leaf_order() -> [&'static str; 2] {{
        {leaf_order}
    }}

    /// The shadow: equality through a widened difference, with its pair count.
    pub fn shadow({left}: i64, {right}: i64) -> (bool, u64) {{
        {shadow_body}
    }}

    /// The expectation: equality computed from the drawn values directly.
    pub fn expectation({left}: i64, {right}: i64) -> (bool, u64) {{
        ({left} == {right}, pair_count())
    }}
{extra}}}
",
        extra = hooks.shadow_extra
    )
}

fn render_abstraction_module() -> String {
    format!(
        "mod {ABSTRACTION_MODULE} {{
    use super::subject;

    /// Builds the production value of one leaf through the production oracle's own environment
    /// and its declaration-checked constructor; a refusal is a construction refusal.
    pub fn lift(
        environment: &subject::Environment,
        leaf: &str,
        value: i64,
    ) -> Result<subject::Value, String> {{
        subject::build(environment, leaf, value)
    }}
}}
"
    )
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
        let values: Vec<i64> = line
            .split(' ')
            .map(|value| value.parse().expect(\"case value\"))
            .collect();
        let (@LEFT@, @RIGHT@) = (values[0], values[1]);
        let (verdict, pairs) = @SHADOW_MODULE@::shadow(@LEFT@, @RIGHT@);
        let shadow_result = format!(\"Completed(verdict={verdict},pairs={pairs})\");
        let lifted = (
            abstraction::lift(&environment, \"@LEFT@\", @LEFT@),
            abstraction::lift(&environment, \"@RIGHT@\", @RIGHT@),
        );
        let disagreement = match lifted {
            (Ok(left), Ok(right)) => match subject::compare(&environment, &left, &right) {
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
            (Err(message), _) | (_, Err(message)) => {
                Some(format!(\"ConstructionRefused({message})\"))
            }
        };
        run += 1;
        if let Some(production) = disagreement {
            disagreements += 1;
            if disagreements <= 16 {
                println!(\"DISAGREE\\t{} {}\\t{production}\\t{shadow_result}\", @LEFT@, @RIGHT@);
            }
        }
    }
    println!(\"DONE\\t{run}\\t{disagreements}\");
}
";

fn render_driver(item: &ShadowItem, shadow_module: &str, abstraction_module: &str) -> String {
    DRIVER
        .replace("@SUBJECT@", &item.subject)
        .replace("@SHADOW@", shadow_module)
        .replace("@ABSTRACTION@", abstraction_module)
        .replace("@SHADOW_MODULE@", SHADOW_MODULE)
        .replace("@LEFT@", &item.leaves[0].name)
        .replace("@RIGHT@", &item.leaves[1].name)
}

/// The family's independent check of its value-bearing helpers (FR-028-AC-18): each helper's
/// returned constant in `source` against the value the production declaration states, read
/// independently of the generator. `declared` pairs a helper name with that value.
///
/// An `order_only` helper is not checked. The composite closure's own check is FR-018-AC-23.
///
/// # Errors
///
/// The name of the first value-bearing helper whose rendered value differs from, or is missing
/// against, its declared value.
pub fn independent_helper_check(
    harness: &KaniShadowHarness,
    declared: &[(&str, u64)],
) -> Result<(), String> {
    for helper in &harness.identity.shared_helpers {
        if helper.mark != HelperMark::ValueBearing {
            continue;
        }
        let expected = declared
            .iter()
            .find(|(name, _)| *name == helper.name)
            .map(|(_, value)| *value)
            .ok_or_else(|| helper.name.clone())?;
        let rendered = helper_constant(&harness.rust.contents, &helper.name);
        if rendered != Some(expected) {
            return Err(helper.name.clone());
        }
    }
    Ok(())
}

fn helper_constant(source: &str, name: &str) -> Option<u64> {
    let file = syn::parse_file(source).ok()?;
    for item in file.items {
        if let syn::Item::Mod(module) = item {
            for inner in module.content?.1 {
                if let syn::Item::Fn(function) = inner {
                    if function.sig.ident == name {
                        let statement = function.block.stmts.first()?;
                        if let syn::Stmt::Expr(syn::Expr::Lit(literal), None) = statement {
                            if let syn::Lit::Int(value) = &literal.lit {
                                return value.base10_parse().ok();
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
