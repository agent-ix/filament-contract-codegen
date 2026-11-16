//! The tool versions the installed Kani launcher reports (FR-028-AC-23).
//!
//! `cargo-kani --version` prints the Kani release on its first line and the CBMC release on its
//! second. The versions are read from the launcher a run uses, never from whichever `kani` or
//! `cbmc` is first on `PATH`, and a launcher that does not report them is recorded as such rather
//! than guessed.

use std::{num::NonZeroUsize, process::Command, time::Duration};

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
        /// The Kani release, for example the `0.68.0` of `Kani Rust Verifier 0.68.0`.
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

    /// Probes the installation's launcher, once per installation and its clones.
    pub(crate) fn of(installation: &KaniInstallation, cancel: &Cancel) -> Self {
        installation
            .version_cache()
            .get_or_init(|| Self::probe(installation, cancel))
            .clone()
    }

    fn probe(installation: &KaniInstallation, cancel: &Cancel) -> Self {
        let mut command = Command::new(&installation.launcher);
        command.arg("--version");
        let failed = || Self::Unreported {
            reason: KaniToolVersionsAbsence::ProbeFailed,
        };
        match run_bounded_launcher(
            command,
            PROBE_BUDGET,
            NonZeroUsize::MIN,
            None,
            cancel,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_launcher_output_parses_and_noise_does_not() {
        assert_eq!(
            KaniToolVersions::parse("Kani Rust Verifier 0.68.0 (cargo plugin)\nCBMC 6.11.0\n"),
            KaniToolVersions::Reported {
                kani: "0.68.0".into(),
                cbmc: "6.11.0".into()
            }
        );
        for text in [
            "",
            "Kani Rust Verifier 0.68.0 (cargo plugin)\n",
            "CBMC 6.11.0\n",
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
