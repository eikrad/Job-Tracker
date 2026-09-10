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
use crate::mail_scan::protocol::ListingEvent;
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
    /// `complete` | `partial` | `failed` | `skipped`
    pub state: &'static str,
    /// Why it is not `complete`, in a sentence the inbox row can show.
    pub error: Option<String>,
}

impl Enrichment {
    pub fn skipped() -> Self {
        Self {
            partial: HashMap::new(),
            state: "skipped",
            error: None,
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            partial: HashMap::new(),
            state: "failed",
            error: Some(redact(&reason.into())),
        }
    }

    /// Classify a successful extraction by how much of [`ENRICHMENT_FIELDS`] it found.
    pub fn from_partial(partial: HashMap<String, Value>) -> Self {
        let found = ENRICHMENT_FIELDS
            .iter()
            .filter(|f| {
                partial
                    .get(**f)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.trim().is_empty())
            })
            .count();

        if found == ENRICHMENT_FIELDS.len() {
            return Self {
                partial,
                state: "complete",
                error: None,
            };
        }
        let missing: Vec<&str> = ENRICHMENT_FIELDS
            .iter()
            .copied()
            .filter(|f| {
                !partial
                    .get(*f)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.trim().is_empty())
            })
            .collect();
        Self {
            partial,
            state: "partial",
            error: Some(format!("not found on the listing page: {}", missing.join(", "))),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self.state, "complete" | "partial")
    }
}

pub trait ListingEnricher: Send + Sync {
    fn enrich(&self, listing: &ListingEvent) -> Enrichment;
}

pub struct LlmEnricher {
    spec: ProviderSpec,
    api_key: String,
}

impl LlmEnricher {
    pub fn new(spec: ProviderSpec, api_key: String) -> Self {
        Self { spec, api_key }
    }
}

/// Strip a fetched page down to text. Never rendered, never injected into the webview
/// (spec §6.3) — the only consumer is the prompt.
pub fn page_text(html: &str) -> String {
    crate::job_search::extract_job_page_text(html, MAX_PAGE_CHARS)
}

impl ListingEnricher for LlmEnricher {
    fn enrich(&self, listing: &ListingEvent) -> Enrichment {
        // The URL is the extractor's parsed anchor, never a model-supplied string, and
        // it is re-validated here and again after every redirect inside `fetch_untrusted`.
        let url = listing.url.trim();
        if url.is_empty() {
            return Enrichment::failed("the listing has no link to enrich from");
        }
        if let Err(e) = net::validate_url_for_untrusted_fetch(url, net::FetchPolicy::default()) {
            return Enrichment::failed(format!("link refused by the fetch guard: {e}"));
        }

        let fetched = match net::fetch_untrusted(url) {
            Ok(f) => f,
            Err(e) => return Enrichment::failed(format!("could not fetch the listing: {e}")),
        };
        if !(200..300).contains(&fetched.status) {
            return Enrichment::failed(format!(
                "the listing page returned HTTP {}",
                fetched.status
            ));
        }

        let text = page_text(&fetched.body);
        if text.trim().is_empty() {
            return Enrichment::failed("the listing page had no readable text");
        }

        match crate::llm::client::extract_with_spec(&self.spec, &self.api_key, &text) {
            Ok(partial) => Enrichment::from_partial(strip_unenrichable(partial)),
            Err(e) => Enrichment::failed(format!("could not read the listing page: {e}")),
        }
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
        assert_eq!(e.state, "complete");
        assert_eq!(e.error, None);
    }

    #[test]
    fn some_fields_present_is_partial_and_names_what_is_missing() {
        let e = Enrichment::from_partial(partial_of(&[("deadline", "2026-10-01")]));
        assert_eq!(e.state, "partial");
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
        assert_eq!(e.state, "partial");
        assert!(e.error.unwrap().contains("deadline"));
    }

    #[test]
    fn failure_keeps_the_listing_usable_but_marks_the_state() {
        let e = Enrichment::failed("connection timed out");
        assert_eq!(e.state, "failed");
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
            assert_eq!(result.state, "failed", "{hostile}");
            assert!(
                result.error.unwrap().contains("fetch guard"),
                "{hostile} must be refused by the guard, not attempted"
            );
        }
    }

    #[test]
    fn a_listing_with_no_link_fails_cleanly() {
        let result = enricher().enrich(&listing_with_url("   "));
        assert_eq!(result.state, "failed");
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
