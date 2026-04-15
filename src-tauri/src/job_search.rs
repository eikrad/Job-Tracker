use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::Manager;
use url::Url;

// ─── Public types ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeywordStat {
  pub keyword: String,
  pub count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobSearchResult {
  pub title: String,
  pub company: String,
  pub location: String,
  pub url: String,
  pub description: String,
  pub published_date: String,
  pub platform: String,
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn get_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
  let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
  // Keep this aligned with db::connection()/init_db, which uses app.db.
  Ok(data_dir.join("data").join("app.db"))
}

fn parse_tags(tags_str: &str) -> Vec<String> {
  let trimmed = tags_str.trim();
  if trimmed.is_empty() {
    return Vec::new();
  }

  // Support JSON array style tags from imports, e.g. ["rust","tauri"].
  if trimmed.starts_with('[') {
    if let Ok(values) = serde_json::from_str::<Vec<String>>(trimmed) {
      return values
        .into_iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    }
  }

  trimmed
    .split(|c: char| [',', ';', '|', '\n', '\t'].contains(&c))
    .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_lowercase())
    .filter(|part| !part.is_empty())
    .collect()
}

fn domain_for_platform(platform: &str) -> &'static str {
  match platform {
    "jobindex" => "jobindex.dk",
    "indeed" => "indeed.",
    _ => "",
  }
}

fn build_provider_query(platform: &str, keywords: &str, location: &str) -> String {
  let domain = domain_for_platform(platform);
  let mut query = keywords.to_string();
  if !location.trim().is_empty() {
    query.push(' ');
    query.push_str(location.trim());
  }
  if !domain.is_empty() {
    query.push(' ');
    query.push_str("site:");
    query.push_str(domain);
  }
  query
}

fn normalize_company(title: &str) -> (String, String) {
  let parts: Vec<&str> = title.splitn(2, " - ").collect();
  if parts.len() == 2 {
    (parts[0].trim().to_string(), parts[1].trim().to_string())
  } else {
    (title.trim().to_string(), String::new())
  }
}

fn keep_platform_result(platform: &str, url: &str) -> bool {
  let domain = domain_for_platform(platform);
  if domain.is_empty() {
    return true;
  }
  url.contains(domain)
}

fn parse_serpapi_results(payload: &str, platform: &str, location: &str) -> Result<Vec<JobSearchResult>, String> {
  let data: Value = serde_json::from_str(payload)
    .map_err(|e| format!("SerpAPI JSON parse error: {e}"))?;
  let Some(items) = data.get("organic_results").and_then(Value::as_array) else {
    return Ok(Vec::new());
  };
  let mut out = Vec::new();
  for item in items {
    let url = item.get("link").and_then(Value::as_str).unwrap_or("").to_string();
    if url.is_empty() || !keep_platform_result(platform, &url) {
      continue;
    }
    let title_raw = item.get("title").and_then(Value::as_str).unwrap_or("Untitled");
    let (title, company) = normalize_company(title_raw);
    let desc = item
      .get("snippet")
      .and_then(Value::as_str)
      .unwrap_or("")
      .chars()
      .take(400)
      .collect::<String>();
    let published = item.get("date").and_then(Value::as_str).unwrap_or("").to_string();
    out.push(JobSearchResult {
      title,
      company,
      location: location.to_string(),
      url,
      description: desc,
      published_date: published,
      platform: platform.to_string(),
    });
  }
  Ok(out)
}

fn parse_brave_results(payload: &str, platform: &str, location: &str) -> Result<Vec<JobSearchResult>, String> {
  let data: Value = serde_json::from_str(payload)
    .map_err(|e| format!("Brave JSON parse error: {e}"))?;
  let Some(items) = data
    .get("web")
    .and_then(|w| w.get("results"))
    .and_then(Value::as_array) else {
    return Ok(Vec::new());
  };
  let mut out = Vec::new();
  for item in items {
    let url = item.get("url").and_then(Value::as_str).unwrap_or("").to_string();
    if url.is_empty() || !keep_platform_result(platform, &url) {
      continue;
    }
    let title_raw = item.get("title").and_then(Value::as_str).unwrap_or("Untitled");
    let (title, company) = normalize_company(title_raw);
    let desc = item
      .get("description")
      .and_then(Value::as_str)
      .unwrap_or("")
      .chars()
      .take(400)
      .collect::<String>();
    let published = item
      .get("page_age")
      .and_then(Value::as_str)
      .unwrap_or("")
      .to_string();
    out.push(JobSearchResult {
      title,
      company,
      location: location.to_string(),
      url,
      description: desc,
      published_date: published,
      platform: platform.to_string(),
    });
  }
  Ok(out)
}

// ─── Tauri commands ────────────────────────────────────────────────────────

/// Return each unique tag from all jobs, sorted by frequency descending.
#[tauri::command]
pub fn get_keyword_stats(app: tauri::AppHandle) -> Result<Vec<KeywordStat>, String> {
  let db_path = get_db_path(&app)?;
  let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

  let mut stmt = conn
    .prepare("SELECT tags FROM jobs WHERE tags IS NOT NULL AND tags != ''")
    .map_err(|e| e.to_string())?;

  let rows: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(0))
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

  let mut counts: HashMap<String, u32> = HashMap::new();
  for tags_str in rows {
    for tag in parse_tags(&tags_str) {
      *counts.entry(tag).or_insert(0) += 1;
    }
  }

  let mut stats: Vec<KeywordStat> = counts
    .into_iter()
    .map(|(keyword, count)| KeywordStat { keyword, count })
    .collect();

  stats.sort_by(|a, b| b.count.cmp(&a.count).then(a.keyword.cmp(&b.keyword)));
  Ok(stats)
}

