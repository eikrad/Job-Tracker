//! Google OAuth 2.0 (PKCE, loopback redirect) + refresh token in OS keyring.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;
use url::Url;

const KEYRING_SERVICE: &str = "JobTracker-GoogleCalendar";
const KEYRING_USER: &str = "oauth_refresh_token";
const GOOGLE_AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

fn app_data_base(app: &AppHandle) -> Result<PathBuf, String> {
  app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))
}

fn client_id_path(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(app_data_base(app)?.join("google_oauth_client_id.txt"))
}

pub fn read_client_id(app: &AppHandle) -> Result<String, String> {
  let path = client_id_path(app)?;
  let s = std::fs::read_to_string(&path).map_err(|e| format!("No Google Client ID saved: {e}"))?;
  let t = s.trim().to_string();
  if t.is_empty() {
    return Err("Google OAuth Client ID is empty. Enter it in Settings.".to_string());
  }
  Ok(t)
}

pub fn write_client_id(app: &AppHandle, client_id: String) -> Result<(), String> {
  let base = app_data_base(app)?;
  std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
  let path = client_id_path(app)?;
  std::fs::write(&path, client_id.trim()).map_err(|e| e.to_string())
}

fn keyring_entry() -> Result<keyring::Entry, String> {
  keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())
}

pub fn has_refresh_token() -> bool {
  keyring_entry()
    .map(|e| e.get_password().map(|p| !p.is_empty()).unwrap_or(false))
    .unwrap_or(false)
}

pub fn store_refresh_token(token: &str) -> Result<(), String> {
  let e = keyring_entry()?;
  e.set_password(token).map_err(|e| e.to_string())
}

pub fn delete_refresh_token() -> Result<(), String> {
  let e = keyring_entry()?;
  let _ = e.delete_credential();
  Ok(())
}

fn gen_code_verifier() -> String {
  const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
  let mut rng = rand::thread_rng();
  (0..64)
    .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
    .collect()
}

fn pkce_challenge(verifier: &str) -> String {
  let hash = Sha256::digest(verifier.as_bytes());
  URL_SAFE_NO_PAD.encode(hash)
}

fn random_state() -> String {
  let mut rng = rand::thread_rng();
  (0..32)
    .map(|_| format!("{:02x}", rng.gen::<u8>()))
    .collect()
}

fn parse_oauth_redirect(buf: &[u8], expected_state: &str) -> Result<String, String> {
  let s = std::str::from_utf8(buf).map_err(|_| "Invalid HTTP request".to_string())?;
  let line = s.lines().next().ok_or_else(|| "Empty HTTP request".to_string())?;
  let parts: Vec<&str> = line.split_whitespace().collect();
  if parts.len() < 2 {
    return Err("Malformed HTTP request line".to_string());
  }
  let path = parts[1];
  let fake = format!("http://127.0.0.1{path}");
  let u = Url::parse(&fake).map_err(|e| format!("Bad redirect URL: {e}"))?;
  let qp: HashMap<String, String> = u.query_pairs().into_owned().collect();
  if let Some(err) = qp.get("error") {
    return Err(format!("Google OAuth error: {err}"));
  }
  let state = qp
    .get("state")
    .ok_or_else(|| "Missing state in redirect".to_string())?;
  if state != expected_state {
    return Err("OAuth state mismatch".to_string());
  }
  qp.get("code")
    .cloned()
    .ok_or_else(|| "Missing authorization code".to_string())
}

fn http_ok_response(stream: &mut TcpStream) -> Result<(), String> {
  let body = b"<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Job Tracker</title></head><body><p>Authorization complete. You can close this window and return to Job Tracker.</p></body></html>";
  let head = format!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    body.len()
  );
  stream
    .write_all(head.as_bytes())
    .and_then(|_| stream.write_all(body))
    .map_err(|e| e.to_string())
}

fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
  let mut buf = Vec::new();
  let mut chunk = [0u8; 1024];
  loop {
    let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
    if n == 0 {
      break;
    }
    buf.extend_from_slice(&chunk[..n]);
    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
      break;
    }
    if buf.len() > 16_384 {
      return Err("Request too large".to_string());
    }
  }
  Ok(buf)
}

/// Exchange authorization code for tokens (blocking).
fn exchange_code_for_tokens(
  client_id: &str,
  redirect_uri: &str,
  code: &str,
  code_verifier: &str,
) -> Result<String, String> {
  let client = reqwest::blocking::Client::new();
  let params = [
    ("client_id", client_id),
    ("code", code),
    ("code_verifier", code_verifier),
    ("grant_type", "authorization_code"),
    ("redirect_uri", redirect_uri),
  ];
  let res = client
    .post(GOOGLE_TOKEN)
    .form(&params)
    .send()
    .map_err(|e| format!("Token request failed: {e}"))?;
  if !res.status().is_success() {
    let t = res.text().unwrap_or_default();
    return Err(format!("Token exchange failed: {t}"));
  }
  let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
  let refresh = v
    .get("refresh_token")
    .and_then(|x| x.as_str())
    .ok_or_else(|| {
      "No refresh_token in response. Try revoking app access in Google Account and connect again with prompt=consent."
        .to_string()
    })?;
  Ok(refresh.to_string())
}

