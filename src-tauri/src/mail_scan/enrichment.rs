//! Enrichment: fetch the listing page and map it into a `NewJob` partial (spec §5.6, §6.3).
//!
//! The mail digest gives a title, a company, and a link. The deadline, the contact, the
//! salary, and the work mode live on the listing page, so enrichment fetches that page
//! and extracts them.
//!
//! Two rules shape this module:
//!
//! 1. **One normalizer, two callers.** The fetched page goes through exactly the same
//!    prompt, schema, and normalizer as the job form's paste-a-job-ad extraction
//!    ([`crate::llm::client::extract_with_spec`]). A second copy would drift, and the
//!    drift would show up as fields that map correctly in one place and not the other.
//!    That is also why there is no separate `enrich.md`: the task is identical, and the
//!    spec's file list assumed two prompts before the shapes turned out to match.
//! 2. **Failure never loses a listing.** A dead link, a timeout, or a 5 MB page leaves
//!    the match enqueued with `enrichment_state` and a reason. A listing that reached
//!    the inbox has already earned its place there; enrichment only makes it better.

use std::collections::HashMap;

use serde_json::Value;

use crate::llm::provider::ProviderSpec;
use crate::mail_scan::listing_page::{self, Board};
use crate::mail_scan::protocol::ListingEvent;
use crate::mail_scan::status::EnrichmentState;
use crate::net;
use crate::secrets::redact;

/// Fields enrichment exists to fill (spec §14.2). Presence of all of them is
/// `complete`; some is `partial`; the distinction is shown, never hidden.
pub const ENRICHMENT_FIELDS: &[&str] = &[
    "deadline",
    "contact_name",
    "contact_email",
    "workplace_city",
    "work_mode",
    "salary_range",
    "contract_type",
];

/// Upper bound on extracted page text handed to the model.
const MAX_PAGE_CHARS: usize = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enrichment {
    /// Normalized `NewJob` fields. Empty when enrichment failed.
    pub partial: HashMap<String, Value>,
    pub state: EnrichmentState,
    /// Why it is not `complete`, in a sentence the inbox row can show.
    pub error: Option<String>,
    /// The listing page as text (already bounded by [`MAX_PAGE_CHARS`]), kept so the
    /// draft carries the full ad rather than the digest's one-line teaser. Present
    /// whenever the page was fetched, even if the model then failed to read it.
    pub page_text: Option<String>,
    /// The employer's own ad, when the board link led off the board to it. The draft
    /// then uses it as the Job's `url` and keeps the board link as `board_url`.
    pub employer_url: Option<String>,
}

/// Shown on an Indeed match: an expected outcome, not a failure.
pub const NOT_FETCHABLE: &str =
    "listing page not fetchable (the board blocks automated readers); the mail snippet is used";

/// Whether a stored enrichment outcome is the expected "board never serves its page"
/// case rather than a real gap. Read back from the stored reason, so rows written
/// before this distinction existed are recognised without a migration.
pub fn is_snippet_only(state: EnrichmentState, error: Option<&str>) -> bool {
    state == EnrichmentState::Skipped && error == Some(NOT_FETCHABLE)
}

impl Enrichment {
    pub fn skipped() -> Self {
        Self {
            partial: HashMap::new(),
            state: EnrichmentState::Skipped,
            error: None,
            page_text: None,
            employer_url: None,
        }
    }

    /// The board never serves its listing page to us. `skipped` with a reason, so the
    /// run does not count it as an enrichment failure.
    pub fn not_fetchable() -> Self {
        Self {
            error: Some(NOT_FETCHABLE.to_string()),
            ..Self::skipped()
        }
    }

    /// The listing page says the posting is gone. Not a failure: the listing is dropped.
    pub fn closed(reason: impl Into<String>) -> Self {
        Self {
            state: EnrichmentState::Closed,
            ..Self::failed(reason)
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            partial: HashMap::new(),
            state: EnrichmentState::Failed,
            error: Some(redact(&reason.into())),
            page_text: None,
            employer_url: None,
        }
    }

