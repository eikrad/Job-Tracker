//! Candidate Profiles — the CV content scoring runs against (spec §6.5).
//!
//! Stored at `$APPDATA/profiles/{short,full}.md`, mode `0600`. The content is never
//! returned to the frontend: Settings sees a size, a modified time, and a hash prefix.
//!
//! Identity is the **SHA-256 of the file contents**, never its mtime. A copy, a restore
//! from backup, or a cloud-sync round-trip changes mtime but not content — and must not
//! re-spend the backlog. See [`super::score_cache`] for the other half of that rule.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    Short,
    Full,
}

impl ProfileKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "short" => Ok(Self::Short),
            "full" => Ok(Self::Full),
            other => Err(format!("Unknown profile kind: {other}")),
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Short => "short.md",
            Self::Full => "full.md",
        }
    }
}

/// Directory name under app data. Also the name the export/backup paths must skip.
pub const PROFILES_DIR: &str = "profiles";

/// A profile's metadata — everything Settings needs and nothing it should not have.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStatus {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    /// First 12 hex chars of the content hash — enough to see that it changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_prefix: Option<String>,
}

impl ProfileStatus {
    fn missing() -> Self {
        Self {
            configured: false,
            size_bytes: None,
            modified_at: None,
            hash_prefix: None,
        }
    }
}

/// Loaded profile: the text to prompt with and the hash that keys its scores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProfile {
    pub text: String,
    pub content_hash: String,
}

pub fn hash_content(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn profiles_dir(app_data: &Path) -> PathBuf {
    app_data.join(PROFILES_DIR)
}

fn profile_path(app_data: &Path, kind: ProfileKind) -> PathBuf {
    profiles_dir(app_data).join(kind.file_name())
}

/// Owner-only permissions. A CV sitting world-readable in a shared home directory is
/// the quiet version of the leak this whole section exists to prevent.
#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), String> {
    // Windows inherits the user-profile ACL, which is already owner-only.
    Ok(())
}

/// Write (or replace) a profile. Content arrives as text, never as a path the caller
/// can point at an arbitrary file we would then read back out through `status`.
pub fn write_profile(app_data: &Path, kind: ProfileKind, content: &str) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("E_PROFILE_UNREADABLE: the profile is empty.".into());
    }
    let dir = profiles_dir(app_data);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    restrict_dir(&dir)?;
    let path = profile_path(app_data, kind);
    fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    restrict_permissions(&path)?;
    Ok(hash_content(content.as_bytes()))
}

