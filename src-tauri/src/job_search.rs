use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tauri::Manager;
use url::Url;

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
    pub freshness_score: f64,
    pub keyword_score: f64,
    pub total_score: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobSearchFallbackHint {
    pub reason: String,
    pub browser_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobSearchResultsBundle {
    pub global_top5: Vec<JobSearchResult>,
    pub top5_per_platform: HashMap<String, Vec<JobSearchResult>>,
    pub all_ranked: Vec<JobSearchResult>,
    pub fallback_hints: HashMap<String, JobSearchFallbackHint>,
}

#[derive(Clone)]
struct ScoredResult {
    result: JobSearchResult,
    score: f64,
}

fn get_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data_dir.join("data").join("app.db"))
}

fn supported_platforms() -> &'static [&'static str] {
    &["jobindex", "indeed", "linkedin", "thehub"]
}

fn is_supported_platform(platform: &str) -> bool {
    supported_platforms().contains(&platform)
}

fn parse_tags(tags_str: &str) -> Vec<String> {
    let trimmed = tags_str.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
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
        .map(|part| {
            part.trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_lowercase()
        })
        .filter(|part| !part.is_empty())
        .collect()
}

fn domain_for_platform(platform: &str) -> &'static str {
    match platform {
        "jobindex" => "jobindex.dk",
        "indeed" => "indeed.",
        "linkedin" => "linkedin.com/jobs",
        "thehub" => "thehub.io",
        _ => "",
    }
}

fn is_archived_result(platform: &str, url: &str, title: &str, description: &str) -> bool {
    if platform != "jobindex" {
        return false;
    }
    let haystack = format!(
        "{} {} {}",
        url.to_lowercase(),
        title.to_lowercase(),
        description.to_lowercase()
    );
    ["archive", "arkiv", "expired", "udløbet", "not active"]
        .iter()
        .any(|pattern| haystack.contains(pattern))
}

fn build_provider_query(platform: &str, keywords: &str, location: &str) -> String {
    let mut query = keywords.to_string();
    if !location.trim().is_empty() {
        query.push(' ');
        query.push_str(location.trim());
    }
    let domain = domain_for_platform(platform);
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
    url.to_lowercase().contains(domain)
}

fn parse_serpapi_results(
    payload: &str,
    platform: &str,
    location: &str,
) -> Result<Vec<JobSearchResult>, String> {
    let data: Value =
        serde_json::from_str(payload).map_err(|e| format!("SerpAPI JSON parse error: {e}"))?;
    let Some(items) = data.get("organic_results").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in items {
        let url = item
            .get("link")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if url.is_empty() || !keep_platform_result(platform, &url) {
            continue;
        }
        let title_raw = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Untitled");
        let (title, company) = normalize_company(title_raw);
        let desc = item
            .get("snippet")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(400)
            .collect::<String>();
        if is_archived_result(platform, &url, &title, &desc) {
            continue;
        }
        let published = item
            .get("date")
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
            freshness_score: 0.0,
            keyword_score: 0.0,
            total_score: 0.0,
        });
    }
    Ok(out)
}

fn parse_brave_results(
    payload: &str,
    platform: &str,
    location: &str,
) -> Result<Vec<JobSearchResult>, String> {
    let data: Value =
        serde_json::from_str(payload).map_err(|e| format!("Brave JSON parse error: {e}"))?;
    let Some(items) = data
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in items {
        let url = item
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if url.is_empty() || !keep_platform_result(platform, &url) {
            continue;
        }
        let title_raw = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Untitled");
        let (title, company) = normalize_company(title_raw);
        let desc = item
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(400)
            .collect::<String>();
        if is_archived_result(platform, &url, &title, &desc) {
            continue;
        }
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
            freshness_score: 0.0,
            keyword_score: 0.0,
            total_score: 0.0,
        });
    }
    Ok(out)
}

fn strip_html_to_text(html: &str) -> String {
    let mut cleaned = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut previous_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            '&' => {
                cleaned.push(ch);
                previous_was_space = false;
            }
            c if c.is_whitespace() => {
                if !previous_was_space {
                    cleaned.push(' ');
                    previous_was_space = true;
                }
            }
            _ => {
                cleaned.push(ch);
                previous_was_space = false;
            }
        }
    }

    cleaned
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn extract_job_page_text_from_html(html: &str) -> String {
    let lower = html.to_lowercase();
    let body_start = lower.find("<body").unwrap_or(0);
    let body_html = &html[body_start..];
    let stripped = strip_html_to_text(body_html);
    stripped.chars().take(12_000).collect()
}