    /// Classify a successful extraction by how much of [`ENRICHMENT_FIELDS`] it found.
    pub fn from_partial(partial: HashMap<String, Value>) -> Self {
        let has = |f: &str| {
            partial
                .get(f)
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty())
        };
        let missing: Vec<&str> = ENRICHMENT_FIELDS.iter().copied().filter(|f| !has(f)).collect();
        let (state, error) = if missing.is_empty() {
            (EnrichmentState::Complete, None)
        } else {
            (
                EnrichmentState::Partial,
                Some(format!("not found on the listing page: {}", missing.join(", "))),
            )
        };
        Self {
            partial,
            state,
            error,
            page_text: None,
            employer_url: None,
        }
    }

    /// Attach the fetched page text. Blank text is dropped, so the draft falls back to
    /// the digest snippet instead of storing nothing.
    pub fn with_page_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        let trimmed = text.trim();
        self.page_text = (!trimmed.is_empty()).then(|| trimmed.to_string());
        self
    }

    pub fn is_usable(&self) -> bool {
        matches!(self.state, EnrichmentState::Complete | EnrichmentState::Partial)
    }
}

pub trait ListingEnricher: Send + Sync {
    fn enrich(&self, listing: &ListingEvent) -> Enrichment;

    /// Enrich from a page [`Self::precheck`] already fetched, so a listing that reaches
    /// the inbox is not fetched twice.
    fn enrich_prefetched(&self, listing: &ListingEvent, _first_page: FetchedPage) -> Enrichment {
        self.enrich(listing)
    }

    /// Fetch the listing's page and say whether the posting is gone. Asked before
    /// scoring, so a dead listing costs no model call. Only a confident "closed" is
    /// [`Precheck::Closed`]: an unreachable or unfetchable page is not evidence.
    fn precheck(&self, _listing: &ListingEvent) -> Precheck {
        Precheck::Unknown
    }
}

/// What the pre-scoring page fetch found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Precheck {
    /// The page says the posting is gone.
    Closed,
    /// The page is live; kept so enrichment can reuse it.
    Open(FetchedPage),
    /// Not fetched, or the fetch failed. Not evidence either way.
    Unknown,
}

/// Whether enrichment will fetch this listing at all. Used to avoid charging the run
/// budget for a listing that is never sent to the model.
pub fn listing_page_fetchable(listing: &ListingEvent) -> bool {
    Board::of(&listing.url) != Board::Indeed
}

// ---------------------------------------------------------------------------
// Seams: the network and the model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPage {
    /// Where the redirects ended — for a Jobindex link, the employer's ad.
    pub final_url: String,
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The fetch guard refused the URL before any request was made.
    Refused(String),
    /// The request was made and failed.
    Failed(String),
}

pub trait PageFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError>;
}

/// Reads a listing page into `NewJob` fields.
pub trait PageReader: Send + Sync {
    fn read(&self, page_text: &str) -> Result<HashMap<String, Value>, String>;
}

/// The production fetcher: [`net::fetch_untrusted`], which re-validates every redirect
/// hop and caps size and time (spec §6.3).
pub struct GuardedFetcher;

impl PageFetcher for GuardedFetcher {
    fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
        net::validate_url_for_untrusted_fetch(url, net::FetchPolicy::default())
            .map_err(FetchError::Refused)?;
        let f = net::fetch_untrusted(url).map_err(FetchError::Failed)?;
        Ok(FetchedPage {
            final_url: f.final_url,
            status: f.status,
            body: f.body,
        })
    }
}

/// The production reader: the job form's extraction prompt and normalizer.
struct LlmPageReader {
    spec: ProviderSpec,
    api_key: String,
}

impl PageReader for LlmPageReader {
    fn read(&self, page_text: &str) -> Result<HashMap<String, Value>, String> {
        crate::llm::client::extract_with_spec(&self.spec, &self.api_key, page_text)
    }
}

pub struct LlmEnricher {
    fetcher: Box<dyn PageFetcher>,
    reader: Box<dyn PageReader>,
}

impl LlmEnricher {
    pub fn new(spec: ProviderSpec, api_key: String) -> Self {
        Self::with_seams(Box::new(GuardedFetcher), Box::new(LlmPageReader { spec, api_key }))
    }

    pub fn with_seams(fetcher: Box<dyn PageFetcher>, reader: Box<dyn PageReader>) -> Self {
        Self { fetcher, reader }
    }

