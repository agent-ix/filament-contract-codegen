//! The tool versions the installed Kani launcher reports (FR-028-AC-23).
//!
//! `cargo-kani --version` prints the Kani release on its first line and the CBMC release on its
//! second. The versions are read from the launcher a run uses, never from whichever `kani` or
//! `cbmc` is first on `PATH`, and a launcher that does not report them is recorded as such rather
//! than guessed.

use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use filament_exact::Cancel;
use serde::Serialize;

use super::{
    launch::{run_bounded_launcher, LaunchOutcome},
    tool::KaniInstallation,
};

/// The wall-clock budget of the version probe.
const PROBE_BUDGET: Duration = Duration::from_secs(60);

/// What a run records of the tools that produced it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum KaniToolVersions {
    /// The launcher reported both releases.
    Reported {
        /// The Kani release the launcher names on its `Kani Rust Verifier` line.
        kani: String,
        /// The CBMC release. The solver the run names is linked into this CBMC build, so this is
        /// also the solver's version.
        cbmc: String,
    },
    /// The launcher did not report them.
    Unreported {
        /// Why no version is recorded.
        reason: KaniToolVersionsAbsence,
    },
}

/// Why a run records no tool versions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniToolVersionsAbsence {
    /// The probe could not be started, was stopped, exited unsuccessfully or overran its budget.
    ProbeFailed,
    /// The probe exited successfully but its output named no Kani release or no CBMC release.
    Unparseable,
}

impl KaniToolVersions {
    /// Reads the output of `cargo-kani --version`.
    pub(crate) fn parse(text: &str) -> Self {
        let release = |prefix: &str| {
            text.lines().find_map(|line| {
                let rest = line.trim().strip_prefix(prefix)?;
                let token = rest.split_whitespace().next()?;
                let valid = token.starts_with(|c: char| c.is_ascii_digit())
                    && token
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
                valid.then(|| token.to_owned())
            })
        };
        match (release("Kani Rust Verifier "), release("CBMC ")) {
            (Some(kani), Some(cbmc)) => Self::Reported { kani, cbmc },
            _ => Self::Unreported {
                reason: KaniToolVersionsAbsence::Unparseable,
            },
        }
    }

    /// The releases the installation's launcher reports. A successful probe is remembered for the
    /// launcher file it came from (path, size and modification time), so a clone given another
    /// launcher, or a launcher replaced on disk, is probed again; a failed probe is never
    /// remembered.
    pub(crate) fn of(installation: &KaniInstallation) -> Self {
        let identity = LauncherIdentity::of(&installation.launcher);
        let mut cache = installation
            .version_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let (Some(identity), Some((cached, versions))) = (&identity, cache.as_ref()) {
            if cached == identity {
                return versions.clone();
            }
        }
        let probed = Self::probe(installation);
        if let (Some(identity), Self::Reported { .. }) = (identity, &probed) {
            *cache = Some((identity, probed.clone()));
        }
        probed
    }

    /// The probe runs under its own cancellation handle, so one run's cancellation never decides
    /// what another run records, and in the launcher's own directory, so a launcher that writes
    /// to a relative path never writes into the caller's working directory.
    fn probe(installation: &KaniInstallation) -> Self {
        let mut command = Command::new(&installation.launcher);
        command.arg("--version");
        command.current_dir(
            installation
                .launcher
                .parent()
                .filter(|directory| !directory.as_os_str().is_empty())
                .map_or_else(std::env::temp_dir, Path::to_path_buf),
        );
        let failed = || Self::Unreported {
            reason: KaniToolVersionsAbsence::ProbeFailed,
        };
        match run_bounded_launcher(
            command,
            PROBE_BUDGET,
            NonZeroUsize::MIN,
            None,
            &Cancel::new(),
            installation.owners(),
        ) {
            Ok(launch) => match launch.outcome {
                LaunchOutcome::Completed {
                    exited_successfully: true,
                    streams,
                    ..
                } => Self::parse(&streams.merged_text()),
                _ => failed(),
            },
            Err(_) => failed(),
        }
    }
}

/// The launcher file a remembered probe belongs to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LauncherIdentity {
    path: PathBuf,
    length: u64,
    modified: Option<SystemTime>,
}

impl LauncherIdentity {
    fn of(launcher: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(launcher).ok()?;
        Some(Self {
            path: std::fs::canonicalize(launcher).ok()?,
            length: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ix_trace_rs::trace("TC-039", "FR-028-AC-23")]
    #[test]
    fn the_real_launcher_output_parses_and_noise_does_not() {
        assert_eq!(
            KaniToolVersions::parse(
                "Kani Rust Verifier 1.2.3-test (cargo plugin)\nCBMC 4.5.6-test\n"
            ),
            KaniToolVersions::Reported {
                kani: "1.2.3-test".into(),
                cbmc: "4.5.6-test".into()
            }
        );
        for text in [
            "",
            "Kani Rust Verifier 1.2.3-test (cargo plugin)\n",
            "CBMC 4.5.6-test\n",
            "Kani Rust Verifier (cargo plugin)\nCBMC unknown\n",
        ] {
            assert_eq!(
                KaniToolVersions::parse(text),
                KaniToolVersions::Unreported {
                    reason: KaniToolVersionsAbsence::Unparseable
                },
                "{text:?}"
            );
        }
    }
}
