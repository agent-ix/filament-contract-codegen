//! Shared native-driver support: resolve externs from the running test's exact Cargo cohort,
//! and give each driver call its own staging directory.
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// A staging directory owned by exactly one driver call, removed on drop.
pub(super) struct Scratch(pub(super) PathBuf);

impl Scratch {
    /// Creates a new directory under `target`. Tests run on parallel threads and can read the
    /// same clock value, so a process-wide sequence number makes the name unique; the clock only
    /// keeps it apart from directories a killed earlier run left behind.
    pub(super) fn create(target: &Path, prefix: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = target.join(format!(
            "{prefix}-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(target).unwrap();
        fs::create_dir(&directory).expect("a staging directory no other driver call holds");
        Self(directory)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn dependency(name: &str) -> PathBuf {
    let executable = std::env::current_exe().expect("running integration executable");
    let deps = executable.parent().expect("Cargo deps directory");
    let profile = deps.parent().expect("Cargo profile directory");
    let executable_name = executable.file_stem().unwrap().to_str().unwrap();
    let hash = executable_name
        .strip_prefix("it-")
        .expect("integration test artifact hash");
    let fingerprints = profile.join(".fingerprint");
    let owner: serde_json::Value = serde_json::from_slice(
        &fs::read(fingerprints.join(format!(
            "filament-contract-codegen-{hash}/test-integration-test-it.json"
        )))
        .expect("running test Cargo fingerprint"),
    )
    .unwrap();
    let matches = owner["deps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|dep| dep[1].as_str() == Some(name))
        .collect::<Vec<_>>();
    let [dependency] = matches.as_slice() else {
        panic!("one direct dependency {name} in running test fingerprint");
    };
    let expected = dependency[3]
        .as_u64()
        .expect("Cargo dependency fingerprint");
    let mut artifacts = Vec::new();
    for directory in fs::read_dir(&fingerprints).unwrap() {
        let directory = directory.unwrap().path();
        let marker = directory.join(format!("lib-{name}"));
        let Ok(contents) = fs::read_to_string(&marker) else {
            continue;
        };
        let Ok(bytes) = u64::from_str_radix(contents.trim(), 16) else {
            continue;
        };
        if bytes.swap_bytes() != expected {
            continue;
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(marker.with_extension("json")).unwrap()).unwrap();
        if metadata["rustc"] != owner["rustc"] {
            continue;
        }
        let suffix = directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .rsplit_once('-')
            .unwrap()
            .1;
        let artifact = deps.join(format!("lib{name}-{suffix}.rlib"));
        if artifact.is_file() {
            artifacts.push(artifact);
        }
    }
    let [artifact] = artifacts.as_slice() else {
        panic!("one exact compiled dependency {name} for {executable_name}: {artifacts:?}");
    };
    artifact.clone()
}

/// Use the toolchain that compiled this test, rather than a changed ambient default.
pub(super) fn compiler(command: &mut std::process::Command) {
    let Some(toolchain) = option_env!("RUSTUP_TOOLCHAIN") else {
        panic!("native driver requires the running test's recorded rustup toolchain");
    };
    command.env("RUSTUP_TOOLCHAIN", toolchain);
}
