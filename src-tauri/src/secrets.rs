//! Provider API keys in the OS keyring (ADR 0005), with a `0600` file fallback.
//!
//! Unit tests use [`MemoryStore`] so CI without a secret service stays green.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

const KEYRING_USER: &str = "api_key";

/// Providers whose secrets live in this store (including Google OAuth tokens — H5).
pub const PROVIDERS: &[&str] = &[
    "gemini",
    "mistral",
    "scaleway",
    "serpapi",
    "brave",
    "google_access_token",
    "google_refresh_token",
];

fn validate_provider(provider: &str) -> Result<(), String> {
    if PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(format!("Unknown secret provider: {provider}"))
    }
}

fn keyring_service(provider: &str) -> String {
    format!("JobTracker-{provider}")
}

pub trait SecretStore: Send + Sync {
    fn set(&self, provider: &str, value: &str) -> Result<(), String>;
    fn get(&self, provider: &str) -> Result<Option<String>, String>;
    fn clear(&self, provider: &str) -> Result<(), String>;
    fn backend_name(&self) -> &'static str;

    fn is_configured(&self, provider: &str) -> Result<bool, String> {
        Ok(self
            .get(provider)?
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false))
    }
}

#[derive(Default)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct MemoryStore {
    inner: Mutex<HashMap<String, String>>,
}

impl SecretStore for MemoryStore {
    fn set(&self, provider: &str, value: &str) -> Result<(), String> {
        validate_provider(provider)?;
        self.inner
            .lock()
            .map_err(|e| e.to_string())?
            .insert(provider.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, provider: &str) -> Result<Option<String>, String> {
        validate_provider(provider)?;
        Ok(self
            .inner
            .lock()
            .map_err(|e| e.to_string())?
            .get(provider)
            .cloned())
    }

    fn clear(&self, provider: &str) -> Result<(), String> {
        validate_provider(provider)?;
        self.inner
            .lock()
            .map_err(|e| e.to_string())?
            .remove(provider);
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "memory"
    }
}

pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    pub fn new(dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(Self { dir })
    }

    fn path_for(&self, provider: &str) -> PathBuf {
        self.dir.join(format!("{provider}.key"))
    }
}

impl SecretStore for FileStore {
    fn set(&self, provider: &str, value: &str) -> Result<(), String> {
        validate_provider(provider)?;
        let path = self.path_for(provider);
        write_secret_file(&path, value)
    }

    fn get(&self, provider: &str) -> Result<Option<String>, String> {
        validate_provider(provider)?;
        let path = self.path_for(provider);
        if !path.exists() {
            return Ok(None);
        }
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let t = s.trim().to_string();
        if t.is_empty() {
            Ok(None)
        } else {
            Ok(Some(t))
        }
    }

    fn clear(&self, provider: &str) -> Result<(), String> {
        validate_provider(provider)?;
        let path = self.path_for(provider);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "file"
    }
}

fn write_secret_file(path: &Path, value: &str) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        f.set_permissions(perms).map_err(|e| e.to_string())?;
    }
    f.write_all(value.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn set(&self, provider: &str, value: &str) -> Result<(), String> {
        validate_provider(provider)?;
        let entry = keyring::Entry::new(&keyring_service(provider), KEYRING_USER)
            .map_err(|e| e.to_string())?;
        entry.set_password(value).map_err(|e| e.to_string())
    }

    fn get(&self, provider: &str) -> Result<Option<String>, String> {
        validate_provider(provider)?;
        let entry = keyring::Entry::new(&keyring_service(provider), KEYRING_USER)
            .map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(p) if !p.trim().is_empty() => Ok(Some(p)),
            Ok(_) => Ok(None),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn clear(&self, provider: &str) -> Result<(), String> {
        validate_provider(provider)?;
        let entry = keyring::Entry::new(&keyring_service(provider), KEYRING_USER)
            .map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn backend_name(&self) -> &'static str {
        "keyring"
    }
}

/// Probe keyring; on failure use a `0600` file store under app data.
pub fn open_store(app: &AppHandle) -> Result<Box<dyn SecretStore>, String> {
    match try_keyring_probe() {
        Ok(()) => Ok(Box::new(KeyringStore)),
        Err(_) => {
            let dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("secrets");
            Ok(Box::new(FileStore::new(dir)?))
        }
    }
}

fn try_keyring_probe() -> Result<(), String> {
    let entry =
        keyring::Entry::new("JobTracker-probe", "probe").map_err(|e| e.to_string())?;
    entry.set_password("ok").map_err(|e| e.to_string())?;
    let _ = entry.delete_credential();
    Ok(())
}

/// Chosen once at startup. Not a `Mutex` — backend I/O must not serialize behind a
/// global lock, since every provider lookup would otherwise queue on one keyring call.
static APP_STORE: OnceLock<Box<dyn SecretStore>> = OnceLock::new();

pub fn init_app_store(app: &AppHandle) -> Result<(), String> {
    let store = open_store(app)?;
    // Already initialized is not an error; the first backend choice wins.
    let _ = APP_STORE.set(store);
    Ok(())
}

fn with_store<R>(f: impl FnOnce(&dyn SecretStore) -> Result<R, String>) -> Result<R, String> {
    let store = APP_STORE
        .get()
        .ok_or_else(|| "Secret store not initialized".to_string())?;
    f(store.as_ref())
}

/// Rust-side read (never exposed to the frontend).
pub fn get_secret(provider: &str) -> Result<Option<String>, String> {
    with_store(|s| s.get(provider))
}

/// Read a secret, treating "absent" and "store unavailable" alike as empty.
pub fn get_secret_or_default(provider: &str) -> String {
    get_secret(provider).ok().flatten().unwrap_or_default()
}

/// Rust-side write (never exposed to the frontend).
pub fn set_secret(provider: &str, value: &str) -> Result<(), String> {
    with_store(|s| s.set(provider, value))
}

/// Rust-side delete (never exposed to the frontend).
pub fn clear_secret(provider: &str) -> Result<(), String> {
    with_store(|s| s.clear(provider))
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub configured: bool,
    pub backend: String,
}

#[tauri::command]
pub fn llm_key_set(provider: String, key: String) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key must not be empty".into());
    }
    with_store(|s| s.set(&provider, trimmed))
}

