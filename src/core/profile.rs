//! The version profile: spellings this build emits (AD-004 `core/profile`, L-11).
//!
//! The emitted oracle crate's `Cargo.toml` is written here once and called by the three oracle
//! emitters. Every first-party dependency it names uses the source this crate's own `Cargo.toml`
//! uses: a Git URL with a release tag or commit revision. The FSL crates use a pinned main commit.
//! A scratch crate seeded with this crate's `Cargo.lock` resolves each first-party crate once and
//! at the locked commit, with no branch resolved at generation or test time.
//! `manifest_sources_match_this_crates_manifest` keeps the two spellings equal.

/// The runtime source every emitted manifest depends on, spelled as in this crate's `Cargo.toml`:
/// the `git` and `rev` keys of the dependency table. Public so a scratch crate that compiles
/// generated code names the runtime the same way and cannot drift from the emitter.
pub const RUNTIME_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/filament-contract-runtime\", rev = \"69522c23fb81375d9f88b07d833338394891bd18\"";

/// The authoritative exact-kernel source used by generated scalar, equality and function
/// oracles.
pub const EXACT_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/filament-exact\", rev = \"b26a8e4c592503fccdc4a0e3d2cce23dd8487616\"";

/// Source of declaration admission and structural equality in generated oracles.
pub const SEMANTIC_VALUE_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/filament-semantic-value\", rev = \"1df71c27e6db56c8118cf017689aa7fab6156cde\"";

/// Source of the checked-package model the function and equality oracles read.
pub const MODEL_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/filament-contract-ir\", rev = \"c9e52417889db09dcb1abe1015320448139269fb\"";

/// Source of the checked-closure evaluator the function oracle names.
pub const EVAL_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/filament-eval\", rev = \"6689fa06123ed46793734b6b5059c8cdef85027d\"";

/// Source of the criterion-binding attribute macro scratch crates use.
pub const TRACE_DEPENDENCY_SOURCE: &str =
    "git = \"https://github.com/agent-ix/ix-trace-rs\", tag = \"v0.1.2\"";

/// Source of FSL's admitted-context facade, pinned to current FSL main.
pub const FSL_REPLAY_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\", rev = \"603c3a2bd3d6413c0f3ba87d319bef341584e46b\"";
/// Source of FSL's structural obligation identity types.
pub const FSL_PACKAGE_DEPENDENCY_SOURCE: &str = "git = \"https://github.com/agent-ix/filament-spec-language\", rev = \"603c3a2bd3d6413c0f3ba87d319bef341584e46b\"";

/// The `[package.metadata.kani]` table every generated oracle crate's manifest carries. CBMC
/// tracks heap objects field by field only up to 64 bytes by default; RT's `Value` and `ValueType`
/// are larger, and a non-field-sensitive read of them cannot be constant-folded, so the crate raises
/// the limit the way RT's own `Cargo.toml` does.
const ORACLE_KANI_METADATA: &str = "[package.metadata.kani]\nunstable = { unstable-options = true }\nflags = { cbmc-args = [\"--max-field-sensitivity-array-size\", \"1024\"] }\n\n";

