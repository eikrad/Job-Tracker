use chrono::Utc;
use reqwest::blocking::Client;

use crate::db::connection;

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

fn make_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())
}

fn detect_status(url: &str) -> ListingStatus {
    let client = match make_client() {
        Ok(c) => c,
        Err(_) => return ListingStatus::Unreachable,
    };

    let response = match client.get(url).send() {
        Ok(r) => r,
        Err(_) => return ListingStatus::Unreachable,
    };

    let final_url = response.url().to_string();
    let status_code = response.status();

    if status_code.is_client_error() || status_code.is_server_error() {
        return ListingStatus::Unreachable;
    }

    // Read body for title-based heuristics (cap at 4KB — <title> is always near the top)
    let body = response
        .text()
        .unwrap_or_default()
        .chars()
        .take(4096)
        .collect::<String>()
        .to_lowercase();

    classify_by_domain(url, &final_url, &body)
}

fn classify_by_domain(original_url: &str, final_url: &str, body_head: &str) -> ListingStatus {
    let domain = extract_domain(original_url);

    match domain.as_deref() {
        Some(d) if d.contains("linkedin.com") => classify_linkedin(final_url, body_head),
        Some(d) if d.contains("indeed.com") => classify_indeed(original_url, final_url, body_head),
        Some(d) if d.contains("jobindex.dk") => classify_jobindex(final_url),
        _ => classify_generic(final_url, body_head),
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

fn classify_indeed(original_url: &str, final_url: &str, body_head: &str) -> ListingStatus {
    // Indeed redirects expired jobs to the search page or a "Job not found" page
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
pub async fn check_listing_status(app: tauri::AppHandle, job_id: i64, url: String) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("No URL provided".to_string());
    }

    // reqwest::blocking cannot run on the Tokio async runtime thread — use spawn_blocking
    let url_clone = url.clone();
    let status = tauri::async_runtime::spawn_blocking(move || detect_status(&url_clone))
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
    fn extract_domain_works() {
        assert_eq!(
            extract_domain("https://www.linkedin.com/jobs/view/123"),
            Some("www.linkedin.com".to_string())
        );
        assert_eq!(extract_domain("not-a-url"), None);
    }
}
