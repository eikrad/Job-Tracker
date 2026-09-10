//! One chat round-trip against a provider, with classified errors and retry (spec §8.3).
//!
//! Both job-form extraction and mail-scan scoring go through here so retry policy,
//! error classification, and redaction cannot drift between the two callers.

use super::provider::{AuthStyle, JsonMode, ProviderSpec};
use crate::secrets::redact;
use serde_json::{json, Value};
use std::time::Duration;

/// Classified provider failure. The distinction is what the circuit breaker and the
/// retry loop key off: a bad key must fail the run immediately, a 503 must not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// 401/403 — bad or revoked key. Never retried.
    Auth(String),
    /// 400/404 — bad model id or malformed request. Never retried.
    Model(String),
    /// 429/5xx/timeout — worth retrying, and what trips the breaker when persistent.
    Transient(String),
    /// Anything else, including unparseable responses. Not retried.
    Other(String),
}

impl LlmError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth(_) => "E_LLM_AUTH",
            Self::Model(_) => "E_LLM_MODEL",
            Self::Transient(_) => "E_LLM_UNAVAILABLE",
            Self::Other(_) => "E_LLM",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Auth(m) | Self::Model(m) | Self::Transient(m) | Self::Other(m) => m,
        }
    }

    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Transient(_))
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

/// Retry schedule. `base_delay` is zero in tests so the suite does not sleep.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub base_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            attempts: 3,
            base_delay: Duration::from_millis(500),
        }
    }
}

impl RetryPolicy {
    /// No waiting between attempts, so the suite does not sleep.
    #[cfg(test)]
    pub fn immediate(attempts: u32) -> Self {
        Self {
            attempts,
            base_delay: Duration::ZERO,
        }
    }
}

pub struct ChatRequest<'a> {
    pub spec: &'a ProviderSpec,
    pub api_key: &'a str,
    /// Framing rule for hostile input (spec §6.2). `None` keeps the legacy
    /// single-user-message shape used by job-form extraction.
    pub system: Option<&'a str>,
    pub user: &'a str,
    pub schema: Option<&'a Value>,
    pub schema_name: &'a str,
}

fn http_client() -> Result<reqwest::blocking::Client, LlmError> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| LlmError::Other(redact(&e.to_string())))
}

fn classify(status: u16, body: &str) -> LlmError {
    let detail: String = redact(body).chars().take(400).collect();
    let msg = format!("HTTP {status}: {detail}");
    match status {
        401 | 403 => LlmError::Auth(msg),
        400 | 404 => LlmError::Model(msg),
        429 => LlmError::Transient(msg),
        s if (500..600).contains(&s) => LlmError::Transient(msg),
        _ => LlmError::Other(msg),
    }
}

/// `Retry-After` in seconds, or as an HTTP date we deliberately do not parse —
/// falling back to the computed backoff is safe, honouring a bogus date is not.
fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

fn backoff_delay(base: Duration, attempt: u32) -> Duration {
    if base.is_zero() {
        return Duration::ZERO;
    }
    use rand::RngExt;
    let mut rng = rand::rng();
    let factor = 1u32 << attempt.min(6);
    let scaled = base.saturating_mul(factor);
    // Full jitter: pick uniformly in [0, scaled] so retries from a stalled batch
    // do not resynchronise into a thundering herd.
    let jitter: f64 = rng.random::<f64>();
    scaled.mul_f64(jitter.clamp(0.05, 1.0))
}

fn build_messages(system: Option<&str>, user: &str) -> Value {
    match system {
        Some(sys) => json!([
            { "role": "system", "content": sys },
            { "role": "user", "content": user },
        ]),
        None => json!([{ "role": "user", "content": user }]),
    }
}

fn openai_body(req: &ChatRequest<'_>) -> Value {
    let mut body = json!({
        "model": req.spec.model_id,
        "messages": build_messages(req.system, req.user),
        "temperature": 0.0,
    });
    match (req.spec.json_mode, req.schema) {
        (JsonMode::Schema, Some(schema)) => {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": { "name": req.schema_name, "schema": schema, "strict": false }
            });
        }
        (JsonMode::Schema, None) | (JsonMode::JsonObject, _) => {
            body["response_format"] = json!({ "type": "json_object" });
        }
        (JsonMode::ResponseMimeType, _) => unreachable!("gemini takes the other path"),
    }
    if let Some(effort) = req.spec.reasoning_effort {
        body["reasoning_effort"] = json!(effort);
    }
    body
}

fn gemini_body(req: &ChatRequest<'_>) -> Value {
    let mut body = json!({
        "contents": [{ "parts": [{ "text": req.user }] }],
        "generationConfig": { "responseMimeType": "application/json", "temperature": 0.0 }
    });
    if let Some(sys) = req.system {
        body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }
    body
}

