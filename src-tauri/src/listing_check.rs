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

    let body = fetched
        .body
        .chars()
        .take(4096)
        .collect::<String>()
        .to_lowercase();

    classify_fetched(url, &fetched.final_url, fetched.status, &fetched.body, &body)
}

/// The older head-of-page rules first, so Archived and Unreachable keep their meaning;
/// a page they call Active is then held against the stricter closed check, which reads
/// the whole page and catches banners the head never reaches.
fn classify_fetched(
    original_url: &str,
    final_url: &str,
    status: u16,
    full_body: &str,
    body_head: &str,
) -> ListingStatus {
    match classify_http_result(original_url, final_url, status, body_head) {
        ListingStatus::Active if is_confidently_closed(original_url, final_url, status, full_body) => {
            ListingStatus::Closed
        }
        other => other,
    }
}

/// Classify a fetched listing page. Separated from network I/O so HTTP status
/// quirks (Jobindex 404 for removed ads) are unit-testable.
fn classify_http_result(
    original_url: &str,
    final_url: &str,
    status: u16,
    body_head: &str,
) -> ListingStatus {
    let domain = extract_domain(original_url).unwrap_or_default();

    // Success and redirects that landed on a document.
    if (200..400).contains(&status) {
        return classify_by_domain(original_url, final_url, body_head);
    }

    // Job boards often return 404 HTML for removed listings — that is Closed,
    // not Unreachable (which means "we couldn't tell").
    if status == 404 {
        if domain.contains("jobindex.dk") {
            return classify_jobindex(final_url, status, body_head);
        }
        if domain.contains("linkedin.com") {
            return classify_linkedin(final_url, body_head);
        }
        let generic = classify_generic(final_url, body_head);
        if generic == ListingStatus::Closed {
            return ListingStatus::Closed;
        }
        // Bare 404 with no phrase still means the listing is gone.
        return ListingStatus::Closed;
    }

    ListingStatus::Unreachable
}

/// Whether a fetched listing page says the posting is gone.
///
/// Deliberately stricter than [`classify_http_result`]: the mail scan drops a listing on
/// a `true`, so a page that merely mentions "404" or "page not found" somewhere in its
/// markup must not count. Only a gone/not-found status, a redirect away from the ad, or
/// an explicit closed phrase does.
pub fn is_confidently_closed(
    original_url: &str,
    final_url: &str,
    status: u16,
    body: &str,
) -> bool {
    if status == 404 || status == 410 {
        return true;
    }
    if !(200..400).contains(&status) {
        return false;
    }
    // Board rules read the page we ended on, not the link we started from: a Jobindex
    // tracking link that lands on an employer site is the employer's page.
    let Ok(landed) = url::Url::parse(final_url) else {
        return false;
    };
    let on_linkedin = on_site(&landed, "linkedin.com");
    let on_jobindex = on_site(&landed, "jobindex.dk");
    if on_linkedin && landed.path().contains("/expired") {
        return true;
    }
    if on_jobindex && landed.path().starts_with("/arkiv") {
        return true;
    }
    if landed.query_pairs().any(|(k, v)| k == "not_found" && v == "true") {
        return true;
    }
    if redirected_up_the_path(original_url, &landed) {
        return true;
    }

    // Phrases are matched against the visible text, not the markup: a live single-page
    // app carries its "job not found" strings in scripts. Only the top of the page, so a
    // listing that mentions closed applications in its own body text is left alone.
    let top: String = crate::job_search::extract_job_page_text(body, 20_000)
        .chars()
        .take(3_000)
        .collect::<String>()
        .to_lowercase();
    const PHRASES: &[&str] = &[
        "no longer accepting",
        "job is no longer available",
        "job no longer available",
        "this job has expired",
        "this position is no longer",
        "position has been filled",
        "vacancy has been filled",
        "position has been closed",
        "job posting has expired",
        "stillingen er besat",
        "stillingen er ikke online",
        "jobbet er ikke længere aktivt",
        "opslaget er udløbet",
        "annoncen er udløbet",
        "siden kan ikke findes",
    ];
    if PHRASES.iter().any(|p| top.contains(p)) {
        return true;
    }
    // Jobindex serves an expired ad as a normal 200 page with a banner, and LinkedIn
    // puts "No longer accepting applications" well past the head. Both phrases are
    // specific enough to search the whole body, but only on their own domain.
    let lower_body = body.to_lowercase();
    (on_jobindex && lower_body.contains("annoncen er udløbet"))
        || (on_linkedin && lower_body.contains("no longer accepting applications"))
}

/// Whether `url` is on `site` or one of its subdomains — never a lookalike such as
/// `notlinkedin.com` or a path that merely mentions the site.
fn on_site(url: &url::Url, site: &str) -> bool {
    url.host_str().is_some_and(|host| {
        let host = host.to_ascii_lowercase();
        host == site || host.ends_with(&format!(".{site}"))
    })
}

