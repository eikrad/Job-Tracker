use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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

/// Strip HTML tags and decode common entities.
fn strip_html(html: &str) -> String {
  let mut result = String::with_capacity(html.len());
  let mut in_tag = false;
  for c in html.chars() {
    match c {
      '<' => in_tag = true,
      '>' => {
        in_tag = false;
        result.push(' ');
      }
      _ if !in_tag => result.push(c),
      _ => {}
    }
  }
  let collapsed = result.split_whitespace().collect::<Vec<_>>().join(" ");
  collapsed
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#39;", "'")
    .replace("&nbsp;", " ")
    .replace("&#8203;", "")
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

/// Parse an RSS/Atom XML string into a list of search results.
fn parse_rss(xml: &str, platform: &str) -> Result<Vec<JobSearchResult>, String> {
  let doc =
    roxmltree::Document::parse(xml).map_err(|e| format!("XML parse error: {e}"))?;

  let mut results = Vec::new();

  for item in doc
    .descendants()
    .filter(|n| n.tag_name().name() == "item")
  {
    let mut title = String::new();
    let mut link = String::new();
    let mut description = String::new();
    let mut pub_date = String::new();
    let mut company = String::new();
    let mut location = String::new();

    for child in item.children().filter(|n| n.is_element()) {
      let tag = child.tag_name().name();
      let text = child.text().unwrap_or("").trim().to_string();

      match tag {
        "title" => title = text,
        "link" => {
          if link.is_empty() {
            link = text;
          }
        }
        "description" => description = strip_html(&text),
        "pubDate" => pub_date = text,
        // Company / author variations
        "company" | "creator" => {
          if company.is_empty() {
            company = text;
          }
        }
        "location" => {
          if location.is_empty() {
            location = text;
          }
        }
        // guid is a reliable fallback URL
        "guid" => {
          if link.is_empty() && text.starts_with("http") {
            link = text;
          }
        }
        _ => {}
      }
    }

    // Try to parse company / location from the title string.
    // Jobindex uses "Title hos Company" or "Title - Company".
    // Indeed uses  "Title - Company - City, Country".
    if company.is_empty() && !title.is_empty() {
      if let Some((left, right)) = title.split_once(" hos ") {
        company = right.trim().to_string();
        title = left.trim().to_string();
      } else {
        let title_for_split = title.clone();
        let parts: Vec<&str> = title_for_split.splitn(3, " - ").collect();
        match parts.len() {
          2 => {
            company = parts[1].trim().to_string();
            title = parts[0].trim().to_string();
          }
          3 => {
            title = parts[0].trim().to_string();
            company = parts[1].trim().to_string();
            if location.is_empty() {
              location = parts[2].trim().to_string();
            }
          }
          _ => {}
        }
      }
    }

    if !link.is_empty() {
      results.push(JobSearchResult {
        title: if title.is_empty() {
          "Untitled".to_string()
        } else {
          title
        },
        company,
        location,
        url: link,
        description: description.chars().take(400).collect(),
        published_date: pub_date,
        platform: platform.to_string(),
      });
    }
  }

  Ok(results)
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

/// Fetch and parse an RSS job feed for the given platform.
/// Supported platforms: "jobindex", "indeed".
#[tauri::command]
pub fn fetch_job_search_rss(
  platform: String,
  keywords: Vec<String>,
  location: Option<String>,
  region: Option<String>,
) -> Result<Vec<JobSearchResult>, String> {
  let query = keywords.join(" ");
  let loc = location.unwrap_or_default();

  let feed_url: String = match platform.as_str() {
    "jobindex" => {
      let mut url = Url::parse("https://www.jobindex.dk/jobsoeg/rss")
        .map_err(|e| e.to_string())?;
      {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("q", &query);
        if !loc.is_empty() {
          pairs.append_pair("where", &loc);
        }
        pairs.append_pair("jobnr", "");
        pairs.append_pair("supid", "0");
        pairs.append_pair("lang", "");
        pairs.append_pair("period", "0");
      }
      url.into()
    }
    "indeed" => {
      let domain = region.as_deref().unwrap_or("dk");
      let base_str = if domain == "com" {
        "https://www.indeed.com/rss".to_string()
      } else {
        format!("https://{}.indeed.com/rss", domain)
      };
      let mut url = Url::parse(&base_str).map_err(|e| e.to_string())?;
      {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("q", &query);
        if !loc.is_empty() {
          pairs.append_pair("l", &loc);
        }
      }
      url.into()
    }
    other => return Err(format!("Unknown platform: {}", other)),
  };

  let client = Client::builder()
    .user_agent(
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
       (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    )
    .timeout(std::time::Duration::from_secs(15))
    .build()
    .map_err(|e| e.to_string())?;

  let response = client
    .get(&feed_url)
    .send()
    .map_err(|e| format!("Request failed: {e}"))?;

  if !response.status().is_success() {
    return Err(format!(
      "HTTP {} from {}",
      response.status().as_u16(),
      feed_url
    ));
  }

  let text = response
    .text()
    .map_err(|e| format!("Failed to read response: {e}"))?;

  parse_rss(&text, &platform)
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
