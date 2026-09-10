//! Locating and vouching for the mail-scan sidecar (spec §6.4, ADR 0002).
//!
//! Two modes, and `probe` reports which one is live:
//!
//! - **Release** — a single frozen PyInstaller binary shipped as a Tauri `externalBin`.
//!   One-file rather than one-dir: `externalBin` copies one file, so a one-dir
//!   launcher would arrive without the `_internal/` runtime it resolves beside itself.
//!   The binary is spawned from an absolute path and its hash is checked against a value
//!   baked in at build time, so a tampered or half-updated install refuses to run
//!   rather than executing whatever is on disk.
//! - **Dev** — `uv run --project <repo>` when a `uv` is on PATH, otherwise a system
//!   Python with `-I`. Never a shell, never a user-supplied interpreter path.
//!
//! The distinction matters because the release path is the one that has to work on a
//! machine with no Python at all.

use std::path::{Path, PathBuf};

/// Expected SHA-256 of the bundled sidecar, injected by the packaging step.
///
/// Empty in dev and in any build that did not run `scripts/build-sidecar.sh`, which is
/// why an empty value means "unpinned" rather than "mismatch": failing every dev build
/// on a missing pin would just train everyone to bypass the check.
pub const SIDECAR_SHA256: Option<&str> = option_env!("JOBTRACKER_SIDECAR_SHA256");

/// How the sidecar will be launched.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum SidecarMode {
    /// Bundled single-file binary, hash-pinned.
    Bundled { path: String, pinned: bool },
    /// `uv run --project <repo>` against the checked-out sources.
    Uv { project: String },
    /// System interpreter with `-I`, for a dev machine without `uv`.
    SystemPython { python: String },
}

impl SidecarMode {
    pub fn describe(&self) -> String {
        match self {
            Self::Bundled { path, pinned } => format!(
                "bundled sidecar at {path} ({})",
                if *pinned { "hash-pinned" } else { "unpinned" }
            ),
            Self::Uv { project } => format!("uv run --project {project}"),
            Self::SystemPython { python } => format!("system python {python} (isolated)"),
        }
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read sidecar: {e}"))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Verify the bundled binary against the baked-in hash.
///
/// Returns whether a pin was actually enforced, so `probe` can tell the user the
/// difference between "verified" and "nothing to verify against".
pub fn verify_pin(path: &Path) -> Result<bool, String> {
    let Some(expected) = SIDECAR_SHA256.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(false);
    };
    let actual = sha256_file(path)?;
    if actual == expected {
        Ok(true)
    } else {
        Err(format!(
            "E_SIDECAR_MISSING: the mail scanner does not match the signature shipped \
             with this build (expected {}…, found {}…). Reinstall the app.",
            &expected[..expected.len().min(12)],
            &actual[..actual.len().min(12)]
        ))
    }
}

/// Name of the bundled executable, per platform.
pub fn bundled_binary_name() -> &'static str {
    if cfg!(windows) {
        "jobtracker-mail-scan.exe"
    } else {
        "jobtracker-mail-scan"
    }
}

/// Where Tauri places an `externalBin` next to the app executable.
fn bundled_candidate() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let direct = dir.join(bundled_binary_name());
    direct.is_file().then_some(direct)
}

fn uv_on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(if cfg!(windows) { "uv.exe" } else { "uv" }))
        .find(|candidate| candidate.is_file())
}

/// Decide how to launch, preferring a bundled binary wherever one exists.
///
/// `python_root` is the repo's `python/` directory; `override_python` is the
/// `JOBTRACKER_MAIL_SCAN_PYTHON` escape hatch used by the test suite.
pub fn resolve(python_root: &Path, override_python: Option<PathBuf>) -> Result<SidecarMode, String> {
    if let Some(path) = bundled_candidate() {
        let pinned = verify_pin(&path)?;
        return Ok(SidecarMode::Bundled {
            path: path.to_string_lossy().into_owned(),
            pinned,
        });
    }

    if let Some(python) = override_python {
        return Ok(SidecarMode::SystemPython {
            python: python.to_string_lossy().into_owned(),
        });
    }

    if uv_on_path().is_some() {
        let project = python_root
            .parent()
            .unwrap_or(python_root)
            .to_string_lossy()
            .into_owned();
        return Ok(SidecarMode::Uv { project });
    }

    if cfg!(debug_assertions) {
        return Ok(SidecarMode::SystemPython {
            python: "python3".into(),
        });
    }

    Err("E_SIDECAR_MISSING: the mail scanner was not found in this install. \
         Reinstall the app, or run from a checkout with `uv` available."
        .into())
}