/// Obtain a fresh access token using refresh_token (blocking).
pub fn access_token_from_refresh(client_id: &str, refresh_token: &str) -> Result<String, String> {
  let client = reqwest::blocking::Client::new();
  let params = [
    ("client_id", client_id),
    ("grant_type", "refresh_token"),
    ("refresh_token", refresh_token),
  ];
  let res = client
    .post(GOOGLE_TOKEN)
    .form(&params)
    .send()
    .map_err(|e| format!("Refresh request failed: {e}"))?;
  if !res.status().is_success() {
    let t = res.text().unwrap_or_default();
    return Err(format!("Token refresh failed: {t}"));
  }
  let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
  v.get("access_token")
    .and_then(|x| x.as_str())
    .map(|s| s.to_string())
    .ok_or_else(|| "No access_token in refresh response".to_string())
}

/// Resolve Bearer access token: manual override, or OAuth refresh from keyring.
pub fn resolve_calendar_access_token(app: &AppHandle, override_token: Option<&str>) -> Result<String, String> {
  if let Some(t) = override_token {
    let t = t.trim();
    if !t.is_empty() {
      return Ok(t.to_string());
    }
  }
  let client_id = read_client_id(app)?;
  let entry = keyring_entry()?;
  let refresh = entry
    .get_password()
    .map_err(|_| "Not signed in to Google. Use Settings → Connect Google, or paste an access token (Advanced).".to_string())?;
  if refresh.trim().is_empty() {
    return Err("Not signed in to Google.".to_string());
  }
  access_token_from_refresh(&client_id, refresh.trim())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleOauthStatus {
  pub connected: bool,
}

#[tauri::command]
pub fn google_oauth_get_client_id(app: AppHandle) -> Result<String, String> {
  let path = client_id_path(&app)?;
  match std::fs::read_to_string(&path) {
    Ok(s) => Ok(s.trim().to_string()),
    Err(_) => Ok(String::new()),
  }
}

#[tauri::command]
pub fn google_oauth_set_client_id(app: AppHandle, client_id: String) -> Result<(), String> {
  write_client_id(&app, client_id)
}

#[tauri::command]
pub fn google_oauth_status() -> Result<GoogleOauthStatus, String> {
  Ok(GoogleOauthStatus {
    connected: has_refresh_token(),
  })
}

#[tauri::command]
pub fn google_oauth_disconnect() -> Result<(), String> {
  delete_refresh_token()
}

#[tauri::command]
pub fn google_oauth_connect(app: AppHandle) -> Result<(), String> {
  let client_id = read_client_id(&app)?;
  let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Cannot bind loopback: {e}"))?;
  let port = listener
    .local_addr()
    .map_err(|e| e.to_string())?
    .port();
  let redirect_uri = format!("http://127.0.0.1:{port}/");
  let verifier = gen_code_verifier();
  let challenge = pkce_challenge(&verifier);
  let state = random_state();

  let mut auth_url = Url::parse(GOOGLE_AUTH).map_err(|e| e.to_string())?;
  auth_url
    .query_pairs_mut()
    .append_pair("client_id", &client_id)
    .append_pair("redirect_uri", &redirect_uri)
    .append_pair("response_type", "code")
    .append_pair("scope", CALENDAR_SCOPE)
    .append_pair("state", &state)
    .append_pair("code_challenge", &challenge)
    .append_pair("code_challenge_method", "S256")
    .append_pair("access_type", "offline")
    .append_pair("prompt", "consent");

  let url_str = auth_url.to_string();
  open::that(&url_str).map_err(|e| format!("Could not open browser: {e}"))?;

  listener
    .set_nonblocking(false)
    .map_err(|e| e.to_string())?;

  let (tx, rx) = mpsc::channel::<Result<String, String>>();
  let state_clone = state.clone();
  let handle = std::thread::spawn(move || {
    let result = (|| -> Result<String, String> {
      let (mut stream, _) = listener.accept().map_err(|e| format!("Accept failed: {e}"))?;
      let buf = read_http_request(&mut stream)?;
      let code = parse_oauth_redirect(&buf, &state_clone)?;
      let _ = http_ok_response(&mut stream);
      Ok(code)
    })();
    let _ = tx.send(result);
  });

  let code = match rx.recv_timeout(Duration::from_secs(300)) {
    Ok(r) => r?,
    Err(_) => {
      return Err("Timed out waiting for Google redirect (5 min).".to_string());
    }
  };
  let _ = handle.join();

  let refresh = exchange_code_for_tokens(&client_id, &redirect_uri, &code, &verifier)?;
  store_refresh_token(&refresh)?;
  Ok(())
}
