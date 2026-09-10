//! HTTP calls to LLM providers using keys from the secret store.

use super::chat::{chat_json, ChatRequest, RetryPolicy};
use super::normalize::parse_partial_new_job_from_llm_text;
use super::provider::{AuthStyle, JsonMode, LlmProvider};
use crate::secrets::{self, redact};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const EXTRACT_PROMPT: &str = include_str!("../../prompts/extract.md");
const EXTRACT_SCHEMA: &str = include_str!("../../prompts/extract.schema.json");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractJobInfoResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn build_prompt(raw_text: &str) -> String {
    EXTRACT_PROMPT.replace("{{RAW_TEXT}}", raw_text)
}

/// Model generation is slower than a normal API call, hence the longer read timeout.
fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

fn map_status_error(status: u16, body: &str) -> String {
    let detail: String = redact(body).chars().take(400).collect();
    match status {
        401 | 403 => format!("E_LLM_AUTH: HTTP {status}: {detail}"),
        400 | 404 => format!("E_LLM_MODEL: HTTP {status}: {detail}"),
        _ => format!("HTTP {status}: {detail}"),
    }
}

/// Call one provider and normalize its answer. Takes the spec rather than the enum so
/// tests can retarget `base_url` at a stub server.
///
/// `pub(crate)` because mail-scan enrichment calls it too: turning job-ad text into a
/// `NewJob` partial is the same task whether the text came from a paste box or from a
/// fetched listing page, and a second copy of this would drift from the first.
pub(crate) fn extract_with_spec(
    spec: &super::provider::ProviderSpec,
    api_key: &str,
    raw_text: &str,
) -> Result<HashMap<String, Value>, String> {
    let prompt = build_prompt(raw_text);
    let schema: Option<Value> = match spec.json_mode {
        JsonMode::Schema => Some(serde_json::from_str(EXTRACT_SCHEMA).map_err(|e| e.to_string())?),
        _ => None,
    };
    // Shared transport: one retry policy, one error classification, one redaction.
    let model_text = chat_json(
        &ChatRequest {
            spec,
            api_key,
            // No system message keeps the exact wire shape this path has always sent.
            system: None,
            user: &prompt,
            schema: schema.as_ref(),
            schema_name: "job_extract",
        },
        RetryPolicy::default(),
    )
    .map_err(|e| e.to_string())?;

    let partial = parse_partial_new_job_from_llm_text(&model_text);
    if partial.is_empty() {
        return Err("Could not parse JSON from the model response.".into());
    }
    Ok(partial)
}

fn run_extract(
    app: &tauri::AppHandle,
    provider: LlmProvider,
    raw_text: &str,
) -> Result<HashMap<String, Value>, String> {
    let key = secrets::get_secret(provider.secret_provider())?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "Add an API key in Settings (Job Tracker).".to_string())?;
    let spec = super::overrides::resolved_spec(app, provider)?;
    extract_with_spec(&spec, &key, raw_text)
}

