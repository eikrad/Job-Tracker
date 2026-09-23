//! Reading a fetched board page: which board it is, where its description lives, and
//! whether it is only a wrapper around the employer's own ad (spec §6.3).
//!
//! Everything here is a pure function of a URL or of page HTML. Fetching stays in
//! [`super::enrichment`], behind the guarded fetch — this module only ever *names* a
//! link, and whatever it names is re-validated before a request is made.

use url::Url;

/// The boards enrichment treats differently. Anything else is [`Board::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Board {
    /// `/c?t=` answers one redirect straight to the employer's ad.
    Jobindex,
    /// The guest page carries the full description; the apply link is hidden.
    LinkedIn,
    /// `viewjob` answers bots with a 401 challenge — never fetchable.
    Indeed,
    Other,
}

fn site(url: &Url) -> String {
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    host.strip_prefix("www.").unwrap_or(&host).to_string()
}

fn is_or_under(site: &str, domain: &str) -> bool {
    site == domain || site.ends_with(&format!(".{domain}"))
}

impl Board {
    pub fn of(url: &str) -> Self {
        let Ok(parsed) = Url::parse(url.trim()) else {
            return Board::Other;
        };
        let site = site(&parsed);
        if is_or_under(&site, "jobindex.dk") {
            Board::Jobindex
        } else if is_or_under(&site, "linkedin.com") {
            Board::LinkedIn
        } else if site.split('.').any(|label| label == "indeed") {
            Board::Indeed
        } else {
            Board::Other
        }
    }
}

/// Same site, ignoring a leading `www.`. Unparseable URLs are never the same site.
pub fn same_site(a: &str, b: &str) -> bool {
    match (Url::parse(a), Url::parse(b)) {
        (Ok(a), Ok(b)) => site(&a) == site(&b),
        _ => false,
    }
}

/// Class of the LinkedIn guest page's description block.
const LINKEDIN_DESCRIPTION_CLASS: &str = "show-more-less-html__markup";

/// The description text of a LinkedIn guest page, if the block is there.
pub fn linkedin_description(html: &str) -> Option<String> {
    let inner = element_inner_html_by_class(html, LINKEDIN_DESCRIPTION_CLASS)?;
    let text = crate::job_search::strip_html_to_text(inner);
    (!text.trim().is_empty()).then_some(text)
}

