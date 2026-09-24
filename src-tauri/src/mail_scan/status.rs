//! The closed sets of state words the mail pipeline stores and sends to the UI.
//!
//! Each word is spelled exactly once, here. The same spelling is what goes into SQLite
//! (the tables' `CHECK` constraints list it), what the frontend receives, and what
//! `FromStr` accepts back, so a typo is a compile error instead of a row that no query
//! ever matches.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!("unknown {} {other:?}", stringify!($name))),
                }
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.as_str()))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                value
                    .as_str()?
                    .parse()
                    .map_err(|e: String| FromSqlError::Other(e.into()))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = <std::borrow::Cow<'de, str>>::deserialize(d)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

text_enum! {
    /// Where a Mail Match is in review (`mail_match_inbox.status`). The schema also
    /// allows `superseded`, a leftover of the retired update suggestions that nothing
    /// writes.
    InboxStatus {
        Pending => "pending",
        Accepted => "accepted",
        Dismissed => "dismissed",
    }
}

text_enum! {
    /// A scored listing's verdict (`mail_scored_sightings.outcome`): worth a human look,
    /// or remembered as below the cutoff so it is not paid for again.
    Verdict {
        Inbox => "inbox",
        UnderCutoff => "under_cutoff",
    }
}

text_enum! {
    /// Whether an inbox row's score is a number the user can trust
    /// (`mail_match_inbox.score_state`).
    ScoreState {
        Ok => "ok",
        /// The model's answer failed validation; the score is NULL and shows as `?`.
        Invalid => "invalid",
        Skipped => "skipped",
    }
}

text_enum! {
    /// How far Enrichment got (`mail_match_inbox.enrichment_state`).
    EnrichmentState {
        Complete => "complete",
        Partial => "partial",
        Failed => "failed",
        /// Not attempted: under the cutoff, or a board that is never fetched.
        Skipped => "skipped",
        /// The listing page says the posting is gone. Never stored: such a listing is
        /// dropped before it reaches the inbox.
        Closed => "closed",
    }
}

text_enum! {
    /// A scan run's lifecycle (`mail_scan_runs.status`, and the progress event).
    RunStatus {
        Running => "running",
        Completed => "completed",
        Cancelled => "cancelled",
        Failed => "failed",
    }
}

text_enum! {
    /// On-disk layout of a mail folder: one mbox file, or a maildir directory.
    SourceKind {
        Mbox => "mbox",
        Maildir => "maildir",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Every variant survives SQLite and JSON unchanged, spelled as the schema does.
    fn round_trips<T>(values: &[T], spelled: &[&str])
    where
        T: ToSql + FromSql + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
        T: std::fmt::Debug + std::str::FromStr<Err = String>,
    {
        let conn = Connection::open_in_memory().unwrap();
        for (value, text) in values.iter().zip(spelled) {
            let stored: String = conn.query_row("SELECT ?1", [value], |r| r.get(0)).unwrap();
            assert_eq!(&stored, text);
            let read: T = conn.query_row("SELECT ?1", [text], |r| r.get(0)).unwrap();
            assert_eq!(&read, value);
            assert_eq!(serde_json::to_string(value).unwrap(), format!("\"{text}\""));
            let parsed: T = serde_json::from_str(&format!("\"{text}\"")).unwrap();
            assert_eq!(&parsed, value);
            assert_eq!(&text.parse::<T>().unwrap(), value);
        }
    }

    #[test]
    fn every_state_word_is_stored_and_sent_as_the_schema_spells_it() {
        use EnrichmentState as E;
        use InboxStatus as I;
        use RunStatus as R;
        round_trips(&[I::Pending, I::Accepted, I::Dismissed], &["pending", "accepted", "dismissed"]);
        round_trips(&[Verdict::Inbox, Verdict::UnderCutoff], &["inbox", "under_cutoff"]);
        round_trips(
            &[ScoreState::Ok, ScoreState::Invalid, ScoreState::Skipped],
            &["ok", "invalid", "skipped"],
        );
        round_trips(
            &[E::Complete, E::Partial, E::Failed, E::Skipped, E::Closed],
            &["complete", "partial", "failed", "skipped", "closed"],
        );
        round_trips(
            &[R::Running, R::Completed, R::Cancelled, R::Failed],
            &["running", "completed", "cancelled", "failed"],
        );
        round_trips(&[SourceKind::Mbox, SourceKind::Maildir], &["mbox", "maildir"]);
    }

    #[test]
    fn an_unknown_word_is_an_error_not_a_default() {
        let conn = Connection::open_in_memory().unwrap();
        let read: rusqlite::Result<InboxStatus> =
            conn.query_row("SELECT 'superseded'", [], |r| r.get(0));
        assert!(read.is_err());
        assert!(serde_json::from_str::<SourceKind>("\"imap\"").is_err());
    }
}