/// A job page that redirects to its site's root or to a parent path is a removed ad:
/// employers send dead links to the careers index or the home page.
fn redirected_up_the_path(original_url: &str, fin: &url::Url) -> bool {
    let Ok(orig) = url::Url::parse(original_url) else {
        return false;
    };
    let host = |u: &url::Url| u.host_str().map(|h| h.trim_start_matches("www.").to_lowercase());
    if host(&orig).is_none() || host(&orig) != host(fin) {
        return false;
    }
    let segments = |u: &url::Url| -> Vec<String> {
        u.path_segments()
            .map(|it| it.filter(|s| !s.is_empty()).map(str::to_lowercase).collect())
            .unwrap_or_default()
    };
    let (o, f) = (segments(&orig), segments(fin));
    f.len() < o.len() && o[..f.len()] == f[..]
}

fn classify_by_domain(original_url: &str, final_url: &str, body_head: &str) -> ListingStatus {
    let domain = extract_domain(original_url).unwrap_or_default();

    if domain.contains("linkedin.com") {
        classify_linkedin(final_url, body_head)
    } else if domain.contains("jobindex.dk") {
        classify_jobindex(final_url, 200, body_head)
    } else {
        classify_generic(final_url, body_head)
    }
}

fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
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

fn classify_jobindex(final_url: &str, status: u16, body_head: &str) -> ListingStatus {
    // Legacy archive redirect (still honour if Jobindex serves it).
    if final_url.contains("/arkiv") || final_url.contains("jobindex.dk/arkiv") {
        return ListingStatus::Archived;
    }
    // Current behaviour: removed ads return HTTP 404 with "Siden kan ikke findes".
    if status == 404
        || body_head.contains("siden kan ikke findes")
        || body_head.contains("page not found")
        || body_head.contains("jobbet er ikke længere aktivt")
    {
        return ListingStatus::Closed;
    }
    ListingStatus::Active
}