    /// The page the listing text should come from, and the employer ad's URL when the
    /// board link led to one.
    ///
    /// At most one fetch beyond the listing's own link, ever: the second URL comes
    /// from page content, which is attacker-controlled, and it goes through the same
    /// guard as the first.
    ///
    /// `Err` is the reason enrichment failed, in a sentence the inbox row can show.
    fn resolve(
        &self,
        listing: &ListingEvent,
        url: &str,
        prefetched: Option<FetchedPage>,
    ) -> Result<(String, Option<String>), ResolveError> {
        let first = match prefetched.map_or_else(|| self.fetcher.fetch(url), Ok) {
            Ok(page) => page,
            Err(FetchError::Refused(e)) => {
                return Err(ResolveError::Failed(format!("link refused by the fetch guard: {e}")))
            }
            Err(FetchError::Failed(e)) => {
                return Err(ResolveError::Failed(format!("could not fetch the listing: {e}")))
            }
        };
        if page_says_closed(url, &first) {
            return Err(ResolveError::Closed(format!(
                "the listing is closed (HTTP {})",
                first.status
            )));
        }
        if !(200..300).contains(&first.status) {
            return Err(ResolveError::Failed(format!(
                "the listing page returned HTTP {}",
                first.status
            )));
        }

        let board = Board::of(&listing.url);
        if board == Board::LinkedIn {
            let text = listing_page::linkedin_description(&first.body)
                .map(|t| t.chars().take(MAX_PAGE_CHARS).collect())
                .unwrap_or_else(|| page_text(&first.body));
            return Ok((text, None));
        }

        // Redirected off the board: that is the employer's ad (Jobindex `/c?t=`).
        if !listing_page::same_site(&first.final_url, url) {
            return Ok((page_text(&first.body), Some(first.final_url)));
        }
        if board == Board::Other {
            if let Some(next) = listing_page::wrapper_link(&first.body, &first.final_url) {
                if let Ok(ad) = self.fetcher.fetch(&next) {
                    if page_says_closed(&next, &ad) {
                        return Err(ResolveError::Closed(format!(
                            "the employer's ad is closed (HTTP {})",
                            ad.status
                        )));
                    }
                    if (200..300).contains(&ad.status) && !page_text(&ad.body).trim().is_empty() {
                        return Ok((page_text(&ad.body), Some(ad.final_url)));
                    }
                }
                // A hop that fails leaves the board page, which is still worth reading.
            }
        }
        Ok((page_text(&first.body), None))
    }
}

/// Why [`LlmEnricher::resolve`] produced no page text.
enum ResolveError {
    Closed(String),
    Failed(String),
}

fn page_says_closed(requested_url: &str, page: &FetchedPage) -> bool {
    crate::listing_check::is_confidently_closed(requested_url, &page.final_url, page.status, &page.body)
}

/// Strip a fetched page down to text. Never rendered, never injected into the webview
/// (spec §6.3) — the only consumer is the prompt.
pub fn page_text(html: &str) -> String {
    crate::job_search::extract_job_page_text(html, MAX_PAGE_CHARS)
}

impl LlmEnricher {
    fn enrich_from(&self, listing: &ListingEvent, prefetched: Option<FetchedPage>) -> Enrichment {
        // The URL is the extractor's parsed anchor, never a model-supplied string, and
        // the fetcher re-validates it and every redirect hop.
        let url = listing.url.trim();
        if url.is_empty() {
            return Enrichment::failed("the listing has no link to enrich from");
        }
        if !listing_page_fetchable(listing) {
            return Enrichment::not_fetchable();
        }

        let (text, employer_url) = match self.resolve(listing, url, prefetched) {
            Ok(resolved) => resolved,
            Err(ResolveError::Closed(reason)) => return Enrichment::closed(reason),
            Err(ResolveError::Failed(reason)) => return Enrichment::failed(reason),
        };
        if text.trim().is_empty() {
            return Enrichment::failed("the listing page had no readable text");
        }

        let enrichment = match self.reader.read(&text) {
            Ok(partial) => Enrichment::from_partial(strip_unenrichable(partial)),
            Err(e) => Enrichment::failed(format!("could not read the listing page: {e}")),
        };
        // The page is worth keeping even when the model could not read it.
        Enrichment {
            employer_url,
            ..enrichment.with_page_text(text)
        }
    }
}