#[cfg(unix)]
fn restrict_dir(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn restrict_dir(_dir: &Path) -> Result<(), String> {
    Ok(())
}

pub fn clear_profile(app_data: &Path, kind: ProfileKind) -> Result<(), String> {
    let path = profile_path(app_data, kind);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn profile_status(app_data: &Path, kind: ProfileKind) -> ProfileStatus {
    let path = profile_path(app_data, kind);
    let Ok(bytes) = fs::read(&path) else {
        return ProfileStatus::missing();
    };
    if bytes.is_empty() {
        return ProfileStatus::missing();
    }
    let modified_at = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
    let full = hash_content(&bytes);
    ProfileStatus {
        configured: true,
        size_bytes: Some(bytes.len() as u64),
        modified_at,
        hash_prefix: Some(full.chars().take(12).collect()),
    }
}

/// Read a profile for scoring. Missing or empty is `E_PROFILE_UNREADABLE` (spec §10) —
/// scoring a CV-less profile would silently produce meaningless scores.
pub fn load_profile(app_data: &Path, kind: ProfileKind) -> Result<LoadedProfile, String> {
    let path = profile_path(app_data, kind);
    let text = fs::read_to_string(&path).map_err(|_| {
        format!(
            "E_PROFILE_UNREADABLE: cannot read the {} profile — re-select it in Settings.",
            kind.file_name()
        )
    })?;
    if text.trim().is_empty() {
        return Err(format!(
            "E_PROFILE_UNREADABLE: the {} profile is empty.",
            kind.file_name()
        ));
    }
    let content_hash = hash_content(text.as_bytes());
    Ok(LoadedProfile { text, content_hash })
}

/// Upper bound on a profile file. A CV is a few KB; anything near this is a mistake,
/// and the whole file goes into every scoring prompt.
pub const MAX_PROFILE_BYTES: u64 = 1024 * 1024;

fn app_profiles_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
pub fn mail_scan_profile_status(
    app: tauri::AppHandle,
    kind: String,
) -> Result<ProfileStatus, String> {
    let kind = ProfileKind::parse(&kind)?;
    Ok(profile_status(&app_profiles_root(&app)?, kind))
}

/// Copy a profile in from a path the user picked in a file dialog.
///
/// Rust reads the file; the CV never crosses the IPC boundary in either direction, so
/// a compromised webview cannot read it back out (spec §6.1, §6.5).
#[tauri::command]
pub fn mail_scan_profile_set_from_path(
    app: tauri::AppHandle,
    kind: String,
    path: String,
) -> Result<ProfileStatus, String> {
    let kind = ProfileKind::parse(&kind)?;
    let root = app_profiles_root(&app)?;
    let src = PathBuf::from(shellexpand::tilde(&path).as_ref());

    let meta = fs::metadata(&src)
        .map_err(|e| format!("E_PROFILE_UNREADABLE: cannot read that file ({e})."))?;
    if !meta.is_file() {
        return Err("E_PROFILE_UNREADABLE: that path is not a regular file.".into());
    }
    if meta.len() > MAX_PROFILE_BYTES {
        return Err(format!(
            "E_PROFILE_UNREADABLE: that file is {} KB; the limit is {} KB.",
            meta.len() / 1024,
            MAX_PROFILE_BYTES / 1024
        ));
    }
    let content = fs::read_to_string(&src).map_err(|_| {
        "E_PROFILE_UNREADABLE: the profile must be UTF-8 text (.md or .txt), not a PDF or Word file."
            .to_string()
    })?;

    write_profile(&root, kind, &content)?;
    Ok(profile_status(&root, kind))
}

#[tauri::command]
pub fn mail_scan_profile_clear(
    app: tauri::AppHandle,
    kind: String,
) -> Result<ProfileStatus, String> {
    let kind = ProfileKind::parse(&kind)?;
    let root = app_profiles_root(&app)?;
    clear_profile(&root, kind)?;
    Ok(profile_status(&root, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn status_never_exposes_profile_content() {
        let dir = tmp();
        write_profile(dir.path(), ProfileKind::Short, "SECRET CV BODY").unwrap();

        let status = profile_status(dir.path(), ProfileKind::Short);
        let json = serde_json::to_string(&status).unwrap();

        assert!(status.configured);
        assert!(
            !json.contains("SECRET CV BODY"),
            "profile status must not carry the CV: {json}"
        );
        assert_eq!(status.size_bytes, Some(14));
        assert_eq!(status.hash_prefix.unwrap().len(), 12);
    }

    #[test]
    fn missing_profile_reports_not_configured_rather_than_erroring() {
        let dir = tmp();
        assert_eq!(
            profile_status(dir.path(), ProfileKind::Full),
            ProfileStatus::missing()
        );
    }

    #[test]
    fn load_profile_maps_missing_to_the_taxonomy_code() {
        let dir = tmp();
        let err = load_profile(dir.path(), ProfileKind::Full).unwrap_err();
        assert!(err.starts_with("E_PROFILE_UNREADABLE"), "{err}");
    }

    #[test]
    fn empty_profile_is_rejected_on_write_and_on_load() {
        let dir = tmp();
        assert!(write_profile(dir.path(), ProfileKind::Short, "   ")
            .unwrap_err()
            .starts_with("E_PROFILE_UNREADABLE"));

        fs::create_dir_all(profiles_dir(dir.path())).unwrap();
        fs::write(profiles_dir(dir.path()).join("short.md"), "  \n ").unwrap();
        assert!(load_profile(dir.path(), ProfileKind::Short)
            .unwrap_err()
            .starts_with("E_PROFILE_UNREADABLE"));
    }

    #[test]
    fn hash_follows_content_not_mtime() {
        let dir = tmp();
        let first = write_profile(dir.path(), ProfileKind::Short, "same body").unwrap();

        // Simulate a copy/restore/sync: mtime moves, bytes do not.
        let path = profiles_dir(dir.path()).join("short.md");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        let handle = fs::File::options().write(true).open(&path).unwrap();
        handle
            .set_times(fs::FileTimes::new().set_modified(later))
            .unwrap();
        drop(handle);

        let after = load_profile(dir.path(), ProfileKind::Short).unwrap().content_hash;
        assert_eq!(first, after, "mtime must not change profile identity");

        let changed = write_profile(dir.path(), ProfileKind::Short, "different body").unwrap();
        assert_ne!(first, changed, "content change must change profile identity");
    }

    #[cfg(unix)]
    #[test]
    fn profile_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp();
        write_profile(dir.path(), ProfileKind::Full, "cv").unwrap();
        let mode = fs::metadata(profiles_dir(dir.path()).join("full.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "profile must not be readable by group or other");
    }

    #[test]
    fn clear_is_idempotent() {
        let dir = tmp();
        write_profile(dir.path(), ProfileKind::Short, "cv").unwrap();
        clear_profile(dir.path(), ProfileKind::Short).unwrap();
        clear_profile(dir.path(), ProfileKind::Short).unwrap();
        assert!(!profile_status(dir.path(), ProfileKind::Short).configured);
    }
}

/// Privacy guards for the profile files (spec §6.5).
///
/// These live here rather than beside the export/backup code because the property
/// belongs to the profiles: whatever those paths grow into later, a CV must not be in
/// them. `backupFolder` defaults to `~/Jottacloud`, so a regression is a CV in a cloud
/// folder, discovered by nobody.
#[cfg(test)]
mod privacy_tests {
    use super::*;
    use crate::db::{copy_backup_assets, BACKED_UP_SUBDIRS};

    #[test]
    fn backup_never_copies_candidate_profiles() {
        let app_data = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();

        // A realistic app-data tree: a profile, and a document that *should* travel.
        write_profile(app_data.path(), ProfileKind::Short, "MY CV — short").unwrap();
        write_profile(app_data.path(), ProfileKind::Full, "MY CV — full").unwrap();
        let docs = app_data.path().join("storage").join("applications");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("cover-letter.pdf"), b"%PDF-1.4 letter").unwrap();

        copy_backup_assets(app_data.path(), dest.path()).unwrap();

        assert!(
            dest.path()
                .join("storage/applications/cover-letter.pdf")
                .exists(),
            "documents must still be backed up"
        );

        let copied = collect_files(dest.path());
        assert!(
            !copied.iter().any(|p| p.contains("profiles")),
            "a CV must never reach the backup folder, found: {copied:?}"
        );
        for leaked in ["MY CV — short", "MY CV — full"] {
            assert!(
                !copied.iter().any(|p| file_contains(dest.path(), p, leaked)),
                "profile content {leaked:?} leaked into the backup"
            );
        }
    }

    #[test]
    fn the_backup_allowlist_does_not_name_the_profiles_directory() {
        // Guards the allowlist itself: adding `profiles` here would be a one-line
        // change that silently exports the user's CV.
        for entry in BACKED_UP_SUBDIRS {
            assert!(
                !entry.split('/').any(|c| c == PROFILES_DIR),
                "{entry} would export the Candidate Profiles"
            );
        }
    }

    #[test]
    fn a_backup_of_an_app_data_dir_with_only_profiles_copies_nothing_sensitive() {
        let app_data = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        write_profile(app_data.path(), ProfileKind::Full, "SENSITIVE CV").unwrap();

        copy_backup_assets(app_data.path(), dest.path()).unwrap();

        assert!(
            collect_files(dest.path()).is_empty(),
            "nothing but empty scaffold directories should be created"
        );
    }

    fn collect_files(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
        out
    }

    fn file_contains(root: &Path, rel: &str, needle: &str) -> bool {
        fs::read_to_string(root.join(rel))
            .map(|s| s.contains(needle))
            .unwrap_or(false)
    }
}