fn send_once(req: &ChatRequest<'_>) -> Result<String, LlmError> {
    let client = http_client()?;
    let base = req.spec.base_url.trim_end_matches('/');

    let (builder, body, pointer) = match req.spec.json_mode {
        JsonMode::ResponseMimeType => {
            let url = format!("{base}/models/{}:generateContent", req.spec.model_id);
            let header = match req.spec.auth {
                AuthStyle::Header(name) => name,
                AuthStyle::Bearer => "x-goog-api-key",
            };
            (
                client.post(&url).header(header, req.api_key.trim()),
                gemini_body(req),
                "/candidates/0/content/parts/0/text",
            )
        }
        JsonMode::Schema | JsonMode::JsonObject => {
            let url = format!("{base}/chat/completions");
            let b = match req.spec.auth {
                AuthStyle::Bearer => client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", req.api_key.trim())),
                AuthStyle::Header(name) => client.post(&url).header(name, req.api_key.trim()),
            };
            (b, openai_body(req), "/choices/0/message/content")
        }
    };

    let res = builder
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| {
            let msg = redact(&e.to_string());
            // A connect/read timeout is exactly the case the breaker exists for.
            if e.is_timeout() || e.is_connect() || e.is_request() {
                LlmError::Transient(msg)
            } else {
                LlmError::Other(msg)
            }
        })?;

    let status = res.status().as_u16();
    let headers = res.headers().clone();
    let text = res
        .text()
        .map_err(|e| LlmError::Transient(redact(&e.to_string())))?;

    if !(200..300).contains(&status) {
        let mut err = classify(status, &text);
        if let (LlmError::Transient(_), Some(wait)) = (&err, retry_after(&headers)) {
            // Smuggle the server's own pacing through so the retry loop can honour it.
            err = LlmError::Transient(format!("retry-after={}s {}", wait.as_secs(), {
                let LlmError::Transient(m) = &err else {
                    unreachable!()
                };
                m.clone()
            }));
        }
        return Err(err);
    }

    let v: Value = serde_json::from_str(&text)
        .map_err(|e| LlmError::Other(format!("Provider returned invalid JSON: {e}")))?;
    let content = extract_message_content(&v, pointer)
        .ok_or_else(|| LlmError::Other("Provider returned no message content.".into()))?;
    if content.trim().is_empty() {
        return Err(LlmError::Other("Provider returned empty message content.".into()));
    }
    Ok(content)
}

/// Read OpenAI `choices[0].message.content` whether it is a string or a parts array.
fn extract_message_content(v: &Value, pointer: &str) -> Option<String> {
    let node = v.pointer(pointer)?;
    if let Some(s) = node.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = node.as_array() {
        let joined: String = arr
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(|t| t.as_str())
                    .or_else(|| part.as_str())
            })
            .collect::<Vec<_>>()
            .join("");
        if !joined.is_empty() {
            return Some(joined);
        }
    }
    None
}

fn parsed_retry_after(err: &LlmError) -> Option<Duration> {
    let msg = err.message();
    let rest = msg.strip_prefix("retry-after=")?;
    let secs = rest.split('s').next()?.parse::<u64>().ok()?;
    Some(Duration::from_secs(secs))
}