fn fetch_job_page_text(url: &str) -> Result<String, String> {
    if url.trim().is_empty() {
        return Ok(String::new());
    }

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("Failed to fetch job page: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch job page: HTTP {}", response.status()));
    }

    let body = response
        .text()
        .map_err(|e| format!("Failed to read job page: {e}"))?;
    Ok(extract_job_page_text_from_html(&body))
}

fn parse_age_days(value: &str) -> Option<f64> {
    let txt = value.trim().to_lowercase();
    if txt.is_empty() {
        return None;
    }
    if txt == "today" {
        return Some(0.0);
    }
    if txt == "yesterday" {
        return Some(1.0);
    }
    if let Ok(days) = txt
        .replace("days ago", "")
        .replace("day ago", "")
        .trim()
        .parse::<f64>()
    {
        return Some(days.max(0.0));
    }
    if let Ok(weeks) = txt
        .replace("weeks ago", "")
        .replace("week ago", "")
        .replace('w', "")
        .trim()
        .parse::<f64>()
    {
        return Some((weeks * 7.0).max(0.0));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(&txt) {
        let age = (Utc::now() - dt.with_timezone(&Utc)).num_days();
        return Some(age.max(0) as f64);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&txt, "%Y-%m-%d") {
        let age = (Utc::now().date_naive() - d).num_days();
        return Some(age.max(0) as f64);
    }
    None
}

fn freshness_score(published_date: &str) -> f64 {
    let Some(days) = parse_age_days(published_date) else {
        return 0.5;
    };
    if days <= 3.0 {
        1.0
    } else if days <= 7.0 {
        0.85
    } else if days <= 14.0 {
        0.65
    } else if days <= 30.0 {
        0.35
    } else if days <= 60.0 {
        0.15
    } else {
        0.05
    }
}

fn fetch_keyword_recency_weights(app: &tauri::AppHandle) -> Result<HashMap<String, f64>, String> {
    let db_path = get_db_path(app)?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT tags, created_at FROM jobs WHERE tags IS NOT NULL AND tags != ''")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut weights: HashMap<String, f64> = HashMap::new();
    for (tags_str, created_at) in rows {
        let age_days = parse_age_days(&created_at).unwrap_or(0.0);
        let decay = f64::exp(-(std::f64::consts::LN_2 / 60.0) * age_days);
        for tag in parse_tags(&tags_str) {
            *weights.entry(tag).or_insert(0.0) += decay;
        }
    }
    Ok(weights)
}

fn dedupe_results(results: Vec<JobSearchResult>) -> Vec<JobSearchResult> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for result in results {
        let key = if !result.url.trim().is_empty() {
            format!(
                "{}|{}",
                result.platform.to_lowercase(),
                result.url.to_lowercase()
            )
        } else {
            format!(
                "{}|{}|{}",
                result.platform.to_lowercase(),
                result.title.to_lowercase(),
                result.company.to_lowercase()
            )
        };
        if seen.insert(key) {
            out.push(result);
        }
    }
    out
}

fn keyword_score(
    result: &JobSearchResult,
    selected_keywords: &[String],
    keyword_weights: &HashMap<String, f64>,
) -> f64 {
    if selected_keywords.is_empty() {
        return 0.0;
    }
    let title = result.title.to_lowercase();
    let description = result.description.to_lowercase();
    let exact_phrase = selected_keywords.join(" ").to_lowercase();

    let max_weight = selected_keywords
        .iter()
        .map(|k| *keyword_weights.get(k).unwrap_or(&0.2))
        .fold(0.2_f64, f64::max);

    let mut sum = 0.0;
    let mut cap = 0.0;
    for kw in selected_keywords {
        let kw_lc = kw.to_lowercase();
        let base_weight = keyword_weights.get(&kw_lc).copied().unwrap_or(0.2) / max_weight;
        let mut field_score = 0.0;
        if title.contains(&kw_lc) {
            field_score += 1.0;
        }
        if description.contains(&kw_lc) {
            field_score += 0.45;
        }
        if !exact_phrase.is_empty()
            && (title.contains(&exact_phrase) || description.contains(&exact_phrase))
        {
            field_score += 0.15;
        }
        sum += base_weight * field_score;
        cap += base_weight * 1.6;
    }
    if cap <= 0.0 {
        0.0
    } else {
        (sum / cap).clamp(0.0, 1.0)
    }
}

