//! Derives the dependency sources the generated manifests name from this crate's own
//! `Cargo.toml` (QSL-2939), so no revision is spelled outside Cargo. Writes
//! `$OUT_DIR/dependency_sources.rs`, included by `src/core/profile.rs`.

use std::{env, fs, path::PathBuf};

/// Each emitted constant and the `[dependencies]` entry whose source it spells.
const SOURCES: [(&str, &str); 8] = [
    ("RUNTIME_DEPENDENCY_SOURCE", "filament-contract-runtime"),
    ("EXACT_DEPENDENCY_SOURCE", "filament-exact"),
    (
        "SEMANTIC_VALUE_DEPENDENCY_SOURCE",
        "filament-semantic-value",
    ),
    ("MODEL_DEPENDENCY_SOURCE", "filament-contract-model"),
    ("EVAL_DEPENDENCY_SOURCE", "filament-eval"),
    ("TRACE_DEPENDENCY_SOURCE", "ix-trace-rs"),
    ("FSL_REPLAY_DEPENDENCY_SOURCE", "fsl-replay"),
    ("FSL_PACKAGE_DEPENDENCY_SOURCE", "fsl-package"),
];

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let manifest_path = root.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    let text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", manifest_path.display()));
    let manifest: toml::Table = text.parse().expect("Cargo.toml is valid TOML");
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("Cargo.toml has [dependencies]");
    let mut generated = String::new();
    for (constant, name) in SOURCES {
        let source = source_of(dependencies, name);
        generated.push_str(&format!(
            "/// The `{name}` source of this crate's `Cargo.toml`, read by `build.rs`.\npub const {constant}: &str = {source:?};\n"
        ));
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("dependency_sources.rs");
    fs::write(&out, generated)
        .unwrap_or_else(|error| panic!("could not write {}: {error}", out.display()));
}

/// The `git = "...", rev = "..."` (or `tag`) spelling of one git dependency.
fn source_of(dependencies: &toml::Table, name: &str) -> String {
    let table = dependencies
        .get(name)
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("Cargo.toml [dependencies] has no table for {name}"));
    let field = |key: &str| table.get(key).and_then(toml::Value::as_str);
    let git = field("git").unwrap_or_else(|| panic!("{name} is not a git dependency"));
    match (field("rev"), field("tag")) {
        (Some(rev), None) => format!("git = \"{git}\", rev = \"{rev}\""),
        (None, Some(tag)) => format!("git = \"{git}\", tag = \"{tag}\""),
        _ => panic!("{name} must name exactly one of rev or tag"),
    }
}