fn list_openai_models(spec: &super::provider::ProviderSpec, api_key: &str) -> Result<Vec<String>, String> {
    let client = http_client()?;
    let url = format!("{}/models", spec.base_url.trim_end_matches('/'));
    let req = match spec.auth {
        AuthStyle::Bearer => client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key.trim())),
        AuthStyle::Header(name) => client.get(&url).header(name, api_key.trim()),
    };
    let res = req.send().map_err(|e| redact(&e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.text().map_err(|e| redact(&e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(map_status_error(status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                ids.push(id.to_string());
            }
        }
    }
    Ok(ids)
}

fn list_gemini_models(spec: &super::provider::ProviderSpec, api_key: &str) -> Result<Vec<String>, String> {
    let client = http_client()?;
    let url = format!("{}/models", spec.base_url.trim_end_matches('/'));
    let auth_header = match spec.auth {
        AuthStyle::Header(name) => name,
        AuthStyle::Bearer => "x-goog-api-key",
    };
    let res = client
        .get(&url)
        .header(auth_header, api_key.trim())
        .send()
        .map_err(|e| redact(&e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.text().map_err(|e| redact(&e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(map_status_error(status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    if let Some(arr) = v.get("models").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(name) = item.get("name").and_then(|x| x.as_str()) {
                // Gemini returns "models/gemini-2.0-flash"
                let short = name.strip_prefix("models/").unwrap_or(name);
                ids.push(short.to_string());
            }
        }
    }
    Ok(ids)
}

fn test_connection_with_spec(
    spec: &super::provider::ProviderSpec,
    api_key: &str,
) -> Result<(String, String), String> {
    let ids = match spec.json_mode {
        JsonMode::ResponseMimeType => list_gemini_models(spec, api_key)?,
        JsonMode::Schema | JsonMode::JsonObject => list_openai_models(spec, api_key)?,
    };
    let configured = spec.model_id.clone();
    let detail = if ids.iter().any(|id| id == &configured || id.ends_with(&configured)) {
        format!("OK — model `{configured}` is listed by the provider.")
    } else if ids.is_empty() {
        format!("Catalogue reachable but empty; configured model is `{configured}`.")
    } else {
        format!(
            "Catalogue reachable ({n} models); `{configured}` not listed — check the model id.",
            n = ids.len()
        )
    };
    Ok((configured, detail))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTestConnectionResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub fn llm_test_connection(app: tauri::AppHandle, provider: String) -> LlmTestConnectionResponse {
    let provider = match LlmProvider::parse(&provider) {
        Ok(p) => p,
        Err(e) => {
            return LlmTestConnectionResponse {
                ok: false,
                model_id: None,
                detail: None,
                error: Some(e),
            };
        }
    };
    let key = match secrets::get_secret(provider.secret_provider()) {
        Ok(Some(k)) if !k.trim().is_empty() => k,
        Ok(_) => {
            return LlmTestConnectionResponse {
                ok: false,
                model_id: None,
                detail: None,
                error: Some("Add an API key in Settings (Job Tracker).".into()),
            };
        }
        Err(e) => {
            return LlmTestConnectionResponse {
                ok: false,
                model_id: None,
                detail: None,
                error: Some(redact(&e)),
            };
        }
    };
    let spec = match super::overrides::resolved_spec(&app, provider) {
        Ok(s) => s,
        Err(e) => {
            return LlmTestConnectionResponse {
                ok: false,
                model_id: None,
                detail: None,
                error: Some(redact(&e)),
            };
        }
    };
    match test_connection_with_spec(&spec, &key) {
        Ok((model_id, detail)) => LlmTestConnectionResponse {
            ok: true,
            model_id: Some(model_id),
            detail: Some(detail),
            error: None,
        },
        Err(e) => LlmTestConnectionResponse {
            ok: false,
            model_id: Some(spec.model_id),
            detail: None,
            error: Some(redact(&e)),
        },
    }
}

#[tauri::command]
pub fn extract_job_info(
    app: tauri::AppHandle,
    raw_text: String,
    provider: String,
) -> ExtractJobInfoResponse {
    if let Some(early) = reject_empty_raw_text(&raw_text) {
        return early;
    }
    let provider = match LlmProvider::parse(&provider) {
        Ok(p) => p,
        Err(e) => {
            return ExtractJobInfoResponse {
                ok: false,
                partial: None,
                error: Some(e),
            };
        }
    };
    match run_extract(&app, provider, &raw_text) {
        Ok(partial) => ExtractJobInfoResponse {
            ok: true,
            partial: Some(partial),
            error: None,
        },
        Err(e) => ExtractJobInfoResponse {
            ok: false,
            partial: None,
            error: Some(redact(&e)),
        },
    }
}

fn reject_empty_raw_text(raw_text: &str) -> Option<ExtractJobInfoResponse> {
    if raw_text.trim().is_empty() {
        Some(ExtractJobInfoResponse {
            ok: false,
            partial: None,
            error: Some("Paste job ad text before extracting.".into()),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use super::super::provider::provider_spec;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    /// Serve one canned JSON response and hand the raw request back to the test.
    fn stub_provider(status_line: &str, body: &str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (base_url, rx)
    }

    /// An OpenAI-shaped success wrapping `content` as the model's reply.
    fn chat_completion(content: &str) -> String {
        json!({ "choices": [{ "message": { "content": content } }] }).to_string()
    }

    #[test]
    fn prompt_includes_raw_text() {
        let p = build_prompt("Hello job");
        assert!(p.contains("Hello job"));
        assert!(!p.contains("{{RAW_TEXT}}"));
    }

    #[test]
    fn extract_schema_asset_is_valid_json() {
        serde_json::from_str::<Value>(EXTRACT_SCHEMA).expect("extract.schema.json must parse");
    }

    #[test]
    fn empty_text_error_shape() {
        let r = reject_empty_raw_text("").expect("empty input should be rejected");
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("Paste job ad"));
        assert!(reject_empty_raw_text("  hello  ").is_none());
    }

    #[test]
    fn test_connection_reports_listed_model() {
        let body = json!({ "data": [{ "id": "deepseek-v4-flash-0731" }, { "id": "other" }] }).to_string();
        let (base_url, rx) = stub_provider("200 OK", &body);
        let spec = provider_spec(LlmProvider::ScalewayDeepseek).with_base_url(&base_url);

        let (model, detail) = test_connection_with_spec(&spec, "sk").unwrap();
        let req = rx.recv().unwrap();

        assert!(req.starts_with("GET /models"), "{req}");
        assert!(req.contains("authorization: Bearer sk"), "{req}");
        assert_eq!(model, "deepseek-v4-flash-0731");
        assert!(detail.contains("listed"), "{detail}");
    }

    #[test]
    fn test_connection_warns_when_model_missing_from_catalogue() {
        let body = json!({ "data": [{ "id": "totally-different-model" }] }).to_string();
        let (base_url, _rx) = stub_provider("200 OK", &body);
        let spec = provider_spec(LlmProvider::Mistral)
            .with_base_url(&base_url)
            .with_model_id("mistral-small-latest");

        let (_model, detail) = test_connection_with_spec(&spec, "sk").unwrap();
        assert!(detail.contains("not listed"), "{detail}");
    }

    #[test]
    fn scaleway_sends_bearer_auth_and_json_schema_mode() {
        let (base_url, rx) = stub_provider("200 OK", &chat_completion(r#"{"company":"Acme"}"#));
        let spec = provider_spec(LlmProvider::ScalewayDeepseek).with_base_url(&base_url);

        let out = extract_with_spec(&spec, "sk-secret", "an ad").unwrap();
        let req = rx.recv().unwrap();

        assert!(req.starts_with("POST /chat/completions"), "{req}");
        // reqwest emits header names lowercased.
        assert!(req.contains("authorization: Bearer sk-secret"), "{req}");
        assert!(req.contains("\"type\":\"json_schema\""), "{req}");
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }

    #[test]
    fn mistral_keeps_json_object_mode() {
        let (base_url, rx) = stub_provider("200 OK", &chat_completion(r#"{"company":"Acme"}"#));
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        extract_with_spec(&spec, "sk-secret", "an ad").unwrap();
        let req = rx.recv().unwrap();

        assert!(req.contains("\"type\":\"json_object\""), "{req}");
        assert!(!req.contains("json_schema"), "{req}");
    }

    #[test]
    fn gemini_uses_goog_api_key_header_and_generate_content() {
        let body = json!({
            "candidates": [{ "content": { "parts": [{ "text": r#"{"company":"Acme"}"# }] } }]
        })
        .to_string();
        let (base_url, rx) = stub_provider("200 OK", &body);
        let spec = provider_spec(LlmProvider::Gemini).with_base_url(&base_url);

        let out = extract_with_spec(&spec, "goog-secret", "an ad").unwrap();
        let req = rx.recv().unwrap();

        assert!(req.contains(":generateContent"), "{req}");
        assert!(req.contains("x-goog-api-key: goog-secret"), "{req}");
        assert!(!req.contains("authorization:"), "{req}");
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }

    #[test]
    fn fenced_json_content_still_parses() {
        let content = "```json\n{\"company\":\"Acme\"}\n```";
        let (base_url, _rx) = stub_provider("200 OK", &chat_completion(content));
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let out = extract_with_spec(&spec, "k", "an ad").unwrap();
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }

    #[test]
    fn unauthorized_maps_to_auth_error() {
        let (base_url, _rx) = stub_provider("401 Unauthorized", r#"{"error":"bad api key"}"#);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let err = extract_with_spec(&spec, "k", "an ad").unwrap_err();
        assert!(err.starts_with("E_LLM_AUTH"), "{err}");
    }

    #[test]
    fn unknown_model_maps_to_model_error() {
        let (base_url, _rx) = stub_provider("404 Not Found", r#"{"error":"no such model"}"#);
        let spec = provider_spec(LlmProvider::ScalewayDeepseek).with_base_url(&base_url);

        let err = extract_with_spec(&spec, "k", "an ad").unwrap_err();
        assert!(err.starts_with("E_LLM_MODEL"), "{err}");
    }

    #[test]
    fn priority_from_model_output_is_dropped() {
        let content = r#"{"company":"Acme","priority":3}"#;
        let (base_url, _rx) = stub_provider("200 OK", &chat_completion(content));
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let out = extract_with_spec(&spec, "k", "an ad").unwrap();
        assert!(!out.contains_key("priority"));
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }
}
