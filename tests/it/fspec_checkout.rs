//! The checked-package fixtures are read from the `FSPEC_DIR` checkout, so a checkout at another
//! revision than the `filament-specification-qualification` dev-dependency pins gives misleading
//! failures. The pinned revision is read from Cargo metadata; nothing here repeats it.

use std::path::PathBuf;
use std::process::Command;

fn pinned_revision() -> String {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata JSON");
    let source = metadata["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == "filament-specification-qualification")
        .and_then(|package| package["source"].as_str())
        .expect("the filament-specification-qualification dev-dependency is locked");
    source
        .rsplit_once('#')
        .map(|(_, revision)| revision.to_owned())
        .expect("a locked git source names its commit")
}

#[test]
fn the_fspec_dir_checkout_is_at_the_pinned_revision() {
    let directory = PathBuf::from(
        std::env::var_os("FSPEC_DIR").expect("FSPEC_DIR names a filament-specification checkout"),
    );
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&directory)
        .output()
        .expect("run git in FSPEC_DIR");
    assert!(
        output.status.success(),
        "FSPEC_DIR {} is not a git checkout",
        directory.display()
    );
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let pinned = pinned_revision();
    assert_eq!(
        actual, pinned,
        "FSPEC_DIR is at {actual}; check out the revision Cargo.lock pins ({pinned})"
    );
}