fn score_and_rank(
    results: Vec<JobSearchResult>,
    selected_keywords: &[String],
    keyword_weights: &HashMap<String, f64>,
) -> Vec<JobSearchResult> {
    let mut scored: Vec<ScoredResult> = results
        .into_iter()
        .map(|mut result| {
            let fresh = freshness_score(&result.published_date);
            let kw = keyword_score(&result, selected_keywords, keyword_weights);
            let total = (0.6 * fresh + 0.4 * kw).clamp(0.0, 1.0);
            result.freshness_score = fresh;
            result.keyword_score = kw;
            result.total_score = total;
            ScoredResult {
                result,
                score: total,
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.into_iter().map(|s| s.result).collect()
}

fn fetch_platform_results(
    platform: &str,
    keywords: &[String],
    location: &str,
    region: &str,
    serp_api_key: &str,
    brave_search_api_key: &str,
) -> Result<Vec<JobSearchResult>, String> {
    if !is_supported_platform(platform) {
        return Err(format!("Unknown platform: {platform}"));
    }
    let query = keywords.join(" ");
    let client = Client::builder()
    .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
    .timeout(std::time::Duration::from_secs(15))
    .build()
    .map_err(|e| e.to_string())?;
    let provider_query = build_provider_query(platform, &query, location);
    let gl = region.to_lowercase();
    let brave_country = gl.to_uppercase();

    if !serp_api_key.trim().is_empty() {
        let response = client
            .get("https://serpapi.com/search.json")
            .query(&[
                ("engine", "google"),
                ("q", provider_query.as_str()),
                ("gl", gl.as_str()),
                ("hl", "en"),
                ("num", "20"),
                ("api_key", serp_api_key.trim()),
            ])
            .send()
            .map_err(|e| format!("SerpAPI request failed: {e}"))?;
        if response.status().is_success() {
            let body = response
                .text()
                .map_err(|e| format!("SerpAPI read failed: {e}"))?;
            let results = parse_serpapi_results(&body, platform, location)?;
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    if !brave_search_api_key.trim().is_empty() {
        let response = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", brave_search_api_key.trim())
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
            return parse_brave_results(&body, platform, location);
        }
    }
    Ok(Vec::new())
}

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

#[tauri::command]
pub fn fetch_job_search_results(
    platform: String,
    keywords: Vec<String>,
    location: Option<String>,
    region: Option<String>,
    serp_api_key: Option<String>,
    brave_search_api_key: Option<String>,
) -> Result<Vec<JobSearchResult>, String> {
    let loc = location.unwrap_or_default();
    let reg = region.unwrap_or_else(|| "dk".to_string());
    let serp_key = serp_api_key.unwrap_or_default();
    let brave_key = brave_search_api_key.unwrap_or_default();
    if serp_key.trim().is_empty() && brave_key.trim().is_empty() {
        return Err(
            "Missing search API keys. Add SerpAPI and/or Brave Search API key in Settings."
                .to_string(),
        );
    }
    fetch_platform_results(&platform, &keywords, &loc, &reg, &serp_key, &brave_key)
}

#[tauri::command]
pub fn fetch_job_search_bundle(
    app: tauri::AppHandle,
    keywords: Vec<String>,
    location: Option<String>,
    region: Option<String>,
    platforms: Vec<String>,
    serp_api_key: Option<String>,
    brave_search_api_key: Option<String>,
) -> Result<JobSearchResultsBundle, String> {
    if keywords.is_empty() {
        return Ok(JobSearchResultsBundle {
            global_top5: Vec::new(),
            top5_per_platform: HashMap::new(),
            all_ranked: Vec::new(),
            fallback_hints: HashMap::new(),
        });
    }
    let loc = location.unwrap_or_default();
    let reg = region.unwrap_or_else(|| "dk".to_string());
    let requested_platforms: Vec<String> = if platforms.is_empty() {
        supported_platforms()
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        platforms
            .into_iter()
            .filter(|p| is_supported_platform(p))
            .collect()
    };
    let serp_key = serp_api_key.unwrap_or_default();
    let brave_key = brave_search_api_key.unwrap_or_default();
    if serp_key.trim().is_empty() && brave_key.trim().is_empty() {
        return Err(
            "Missing search API keys. Add SerpAPI and/or Brave Search API key in Settings."
                .to_string(),
        );
    }

    let keyword_weights = fetch_keyword_recency_weights(&app)?;
    let mut all_raw: Vec<JobSearchResult> = Vec::new();
    let mut top5_per_platform: HashMap<String, Vec<JobSearchResult>> = HashMap::new();
    let mut fallback_hints: HashMap<String, JobSearchFallbackHint> = HashMap::new();

    for platform in requested_platforms {
        let platform_results =
            fetch_platform_results(&platform, &keywords, &loc, &reg, &serp_key, &brave_key)?;
        if platform_results.is_empty() {
            let browser_url = build_search_url(
                platform.clone(),
                keywords.clone(),
                Some(loc.clone()),
                Some(reg.clone()),
            )?;
            fallback_hints.insert(
                platform.clone(),
                JobSearchFallbackHint {
                    reason: "No API results found for this platform.".to_string(),
                    browser_url,
                },
            );
            top5_per_platform.insert(platform, Vec::new());
            continue;
        }
        let ranked = score_and_rank(
            dedupe_results(platform_results),
            &keywords,
            &keyword_weights,
        );
        top5_per_platform.insert(platform.clone(), ranked.iter().take(5).cloned().collect());
        all_raw.extend(ranked);
    }

    let all_ranked = score_and_rank(dedupe_results(all_raw), &keywords, &keyword_weights);
    Ok(JobSearchResultsBundle {
        global_top5: all_ranked.iter().take(5).cloned().collect(),
        top5_per_platform,
        all_ranked,
        fallback_hints,
    })
}

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
            let mut u = Url::parse("https://www.jobindex.dk/jobsoeg").map_err(|e| e.to_string())?;
            let mut p = u.query_pairs_mut();
            p.append_pair("q", &query);
            if !loc.is_empty() {
                p.append_pair("where", &loc);
            }
            drop(p);
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
            let mut p = u.query_pairs_mut();
            p.append_pair("q", &query);
            if !loc.is_empty() {
                p.append_pair("l", &loc);
            }
            drop(p);
            u.to_string()
        }
        "linkedin" => {
            let mut u =
                Url::parse("https://www.linkedin.com/jobs/search").map_err(|e| e.to_string())?;
            let mut p = u.query_pairs_mut();
            p.append_pair("keywords", &query);
            if !loc.is_empty() {
                p.append_pair("location", &loc);
            }
            drop(p);
            u.to_string()
        }
        "thehub" => {
            let mut u = Url::parse("https://thehub.io/jobs").map_err(|e| e.to_string())?;
            let mut p = u.query_pairs_mut();
            p.append_pair("q", &query);
            if !loc.is_empty() {
                p.append_pair("city", &loc);
            }
            drop(p);
            u.to_string()
        }
        other => return Err(format!("Unknown platform: {other}")),
    };
    Ok(url)
}

#[tauri::command]
pub fn open_url_in_browser(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Could not open browser: {e}"))
}

#[tauri::command]
pub fn fetch_job_search_result_page_text(url: String) -> Result<String, String> {
    fetch_job_page_text(&url)
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_tags ─────────────────────────────────────────────────────────

    #[test]
    fn parse_tags_empty_returns_empty() {
        assert!(parse_tags("").is_empty());
        assert!(parse_tags("   ").is_empty());
    }

    #[test]
    fn parse_tags_comma_separated() {
        let tags = parse_tags("Rust, Tauri, React");
        assert_eq!(tags, vec!["rust", "tauri", "react"]);
    }

    #[test]
    fn parse_tags_semicolon_separator() {
        let tags = parse_tags("python;django;postgres");
        assert_eq!(tags, vec!["python", "django", "postgres"]);
    }

    #[test]
    fn parse_tags_pipe_separator() {
        let tags = parse_tags("go|grpc|kubernetes");
        assert_eq!(tags, vec!["go", "grpc", "kubernetes"]);
    }

    #[test]
    fn parse_tags_newline_separator() {
        let tags = parse_tags("a\nb\nc");
        assert_eq!(tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_tags_strips_quotes() {
        let tags = parse_tags(r#""Rust","TypeScript""#);
        assert_eq!(tags, vec!["rust", "typescript"]);
    }

    #[test]
    fn parse_tags_json_array_style() {
        let tags = parse_tags(r#"["rust","tauri","react"]"#);
        assert_eq!(tags, vec!["rust", "tauri", "react"]);
    }

    #[test]
    fn parse_tags_json_array_whitespace_trimmed() {
        let tags = parse_tags(r#"["  Rust  ", "  Node.js  "]"#);
        assert_eq!(tags, vec!["rust", "node.js"]);
    }

    #[test]
    fn parse_tags_normalises_to_lowercase() {
        let tags = parse_tags("TypeScript, REACT, Vue.js");
        assert_eq!(tags, vec!["typescript", "react", "vue.js"]);
    }

    #[test]
    fn parse_tags_filters_empty_parts() {
        let tags = parse_tags("rust,,,,typescript");
        assert_eq!(tags, vec!["rust", "typescript"]);
    }

    // ── normalize_company ──────────────────────────────────────────────────

    #[test]
    fn normalize_company_splits_on_first_dash() {
        let (title, company) = normalize_company("Senior Developer - Acme Corp");
        assert_eq!(title, "Senior Developer");
        assert_eq!(company, "Acme Corp");
    }

    #[test]
    fn normalize_company_no_dash_returns_whole_title_empty_company() {
        let (title, company) = normalize_company("Software Engineer");
        assert_eq!(title, "Software Engineer");
        assert_eq!(company, "");
    }

    #[test]
    fn normalize_company_splits_only_on_first_dash() {
        // "A - B - C" → title="A", company="B - C"
        let (title, company) = normalize_company("Frontend Engineer - Acme Corp - Copenhagen");
        assert_eq!(title, "Frontend Engineer");
        assert_eq!(company, "Acme Corp - Copenhagen");
    }

    #[test]
    fn normalize_company_trims_whitespace() {
        let (title, company) = normalize_company("  Dev  -  BigCo  ");
        assert_eq!(title, "Dev");
        assert_eq!(company, "BigCo");
    }

    // ── domain_for_platform ────────────────────────────────────────────────

    #[test]
    fn domain_for_platform_jobindex() {
        assert_eq!(domain_for_platform("jobindex"), "jobindex.dk");
    }

    #[test]
    fn domain_for_platform_indeed() {
        assert_eq!(domain_for_platform("indeed"), "indeed.");
    }

    #[test]
    fn domain_for_platform_known_and_unknown() {
        assert_eq!(domain_for_platform("linkedin"), "linkedin.com/jobs");
        assert_eq!(domain_for_platform("thehub"), "thehub.io");
        assert_eq!(domain_for_platform("unknown"), "");
    }

    // ── keep_platform_result ───────────────────────────────────────────────

    #[test]
    fn keep_platform_result_jobindex_matching_url() {
        assert!(keep_platform_result(
            "jobindex",
            "https://www.jobindex.dk/job/123"
        ));
    }

    #[test]
    fn keep_platform_result_jobindex_non_matching_url() {
        assert!(!keep_platform_result(
            "jobindex",
            "https://www.example.com/job/123"
        ));
    }

    #[test]
    fn keep_platform_result_indeed_matching_url() {
        assert!(keep_platform_result(
            "indeed",
            "https://dk.indeed.com/viewjob?jk=abc"
        ));
    }

    #[test]
    fn keep_platform_result_indeed_non_matching_url() {
        assert!(!keep_platform_result(
            "indeed",
            "https://www.example.com/jobs"
        ));
    }

    #[test]
    fn keep_platform_result_linkedin_applies_domain_filter() {
        assert!(keep_platform_result(
            "linkedin",
            "https://www.linkedin.com/jobs/view/123"
        ));
        assert!(!keep_platform_result(
            "linkedin",
            "https://www.anything.com/job/1"
        ));
    }

    // ── build_provider_query ───────────────────────────────────────────────

    #[test]
    fn build_provider_query_jobindex_adds_site_domain() {
        let q = build_provider_query("jobindex", "rust developer", "");
        assert!(q.contains("site:jobindex.dk"));
        assert!(q.contains("rust developer"));
    }

    #[test]
    fn build_provider_query_indeed_adds_site_domain() {
        let q = build_provider_query("indeed", "react", "Copenhagen");
        assert!(q.contains("site:indeed."));
        assert!(q.contains("Copenhagen"));
    }

    #[test]
    fn build_provider_query_unknown_platform_no_site() {
        let q = build_provider_query("unknown", "job", "");
        assert!(!q.contains("site:"));
    }

    #[test]
    fn build_provider_query_empty_location_not_added() {
        let q = build_provider_query("jobindex", "python", "");
        let parts: Vec<&str> = q.split_whitespace().collect();
        // Should be: "python site:jobindex.dk"
        assert_eq!(parts[0], "python");
        assert!(q.contains("site:jobindex.dk"));
        assert!(!q.contains("  ")); // no double spaces
    }

    // ── parse_serpapi_results ─────────────────────────────────────────────

    #[test]
    fn parse_serpapi_results_basic() {
        let payload = r#"{
      "organic_results": [
        {
          "link": "https://www.jobindex.dk/job/1",
          "title": "Senior Dev - Acme Corp",
          "snippet": "Great opportunity for developers",
          "date": "3 days ago"
        }
      ]
    }"#;
        let results = parse_serpapi_results(payload, "jobindex", "Copenhagen").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Senior Dev");
        assert_eq!(results[0].company, "Acme Corp");
        assert_eq!(results[0].description, "Great opportunity for developers");
        assert_eq!(results[0].published_date, "3 days ago");
        assert_eq!(results[0].location, "Copenhagen");
        assert_eq!(results[0].platform, "jobindex");
    }

    #[test]
    fn parse_serpapi_results_filters_non_platform_urls() {
        let payload = r#"{
      "organic_results": [
        {
          "link": "https://www.other.com/job/1",
          "title": "Some Job"
        },
        {
          "link": "https://www.jobindex.dk/job/2",
          "title": "Real Job"
        }
      ]
    }"#;
        let results = parse_serpapi_results(payload, "jobindex", "").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://www.jobindex.dk/job/2");
    }

    #[test]
    fn parse_serpapi_results_truncates_description_at_400() {
        let long_snippet = "x".repeat(500);
        let payload = format!(
            r#"{{"organic_results":[{{"link":"https://www.jobindex.dk/job/1","title":"Dev","snippet":"{}"}}]}}"#,
            long_snippet
        );
        let results = parse_serpapi_results(&payload, "jobindex", "").unwrap();
        assert_eq!(results[0].description.chars().count(), 400);
    }

    #[test]
    fn parse_serpapi_results_empty_organic_results() {
        let payload = r#"{"organic_results":[]}"#;
        let results = parse_serpapi_results(payload, "jobindex", "").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_serpapi_results_missing_organic_results_key() {
        let payload = r#"{"search_metadata":{}}"#;
        let results = parse_serpapi_results(payload, "jobindex", "").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_serpapi_results_invalid_json_returns_error() {
        let err = parse_serpapi_results("not json {{{", "jobindex", "").unwrap_err();
        assert!(err.contains("SerpAPI JSON parse error"));
    }

    // ── parse_brave_results ───────────────────────────────────────────────

    #[test]
    fn parse_brave_results_basic() {
        let payload = r#"{
      "web": {
        "results": [
          {
            "url": "https://dk.indeed.com/viewjob?jk=abc",
            "title": "Backend Engineer - Beta AS",
            "description": "Looking for a backend engineer",
            "page_age": "2026-04-10T12:00:00"
          }
        ]
      }
    }"#;
        let results = parse_brave_results(payload, "indeed", "Aarhus").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Backend Engineer");
        assert_eq!(results[0].company, "Beta AS");
        assert_eq!(results[0].description, "Looking for a backend engineer");
        assert_eq!(results[0].location, "Aarhus");
        assert_eq!(results[0].platform, "indeed");
    }

    #[test]
    fn parse_brave_results_filters_non_platform_urls() {
        let payload = r#"{
      "web": {
        "results": [
          {"url": "https://www.other.com/job/1", "title": "Filtered"},
          {"url": "https://dk.indeed.com/viewjob?jk=xyz", "title": "Kept"}
        ]
      }
    }"#;
        let results = parse_brave_results(payload, "indeed", "").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://dk.indeed.com/viewjob?jk=xyz");
    }

    #[test]
    fn parse_brave_results_missing_web_key() {
        let payload = r#"{"type":"search"}"#;
        let results = parse_brave_results(payload, "indeed", "").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_brave_results_invalid_json_returns_error() {
        let err = parse_brave_results("{{ bad json", "indeed", "").unwrap_err();
        assert!(err.contains("Brave JSON parse error"));
    }

    #[test]
    fn extract_job_page_text_from_html_strips_tags_and_entities() {
        let html = r#"
        <html>
          <body>
            <article>
              <h1>Senior Developer</h1>
              <p>Build APIs &amp; integrations</p>
              <p>Apply from Copenhagen</p>
            </article>
          </body>
        </html>
        "#;
        let text = extract_job_page_text_from_html(html);
        assert!(text.contains("Senior Developer"));
        assert!(text.contains("Build APIs & integrations"));
        assert!(text.contains("Apply from Copenhagen"));
        assert!(!text.contains("<article>"));
    }

    // ── build_search_url ───────────────────────────────────────────────────

    #[test]
    fn build_search_url_jobindex_contains_base_and_query() {
        let url = build_search_url(
            "jobindex".to_string(),
            vec!["react".to_string(), "typescript".to_string()],
            None,
            None,
        )
        .unwrap();
        assert!(url.contains("jobindex.dk/jobsoeg"));
        assert!(url.contains("react"));
        assert!(url.contains("typescript"));
    }

    #[test]
    fn build_search_url_jobindex_with_location() {
        let url = build_search_url(
            "jobindex".to_string(),
            vec!["rust".to_string()],
            Some("Copenhagen".to_string()),
            None,
        )
        .unwrap();
        assert!(url.contains("where=Copenhagen") || url.contains("where=Copenhagen"));
        assert!(url.contains("jobindex.dk"));
    }

    #[test]
    fn build_search_url_jobindex_no_location_omits_where() {
        let url =
            build_search_url("jobindex".to_string(), vec!["go".to_string()], None, None).unwrap();
        assert!(!url.contains("where="));
    }

    #[test]
    fn build_search_url_indeed_dk_default() {
        let url = build_search_url(
            "indeed".to_string(),
            vec!["python".to_string()],
            None,
            Some("dk".to_string()),
        )
        .unwrap();
        assert!(url.contains("dk.indeed.com"));
    }

    #[test]
    fn build_search_url_indeed_de_region() {
        let url = build_search_url(
            "indeed".to_string(),
            vec!["java".to_string()],
            None,
            Some("de".to_string()),
        )
        .unwrap();
        assert!(url.contains("de.indeed.com"));
    }

    #[test]
    fn build_search_url_indeed_com_international() {
        let url = build_search_url(
            "indeed".to_string(),
            vec!["scala".to_string()],
            None,
            Some("com".to_string()),
        )
        .unwrap();
        assert!(url.contains("www.indeed.com"));
        assert!(!url.contains("com.indeed.com"));
    }

    #[test]
    fn build_search_url_linkedin_uses_keywords_param() {
        let url = build_search_url(
            "linkedin".to_string(),
            vec!["devops".to_string()],
            None,
            None,
        )
        .unwrap();
        assert!(url.contains("linkedin.com/jobs/search"));
        assert!(url.contains("keywords="));
        assert!(url.contains("devops"));
    }

    #[test]
    fn build_search_url_linkedin_with_location() {
        let url = build_search_url(
            "linkedin".to_string(),
            vec!["sre".to_string()],
            Some("Aarhus".to_string()),
            None,
        )
        .unwrap();
        assert!(url.contains("location="));
        assert!(url.contains("Aarhus"));
    }

    #[test]
    fn build_search_url_unknown_platform_returns_err() {
        let err = build_search_url("monster".to_string(), vec!["x".to_string()], None, None);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Unknown platform"));
    }

    #[test]
    fn build_search_url_multiple_keywords_joined_with_space() {
        let url = build_search_url(
            "jobindex".to_string(),
            vec!["react".to_string(), "node".to_string()],
            None,
            None,
        )
        .unwrap();
        // URL-encoded space is %20; both keywords must appear
        assert!(url.contains("react"));
        assert!(url.contains("node"));
    }
}
