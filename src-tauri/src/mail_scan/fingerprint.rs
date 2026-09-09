//! Tiered fingerprint helpers (spec §5.1) — must match Python + TS fixtures.

#![allow(dead_code)] // Used by tests now; persist/clustering wiring lands with dismissals/cursors.

pub fn normalize_text(value: &str) -> String {
    let mut text = strip_diacritics(value);
    text = text.to_lowercase();
    text = strip_gender_marks(&text);
    text = strip_remote_suffix(&text);
    loop {
        let stripped = strip_legal_suffix(&text);
        let stripped = stripped.trim_end_matches([' ', '.', ',']).to_string();
        if stripped == text {
            break;
        }
        text = stripped;
    }
    text = collapse_punct_and_ws(&text);
    text
}

fn strip_diacritics(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            'ø' | 'Ø' => out.push('o'),
            'å' | 'Å' => out.push('a'),
            'ä' | 'Ä' => out.push('a'),
            'ö' | 'Ö' => out.push('o'),
            'ü' | 'Ü' => out.push('u'),
            'æ' | 'Æ' => out.push_str("ae"),
            'ß' => out.push_str("ss"),
            // Common combining leftovers after NFKD-style maps above.
            '\u{0300}'..='\u{036f}' => {}
            other => out.push(other),
        }
    }
    out
}

fn strip_gender_marks(text: &str) -> String {
    let mut out = text.to_string();
    for mark in ["(m/w/d)", "(m/f/d)", "(m/w)", "(w/m/d)", "(f/m/d)"] {
        out = out.replace(mark, "");
    }
    out
}

fn strip_remote_suffix(text: &str) -> String {
    // After lowercasing: " - remote" or " – remote"
    for sep in ["- remote", "\u{2013} remote"] {
        if let Some(i) = text.find(sep) {
            // ensure start is boundary-ish
            if i == 0 || text.as_bytes().get(i - 1).is_some_and(|b| b.is_ascii_whitespace()) {
                return text[..i].trim_end().to_string();
            }
        }
    }
    // Also match "–remote" / "-remote" without requiring space before sep when preceded by space
    if let Some(i) = text.rfind(" remote") {
        let before = &text[..i];
        if before.ends_with('-') || before.ends_with('\u{2013}') {
            return before.trim_end_matches(['-', '\u{2013}', ' ']).to_string();
        }
    }
    text.to_string()
}

fn strip_legal_suffix(text: &str) -> String {
    let suffixes = ["a/s", "aps", "gmbh", "ivs", "ab", "as", "ltd", "inc"];
    let trimmed = text.trim_end();
    for suf in suffixes {
        if let Some(rest) = trimmed.strip_suffix(suf) {
            let boundary_ok = rest.is_empty()
                || rest.ends_with(|c: char| c.is_whitespace() || c == '.');
            if boundary_ok {
                return rest.trim_end_matches([' ', '.']).trim_end().to_string();
            }
        }
    }
    trimmed.to_string()
}

fn collapse_punct_and_ws(text: &str) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' || c == '|' {
            out.push(c);
            last_space = false;
        } else if (c.is_whitespace() || !c.is_control()) && !last_space && !out.is_empty() {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

pub fn weak_key(company: &str, title: &str, location: &str) -> String {
    format!(
        "{}|{}|{}",
        normalize_text(company),
        normalize_text(title),
        normalize_text(location)
    )
}

pub fn canonical_url(url: &str) -> String {
    let raw = url.trim();
    let (scheme_host_path, query) = match raw.split_once('?') {
        Some((a, b)) => (a, b.split('#').next().unwrap_or(b)),
        None => (raw.split('#').next().unwrap_or(raw), ""),
    };
    let (scheme, rest) = if let Some((s, r)) = scheme_host_path.split_once("://") {
        (s.to_lowercase(), r)
    } else {
        ("https".to_string(), scheme_host_path)
    };
    let (host_part, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host_part = host_part.rsplit('@').next().unwrap_or(host_part);
    let (host, port) = if host_part.starts_with('[') {
        (host_part.to_lowercase(), None)
    } else if let Some((h, p)) = host_part.rsplit_once(':') {
        if p.chars().all(|c| c.is_ascii_digit()) {
            (h.to_lowercase(), Some(p.to_string()))
        } else {
            (host_part.to_lowercase(), None)
        }
    } else {
        (host_part.to_lowercase(), None)
    };
    let mut host = host;
    if let Some(stripped) = host.strip_prefix("www.") {
        host = stripped.to_string();
    }
    let mut path = path.to_string();
    while path.len() > 1 && path.ends_with('/') {
        path.pop();
    }

    let tracking = [
        "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", "gclid", "fbclid",
        "from", "vjk", "trk", "refid", "refId",
    ];
    let pairs: Vec<(String, String)> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (k.to_string(), v.to_string()),
            None => (p.to_string(), String::new()),
        })
        .collect();

    if host.contains("indeed.") && path.trim_end_matches('/').ends_with("/rc/clk") {
        if let Some((_, jk)) = pairs
            .iter()
            .find(|(k, v)| k.eq_ignore_ascii_case("jk") && !v.is_empty())
        {
            return format!("https://{host}/viewjob?jk={jk}");
        }
    }

    let kept: Vec<String> = pairs
        .into_iter()
        .filter(|(k, _)| !tracking.iter().any(|t| t.eq_ignore_ascii_case(k)))
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    let netloc = match port {
        Some(p) => format!("{host}:{p}"),
        None => host,
    };
    let base = format!("{scheme}://{netloc}{path}");
    if kept.is_empty() {
        base
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

pub fn fingerprint(
    board: Option<&str>,
    external_id: Option<&str>,
    url: &str,
    company: &str,
    title: &str,
    location: &str,
) -> (Option<String>, String) {
    let strong = match (board, external_id) {
        (Some(b), Some(id)) if !b.is_empty() && !id.is_empty() => Some(format!("{b}:{id}")),
        _ if !url.trim().is_empty() => Some(format!("url:{}", canonical_url(url))),
        _ => None,
    };
    (strong, weak_key(company, title, location))
}

pub fn cluster_id(strong: Option<&str>, weak: &str) -> String {
    match strong {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => format!("weak:{weak}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    struct FixtureFile {
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Case {
        id: String,
        input: Input,
        expected_strong: Option<String>,
        expected_weak: String,
        expected_cluster: String,
    }

    #[derive(Deserialize)]
    struct Input {
        company: String,
        title: String,
        location: String,
        url: String,
        board: Option<String>,
        external_id: Option<String>,
    }

    #[test]
    fn fingerprints_fixture_matches() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/fingerprints.json"
        );
        let raw = fs::read_to_string(path).expect("fingerprints.json");
        let fixture: FixtureFile = serde_json::from_str(&raw).unwrap();
        for case in fixture.cases {
            let (strong, weak) = fingerprint(
                case.input.board.as_deref(),
                case.input.external_id.as_deref(),
                &case.input.url,
                &case.input.company,
                &case.input.title,
                &case.input.location,
            );
            assert_eq!(weak, case.expected_weak, "weak {}", case.id);
            assert_eq!(strong, case.expected_strong, "strong {}", case.id);
            assert_eq!(
                cluster_id(strong.as_deref(), &weak),
                case.expected_cluster,
                "cluster {}",
                case.id
            );
        }
    }
}