/// Inner HTML of the first element whose `class` attribute contains `class_name`,
/// matched to its own closing tag (nested elements of the same name are counted).
fn element_inner_html_by_class<'a>(html: &'a str, class_name: &str) -> Option<&'a str> {
    let lower = html.to_ascii_lowercase();
    let class_at = lower.find(&class_name.to_ascii_lowercase())?;
    let open_start = lower[..class_at].rfind('<')?;
    let open_end = class_at + lower[class_at..].find('>')? + 1;
    let name: String = lower[open_start + 1..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if name.is_empty() {
        return None;
    }
    let (open_tag, close_tag) = (format!("<{name}"), format!("</{name}"));
    let mut depth = 1usize;
    let mut pos = open_end;
    while pos < lower.len() {
        let next_open = lower[pos..].find(&open_tag).map(|i| pos + i);
        let next_close = lower[pos..].find(&close_tag).map(|i| pos + i)?;
        match next_open {
            // Only a real tag of that name opens a level (`<divider>` is not `<div`).
            Some(o) if o < next_close => {
                let after = lower[o + open_tag.len()..].chars().next();
                if matches!(after, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
                    depth += 1;
                }
                pos = o + open_tag.len();
            }
            _ => {
                depth -= 1;
                if depth == 0 {
                    return Some(&html[open_end..next_close]);
                }
                pos = next_close + close_tag.len();
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Anchor {
    href: String,
    text: String,
}

/// Every `<a href>` on the page with its visible text.
fn anchors(html: &str) -> Vec<Anchor> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(i) = lower[pos..].find("<a") {
        let start = pos + i;
        pos = start + 2;
        if !lower[pos..].starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        let Some(tag_end) = lower[start..].find('>').map(|j| start + j) else {
            break;
        };
        let Some(href) = attribute(&html[start..tag_end], "href") else {
            continue;
        };
        let close = lower[tag_end..]
            .find("</a")
            .map(|j| tag_end + j)
            .unwrap_or(lower.len());
        let text = crate::job_search::strip_html_to_text(&html[tag_end + 1..close]);
        out.push(Anchor { href, text });
        pos = close;
    }
    out
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let preceded_by_space = lower[..at].ends_with(|c: char| c.is_whitespace());
        let rest = lower[from..].trim_start();
        if !preceded_by_space || !rest.starts_with('=') {
            continue;
        }
        let value_start = tag.len() - rest.len() + 1;
        let value = tag[value_start..].trim_start();
        let (quote, body) = match value.chars().next()? {
            q @ ('"' | '\'') => (Some(q), &value[1..]),
            _ => (None, value),
        };
        let end = match quote {
            Some(q) => body.find(q)?,
            None => body.find(|c: char| c.is_whitespace()).unwrap_or(body.len()),
        };
        return Some(body[..end].replace("&amp;", "&"));
    }
    None
}

/// Anchor texts that say "the real ad is over there". Lower-case; matched as substrings
/// of the anchor's collapsed text.
const WRAPPER_ANCHOR_TEXTS: &[&str] = &[
    "se hele annoncen",
    "gå til annoncen",
    "læs hele jobopslaget",
    "se hele jobopslaget",
    "apply on company site",
    "view full job",
    "zur stellenanzeige",
    "zur vollständigen stellenanzeige",
];

/// Sites that are never the ad a wrapper points at (share buttons, app stores, …).
const NOT_AN_AD: &[&str] = &[
    "facebook.com",
    "twitter.com",
    "x.com",
    "instagram.com",
    "youtube.com",
    "linkedin.com",
    "google.com",
    "apple.com",
    "tiktok.com",
];

/// A wrapper page is thin; a page with more text than this is the ad itself, and its
/// lone external link is more likely the employer's homepage than the ad.
const WRAPPER_MAX_TEXT_CHARS: usize = 2_000;

/// If this page only wraps the full ad, the link to it.
///
/// Two rules, in order: an anchor whose text says "see the full ad"
/// ([`WRAPPER_ANCHOR_TEXTS`]); or, on a thin page, exactly one distinct link to
/// another site that is not a share button. Only `http(s)` links are ever returned.
pub fn wrapper_link(html: &str, page_url: &str) -> Option<String> {
    let base = Url::parse(page_url).ok()?;
    let resolved: Vec<(Url, String)> = anchors(html)
        .into_iter()
        .filter_map(|a| {
            let url = base.join(a.href.trim()).ok()?;
            matches!(url.scheme(), "http" | "https").then_some((url, a.text.to_lowercase()))
        })
        .collect();

    if let Some((url, _)) = resolved
        .iter()
        .find(|(_, text)| WRAPPER_ANCHOR_TEXTS.iter().any(|w| text.contains(w)))
    {
        return Some(url.to_string());
    }

    let page_text = crate::job_search::strip_html_to_text(html);
    if page_text.chars().count() > WRAPPER_MAX_TEXT_CHARS {
        return None;
    }
    let page_site = site(&base);
    let mut external: Vec<String> = resolved
        .iter()
        .filter(|(url, _)| {
            let s = site(url);
            s != page_site && !NOT_AN_AD.iter().any(|d| is_or_under(&s, d))
        })
        .map(|(url, _)| url.to_string())
        .collect();
    external.sort();
    external.dedup();
    match external.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boards_are_recognised_by_their_site() {
        assert_eq!(Board::of("https://www.jobindex.dk/c?t=h1"), Board::Jobindex);
        assert_eq!(
            Board::of("https://www.linkedin.com/jobs/view/1"),
            Board::LinkedIn
        );
        assert_eq!(
            Board::of("https://dk.indeed.com/viewjob?jk=a"),
            Board::Indeed
        );
        assert_eq!(
            Board::of("https://de.indeed.com/viewjob?jk=a"),
            Board::Indeed
        );
        assert_eq!(Board::of("https://www.jobbank.dk/job/1"), Board::Other);
        assert_eq!(Board::of("https://notjobindex.dk/"), Board::Other);
    }

    #[test]
    fn attributes_are_read_whatever_the_quoting() {
        assert_eq!(
            attribute(r#"<a class="x" href="/a?b=1&amp;c=2""#, "href").as_deref(),
            Some("/a?b=1&c=2")
        );
        assert_eq!(attribute("<a href='/b'", "href").as_deref(), Some("/b"));
        assert_eq!(
            attribute("<a href=/c target=_blank", "href").as_deref(),
            Some("/c")
        );
        assert_eq!(attribute(r#"<a data-href="/no""#, "href"), None);
    }
}