impl ListingEnricher for LlmEnricher {
    fn precheck(&self, listing: &ListingEvent) -> Precheck {
        let url = listing.url.trim();
        if url.is_empty() || !listing_page_fetchable(listing) {
            return Precheck::Unknown;
        }
        match self.fetcher.fetch(url) {
            Ok(page) if page_says_closed(url, &page) => Precheck::Closed,
            Ok(page) => Precheck::Open(page),
            Err(_) => Precheck::Unknown,
        }
    }

    fn enrich(&self, listing: &ListingEvent) -> Enrichment {
        self.enrich_from(listing, None)
    }

    fn enrich_prefetched(&self, listing: &ListingEvent, first_page: FetchedPage) -> Enrichment {
        self.enrich_from(listing, Some(first_page))
    }
}

/// Drop fields enrichment must never assert.
///
/// `priority` is already dropped by the normalizer; `status` is the user's workflow
/// state and is not something a job ad gets to set (spec §0.1, §5.2).
fn strip_unenrichable(mut partial: HashMap<String, Value>) -> HashMap<String, Value> {
    for forbidden in ["priority", "status", "id", "mail_score"] {
        partial.remove(forbidden);
    }
    partial
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn partial_of(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), json!(*v)))
            .collect()
    }

    #[test]
    fn all_fields_present_is_complete() {
        let full: Vec<(&str, &str)> = ENRICHMENT_FIELDS.iter().map(|f| (*f, "value")).collect();
        let e = Enrichment::from_partial(partial_of(&full));
        assert_eq!(e.state, EnrichmentState::Complete);
        assert_eq!(e.error, None);
    }

    #[test]
    fn some_fields_present_is_partial_and_names_what_is_missing() {
        let e = Enrichment::from_partial(partial_of(&[("deadline", "2026-10-01")]));
        assert_eq!(e.state, EnrichmentState::Partial);
        let reason = e.error.unwrap();
        assert!(reason.contains("salary_range"), "{reason}");
        assert!(
            !reason.contains("deadline"),
            "a field we found must not be listed as missing: {reason}"
        );
    }

    #[test]
    fn a_blank_string_does_not_count_as_found() {
        let e = Enrichment::from_partial(partial_of(&[("deadline", "   ")]));
        assert_eq!(e.state, EnrichmentState::Partial);
        assert!(e.error.unwrap().contains("deadline"));
    }

    #[test]
    fn failure_keeps_the_listing_usable_but_marks_the_state() {
        let e = Enrichment::failed("connection timed out");
        assert_eq!(e.state, EnrichmentState::Failed);
        assert!(e.partial.is_empty());
        assert!(!e.is_usable());
        assert!(e.error.unwrap().contains("timed out"));
    }

    #[test]
    fn partial_and_complete_are_both_usable() {
        assert!(Enrichment::from_partial(partial_of(&[("deadline", "x")])).is_usable());
        let full: Vec<(&str, &str)> = ENRICHMENT_FIELDS.iter().map(|f| (*f, "v")).collect();
        assert!(Enrichment::from_partial(partial_of(&full)).is_usable());
    }

    #[test]
    fn a_listing_page_can_never_set_priority_or_status() {
        // The whole §0.1 correction: a mail score is advisory, and an ad does not get
        // to decide where the job sits in the user's workflow.
        let mut raw = partial_of(&[("deadline", "2026-10-01"), ("company", "Acme")]);
        raw.insert("priority".into(), json!(1));
        raw.insert("status".into(), json!("Applied"));

        let cleaned = strip_unenrichable(raw);

        assert!(!cleaned.contains_key("priority"));
        assert!(!cleaned.contains_key("status"));
        assert_eq!(cleaned.get("deadline").unwrap(), &json!("2026-10-01"));
    }

    #[test]
    fn a_failure_reason_is_redacted() {
        let e = Enrichment::failed("failed with key sk-abcdefghijklmnopqrstuvwxyz0123456789");
        assert!(
            !e.error.unwrap().contains("sk-abcdefghijklmnopqrstuvwxyz0123456789"),
            "enrichment errors reach the inbox row and the run log"
        );
    }

    #[test]
    fn page_text_strips_markup_before_it_reaches_the_prompt() {
        let html = "<html><body><h1>Rust Engineer</h1><p>Apply by 2026-10-01</p>\
                    <script>alert(1)</script></body></html>";
        let text = page_text(html);
        assert!(text.contains("Rust Engineer"));
        assert!(text.contains("2026-10-01"));
        assert!(!text.contains('<'), "no markup may reach the model: {text}");
        assert!(!text.contains("alert("), "scripts must not be read as prose: {text}");
    }
}