/// Return distinct workplace cities from the jobs table, ordered alphabetically.
#[tauri::command]
pub fn get_location_suggestions(app: tauri::AppHandle) -> Result<Vec<String>, String> {
  let db_path = get_db_path(&app)?;
  let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

  let mut stmt = conn
    .prepare(
      "SELECT workplace_city, COUNT(*) as cnt \
       FROM jobs \
       WHERE workplace_city IS NOT NULL AND workplace_city != '' \
       GROUP BY workplace_city \
       ORDER BY cnt DESC, workplace_city ASC",
    )
    .map_err(|e| e.to_string())?;

  let cities: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(0))
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

  Ok(cities)
}

/// Fetch and parse job search results using SerpAPI first, Brave Search as fallback.
/// Supported platforms: "jobindex", "indeed".
#[tauri::command]
pub fn fetch_job_search_results(
  platform: String,
  keywords: Vec<String>,
  location: Option<String>,
  region: Option<String>,
  serp_api_key: Option<String>,
  brave_search_api_key: Option<String>,
) -> Result<Vec<JobSearchResult>, String> {
  let query = keywords.join(" ");
  let loc = location.unwrap_or_default();
  match platform.as_str() {
    "jobindex" | "indeed" => {}
    other => return Err(format!("Unknown platform: {}", other)),
  }

  let client = Client::builder()
    .user_agent(
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
       (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    )
    .timeout(std::time::Duration::from_secs(15))
    .build()
    .map_err(|e| e.to_string())?;

  let provider_query = build_provider_query(&platform, &query, &loc);
  let gl = region
    .as_deref()
    .unwrap_or("dk")
    .to_lowercase();
  let brave_country = gl.to_uppercase();

  let serp_key = serp_api_key.unwrap_or_default();
  if !serp_key.trim().is_empty() {
    let response = client
      .get("https://serpapi.com/search.json")
      .query(&[
        ("engine", "google"),
        ("q", provider_query.as_str()),
        ("gl", gl.as_str()),
        ("hl", "en"),
        ("num", "20"),
        ("api_key", serp_key.trim()),
      ])
      .send()
      .map_err(|e| format!("SerpAPI request failed: {e}"))?;

    if response.status().is_success() {
      let body = response
        .text()
        .map_err(|e| format!("SerpAPI read failed: {e}"))?;
      let results = parse_serpapi_results(&body, &platform, &loc)?;
      if !results.is_empty() {
        return Ok(results);
      }
    }
  }

  let brave_key = brave_search_api_key.unwrap_or_default();
  if !brave_key.trim().is_empty() {
    let response = client
      .get("https://api.search.brave.com/res/v1/web/search")
      .header("X-Subscription-Token", brave_key.trim())
      .query(&[
        ("q", provider_query.as_str()),
        ("country", brave_country.as_str()),
        ("search_lang", "en"),
        ("count", "20"),
      ])
      .send()
      .map_err(|e| format!("Brave request failed: {e}"))?;

    if response.status().is_success() {
      let body = response
        .text()
        .map_err(|e| format!("Brave read failed: {e}"))?;
      let results = parse_brave_results(&body, &platform, &loc)?;
      if !results.is_empty() {
        return Ok(results);
      }
    }
  }

  if serp_key.trim().is_empty() && brave_key.trim().is_empty() {
    return Err("Missing search API keys. Add SerpAPI and/or Brave Search API key in Settings.".to_string());
  }

  Ok(Vec::new())
}

/// Build and return the browser search URL without opening it.
/// (Used by the frontend to show/copy the URL or as a fallback.)
#[tauri::command]
pub fn build_search_url(
  platform: String,
  keywords: Vec<String>,
  location: Option<String>,
  region: Option<String>,
) -> Result<String, String> {
  let query = keywords.join(" ");
  let loc = location.unwrap_or_default();

  let url = match platform.as_str() {
    "jobindex" => {
      let mut u = Url::parse("https://www.jobindex.dk/jobsoeg")
        .map_err(|e| e.to_string())?;
      {
        let mut p = u.query_pairs_mut();
        p.append_pair("q", &query);
        if !loc.is_empty() {
          p.append_pair("where", &loc);
        }
      }
      u.to_string()
    }
    "indeed" => {
      let domain = region.as_deref().unwrap_or("dk");
      let base_str = if domain == "com" {
        "https://www.indeed.com/jobs".to_string()
      } else {
        format!("https://{}.indeed.com/jobs", domain)
      };
      let mut u = Url::parse(&base_str).map_err(|e| e.to_string())?;
      {
        let mut p = u.query_pairs_mut();
        p.append_pair("q", &query);
        if !loc.is_empty() {
          p.append_pair("l", &loc);
        }
      }
      u.to_string()
    }
    "linkedin" => {
      let mut u =
        Url::parse("https://www.linkedin.com/jobs/search").map_err(|e| e.to_string())?;
      {
        let mut p = u.query_pairs_mut();
        p.append_pair("keywords", &query);
        if !loc.is_empty() {
          p.append_pair("location", &loc);
        }
      }
      u.to_string()
    }
    other => return Err(format!("Unknown platform: {}", other)),
  };

  Ok(url)
}

/// Open a URL in the system default browser.
#[tauri::command]
pub fn open_url_in_browser(url: String) -> Result<(), String> {
  open::that(&url).map_err(|e| format!("Could not open browser: {e}"))
}
