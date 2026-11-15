//! The proof module of a bounded-shadow harness (FR-028-AC-13, FR-015-AC-7).
//!
//! Every proof the generator emits is spelled in the shared renderer so that the cover-last
//! inspection drives it: the shadow's proof draws each leaf inside its drawn bound, asserts the
//! shadow against its expectation on the verdict and the pair count, and ends with its one
//! `kani::cover!`.

use crate::kani::abi::i64_literal;

/// What the proof module is made of.
pub(in crate::kani::generate) struct ProofModule<'a> {
    /// The proof module's name.
    pub(in crate::kani::generate) module: &'a str,
    /// The proof function's name.
    pub(in crate::kani::generate) function: &'a str,
    /// The module holding `shadow` and `expectation`.
    pub(in crate::kani::generate) shadow_module: &'a str,
    /// Each drawn leaf with its inclusive bound, in draw order; two leaves.
    pub(in crate::kani::generate) draws: &'a [(&'a str, i64, i64)],
    /// Text placed before the proof attribute.
    pub(in crate::kani::generate) before_proof: &'a str,
    /// Text placed before the assertions.
    pub(in crate::kani::generate) before_assertions: &'a str,
}

/// The `#[cfg(kani)]` module with the one proof of the shadow.
pub(in crate::kani::generate) fn proof_module(spec: &ProofModule<'_>) -> String {
    let mut draws = String::new();
    for (name, minimum, maximum) in spec.draws {
        draws.push_str(&format!(
            "        let {name}: i64 = kani::any();\n        kani::assume({name} >= {} && {name} <= {});\n",
            i64_literal(*minimum),
            i64_literal(*maximum)
        ));
    }
    let (left, right) = (spec.draws[0].0, spec.draws[1].0);
    format!(
        "#[cfg(kani)]
pub mod {module} {{
    use super::{shadow_module}::{{expectation, shadow}};
{before_proof}
    #[kani::proof]
    pub fn {function}() {{
{draws}{before_assertions}        let (verdict, pairs) = shadow({left}, {right});
        let (expected_verdict, expected_pairs) = expectation({left}, {right});
        assert!(verdict == expected_verdict);
        assert!(pairs == expected_pairs);
        kani::cover!(expected_verdict);
    }}
}}
",
        module = spec.module,
        shadow_module = spec.shadow_module,
        function = spec.function,
        before_proof = spec.before_proof,
        before_assertions = spec.before_assertions,
    )
}