/// Send with retry. Only [`LlmError::Transient`] is retried — a bad key or a bad model
/// id fails immediately rather than burning three round-trips to say the same thing.
pub fn chat_json(req: &ChatRequest<'_>, policy: RetryPolicy) -> Result<String, LlmError> {
    let mut last = LlmError::Other("no attempt made".into());
    for attempt in 0..policy.attempts.max(1) {
        match send_once(req) {
            Ok(text) => return Ok(text),
            Err(e) if e.is_transient() => {
                last = e;
                if attempt + 1 < policy.attempts.max(1) {
                    let wait = parsed_retry_after(&last)
                        .unwrap_or_else(|| backoff_delay(policy.base_delay, attempt));
                    if !wait.is_zero() {
                        std::thread::sleep(wait);
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{provider_spec, LlmProvider};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    /// Serve a scripted sequence of responses, one per connection, and report how
    /// many requests actually arrived.
    fn stub_sequence(responses: Vec<(&'static str, String)>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = hits.clone();
        thread::spawn(move || {
            for (status_line, body) in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                hits2.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 16384];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (base_url, hits)
    }

    fn stub_capture(status_line: &str, body: &str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 16384];
                let n = stream.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (base_url, rx)
    }

    fn completion(content: &str) -> String {
        json!({ "choices": [{ "message": { "content": content } }] }).to_string()
    }

    fn request<'a>(spec: &'a ProviderSpec, system: Option<&'a str>) -> ChatRequest<'a> {
        ChatRequest {
            spec,
            api_key: "sk-test",
            system,
            user: "listing text",
            schema: None,
            schema_name: "score",
        }
    }

    #[test]
    fn system_message_is_sent_as_its_own_role() {
        let (base_url, rx) = stub_capture("200 OK", &completion(r#"{"score":3}"#));
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        chat_json(&request(&spec, Some("data not instructions")), RetryPolicy::immediate(1)).unwrap();
        let req = rx.recv().unwrap();

        assert!(req.contains(r#""role":"system""#), "{req}");
        assert!(req.contains("data not instructions"), "{req}");
    }

    #[test]
    fn omitting_system_keeps_single_user_message() {
        let (base_url, rx) = stub_capture("200 OK", &completion(r#"{"score":3}"#));
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        chat_json(&request(&spec, None), RetryPolicy::immediate(1)).unwrap();
        let req = rx.recv().unwrap();

        assert!(!req.contains(r#""role":"system""#), "{req}");
    }

    #[test]
    fn transient_failure_is_retried_then_succeeds() {
        let (base_url, hits) = stub_sequence(vec![
            ("503 Service Unavailable", r#"{"error":"busy"}"#.to_string()),
            ("200 OK", completion(r#"{"score":8}"#)),
        ]);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let out = chat_json(&request(&spec, None), RetryPolicy::immediate(3)).unwrap();

        assert!(out.contains("\"score\":8"));
        assert_eq!(hits.load(Ordering::SeqCst), 2, "should have retried exactly once");
    }

    #[test]
    fn auth_failure_is_not_retried() {
        let (base_url, hits) = stub_sequence(vec![
            ("401 Unauthorized", r#"{"error":"bad key"}"#.to_string()),
            ("200 OK", completion(r#"{"score":8}"#)),
        ]);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let err = chat_json(&request(&spec, None), RetryPolicy::immediate(3)).unwrap_err();

        assert!(matches!(err, LlmError::Auth(_)), "{err:?}");
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "a revoked key must fail fast, not three times"
        );
    }

    #[test]
    fn bad_model_id_is_not_retried() {
        let (base_url, hits) = stub_sequence(vec![
            ("404 Not Found", r#"{"error":"no such model"}"#.to_string()),
            ("200 OK", completion(r#"{"score":8}"#)),
        ]);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let err = chat_json(&request(&spec, None), RetryPolicy::immediate(3)).unwrap_err();

        assert!(matches!(err, LlmError::Model(_)), "{err:?}");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn exhausted_retries_report_transient() {
        let (base_url, hits) = stub_sequence(vec![
            ("500 Internal Server Error", "{}".to_string()),
            ("500 Internal Server Error", "{}".to_string()),
            ("500 Internal Server Error", "{}".to_string()),
        ]);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let err = chat_json(&request(&spec, None), RetryPolicy::immediate(3)).unwrap_err();

        assert!(err.is_transient(), "{err:?}");
        assert_eq!(err.code(), "E_LLM_UNAVAILABLE");
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn schema_mode_sends_json_schema_block() {
        let (base_url, rx) = stub_capture("200 OK", &completion(r#"{"score":1}"#));
        let spec = provider_spec(LlmProvider::ScalewayDeepseek).with_base_url(&base_url);
        let schema = json!({ "type": "object" });
        let req = ChatRequest {
            spec: &spec,
            api_key: "sk",
            system: None,
            user: "x",
            schema: Some(&schema),
            schema_name: "score_pass1",
        };

        chat_json(&req, RetryPolicy::immediate(1)).unwrap();
        let sent = rx.recv().unwrap();

        assert!(sent.contains(r#""type":"json_schema""#), "{sent}");
        assert!(sent.contains(r#""name":"score_pass1""#), "{sent}");
        assert!(sent.contains(r#""temperature":0.0"#), "{sent}");
    }

    #[test]
    fn api_key_never_appears_in_a_classified_error() {
        let (base_url, _hits) = stub_sequence(vec![(
            "401 Unauthorized",
            r#"{"error":"key sk-abcdefghijklmnopqrstuvwxyz0123456789 rejected"}"#.to_string(),
        )]);
        let spec = provider_spec(LlmProvider::Mistral).with_base_url(&base_url);

        let err = chat_json(&request(&spec, None), RetryPolicy::immediate(1)).unwrap_err();

        assert!(
            !err.message().contains("sk-abcdefghijklmnopqrstuvwxyz0123456789"),
            "redaction must cover provider error bodies: {err}"
        );
    }
}