/// Program and arguments for a mode. Config always arrives on stdin, never in argv,
/// so nothing sensitive is visible in a process listing (spec §6.6).
pub fn command_for(mode: &SidecarMode, scan_argv: &[&str]) -> (PathBuf, Vec<String>) {
    match mode {
        SidecarMode::Bundled { path, .. } => {
            // A frozen binary is not an interpreter, so the interpreter-only argv is
            // dropped: `-I`, and `-m` together with the module name that follows it.
            // Everything else — the subcommand and its options — must survive intact.
            let mut args = Vec::new();
            let mut drop_module_name = false;
            for arg in scan_argv {
                if drop_module_name {
                    drop_module_name = false;
                    continue;
                }
                match *arg {
                    "-I" => continue,
                    "-m" => {
                        drop_module_name = true;
                        continue;
                    }
                    other => args.push(other.to_string()),
                }
            }
            (PathBuf::from(path), args)
        }
        SidecarMode::Uv { project } => (
            PathBuf::from("uv"),
            [
                "run",
                "--project",
                project.as_str(),
                "--no-sync",
                "python",
            ]
            .iter()
            .map(|a| (*a).to_string())
            .chain(scan_argv.iter().map(|a| (*a).to_string()))
            .collect(),
        ),
        SidecarMode::SystemPython { python } => (
            PathBuf::from(python),
            scan_argv.iter().map(|a| (*a).to_string()).collect(),
        ),
    }
}

/// What Settings shows about the sidecar.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarProbe {
    pub available: bool,
    pub mode: Option<SidecarMode>,
    pub description: String,
    pub hash_pinned: bool,
    pub error: Option<String>,
}

pub fn probe(python_root: &Path, override_python: Option<PathBuf>) -> SidecarProbe {
    match resolve(python_root, override_python) {
        Ok(mode) => {
            let hash_pinned = matches!(mode, SidecarMode::Bundled { pinned: true, .. });
            SidecarProbe {
                available: true,
                description: mode.describe(),
                mode: Some(mode),
                hash_pinned,
                error: None,
            }
        }
        Err(e) => SidecarProbe {
            available: false,
            mode: None,
            description: "not available".into(),
            hash_pinned: false,
            error: Some(e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unpinned_build_verifies_nothing_rather_than_failing() {
        // Dev builds carry no baked hash. Failing them would train everyone to
        // bypass the check, which is worse than not having one.
        let file = tempfile::NamedTempFile::new().unwrap();
        if SIDECAR_SHA256.map(str::trim).is_none_or(str::is_empty) {
            assert!(!verify_pin(file.path()).unwrap());
        }
    }

    #[test]
    fn a_missing_binary_is_an_error_not_a_silent_false() {
        let missing = Path::new("/nonexistent/jobtracker-mail-scan");
        if SIDECAR_SHA256.map(str::trim).is_some_and(|s| !s.is_empty()) {
            assert!(verify_pin(missing).is_err());
        }
    }

    #[test]
    fn dev_falls_back_to_a_system_interpreter() {
        let mode = resolve(Path::new("/repo/python"), Some(PathBuf::from("/usr/bin/python3")))
            .unwrap();
        assert_eq!(
            mode,
            SidecarMode::SystemPython {
                python: "/usr/bin/python3".into()
            }
        );
    }

    #[test]
    fn the_uv_command_never_goes_through_a_shell() {
        let (program, args) = command_for(
            &SidecarMode::Uv {
                project: "/repo".into(),
            },
            &["-I", "-m", "mail_scan", "scan"],
        );
        assert_eq!(program, PathBuf::from("uv"));
        assert!(args.contains(&"--project".to_string()));
        assert!(args.contains(&"scan".to_string()));
        assert!(
            !args.iter().any(|a| a.contains("&&") || a.contains(';')),
            "argv must never be shell syntax: {args:?}"
        );
    }

    #[test]
    fn the_bundled_command_drops_interpreter_flags_but_keeps_the_subcommand() {
        // Asserted against the *real* SCAN_ARGV, not a stand-in. An earlier version of
        // this test used a made-up argv without `--protocol 1`, which hid a filter that
        // stripped `--protocol` and left its value `1` behind as a stray argument.
        let (program, args) = command_for(
            &SidecarMode::Bundled {
                path: "/opt/app/jobtracker-mail-scan".into(),
                pinned: true,
            },
            crate::mail_scan::spawn::SCAN_ARGV,
        );
        assert_eq!(program, PathBuf::from("/opt/app/jobtracker-mail-scan"));
        assert_eq!(
            args,
            vec!["scan".to_string(), "--protocol".to_string(), "1".to_string()]
        );
    }

    #[test]
    fn every_mode_passes_the_protocol_through() {
        // The sidecar refuses to run without it, so a mode that loses it is a mode
        // that fails at the first spawn on a user's machine.
        for mode in [
            SidecarMode::Bundled { path: "/opt/x".into(), pinned: true },
            SidecarMode::Uv { project: "/repo".into() },
            SidecarMode::SystemPython { python: "python3".into() },
        ] {
            let (_, args) = command_for(&mode, crate::mail_scan::spawn::SCAN_ARGV);
            let protocol = args.iter().position(|a| a == "--protocol");
            assert!(protocol.is_some(), "{mode:?} lost --protocol: {args:?}");
            assert_eq!(args.get(protocol.unwrap() + 1), Some(&"1".to_string()), "{mode:?}");
            assert!(args.contains(&"scan".to_string()), "{mode:?} lost the subcommand");
        }
    }

    #[test]
    fn probe_names_the_active_mode() {
        let probe = probe(Path::new("/repo/python"), Some(PathBuf::from("python3")));
        assert!(probe.available);
        assert!(probe.description.contains("python3"), "{}", probe.description);
    }

    #[test]
    fn a_binary_name_is_platform_correct() {
        let name = bundled_binary_name();
        assert!(name.starts_with("jobtracker-mail-scan"));
        assert_eq!(name.ends_with(".exe"), cfg!(windows));
    }
}
