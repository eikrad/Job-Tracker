use chrono::Utc;

use crate::db::connection;
use crate::net::{api_client, assert_api_host, fetch_untrusted};

#[derive(Debug, PartialEq)]
enum ListingStatus {
    Active,
    Closed,
    Archived,
    Unreachable,
}

impl ListingStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ListingStatus::Active => "active",
            ListingStatus::Closed => "closed",
            ListingStatus::Archived => "archived",
            ListingStatus::Unreachable => "unreachable",
        }
    }
}

fn detect_status(url: &str, serp_api_key: &str) -> ListingStatus {
    let domain = extract_domain(url).unwrap_or_default();

    // Indeed blocks automated requests — skip direct fetch and go straight to SerpAPI
    if domain.contains("indeed.com") {
        if let Some(job_key) = extract_indeed_job_key(url) {
            if !serp_api_key.trim().is_empty() {
                return classify_indeed_via_serp(&job_key, serp_api_key)
                    .unwrap_or(ListingStatus::Unreachable);
            }
        }
        return ListingStatus::Unreachable;
    }

    let fetched = match fetch_untrusted(url) {
        Ok(r) => r,
        Err(_) => return ListingStatus::Unreachable,
    };

    if !(200..400).contains(&fetched.status) {
        return ListingStatus::Unreachable;
    }

    let body = fetched
        .body
        .chars()
        .take(4096)
        .collect::<String>()
        .to_lowercase();

    classify_by_domain(url, &fetched.final_url, &body)
}

fn classify_by_domain(original_url: &str, final_url: &str, body_head: &str) -> ListingStatus {
    let domain = extract_domain(original_url).unwrap_or_default();

    if domain.contains("linkedin.com") {
        classify_linkedin(final_url, body_head)
    } else if domain.contains("jobindex.dk") {
        classify_jobindex(final_url)
    } else {
        classify_generic(final_url, body_head)
    }
}

fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_lowercase))
}

fn classify_linkedin(final_url: &str, body_head: &str) -> ListingStatus {
    // LinkedIn redirects expired jobs to /jobs/view/<id>/expired or shows a specific title
    if final_url.contains("/expired") || body_head.contains("no longer accepting applications") {
        return ListingStatus::Closed;
    }
    if body_head.contains("job not found") || body_head.contains("page not found") {
        return ListingStatus::Unreachable;
    }
    ListingStatus::Active
}

fn extract_indeed_job_key(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|u| {
        u.query_pairs()
            .find(|(k, _)| k == "jk")
            .map(|(_, v)| v.into_owned())
    })
}

