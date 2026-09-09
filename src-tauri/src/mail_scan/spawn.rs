//! Sidecar process spawn with an isolated environment (spec §6.4 / §4.4).

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Fixed argv for the sidecar — config is stdin-only.
pub const SCAN_ARGV: &[&str] = &["-m", "mail_scan", "scan", "--protocol", "1"];

pub struct SpawnedScan {
    child: Arc<Mutex<Child>>,
    pub stdout: Option<ChildStdout>,
}

impl SpawnedScan {
    pub fn child_handle(&self) -> Arc<Mutex<Child>> {
        Arc::clone(&self.child)
    }

    /// Block until the child exits (normal completion path).
    pub fn wait(&self) -> Result<Option<i32>, String> {
        let mut child = self.child.lock().map_err(|e| e.to_string())?;
        match child.try_wait() {
            Ok(Some(status)) => Ok(status.code()),
            Ok(None) => {
                let status = child.wait().map_err(|e| e.to_string())?;
                Ok(status.code())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// After `cancel_file` appears, escalate: SIGTERM at 5s, SIGKILL at 10s (§4.4).
/// Returns when the child has exited (or was never running).
pub fn watch_cancel_escalation(child: Arc<Mutex<Child>>, cancel_file: PathBuf) {
    loop {
        {
            let mut guard = match child.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            match guard.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(_) => return,
            }
        }
        if cancel_file.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let start = Instant::now();
    let mut signaled_term = false;
    loop {
        {
            let mut guard = match child.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            match guard.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(_) => return,
            }
            let elapsed = start.elapsed();
            if elapsed >= Duration::from_secs(10) {
                let _ = guard.kill();
                let _ = guard.wait();
                return;
            }
            if !signaled_term && elapsed >= Duration::from_secs(5) {
                signaled_term = true;
                #[cfg(unix)]
                {
                    let pid = guard.id();
                    let _ = Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .status();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Build the child environment: constructed, not inherited.
///
/// `PYTHONPATH` is set deliberately so `-m mail_scan` resolves in dev; `PYTHONHOME`
/// and `PYTHONSTARTUP` stay absent. Release packaging (PyInstaller / externalBin) is PR C.
pub fn isolated_env(python_root: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("PYTHONNOUSERSITE".into(), "1".into());
    env.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
    env.insert("PYTHONUTF8".into(), "1".into());
    env.insert(
        "PYTHONPATH".into(),
        python_root.to_string_lossy().into_owned(),
    );
    env.insert("LC_ALL".into(), "C.UTF-8".into());
    env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
    env
}

/// Dev-only spawn. Refuse outside debug builds unless explicitly opted in.
pub fn spawn_scan(
    python: &Path,
    python_root: &Path,
    config_json: &str,
) -> Result<SpawnedScan, String> {
    ensure_dev_spawn_allowed()?;
    let env = isolated_env(python_root);
    assert_env_contract(&env)?;

    let mut cmd = Command::new(python);
    cmd.args(SCAN_ARGV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(&env);
    // Per-run cwd: prefer a temp dir so the repo root is never the child's cwd.
    let work = std::env::temp_dir().join("jobtracker-mail-scan-cwd");
    let _ = std::fs::create_dir_all(&work);
    cmd.current_dir(&work);

    // Stay in the parent's process group so killing the app reaps the child (Unix).
    // Do NOT call setpgid(0,0) — that orphans the sidecar on parent death.

    let mut child = cmd.spawn().map_err(|e| format!("spawn mail_scan: {e}"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "sidecar stdin missing".to_string())?;
        stdin
            .write_all(config_json.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    drop(child.stdin.take());

    let stdout = child.stdout.take();
    Ok(SpawnedScan {
        child: Arc::new(Mutex::new(child)),
        stdout,
    })
}

fn ensure_dev_spawn_allowed() -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    if std::env::var_os("JOBTRACKER_MAIL_SCAN_DEV").is_some() {
        return Ok(());
    }
    Err(
        "Mail scan sidecar spawn is disabled in release builds until PR C packaging \
         (set JOBTRACKER_MAIL_SCAN_DEV=1 only for explicit local testing)."
            .into(),
    )
}

fn assert_env_contract(env: &HashMap<String, String>) -> Result<(), String> {
    if env.contains_key("PYTHONHOME") || env.contains_key("PYTHONSTARTUP") {
        return Err("isolated env must not set PYTHONHOME/PYTHONSTARTUP".into());
    }
    if env.get("PYTHONNOUSERSITE").map(String::as_str) != Some("1") {
        return Err("PYTHONNOUSERSITE=1 required".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_env_omits_sensitive_python_vars() {
        let env = isolated_env(Path::new("/tmp/python"));
        assert!(!env.contains_key("PYTHONHOME"));
        assert!(!env.contains_key("PYTHONSTARTUP"));
        assert_eq!(env.get("PYTHONNOUSERSITE").map(String::as_str), Some("1"));
        assert_eq!(
            env.get("PYTHONDONTWRITEBYTECODE").map(String::as_str),
            Some("1")
        );
        assert!(env.contains_key("PYTHONPATH"));
    }

    #[test]
    fn scan_argv_is_stdin_config_only() {
        assert_eq!(SCAN_ARGV, &["-m", "mail_scan", "scan", "--protocol", "1"]);
        assert!(
            !SCAN_ARGV.iter().any(|a| a.contains('{') || a.contains("run_id")),
            "config must not appear on argv"
        );
    }
}
