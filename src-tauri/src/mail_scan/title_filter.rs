//! Title blocklist: listings whose title contains a word or phrase the user never wants.
//!
//! Runs in the gate, before any page fetch or model call, so a skipped listing is free.
//! Matching is on whole words, case-insensitive, so "lead" does not hit "leadership" or
//! "Platform". `*` at the start or end of a word matches any letters there, which is
//! how compound words are caught: `*leder` matches "teamleder" and "afdelingsleder".

#[derive(Debug, Clone, Default)]
pub struct TitleFilter {
    phrases: Vec<Vec<Token>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Exact(String),
    Prefix(String),
    Suffix(String),
}

impl Token {
    fn parse(word: &str, star_before: bool, star_after: bool) -> Self {
        match (star_before, star_after) {
            (true, false) => Token::Suffix(word.to_string()),
            (false, true) => Token::Prefix(word.to_string()),
            _ => Token::Exact(word.to_string()),
        }
    }

    fn matches(&self, word: &str) -> bool {
        match self {
            Token::Exact(w) => word == w,
            Token::Prefix(w) => word.starts_with(w.as_str()),
            Token::Suffix(w) => word.ends_with(w.as_str()),
        }
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// One blocklist entry as tokens. `*` counts only next to a word; anywhere else it is
/// ignored like any other punctuation.
fn parse_entry(entry: &str) -> Vec<Token> {
    entry
        .split_whitespace()
        .filter_map(|chunk| {
            let star_before = chunk.starts_with('*');
            let star_after = chunk.ends_with('*');
            let word = words(chunk).into_iter().next()?;
            Some(Token::parse(&word, star_before, star_after))
        })
        .collect()
}

impl TitleFilter {
    pub fn new(entries: &[String]) -> Self {
        let phrases = entries
            .iter()
            .map(|e| parse_entry(e))
            .filter(|tokens| !tokens.is_empty())
            .collect();
        Self { phrases }
    }

    pub fn matches(&self, title: &str) -> bool {
        if self.phrases.is_empty() {
            return false;
        }
        let title_words = words(title);
        self.phrases.iter().any(|phrase| {
            title_words.windows(phrase.len()).any(|window| {
                window.iter().zip(phrase).all(|(word, token)| token.matches(word))
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(entries: &[&str]) -> TitleFilter {
        TitleFilter::new(&entries.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn a_word_matches_whole_words_in_any_case() {
        let f = filter(&["lead"]);
        assert!(f.matches("Lead Data Scientist"));
        assert!(f.matches("Senior AI engineer (LEAD)"));
        assert!(f.matches("Data Science Lead, Copenhagen"));
        assert!(!f.matches("Leadership Programme Coordinator"));
        assert!(!f.matches("Misleading Data Analyst"));
    }

    #[test]
    fn a_phrase_must_match_in_order_and_together() {
        let f = filter(&["head of"]);
        assert!(f.matches("Head of Data"));
        assert!(!f.matches("Head Chef of the Year"));
        assert!(!f.matches("Ahead of the curve analyst"));
    }

    #[test]
    fn a_star_catches_compound_words() {
        let f = filter(&["*leder", "manag*"]);
        assert!(f.matches("Teamleder til dataafdelingen"));
        assert!(f.matches("Afdelingsleder"));
        assert!(f.matches("Product Manager"));
        assert!(f.matches("Managing Consultant"));
        assert!(f.matches("Leder til data"), "the star allows nothing before it too");
        assert!(!f.matches("Ledergruppe sekretær"), "*leder must end the word");
        assert!(!f.matches("Data Engineer"));
    }

    #[test]
    fn danish_letters_and_punctuation_are_handled() {
        let f = filter(&["chef", "team-lead"]);
        assert!(!f.matches("Køkkenchef"), "no star, so not a whole-word match");
        assert!(f.matches("Chef for data"));
        assert!(f.matches("Team Lead – ML"));
        assert!(f.matches("Team-lead, ML"));
    }

    #[test]
    fn an_empty_or_blank_list_never_matches() {
        assert!(!filter(&[]).matches("Lead Data Scientist"));
        let blank = filter(&["", "   ", "*"]);
        assert!(!blank.matches("Lead Data Scientist"));
    }
}
