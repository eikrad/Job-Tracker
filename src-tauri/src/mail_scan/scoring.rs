//! Stub scorer / enricher seam for PR C.

use crate::mail_scan::protocol::ListingEvent;

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub score: i32,
    pub reason: String,
    pub score_state: &'static str,
    pub enrichment_state: &'static str,
    pub outcome: &'static str,
}

pub trait ListingScorer: Send + Sync {
    fn score(&self, listing: &ListingEvent) -> ScoreResult;
}

/// Fixed advisory score — no LLM (PR B).
pub struct StubScorer;

impl ListingScorer for StubScorer {
    fn score(&self, listing: &ListingEvent) -> ScoreResult {
        let _ = listing;
        ScoreResult {
            score: 5,
            reason: "stub score (mail scan PR B)".into(),
            score_state: "ok",
            enrichment_state: "skipped",
            outcome: "inbox",
        }
    }
}