/// The fetch guard as enrichment sees it (spec §6.3).
#[cfg(test)]
mod fetch_guard_tests {
    use super::*;
    use crate::llm::provider::{provider_spec, LlmProvider};
    use crate::mail_scan::protocol::FingerprintKeys;

    fn listing_with_url(url: &str) -> ListingEvent {
        ListingEvent {
            source: "indeed".into(),
            message_id: "<m@x>".into(),
            message_date: "2026-09-08T06:12:00Z".into(),
            seq: 0,
            title: "Rust Engineer".into(),
            company: "Acme".into(),
            location: "København".into(),
            url: url.into(),
            external_ref: None,
            snippet: "body".into(),
            posted_at: None,
            fingerprint: FingerprintKeys {
                strong: Some("indeed:x".into()),
                weak: "acme|dev|kbh".into(),
            },
            extractor: "indeed".into(),
            extractor_confidence: 0.9,
        }
    }

    fn enricher() -> LlmEnricher {
        LlmEnricher::new(provider_spec(LlmProvider::Mistral), "sk-test".into())
    }

    #[test]
    fn an_internal_url_is_refused_before_any_request_is_made() {
        // No stub server is listening on any of these; the test passing quickly is
        // itself the evidence that no connection was attempted.
        for hostile in [
            "http://169.254.169.254/latest/meta-data/",
            "http://127.0.0.1:9/",
            "http://10.0.0.1/internal",
            "file:///etc/passwd",
        ] {
            let result = enricher().enrich(&listing_with_url(hostile));
            assert_eq!(result.state, EnrichmentState::Failed, "{hostile}");
            assert!(
                result.error.unwrap().contains("fetch guard"),
                "{hostile} must be refused by the guard, not attempted"
            );
        }
    }

    #[test]
    fn a_listing_with_no_link_fails_cleanly() {
        let result = enricher().enrich(&listing_with_url("   "));
        assert_eq!(result.state, EnrichmentState::Failed);
        assert!(result.error.unwrap().contains("no link"));
    }

    #[test]
    fn a_failed_enrichment_never_yields_fields() {
        let result = enricher().enrich(&listing_with_url("http://127.0.0.1:9/"));
        assert!(
            result.partial.is_empty(),
            "a failed fetch must not leave half-filled fields on the draft"
        );
    }
}

/// Following a board link to the employer's own ad (fake fetcher, fake reader).
#[cfg(test)]
mod following_tests {
    use super::*;
    use crate::mail_scan::protocol::FingerprintKeys;
    use std::sync::{Arc, Mutex};

    /// Serves canned pages by URL and records every URL it was asked for.
    #[derive(Clone, Default)]
    struct FakeFetcher {
        pages: HashMap<String, FetchedPage>,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl FakeFetcher {
        fn page(mut self, url: &str, final_url: &str, body: &str) -> Self {
            self.pages.insert(
                url.to_string(),
                FetchedPage {
                    final_url: final_url.to_string(),
                    status: 200,
                    body: body.to_string(),
                },
            );
            self
        }
        fn status_page(mut self, url: &str, status: u16, body: &str) -> Self {
            self.pages.insert(
                url.to_string(),
                FetchedPage {
                    final_url: url.to_string(),
                    status,
                    body: body.to_string(),
                },
            );
            self
        }
        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    impl PageFetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
            self.asked.lock().unwrap().push(url.to_string());
            self.pages
                .get(url)
                .cloned()
                .ok_or_else(|| FetchError::Refused(format!("no such page in the fake: {url}")))
        }
    }

    /// Stands in for the model: returns a fixed field and records the text it read.
    #[derive(Clone, Default)]
    struct FakeReader {
        read: Arc<Mutex<Vec<String>>>,
    }

