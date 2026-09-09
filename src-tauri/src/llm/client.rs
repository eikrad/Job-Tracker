//! HTTP calls to LLM providers using keys from the secret store.

use super::normalize::parse_partial_new_job_from_llm_text;
use super::provider::{provider_spec, AuthStyle, JsonMode, LlmProvider};
use crate::secrets::{self, redact};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const EXTRACT_PROMPT: &str = include_str!("../../prompts/extract.md");

const EXTRACT_SCHEMA: &str = r#"{
  "type": "object",
  "additionalProperties": true,
  "properties": {
    "company": { "type": "string" },
    "title": { "type": "string" },
    "url": { "type": "string" },
    "deadline": { "type": "string" },
    "interview_date": { "type": "string" },
    "start_date": { "type": "string" },
    "tags": { "type": "string" },
    "detected_language": { "type": "string" },
    "notes": { "type": "string" },
    "contact_name": { "type": "string" },
    "contact_email": { "type": "string" },
    "contact_phone": { "type": "string" },
    "workplace_street": { "type": "string" },
    "workplace_city": { "type": "string" },
    "workplace_postal_code": { "type": "string" },
    "work_mode": { "type": "string" },
    "salary_range": { "type": "string" },
    "contract_type": { "type": "string" },
    "reference_number": { "type": "string" },
    "source": { "type": "string" }
  }
}"#;

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

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

fn map_status_error(status: u16, body: &str) -> String {
    let redacted = redact(body);
    if status == 401 || status == 403 {
        return format!("E_LLM_AUTH: HTTP {status}: {}", redacted.chars().take(400).collect::<String>());
    }
    if status == 404 || status == 400 {
        return format!("E_LLM_MODEL: HTTP {status}: {}", redacted.chars().take(400).collect::<String>());
    }
    format!("HTTP {status}: {}", redacted.chars().take(400).collect::<String>())
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

    let mut req = client.post(&url).header("Content-Type", "application/json");
    match spec.auth {
        AuthStyle::Bearer => {
            req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
        }
        AuthStyle::Header(name) => {
            req = req.header(name, api_key.trim());
        }
    }

    let res = req.json(&body).send().map_err(|e| redact(&e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.text().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) {
        return Err(map_status_error(status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
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
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key.trim())
        .json(&body)
        .send()
        .map_err(|e| redact(&e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.text().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) {
        return Err(map_status_error(status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let content = v
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|c| c.as_str())
        .ok_or_else(|| {
            "Gemini returned no text (check API key, quota, or safety filters).".to_string()
        })?;
    Ok(content.to_string())
}

fn run_extract(provider: LlmProvider, raw_text: &str) -> Result<HashMap<String, Value>, String> {
    let key = secrets::get_secret(provider.secret_provider())?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "Add an API key in Settings (Job Tracker).".to_string())?;
    let prompt = build_prompt(raw_text);
    let spec = provider_spec(provider);
    let model_text = match provider {
        LlmProvider::Gemini => extract_gemini(&spec, &key, &prompt)?,
        LlmProvider::Mistral | LlmProvider::ScalewayDeepseek => {
            extract_openai_compatible(&spec, &key, &prompt)?
        }
    };
    let partial = parse_partial_new_job_from_llm_text(&model_text);
    if partial.is_empty() {
        return Err("Could not parse JSON from the model response.".into());
    }
    Ok(partial)
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

    #[test]
    fn prompt_includes_raw_text() {
        let p = build_prompt("Hello job");
        assert!(p.contains("Hello job"));
        assert!(!p.contains("{{RAW_TEXT}}"));
    }

    #[test]
    fn empty_text_error_shape() {
        let r = extract_job_info(String::new(), "mistral".into());
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("Paste job ad"));
    }
}
