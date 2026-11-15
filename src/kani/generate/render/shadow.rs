//! The proof module of a bounded-shadow harness (FR-028-AC-13, FR-015-AC-7, FR-015-AC-70,
//! FR-015-AC-71).
//!
//! Every proof the generator emits is spelled in the shared renderer so that the cover-last
//! inspection drives it: the shadow's proof draws each leaf inside its drawn bound, asserts the
//! shadow against its expectation on the verdict and the pair count, and ends with its one
//! `kani::cover!`.

use crate::kani::abi::i64_literal;

/// What one drawn leaf is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::kani::generate) enum DrawDomain {
    /// A Boolean: a `bool` with no assumption.
    Bool,
    /// An integer inside an inclusive `i64` bound.
    Int(i64, i64),
}

/// One drawn leaf.
#[derive(Clone, Copy, Debug)]
pub(in crate::kani::generate) struct Draw<'a> {
    /// The leaf's Rust name.
    pub(in crate::kani::generate) name: &'a str,
    /// Its domain.
    pub(in crate::kani::generate) domain: DrawDomain,
    /// A condition over earlier leaves: the leaf is drawn only when it holds, and is the
    /// canonical zero otherwise (a payload of an absent option or a non-present slot).
    pub(in crate::kani::generate) when: Option<&'a str>,
}

/// What the proof module is made of.
pub(in crate::kani::generate) struct ProofModule<'a> {
    /// The proof module's name.
    pub(in crate::kani::generate) module: &'a str,
    /// The proof function's name.
    pub(in crate::kani::generate) function: &'a str,
    /// The module holding `shadow` and `expectation`.
    pub(in crate::kani::generate) shadow_module: &'a str,
    /// Each drawn leaf, in draw order.
    pub(in crate::kani::generate) draws: &'a [Draw<'a>],
    /// Lines that bind a leaf to exactly its literal value, placed after the draws.
    pub(in crate::kani::generate) pinned: &'a [(&'a str, String)],
    /// The arguments `shadow` and `expectation` take, in order.
    pub(in crate::kani::generate) arguments: &'a [&'a str],
    /// The expression the cover states: that the drawn operands compare equal.
    pub(in crate::kani::generate) cover: &'a str,
    /// Text placed before the proof attribute.
    pub(in crate::kani::generate) before_proof: &'a str,
    /// Text placed before the assertions.
    pub(in crate::kani::generate) before_assertions: &'a str,
}

fn draw_lines(draw: &Draw<'_>) -> String {
    let name = draw.name;
    let (ty, zero) = match draw.domain {
        DrawDomain::Bool => ("bool", "false"),
        DrawDomain::Int(..) => ("i64", "0"),
    };
    let assumption = match draw.domain {
        DrawDomain::Bool => String::new(),
        DrawDomain::Int(minimum, maximum) => format!(
            "kani::assume({name} >= {} && {name} <= {});",
            i64_literal(minimum),
            i64_literal(maximum)
        ),
    };
    match draw.when {
        None => format!(
            "        let {name}: {ty} = kani::any();\n{}",
            if assumption.is_empty() {
                String::new()
            } else {
                format!("        {assumption}\n")
            }
        ),
        Some(condition) => format!(
            "        let {name}: {ty} = if {condition} {{ let {name}: {ty} = kani::any(); {assumption} {name} }} else {{ {zero} }};\n"
        ),
    }
}

/// The `#[cfg(kani)]` module with the one proof of the shadow.
pub(in crate::kani::generate) fn proof_module(spec: &ProofModule<'_>) -> String {
    let mut draws = String::new();
    for draw in spec.draws {
        draws.push_str(&draw_lines(draw));
    }
    for (name, literal) in spec.pinned {
        draws.push_str(&format!("        let {name} = {literal};\n"));
    }
    let arguments = spec.arguments.join(", ");
    format!(
        "#[cfg(kani)]
pub mod {module} {{
    use super::{shadow_module}::{{expectation, shadow}};
{before_proof}
    #[kani::proof]
    pub fn {function}() {{
{draws}{before_assertions}        let (verdict, pairs) = shadow({arguments});
        let (expected_verdict, expected_pairs) = expectation({arguments});
        assert!(verdict == expected_verdict);
        assert!(pairs == expected_pairs);
        kani::cover!({cover});
    }}
}}
",
        module = spec.module,
        shadow_module = spec.shadow_module,
        function = spec.function,
        before_proof = spec.before_proof,
        before_assertions = spec.before_assertions,
        cover = spec.cover,
    )
}
