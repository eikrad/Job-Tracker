//! HTTP calls to LLM providers using keys from the secret store.

use super::normalize::parse_partial_new_job_from_llm_text;
use super::provider::{provider_spec, AuthStyle, JsonMode, LlmProvider};
use crate::secrets::{self, redact};
use serde::Serialize;
use serde_json::{json, Value};
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

/// Send a prepared request and return the response body, mapping non-2xx to a
/// classified, redacted error. Shared by every provider path.
fn send_for_body(req: reqwest::blocking::RequestBuilder, body: &Value) -> Result<Value, String> {
    let res = req
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .map_err(|e| redact(&e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.text().map_err(|e| redact(&e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(map_status_error(status, &text));
    }
    serde_json::from_str(&text).map_err(|e| format!("Provider returned invalid JSON: {e}"))
}

fn extract_openai_compatible(
    spec: &super::provider::ProviderSpec,
    api_key: &str,
    prompt: &str,
) -> Result<String, String> {
    let client = http_client()?;
    let url = format!("{}/chat/completions", spec.base_url.trim_end_matches('/'));

    let mut body = json!({
        "model": spec.model_id,
        "messages": [{ "role": "user", "content": prompt }],
        "temperature": 0.0,
    });

    match spec.json_mode {
        JsonMode::Schema => {
            let schema: Value = serde_json::from_str(EXTRACT_SCHEMA).map_err(|e| e.to_string())?;
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "job_extract",
                    "schema": schema,
                    "strict": false
                }
            });
        }
        JsonMode::JsonObject => {
            body["response_format"] = json!({ "type": "json_object" });
        }
        JsonMode::ResponseMimeType => unreachable!("openai path"),
    }

    let req = match spec.auth {
        AuthStyle::Bearer => client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key.trim())),
        AuthStyle::Header(name) => client.post(&url).header(name, api_key.trim()),
    };

    let v = send_for_body(req, &body)?;
    let content = v
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "Provider returned no message content.".to_string())?;
    if content.trim().is_empty() {
        return Err("Provider returned empty message content.".into());
    }
    Ok(content.to_string())
}

fn extract_gemini(spec: &super::provider::ProviderSpec, api_key: &str, prompt: &str) -> Result<String, String> {
    let client = http_client()?;
    let url = format!(
        "{}/models/{}:generateContent",
        spec.base_url.trim_end_matches('/'),
        spec.model_id
    );
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "responseMimeType": "application/json" }
    });
    let auth_header = match spec.auth {
        AuthStyle::Header(name) => name,
        AuthStyle::Bearer => "x-goog-api-key",
    };
    let req = client.post(&url).header(auth_header, api_key.trim());

    let v = send_for_body(req, &body)?;
    let content = v
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|c| c.as_str())
        .ok_or_else(|| {
            "Gemini returned no text (check API key, quota, or safety filters).".to_string()
        })?;
    Ok(content.to_string())
}

/// Call one provider and normalize its answer. Takes the spec rather than the enum so
/// tests can retarget `base_url` at a stub server.
fn extract_with_spec(
    spec: &super::provider::ProviderSpec,
    api_key: &str,
    raw_text: &str,
) -> Result<HashMap<String, Value>, String> {
    let prompt = build_prompt(raw_text);
    // The wire shape follows from the spec, not from the provider identity.
    let model_text = match spec.json_mode {
        JsonMode::ResponseMimeType => extract_gemini(spec, api_key, &prompt)?,
        JsonMode::Schema | JsonMode::JsonObject => {
            extract_openai_compatible(spec, api_key, &prompt)?
        }
    };
    let partial = parse_partial_new_job_from_llm_text(&model_text);
    if partial.is_empty() {
        return Err("Could not parse JSON from the model response.".into());
    }
    Ok(partial)
}

fn run_extract(provider: LlmProvider, raw_text: &str) -> Result<HashMap<String, Value>, String> {
    let key = secrets::get_secret(provider.secret_provider())?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "Add an API key in Settings (Job Tracker).".to_string())?;
    extract_with_spec(&provider_spec(provider), &key, raw_text)
}

#[tauri::command]
pub fn extract_job_info(raw_text: String, provider: String) -> ExtractJobInfoResponse {
    if raw_text.trim().is_empty() {
        return ExtractJobInfoResponse {
            ok: false,
            partial: None,
            error: Some("Paste job ad text before extracting.".into()),
        };
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
    match run_extract(provider, &raw_text) {
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

#[cfg(test)]
mod tests {
    use super::*;
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
        let r = extract_job_info(String::new(), "mistral".into());
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("Paste job ad"));
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