    impl PageReader for FakeReader {
        fn read(&self, page_text: &str) -> Result<HashMap<String, Value>, String> {
            self.read.lock().unwrap().push(page_text.to_string());
            Ok(HashMap::from([(
                "deadline".to_string(),
                Value::String("2026-10-01".into()),
            )]))
        }
    }

    fn listing(url: &str, board: &str) -> ListingEvent {
        ListingEvent {
            source: board.into(),
            message_id: "<m@x>".into(),
            message_date: "2026-09-08T06:12:00Z".into(),
            seq: 0,
            title: "Geodata Analyst".into(),
            company: "Acme".into(),
            location: "København".into(),
            url: url.into(),
            external_ref: None,
            snippet: "the mail's one-line teaser".into(),
            posted_at: None,
            fingerprint: FingerprintKeys {
                strong: Some(format!("{board}:1")),
                weak: "acme|geodata analyst|kobenhavn".into(),
            },
            extractor: board.into(),
            extractor_confidence: 0.9,
        }
    }

    fn enrich(fetcher: &FakeFetcher, reader: &FakeReader, l: &ListingEvent) -> Enrichment {
        LlmEnricher::with_seams(Box::new(fetcher.clone()), Box::new(reader.clone())).enrich(l)
    }

    const JOBINDEX: &str = "https://www.jobindex.dk/c?t=h1000001";
    const EMPLOYER_AD: &str = "https://candidate.hr-manager.net/ad/123";

    #[test]
    fn a_jobindex_link_is_followed_to_the_employer_ad() {
        let fetcher = FakeFetcher::default().page(
            JOBINDEX,
            EMPLOYER_AD,
            "<html><body><h1>Geodata Analyst</h1><p>Apply by 1 October.</p></body></html>",
        );
        let reader = FakeReader::default();

        let e = enrich(&fetcher, &reader, &listing(JOBINDEX, "jobindex"));

        assert_eq!(e.employer_url.as_deref(), Some(EMPLOYER_AD));
        assert!(e.page_text.unwrap().contains("Apply by 1 October"));
        assert!(reader.read.lock().unwrap()[0].contains("Geodata Analyst"));
        assert_eq!(e.state, EnrichmentState::Partial);
    }

    #[test]
    fn a_removed_listing_is_closed_and_never_read_by_the_model() {
        for status in [404, 410] {
            let fetcher = FakeFetcher::default().status_page(JOBINDEX, status, "<p>gone</p>");
            let reader = FakeReader::default();

            let e = enrich(&fetcher, &reader, &listing(JOBINDEX, "jobindex"));

            assert_eq!(e.state, EnrichmentState::Closed, "HTTP {status}");
            assert!(reader.read.lock().unwrap().is_empty(), "no model call for a dead page");
        }
    }

    #[test]
    fn the_precheck_flags_only_confidently_closed_pages() {
        let ok = "https://careers.acme.example/jobs/1";
        let gone = "https://careers.acme.example/jobs/2";
        let down = "https://careers.acme.example/jobs/3";
        let fetcher = FakeFetcher::default()
            .page(ok, ok, "<p>Apply now</p>")
            .status_page(gone, 410, "")
            .status_page(down, 503, "");
        let enricher =
            LlmEnricher::with_seams(Box::new(fetcher), Box::new(FakeReader::default()));
        let check = |url: &str| enricher.precheck(&listing(url, "other"));

        assert!(matches!(check(ok), Precheck::Open(_)));
        assert_eq!(check(gone), Precheck::Closed);
        assert_ne!(check(down), Precheck::Closed, "an outage is not a closing");
        assert_eq!(check("https://nowhere.example/x"), Precheck::Unknown);
    }

    #[test]
    fn enrichment_reuses_the_prechecked_page_instead_of_fetching_again() {
        let url = "https://careers.acme.example/jobs/1";
        let fetcher = FakeFetcher::default().page(url, url, "<p>Apply now</p>");
        let enricher =
            LlmEnricher::with_seams(Box::new(fetcher.clone()), Box::new(FakeReader::default()));
        let l = listing(url, "other");

        let Precheck::Open(page) = enricher.precheck(&l) else {
            panic!("a live page is open");
        };
        let e = enricher.enrich_prefetched(&l, page);

        assert_eq!(fetcher.asked(), vec![url.to_string()], "one fetch for both steps");
        assert_eq!(e.page_text.as_deref(), Some("Apply now"));
    }