fn classify_generic(final_url: &str, body_head: &str) -> ListingStatus {
    let _ = final_url;
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
        "siden kan ikke findes",
    ];
    if closed_signals.iter().any(|s| body_head.contains(s)) {
        return ListingStatus::Closed;
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
    fn the_button_catches_banners_past_the_head_and_keeps_archived() {
        let filler = "<div>x</div>".repeat(3000);
        let ji = "https://www.jobindex.dk/vis-job/r1";
        let expired = format!("{filler}<h2>Annoncen er udløbet!</h2>");
        assert_eq!(classify_fetched(ji, ji, 200, &expired, "<div>x</div>"), ListingStatus::Closed);
        assert_eq!(classify_fetched(ji, ji, 200, &filler, "<div>x</div>"), ListingStatus::Active);
        assert_eq!(
            classify_fetched(ji, "https://www.jobindex.dk/arkiv/1", 200, "", ""),
            ListingStatus::Archived
        );
        assert_eq!(classify_fetched(ji, ji, 429, "", ""), ListingStatus::Unreachable);
    }

    #[test]
    fn confident_closed_needs_a_gone_status_or_an_explicit_phrase() {
        let u = "https://example.com/job/1";
        assert!(is_confidently_closed(u, u, 404, ""));
        assert!(is_confidently_closed(u, u, 410, ""));
        assert!(is_confidently_closed(u, u, 200, "<h1>This job has expired</h1>"));
        assert!(!is_confidently_closed(u, u, 200, "<p>ref 404 page not found in css</p>"));
        assert!(!is_confidently_closed(u, u, 503, "no longer accepting"));
    }

    #[test]
    fn closed_banners_deep_in_a_board_page_are_found() {
        let filler = "<div>x</div>".repeat(3000);
        let jobindex = "https://www.jobindex.dk/vis-job/r1";
        let expired = format!("{filler}<h2>Annoncen er udløbet!</h2>");
        assert!(is_confidently_closed(jobindex, jobindex, 200, &expired));
        assert!(!is_confidently_closed(jobindex, jobindex, 200, &filler));

        let linkedin = "https://www.linkedin.com/jobs/view/1";
        let closed = format!("{filler}<figcaption>No longer accepting applications</figcaption>");
        assert!(is_confidently_closed(linkedin, linkedin, 200, &closed));
        assert!(!is_confidently_closed(linkedin, linkedin, 200, &filler));
        // The whole-body phrases are tied to their own board.
        let other = "https://careers.acme.example/1";
        assert!(!is_confidently_closed(other, other, 200, &expired));
        // A board's tracking link that lands on an employer page is the employer's page.
        assert!(!is_confidently_closed("https://www.jobindex.dk/c?t=1", other, 200, &expired));
    }

    #[test]
    fn board_url_rules_need_the_board_host_not_a_mention_of_it() {
        let job = "https://careers.acme.example/jobs/1";
        let lookalike = "https://notlinkedin.com/jobs/view/1/expired";
        assert!(!is_confidently_closed(lookalike, lookalike, 200, ""));
        let mention = "https://careers.acme.example/jobs/1?from=jobindex.dk/arkiv";
        assert!(!is_confidently_closed(job, mention, 200, ""));
        let param = "https://careers.acme.example/jobs/1?ref=not_found=true";
        assert!(!is_confidently_closed(job, param, 200, ""));
        let li = "https://dk.linkedin.com/jobs/view/1";
        assert!(is_confidently_closed(li, "https://dk.linkedin.com/jobs/view/1/expired", 200, ""));
    }

    #[test]
    fn a_redirect_up_the_path_or_to_not_found_is_closed() {
        let job = "https://apply.workable.com/acme/j/F3DE6D5215/";
        assert!(is_confidently_closed(job, "https://apply.workable.com/acme/?not_found=true", 200, ""));
        assert!(is_confidently_closed(job, "https://apply.workable.com/acme/", 200, ""));
        assert!(is_confidently_closed(job, "https://apply.workable.com/", 200, ""));
        // Same depth, a longer canonical path, another site, or a language prefix: still open.
        assert!(!is_confidently_closed(job, job, 200, ""));
        assert!(!is_confidently_closed("https://x.dk/job/1", "https://x.dk/job/1/title", 200, ""));
        assert!(!is_confidently_closed("https://www.jobindex.dk/c?t=1", "https://employer.dk/ad/1", 200, ""));
        assert!(!is_confidently_closed("https://x.dk/da/jobs/1", "https://x.dk/jobs/1", 200, ""));
    }

    #[test]
    fn closed_phrases_count_in_visible_text_but_not_in_scripts_or_deep_in_the_ad() {
        let u = "https://careers.acme.example/job/1";
        let shell = "<html><body><nav>Jobs</nav><p>Sorry, this position has been filled.</p></body></html>";
        assert!(is_confidently_closed(u, u, 200, shell));
        let hr = "<body><div>Info</div><p>Stillingen er ikke online.</p></body>";
        assert!(is_confidently_closed(u, u, 200, hr));
        let script = r#"<body><script>var t={"err":"This job is no longer available"}</script><p>Apply</p></body>"#;
        assert!(!is_confidently_closed(u, u, 200, script));
        let deep = format!("<body><p>{}</p><p>we are no longer accepting agency CVs</p></body>", "word ".repeat(1500));
        assert!(!is_confidently_closed(u, u, 200, &deep));
    }

    #[test]
    fn jobindex_http_404_is_closed_not_unreachable() {
        // Live Jobindex removed ads return 404 HTML titled "Siden kan ikke findes".
        // Treating that as Unreachable was the user-visible bug.
        assert_eq!(
            classify_http_result(
                "https://www.jobindex.dk/jobannonce/1323456",
                "https://www.jobindex.dk/jobannonce/1323456",
                404,
                "<title>Siden kan ikke findes | Jobindex</title>"
            ),
            ListingStatus::Closed
        );
    }

    #[test]
    fn jobindex_annonce_path_active_on_200() {
        assert_eq!(
            classify_http_result(
                "https://www.jobindex.dk/jobannonce/999001",
                "https://www.jobindex.dk/jobannonce/999001",
                200,
                "<title>Software Engineer - Acme | Jobindex</title>"
            ),
            ListingStatus::Active
        );
    }

    #[test]
    fn generic_http_404_is_closed() {
        assert_eq!(
            classify_http_result(
                "https://careers.example.com/jobs/42",
                "https://careers.example.com/jobs/42",
                404,
                "<title>Not Found</title>"
            ),
            ListingStatus::Closed
        );
    }

    #[test]
    fn http_500_is_unreachable() {
        assert_eq!(
            classify_http_result(
                "https://example.com/jobs/42",
                "https://example.com/jobs/42",
                500,
                "internal error"
            ),
            ListingStatus::Unreachable
        );
    }

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
            classify_linkedin(
                "https://linkedin.com/jobs/view/123",
                "<title>Software Engineer at Acme</title>"
            ),
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
            classify_jobindex("https://www.jobindex.dk/job/arkiv/123456", 200, ""),
            ListingStatus::Archived
        );
    }

    #[test]
    fn jobindex_active_on_normal_url() {
        assert_eq!(
            classify_jobindex("https://www.jobindex.dk/job/123456", 200, ""),
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
            classify_generic(
                "https://example.com/jobs/42",
                "<title>Software Engineer - Acme</title>"
            ),
            ListingStatus::Active
        );
    }

    #[test]
    #[ignore = "hits the live Jobindex network; run with --ignored"]
    fn live_jobindex_removed_ad_is_closed() {
        let status = detect_status("https://www.jobindex.dk/jobannonce/1323456", "");
        assert_eq!(
            status,
            ListingStatus::Closed,
            "removed Jobindex ads currently 404; must not report unreachable"
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
            #[serde(default)]
            http_status: Option<u16>,
        }
        let raw = include_str!("../fixtures/listing_status_baseline.json");
        let cases: Vec<Case> = serde_json::from_str(raw).expect("baseline fixture");
        assert!(cases.len() >= 7, "baseline should cover several boards");

        for case in cases {
            let status = case
                .http_status
                .unwrap_or(200);
            let got = if extract_domain(&case.original_url)
                .unwrap_or_default()
                .contains("indeed.com")
            {
                classify_indeed(&case.original_url, &case.final_url, &case.body_head)
            } else {
                classify_http_result(&case.original_url, &case.final_url, status, &case.body_head)
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