#[tauri::command]
pub fn llm_key_status(provider: String) -> Result<KeyStatus, String> {
    with_store(|s| {
        Ok(KeyStatus {
            configured: s.is_configured(&provider)?,
            backend: s.backend_name().to_string(),
        })
    })
}

#[tauri::command]
pub fn llm_key_clear(provider: String) -> Result<(), String> {
    with_store(|s| s.clear(&provider))
}

/// Replace high-entropy tokens in log/error strings.
pub fn redact(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut buf = String::new();
    for ch in s.chars() {
        if is_secret_char(ch) {
            buf.push(ch);
        } else {
            flush_secret_buf(&mut out, &mut buf);
            out.push(ch);
        }
    }
    flush_secret_buf(&mut out, &mut buf);
    out
}

fn flush_secret_buf(out: &mut String, buf: &mut String) {
    if buf.len() >= 32 {
        out.push_str("***");
    } else {
        out.push_str(buf);
    }
    buf.clear();
}

fn is_secret_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '/' | '=')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_set_status_clear() {
        let store = MemoryStore::default();
        assert!(!store.is_configured("gemini").unwrap());
        store.set("gemini", "sk-test-key").unwrap();
        assert!(store.is_configured("gemini").unwrap());
        assert_eq!(store.get("gemini").unwrap().as_deref(), Some("sk-test-key"));
        store.clear("gemini").unwrap();
        assert!(!store.is_configured("gemini").unwrap());
    }

    #[test]
    fn status_json_never_contains_secret() {
        let store = MemoryStore::default();
        store
            .set("mistral", "super-secret-key-material-xxxxx")
            .unwrap();
        let status = KeyStatus {
            configured: store.is_configured("mistral").unwrap(),
            backend: store.backend_name().to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("configured"));
        assert!(json.contains("backend"));
        assert!(!json.contains("super-secret"));
        assert!(!json.contains("key-material"));
    }

    #[test]
    fn google_oauth_tokens_are_known_providers() {
        // google_oauth.rs stores the refresh token under this exact name.
        let store = MemoryStore::default();
        store.set("google_refresh_token", "1//refresh").unwrap();
        assert_eq!(
            store.get("google_refresh_token").unwrap().as_deref(),
            Some("1//refresh")
        );
        assert!(validate_provider("google_access_token").is_ok());
        assert!(validate_provider("google_oauth_token").is_err());
    }

    #[test]
    fn redact_long_token() {
        let token = "a".repeat(40);
        let msg = format!("upstream failed with token {token} in body");
        let redacted = redact(&msg);
        assert!(!redacted.contains(&token));
        assert!(redacted.contains("***"));
    }

    #[test]
    fn redact_leaves_short_text() {
        assert_eq!(redact("hello world"), "hello world");
        assert_eq!(redact(""), "");
        assert_eq!(redact("café"), "café");
    }

    #[test]
    fn file_store_roundtrip_and_mode() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path().to_path_buf()).unwrap();
        store.set("brave", "brave-secret-value").unwrap();
        assert_eq!(
            store.get("brave").unwrap().as_deref(),
            Some("brave-secret-value")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(dir.path().join("brave.key")).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
        store.clear("brave").unwrap();
        assert!(store.get("brave").unwrap().is_none());
    }

    #[test]
    #[ignore = "requires OS secret service; run locally with --ignored"]
    fn keyring_integration_smoke() {
        let store = KeyringStore;
        store
            .set("gemini", "integration-probe-key-not-real")
            .unwrap();
        assert!(store.is_configured("gemini").unwrap());
        store.clear("gemini").unwrap();
    }
}