    #[test]
    fn a_page_that_says_the_position_is_filled_is_closed() {
        let url = "https://careers.acme.example/jobs/9";
        let fetcher = FakeFetcher::default().page(
            url,
            url,
            "<html><body><h1>Sorry, this position has been filled.</h1></body></html>",
        );

        let e = enrich(&fetcher, &FakeReader::default(), &listing(url, "other"));

        assert_eq!(e.state, EnrichmentState::Closed);
    }

    #[test]
    fn a_server_error_is_a_failure_not_a_closed_listing() {
        let fetcher = FakeFetcher::default().status_page(JOBINDEX, 503, "<p>try later</p>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(JOBINDEX, "jobindex"));

        assert_eq!(e.state, EnrichmentState::Failed);
    }

    #[test]
    fn a_live_page_that_only_mentions_404_stays_open() {
        let url = "https://careers.acme.example/jobs/10";
        let fetcher = FakeFetcher::default().page(
            url,
            url,
            "<html><style>.e404{}</style><body><p>Apply now. Ref 404-22.</p></body></html>",
        );

        let e = enrich(&fetcher, &FakeReader::default(), &listing(url, "other"));

        assert_ne!(e.state, EnrichmentState::Closed);
    }

    #[test]
    fn a_closed_employer_ad_behind_a_wrapper_page_is_closed() {
        let board = "https://www.jobbank.dk/job/123";
        let full_ad = "https://careers.acme.example/jobs/42";
        let fetcher = FakeFetcher::default()
            .page(board, board, &format!(r#"<a href="{full_ad}">Apply on the employer site</a>"#))
            .status_page(full_ad, 404, "gone");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "other"));

        assert_eq!(e.state, EnrichmentState::Closed);
    }

    #[test]
    fn a_jobindex_ad_hosted_on_jobindex_keeps_the_board_url() {
        let hosted = "https://www.jobindex.dk/jobannonce/h1000001/geodata-analyst";
        let fetcher = FakeFetcher::default().page(JOBINDEX, hosted, "<p>Full ad on the board</p>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(JOBINDEX, "jobindex"));

        assert_eq!(e.employer_url, None, "the board itself is not an employer ad");
    }

    #[test]
    fn a_linkedin_listing_reads_only_the_job_description() {
        let url = "https://www.linkedin.com/jobs/view/4000000001";
        let page = r#"<html><body><nav>Sign in to see who you know</nav>
            <div class="description__text"><div class="show-more-less-html__markup relative">
              <p>You will build geodata pipelines.</p><div><ul><li>Python</li></ul></div>
            </div></div>
            <footer>LinkedIn Corporation © 2026</footer></body></html>"#;
        let fetcher = FakeFetcher::default().page(url, url, page);

        let e = enrich(&fetcher, &FakeReader::default(), &listing(url, "linkedin"));

        let text = e.page_text.unwrap();
        assert!(text.contains("geodata pipelines"), "{text}");
        assert!(text.contains("Python"), "nested markup inside the description is kept: {text}");
        assert!(!text.contains("Sign in"), "page chrome is not the listing: {text}");
        assert!(!text.contains("LinkedIn Corporation"), "{text}");
        assert_eq!(e.employer_url, None, "guests never see the external apply link");
        assert_eq!(fetcher.asked().len(), 1);
    }

    #[test]
    fn a_linkedin_page_without_the_description_block_falls_back_to_the_whole_page() {
        let url = "https://www.linkedin.com/jobs/view/4000000002";
        let fetcher = FakeFetcher::default().page(url, url, "<body><p>Whole page text</p></body>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(url, "linkedin"));

        assert!(e.page_text.unwrap().contains("Whole page text"));
    }

    #[test]
    fn an_indeed_listing_is_not_fetched_and_says_so_without_failing() {
        let url = "https://dk.indeed.com/viewjob?jk=0123456789abcdef";
        let fetcher = FakeFetcher::default();
        let reader = FakeReader::default();

        let e = enrich(&fetcher, &reader, &listing(url, "indeed"));

        assert!(fetcher.asked().is_empty(), "Indeed answers bots with a 401 challenge");
        assert!(reader.read.lock().unwrap().is_empty());
        assert_ne!(e.state, EnrichmentState::Failed, "an expected outcome is not a failure");
        assert!(e.error.unwrap().contains("not fetchable"));
    }

    #[test]
    fn a_wrapper_page_is_followed_one_hop_to_the_full_ad() {
        let board = "https://www.jobbank.dk/job/123";
        let full_ad = "https://careers.acme.example/jobs/42";
        let fetcher = FakeFetcher::default()
            .page(
                board,
                board,
                r#"<body><p>Geodata Analyst hos Acme</p>
                   <a href="https://www.facebook.com/share">Del</a>
                   <a href="https://careers.acme.example/jobs/42">Se hele annoncen</a></body>"#,
            )
            .page(full_ad, full_ad, "<body><h1>Geodata Analyst</h1><p>The full ad.</p></body>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "jobbank"));

        assert_eq!(fetcher.asked(), vec![board.to_string(), full_ad.to_string()]);
        assert_eq!(e.employer_url.as_deref(), Some(full_ad));
        assert!(e.page_text.unwrap().contains("The full ad."));
    }

    #[test]
    fn a_relative_wrapper_link_is_resolved_against_the_page() {
        let board = "https://www.jobbank.dk/job/123";
        let full_ad = "https://www.jobbank.dk/job/123/full";
        let fetcher = FakeFetcher::default()
            .page(board, board, r#"<a href="/job/123/full">Læs hele jobopslaget</a>"#)
            .page(full_ad, full_ad, "<p>The full ad.</p>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "jobbank"));

        assert_eq!(fetcher.asked().last().map(String::as_str), Some(full_ad));
        assert!(e.page_text.unwrap().contains("The full ad."));
    }

    #[test]
    fn a_single_dominant_external_link_on_a_thin_page_is_followed() {
        let board = "https://jobs.example-board.dk/l/9";
        let full_ad = "https://acme.example/careers/9";
        let fetcher = FakeFetcher::default()
            .page(
                board,
                board,
                r#"<body><p>Geodata Analyst</p><a href="/">Home</a>
                   <a href="https://acme.example/careers/9">Acme</a>
                   <a href="https://twitter.com/board">Follow us</a></body>"#,
            )
            .page(full_ad, full_ad, "<p>The full ad.</p>");

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "other"));

        assert_eq!(e.employer_url.as_deref(), Some(full_ad));
    }

    #[test]
    fn never_more_than_one_extra_hop() {
        let board = "https://www.jobbank.dk/job/1";
        let hop1 = "https://a.example/1";
        let fetcher = FakeFetcher::default()
            .page(board, board, r#"<a href="https://a.example/1">Gå til annoncen</a>"#)
            .page(hop1, hop1, r#"<p>Still a teaser</p><a href="https://b.example/2">Apply on company site</a>"#);

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "jobbank"));

        assert_eq!(fetcher.asked().len(), 2, "{:?}", fetcher.asked());
        assert_eq!(e.employer_url.as_deref(), Some(hop1));
        assert!(e.page_text.unwrap().contains("Still a teaser"));
    }

    #[test]
    fn a_hop_that_cannot_be_fetched_keeps_the_first_page() {
        let board = "https://www.jobbank.dk/job/2";
        let fetcher = FakeFetcher::default().page(
            board,
            board,
            r#"<p>Teaser text</p><a href="http://127.0.0.1/admin">Se hele annoncen</a>"#,
        );

        let e = enrich(&fetcher, &FakeReader::default(), &listing(board, "jobbank"));

        assert_eq!(e.state, EnrichmentState::Partial, "the listing is still enriched from the board page");
        assert_eq!(e.employer_url, None);
        assert!(e.page_text.unwrap().contains("Teaser text"));
    }

    #[test]
    fn a_non_web_wrapper_link_is_never_followed() {
        let board = "https://www.jobbank.dk/job/3";
        let fetcher = FakeFetcher::default().page(
            board,
            board,
            r#"<p>Teaser</p><a href="mailto:hr@acme.example">Se hele annoncen</a>"#,
        );

        enrich(&fetcher, &FakeReader::default(), &listing(board, "jobbank"));

        assert_eq!(fetcher.asked(), vec![board.to_string()]);
    }
}
