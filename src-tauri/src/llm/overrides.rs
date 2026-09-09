//! Per-provider base_url / model_id overrides (Settings → advanced).
//! Stored under app data — not secrets; empty fields mean “use registry default”.

use super::provider::{provider_spec, LlmProvider, ProviderSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

fn overrides_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("llm_provider_overrides.json"))
}

fn load_map(app: &AppHandle) -> Result<HashMap<String, ProviderOverride>, String> {
    let path = overrides_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return Ok(HashMap::new());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Invalid llm_provider_overrides.json: {e}"))
}

fn save_map(app: &AppHandle, map: &HashMap<String, ProviderOverride>) -> Result<(), String> {
    let path = overrides_path(app)?;
    let raw = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| e.to_string())
}

fn provider_key(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::ScalewayDeepseek => "scaleway_deepseek",
        LlmProvider::Mistral => "mistral",
        LlmProvider::Gemini => "gemini",
    }
}

fn nonempty(opt: Option<String>) -> Option<String> {
    opt.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

/// Registry defaults with optional Settings overrides applied.
pub fn resolved_spec(app: &AppHandle, provider: LlmProvider) -> Result<ProviderSpec, String> {
    let mut spec = provider_spec(provider);
    let map = load_map(app)?;
    if let Some(ov) = map.get(provider_key(provider)) {
        if let Some(url) = nonempty(ov.base_url.clone()) {
            spec.base_url = url;
        }
        if let Some(model) = nonempty(ov.model_id.clone()) {
            spec.model_id = model;
        }
    }
    Ok(spec)
}

#[tauri::command]
pub fn llm_provider_override_get(
    app: AppHandle,
    provider: String,
) -> Result<ProviderOverride, String> {
    let p = LlmProvider::parse(&provider)?;
    let map = load_map(&app)?;
    Ok(map.get(provider_key(p)).cloned().unwrap_or_default())
}

#[tauri::command]
pub fn llm_provider_override_set(
    app: AppHandle,
    provider: String,
    base_url: Option<String>,
    model_id: Option<String>,
) -> Result<(), String> {
    let p = LlmProvider::parse(&provider)?;
    let key = provider_key(p).to_string();
    let mut map = load_map(&app)?;
    let ov = ProviderOverride {
        base_url: nonempty(base_url),
        model_id: nonempty(model_id),
    };
    if ov.base_url.is_none() && ov.model_id.is_none() {
        map.remove(&key);
    } else {
        map.insert(key, ov);
    }
    save_map(&app, &map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonempty_trims() {
        assert_eq!(nonempty(Some("  x  ".into())), Some("x".into()));
        assert_eq!(nonempty(Some("   ".into())), None);
        assert_eq!(nonempty(None), None);
    }
}
