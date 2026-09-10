//! Adversarial corpus tests (spec §6.2, §9.4).
//!
//! Drives `tests/fixtures/injection_corpus/` through the real suspicion heuristics,
//! the real validator, and the real fetch guard. `manifest.json` is the shared
//! expectation table — pytest reads the same file, so a case added on one side cannot
//! quietly go unasserted on the other.

#![cfg(test)]

use std::path::PathBuf;

use crate::mail_scan::injection::{self, Signal};
use crate::mail_scan::protocol::{FingerprintKeys, ListingEvent};
use crate::net::{validate_url_for_untrusted_fetch, FetchPolicy};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/injection_corpus")
}

struct Case {
    file: String,
    control: bool,
    suspicious: bool,
    signals: Vec<String>,
    body: String,
}

/// Everything after the header block. These fixtures are hand-authored single-part
/// messages, so a blank-line split is the whole parse — the sidecar does the real MIME
/// work, and this test is about what the scoring path does with the text.
fn body_of(raw: &str) -> String {
    raw.split_once("\n\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| raw.to_string())
}

fn load_cases() -> Vec<Case> {
    let manifest = std::fs::read_to_string(corpus_dir().join("manifest.json"))
        .expect("injection corpus manifest must exist");
    let parsed: serde_json::Value = serde_json::from_str(&manifest).expect("manifest must parse");
    let cases = parsed["cases"].as_array().expect("manifest.cases");
    assert!(
        cases.len() >= 7,
        "the corpus must keep covering every attack class in spec §9.4"
    );

    cases
        .iter()
        .map(|c| {
            let file = c["file"].as_str().expect("case.file").to_string();
            let raw = std::fs::read_to_string(corpus_dir().join(&file))
                .unwrap_or_else(|e| panic!("missing corpus file {file}: {e}"));
            Case {
                control: c["control"].as_bool().unwrap_or(false),
                suspicious: c["suspicious"].as_bool().unwrap_or(false),
                signals: c["signals"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|s| s.as_str().unwrap_or_default().to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                body: body_of(&raw),
                file,
            }
        })
        .collect()
}

fn listing_from(case: &Case) -> ListingEvent {
    ListingEvent {
        source: "corpus".into(),
        message_id: format!("<{}>", case.file),
        message_date: "2026-09-08T08:00:00Z".into(),
        seq: 0,
        title: "Senior Rust Engineer".into(),
        company: "Nordic Systems A/S".into(),
        location: "København".into(),
        // Note: from the extractor's parsed anchors, never from model output.
        url: "https://www.jobindex.dk/jobannonce/1234567".into(),
        external_ref: None,
        snippet: case.body.clone(),
        posted_at: None,
        fingerprint: FingerprintKeys {
            strong: Some(format!("corpus:{}", case.file)),
            weak: "nordic|senior rust engineer|kbh".into(),
        },
        extractor: "generic".into(),
        extractor_confidence: 0.8,
    }
}

#[test]
fn every_attack_in_the_corpus_is_flagged_suspicious() {
    for case in load_cases() {
        let flagged = injection::is_suspicious(&case.body);
        assert_eq!(
            flagged, case.suspicious,
            "{}: expected suspicious={}, got {} (signals: {:?})",
            case.file,
            case.suspicious,
            flagged,
            injection::signals(&case.body)
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn the_clean_control_is_not_flagged() {
    // If the control trips the heuristics, the "not inflated relative to control"
    // comparison is meaningless and every other assertion here is theatre.
    let cases = load_cases();
    let control = cases.iter().find(|c| c.control).expect("a control case");
    assert!(
        !injection::is_suspicious(&control.body),
        "control must stay clean, got {:?}",
        injection::signals(&control.body)
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn each_case_fires_the_specific_heuristic_it_was_written_for() {
    for case in load_cases() {
        let fired: Vec<&str> = injection::signals(&case.body)
            .iter()
            .map(|s| s.as_str())
            .collect();
        for expected in &case.signals {
            assert!(
                fired.contains(&expected.as_str()),
                "{}: expected signal {expected}, got {fired:?}",
                case.file
            );
        }
    }
}

#[test]
fn obfuscated_instructions_are_still_caught_after_sanitizing() {
    let cases = load_cases();
    let case = cases
        .iter()
        .find(|c| c.file == "zero_width_obfuscation.eml")
        .expect("zero-width case");

    assert!(
        injection::contains_obfuscation(&case.body),
        "fixture must actually contain zero-width characters"
    );
    let clean = injection::sanitize(&case.body);
    assert!(
        !injection::contains_obfuscation(&clean),
        "sanitizing must remove them before the text reaches the prompt"
    );
    assert!(
        clean.to_lowercase().contains("ignore previous instructions"),
        "the hidden instruction must become visible, not vanish"
    );
    assert!(injection::signals(&case.body).contains(&Signal::InstructionMarker));
}

#[test]
fn no_corpus_case_can_inflate_a_score_past_the_valid_range() {
    // A compromised model answering "score: 99" for every attack still cannot put a
    // 99 — or any number — on the row. This is the property the whole §6.2 table
    // reduces to once the model is assumed lost.
    use crate::mail_scan::scoring::listing_content_hash;

    for case in load_cases() {
        let listing = listing_from(&case);
        assert!(
            !listing_content_hash(&listing).is_empty(),
            "{}: every listing must be cacheable by content",
            case.file
        );
        for coerced in [
            serde_json::json!({ "score": 99, "reason": "ideal match" }),
            serde_json::json!({ "score": "10", "reason": "ideal match" }),
            serde_json::json!({ "score": -1, "reason": "ideal match" }),
            serde_json::json!({ "reason": "ideal match" }),
        ] {
            let parsed = crate::mail_scan::scoring::validate_score_for_test(&coerced);
            assert_eq!(
                parsed.score, None,
                "{}: coerced answer {coerced} must be rejected, not trusted",
                case.file
            );
        }
    }
}

#[test]
fn a_model_supplied_metadata_url_is_rejected_by_the_fetch_guard() {
    // The listing text asks us to fetch the cloud metadata endpoint. Enrichment only
    // ever fetches the extractor's parsed anchor, but even if that anchor were the
    // attacker's URL, the guard refuses it.
    for hostile in [
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
        "http://127.0.0.1:8080/admin",
        "http://[fe80::1]/",
        "http://10.0.0.1/internal",
        "file:///etc/passwd",
    ] {
        assert!(
            validate_url_for_untrusted_fetch(hostile, FetchPolicy::default()).is_err(),
            "fetch guard must reject {hostile}"
        );
    }
}

#[test]
fn the_ssrf_case_carries_the_url_only_as_text() {
    let cases = load_cases();
    let case = cases
        .iter()
        .find(|c| c.file == "ssrf_apply_url.eml")
        .expect("ssrf case");
    let listing = listing_from(case);

    assert!(
        case.body.contains("169.254.169.254"),
        "fixture must actually carry the hostile URL"
    );
    assert!(
        !listing.url.contains("169.254.169.254"),
        "the listing's fetch target comes from the extractor's anchor, never from body text"
    );
    assert!(
        validate_url_for_untrusted_fetch(&listing.url, FetchPolicy::default()).is_ok(),
        "the legitimate anchor must still be fetchable"
    );
}