/// The `Cargo.toml` of a generated oracle crate named `crate_name`: a library over the runtime's
/// `exact` feature, exact kernel and declaration-checking leaf; forbids unsafe code and carries Kani metadata.
pub(crate) fn oracle_crate_manifest(crate_name: &str) -> String {
    format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nfilament-contract-runtime = {{ {RUNTIME_DEPENDENCY_SOURCE}, features = [\"exact\"] }}\nfilament-exact = {{ {EXACT_DEPENDENCY_SOURCE} }}\nfilament-semantic-value = {{ {SEMANTIC_VALUE_DEPENDENCY_SOURCE} }}\n\n[lints.rust]\nunsafe_code = \"forbid\"\n\n{ORACLE_KANI_METADATA}[workspace]\n"
    )
}

/// The concrete function oracle's checked-closure facade and direct leaf dependencies. The
/// `filament-contract-codegen` line stays `branch = "main"`: this crate has no usable release tag
/// (its only tag, v0.1.0, predates the pins above) and every test replaces the line with a `path`
/// dependency on the checkout under test, so no emitted crate resolves CG main at test time.
pub(crate) fn function_oracle_crate_manifest(crate_name: &str) -> String {
    format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nfilament-contract-codegen = {{ git = \"https://github.com/agent-ix/filament-contract-codegen\", branch = \"main\" }}\nfilament-contract-model = {{ {MODEL_DEPENDENCY_SOURCE} }}\nfsl-replay = {{ {FSL_REPLAY_DEPENDENCY_SOURCE} }}\nfsl-package = {{ {FSL_PACKAGE_DEPENDENCY_SOURCE} }}\nfilament-eval = {{ {EVAL_DEPENDENCY_SOURCE} }}\nfilament-exact = {{ {EXACT_DEPENDENCY_SOURCE} }}\nfilament-semantic-value = {{ {SEMANTIC_VALUE_DEPENDENCY_SOURCE} }}\n\n[lints.rust]\nunsafe_code = \"forbid\"\n\n{ORACLE_KANI_METADATA}[workspace]\n"
    )
}

/// The equality ABI needs the public admitted-context facade and FSL identity types.
pub(crate) fn equality_oracle_crate_manifest(crate_name: &str) -> String {
    let manifest = oracle_crate_manifest(crate_name);
    manifest.replacen(
        "[dependencies]\n",
        &format!("[dependencies]\nfilament-contract-model = {{ {MODEL_DEPENDENCY_SOURCE} }}\nfsl-replay = {{ {FSL_REPLAY_DEPENDENCY_SOURCE} }}\nfsl-package = {{ {FSL_PACKAGE_DEPENDENCY_SOURCE} }}\n"),
        1,
    )
}

/// The isolated observer workspace; only its dev dependencies enable charge logging.
pub(crate) fn scalar_observer_manifest() -> String {
    format!(
        "[package]\nname = \"filament-scalar-native-observer\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\nautobins = false\nautoexamples = false\n\n[[example]]\nname = \"ir791_native\"\npath = \"../src/bin/ir791_native.rs\"\n\n[dev-dependencies]\nfilament-contract-runtime = {{ {RUNTIME_DEPENDENCY_SOURCE}, features = [\"exact\"] }}\nfilament-exact = {{ {EXACT_DEPENDENCY_SOURCE}, features = [\"test-support\"] }}\n\n[workspace]\nresolver = \"2\"\n"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        equality_oracle_crate_manifest, function_oracle_crate_manifest, oracle_crate_manifest,
        scalar_observer_manifest, EVAL_DEPENDENCY_SOURCE, EXACT_DEPENDENCY_SOURCE,
        FSL_PACKAGE_DEPENDENCY_SOURCE, FSL_REPLAY_DEPENDENCY_SOURCE, MODEL_DEPENDENCY_SOURCE,
        RUNTIME_DEPENDENCY_SOURCE, SEMANTIC_VALUE_DEPENDENCY_SOURCE, TRACE_DEPENDENCY_SOURCE,
    };
    use ix_trace_rs::trace;

    #[trace("FR-014-AC-8", "TC-024")]
    #[test]
    fn runtime_boolean_outcome_has_the_kernel_type() {
        use filament_exact::{Meter, Outcome, ScalarLimits};

        fn accepts_kernel_outcome(_: Outcome<bool>) {}

        let mut meter = Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
        let produced = filament_contract_runtime::scalar::evaluate_boolean_short_circuit(
            filament_contract_runtime::scalar::ShortCircuitConnective::And,
            true,
            || Outcome::Completed(true),
            &mut meter,
        );
        accepts_kernel_outcome(produced);
    }

    #[trace("TC-024", "TC-029", "TC-031")]
    #[test]
    fn tc_024_oracle_manifest_names_the_runtime_and_kernel_by_rev_never_by_branch() {
        let manifest = oracle_crate_manifest("probe-crate");
        assert!(manifest.contains("name = \"probe-crate\""));
        assert!(manifest.contains(&format!(
            "filament-contract-runtime = {{ {RUNTIME_DEPENDENCY_SOURCE}, features = [\"exact\"] }}"
        )));
        assert!(manifest.contains(&format!("filament-exact = {{ {EXACT_DEPENDENCY_SOURCE} }}")));
        assert!(!manifest.contains("branch ="));
        assert!(!manifest.contains("test-support"));
        assert!(!manifest.contains("[[example]]"));
        assert!(manifest.contains("publish = false"));
        assert!(manifest.contains("unsafe_code = \"forbid\""));
        assert!(manifest.contains("--max-field-sensitivity-array-size"));
    }
    #[trace("FR-018-AC-37", "FR-014-AC-8", "TC-029")]
    #[test]
    fn only_equality_manifests_acquire_the_admitted_context_facade() {
        let equality = equality_oracle_crate_manifest("equality-probe");
        assert!(equality.contains(&format!(
            "fsl-replay = {{ {FSL_REPLAY_DEPENDENCY_SOURCE} }}"
        )));
        assert!(equality.contains(&format!(
            "fsl-package = {{ {FSL_PACKAGE_DEPENDENCY_SOURCE} }}"
        )));
        assert!(!equality.contains("branch ="));
        assert!(!equality.contains("test-support"));
        assert!(!oracle_crate_manifest("scalar-probe").contains("fsl-replay"));
        assert!(!scalar_observer_manifest().contains("fsl-replay"));
    }

    #[trace("FR-021-AC-32", "TC-031")]
    #[test]
    fn function_manifest_uses_the_checked_closure_facade_and_direct_leaves() {
        let function = function_oracle_crate_manifest("function-probe");
        for dependency in [
            "filament-contract-codegen",
            "filament-contract-model",
            "fsl-replay",
            "fsl-package",
            "filament-eval",
            "filament-exact",
            "filament-semantic-value",
        ] {
            assert!(function.contains(&format!("{dependency} = {{ git = ")));
        }
        assert!(!function.contains("filament-contract-runtime"));
        // Only this crate itself is named by branch: it has no release tag to name.
        for line in function.lines().filter(|line| line.contains("branch =")) {
            assert!(line.starts_with("filament-contract-codegen = "), "{line}");
        }
    }

    /// Every first-party source an emitted manifest names is the source this crate's own
    /// `Cargo.toml` names, so the scratch graph resolves each crate once and from the locked
    /// source and no second source of a crate is resolved at test time.
    #[trace("FR-014-AC-8", "TC-024")]
    #[test]
    fn manifest_sources_match_this_crates_manifest() {
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        )
        .unwrap();
        let lines = |name: &str| {
            let found = manifest
                .lines()
                .filter(|line| line.starts_with(&format!("{name} = ")))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(!found.is_empty(), "{name} in Cargo.toml");
            found
        };
        for (name, source) in [
            ("filament-contract-runtime", RUNTIME_DEPENDENCY_SOURCE),
            ("filament-exact", EXACT_DEPENDENCY_SOURCE),
            ("filament-semantic-value", SEMANTIC_VALUE_DEPENDENCY_SOURCE),
            ("filament-contract-model", MODEL_DEPENDENCY_SOURCE),
            ("filament-eval", EVAL_DEPENDENCY_SOURCE),
            ("ix-trace-rs", TRACE_DEPENDENCY_SOURCE),
            ("fsl-replay", FSL_REPLAY_DEPENDENCY_SOURCE),
            ("fsl-package", FSL_PACKAGE_DEPENDENCY_SOURCE),
        ] {
            for line in lines(name) {
                assert!(
                    line.contains(source),
                    "{name}: Cargo.toml `{line}` does not spell `{source}`"
                );
            }
        }
    }

    #[trace("FR-017-AC-52", "TC-027")]
    #[test]
    fn observer_is_an_isolated_dev_only_example_over_the_canonical_adapter() {
        let manifest = scalar_observer_manifest();
        assert!(manifest.contains("autobins = false\nautoexamples = false"));
        assert_eq!(manifest.matches("[[example]]").count(), 1);
        assert!(manifest.contains("path = \"../src/bin/ir791_native.rs\""));
        assert!(manifest.contains("[workspace]\nresolver = \"2\""));
        assert!(!manifest.contains("[dependencies]"));
        assert!(!manifest.contains("[build-dependencies]"));
        assert!(!manifest.contains("[lib]"));
        let (_, dev) = manifest.split_once("[dev-dependencies]").unwrap();
        assert!(dev.contains(&format!(
            "filament-contract-runtime = {{ {RUNTIME_DEPENDENCY_SOURCE}, features = [\"exact\"] }}"
        )));
        assert!(dev.contains(&format!(
            "filament-exact = {{ {EXACT_DEPENDENCY_SOURCE}, features = [\"test-support\"] }}"
        )));
        let production = oracle_crate_manifest("probe-crate");
        assert!(!production.contains("test-support"));
        assert!(!production.contains("[dev-dependencies]"));
        assert!(!production.contains("[[example]]"));
    }
}
