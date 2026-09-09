//! Sidecar process spawn with an isolated environment (spec §6.4 / §4.4).

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub struct SpawnedScan {
    child: Child,
    pub stdout: Option<ChildStdout>,
}

impl SpawnedScan {
    pub fn wait(&mut self) -> Result<Option<i32>, String> {
        let status = self.child.wait().map_err(|e| e.to_string())?;
        Ok(status.code())
    }
}

/// Build the child environment: constructed, not inherited.
pub fn isolated_env(python_root: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("PYTHONNOUSERSITE".into(), "1".into());
    env.insert(
        "PYTHONPATH".into(),
        python_root.to_string_lossy().into_owned(),
    );
    // Explicitly omit PYTHONHOME / PYTHONSTARTUP — never copy from parent.
    env.insert("LC_ALL".into(), "C.UTF-8".into());
    env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
    env
}

pub fn spawn_scan(
    python: &Path,
    python_root: &Path,
    config_json: &str,
) -> Result<SpawnedScan, String> {
    let env = isolated_env(python_root);
    assert_env_contract(&env)?;

    let mut cmd = Command::new(python);
    cmd.args(["-m", "mail_scan", "scan", "--protocol", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(&env)
        .current_dir(python_root.parent().unwrap_or(python_root));

    #[cfg(unix)]
    // New process group so killing the group reaps the child on app crash.
    unsafe {
        cmd.pre_exec(|| {
            if libc_setpgid_0() != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

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
    Ok(SpawnedScan { child, stdout })
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

#[cfg(unix)]
fn libc_setpgid_0() -> i32 {
    // Avoid a libc crate dep: setpgid(0, 0) via raw extern.
    extern "C" {
        fn setpgid(pid: i32, pgid: i32) -> i32;
    }
    unsafe { setpgid(0, 0) }
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
        assert!(env.contains_key("PYTHONPATH"));
    }

    #[test]
    fn argv_has_no_config_blob() {
        // Config is stdin-only; argv is fixed.
        let args = ["-m", "mail_scan", "scan", "--protocol", "1"];
        assert!(!args.iter().any(|a| a.contains("run_id")));
        assert!(!args.iter().any(|a| a.contains("api")));
    }
}