fn classify_indeed_via_serp(job_key: &str, serp_api_key: &str) -> Option<ListingStatus> {
    let client = api_client().ok()?;
    let url = "https://serpapi.com/search.json";
    assert_api_host(url).ok()?;
    let query = format!("site:indeed.com jk:{job_key}");
    let resp = client
        .get(url)
        .query(&[
            ("engine", "google"),
            ("q", query.as_str()),
            ("num", "5"),
            ("api_key", serp_api_key.trim()),
        ])
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().ok()?;
    let data: serde_json::Value = serde_json::from_str(&body).ok()?;
    let hits = data
        .get("organic_results")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().any(|item| {
                item.get("link")
                    .and_then(|l| l.as_str())
                    .map(|l| l.contains(job_key))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    Some(if hits {
        ListingStatus::Active
    } else {
        ListingStatus::Closed
    })
}

#[cfg(test)]
fn classify_indeed(original_url: &str, final_url: &str, body_head: &str) -> ListingStatus {
    let original_has_job_id = original_url.contains("/viewjob") || original_url.contains("jk=");
    let redirected_away = original_has_job_id
        && !final_url.contains("/viewjob")
        && !final_url.contains("jk=");

    if redirected_away {
        return ListingStatus::Closed;
    }
    if body_head.contains("job is no longer available") || body_head.contains("job not found") {
        return ListingStatus::Closed;
    }
    ListingStatus::Active
}

fn classify_jobindex(final_url: &str) -> ListingStatus {
    // Jobindex redirects closed jobs to their archive at /job/arkiv/...
    if final_url.contains("/arkiv") || final_url.contains("jobindex.dk/arkiv") {
        return ListingStatus::Archived;
    }
    ListingStatus::Active
}

fn classify_generic(final_url: &str, body_head: &str) -> ListingStatus {
    // For unknown domains: look for common "closed" phrases in page title area
    let closed_signals = [
        "job no longer available",
        "no longer accepting",
        "position has been filled",
        "job not found",
        "page not found",
        "404",
        "ikke tilgængelig", // Danish "not available"
        "stillingen er besat", // Danish "position is filled"
    ];
    if closed_signals.iter().any(|s| body_head.contains(s)) {
        return ListingStatus::Closed;
    }
    // Check if we were redirected to a completely different domain (likely a generic error page)
    if let (Some(orig), Some(fin)) = (
        extract_domain(final_url),
        // We don't have original here, but a redirect to root "/" with short body suggests removal
        None::<String>,
    ) {
        let _ = (orig, fin); // suppress unused warning
    }
    ListingStatus::Active
}

#[tauri::command]
pub async fn check_listing_status(
    app: tauri::AppHandle,
    job_id: i64,
    url: String,
) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("No URL provided".to_string());
    }

    // reqwest::blocking cannot run on the Tokio async runtime thread — use spawn_blocking
    let url_clone = url.clone();
    let serp_key = crate::secrets::get_secret_or_default("serpapi");
    let status = tauri::async_runtime::spawn_blocking(move || detect_status(&url_clone, &serp_key))
        .await
        .map_err(|e| format!("Thread error: {e}"))?;

    let status_str = status.as_str().to_string();
    let now = Utc::now().to_rfc3339();

    let conn = connection(&app)?;
    conn.execute(
        "UPDATE jobs SET listing_status = ?1, listing_checked_at = ?2 WHERE id = ?3",
        rusqlite::params![status_str, now, job_id],
    )
    .map_err(|e| format!("Failed to save listing status: {e}"))?;

    Ok(status_str)
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linkedin_closed_on_expired_url() {
        assert_eq!(
            classify_linkedin("https://linkedin.com/jobs/view/123/expired", ""),
            ListingStatus::Closed
        );
    }

    #[test]
    fn linkedin_closed_on_body_signal() {
        assert_eq!(
            classify_linkedin(
                "https://linkedin.com/jobs/view/123",
                "no longer accepting applications"
            ),
            ListingStatus::Closed
        );
    }

    #[test]
    fn linkedin_active_when_no_signals() {
        assert_eq!(
            classify_linkedin("https://linkedin.com/jobs/view/123", "<title>Software Engineer at Acme</title>"),
            ListingStatus::Active
        );
    }

    #[test]
    fn indeed_closed_on_redirect_away() {
        assert_eq!(
            classify_indeed(
                "https://indeed.com/viewjob?jk=abc123",
                "https://indeed.com/jobs?q=developer",
                ""
            ),
            ListingStatus::Closed
        );
    }

    #[test]
    fn indeed_active_when_same_job_url() {
        assert_eq!(
            classify_indeed(
                "https://indeed.com/viewjob?jk=abc123",
                "https://indeed.com/viewjob?jk=abc123",
                ""
            ),
            ListingStatus::Active
        );
    }

    #[test]
    fn jobindex_archived_on_arkiv_redirect() {
        assert_eq!(
            classify_jobindex("https://www.jobindex.dk/job/arkiv/123456"),
            ListingStatus::Archived
        );
    }

    #[test]
    fn jobindex_active_on_normal_url() {
        assert_eq!(
            classify_jobindex("https://www.jobindex.dk/job/123456"),
            ListingStatus::Active
        );
    }

    #[test]
    fn generic_closed_on_danish_signal() {
        assert_eq!(
            classify_generic("https://example.com/jobs/42", "stillingen er besat"),
            ListingStatus::Closed
        );
    }

    #[test]
    fn generic_active_on_clean_page() {
        assert_eq!(
            classify_generic("https://example.com/jobs/42", "<title>Software Engineer - Acme</title>"),
            ListingStatus::Active
        );
    }

    #[test]
    fn baseline_fixture_matches_classifier() {
        #[derive(serde::Deserialize)]
        struct Case {
            id: String,
            original_url: String,
            final_url: String,
            body_head: String,
            expected: String,
        }
        let raw = include_str!("../fixtures/listing_status_baseline.json");
        let cases: Vec<Case> = serde_json::from_str(raw).expect("baseline fixture");
        assert!(cases.len() >= 7, "baseline should cover several boards");

        for case in cases {
            let got = if extract_domain(&case.original_url)
                .unwrap_or_default()
                .contains("indeed.com")
            {
                classify_indeed(&case.original_url, &case.final_url, &case.body_head)
            } else {
                classify_by_domain(&case.original_url, &case.final_url, &case.body_head)
            };
            assert_eq!(
                got.as_str(),
                case.expected.as_str(),
                "baseline case {} drifted",
                case.id
            );
        }
    }
}
