//! Line-bounded NDJSON protocol reader (spec §4.3).

#![allow(dead_code)] // Event payloads are deserialized for forward-compat; not all fields are read yet.

use std::io::Read;

use serde::Deserialize;

pub const MAX_LINE_BYTES: usize = 256 * 1024;

#[derive(Debug)]
pub enum ProtocolError {
    Oversize,
    Malformed(String),
    Io(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::Oversize => write!(f, "E_PROTOCOL_OVERSIZE"),
            ProtocolError::Malformed(d) => write!(f, "malformed event: {d}"),
            ProtocolError::Io(d) => write!(f, "io: {d}"),
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Started(StartedEvent),
    SourceStarted(SourceStartedEvent),
    Listing(Box<ListingEvent>),
    SourceFinished(SourceFinishedEvent),
    Warning(WarningEvent),
    Finished(FinishedEvent),
    Unknown { t: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartedEvent {
    pub protocol: u32,
    pub run_id: String,
    pub sidecar_version: String,
    pub sources: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStartedEvent {
    pub source: String,
    pub estimated_messages: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListingEvent {
    pub source: String,
    pub message_id: String,
    pub message_date: String,
    pub seq: u32,
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub external_ref: Option<ExternalRef>,
    pub snippet: String,
    pub posted_at: Option<String>,
    pub fingerprint: FingerprintKeys,
    pub extractor: String,
    pub extractor_confidence: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRef {
    pub board: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FingerprintKeys {
    pub strong: Option<String>,
    pub weak: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFinishedEvent {
    pub source: String,
    pub messages_read: u32,
    pub listings: u32,
    pub skipped: u32,
    pub cursor: CursorEvent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorEvent {
    pub size: u64,
    pub mtime_ns: u64,
    pub offset: u64,
    pub last_message_id: Option<String>,
    pub sentinel_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarningEvent {
    pub code: String,
    pub source: Option<String>,
    pub message_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishedEvent {
    pub listings_total: u32,
    pub messages_total: u32,
    pub duration_ms: u32,
    pub cancelled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TypePeek {
    t: String,
}

fn parse_payload<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T, ProtocolError> {
    let mut value: serde_json::Value =
        serde_json::from_str(line).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
    if let Some(obj) = value.as_object_mut() {
        obj.remove("t");
    }
    serde_json::from_value(value).map_err(|e| ProtocolError::Malformed(e.to_string()))
}

/// Parse one NDJSON object. Unknown `t` → [`Event::Unknown`].
pub fn parse_event_line(line: &str) -> Result<Event, ProtocolError> {
    let peek: TypePeek =
        serde_json::from_str(line).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
    match peek.t.as_str() {
        "started" => Ok(Event::Started(parse_payload(line)?)),
        "source_started" => Ok(Event::SourceStarted(parse_payload(line)?)),
        "listing" => Ok(Event::Listing(Box::new(parse_payload(line)?))),
        "source_finished" => Ok(Event::SourceFinished(parse_payload(line)?)),
        "warning" => Ok(Event::Warning(parse_payload(line)?)),
        "finished" => Ok(Event::Finished(parse_payload(line)?)),
        other => Ok(Event::Unknown {
            t: other.to_string(),
        }),
    }
}

/// Read one line with a hard byte cap (does not buffer an oversize line).
/// Blank lines are skipped. `Ok(None)` only on true EOF.
pub fn read_event_line<R: Read>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    max_bytes: usize,
) -> Result<Option<Event>, ProtocolError> {
    loop {
        buf.clear();
        let mut hit_eof = false;
        let mut byte = [0u8; 1];
        loop {
            match reader.read(&mut byte) {
                Ok(0) => {
                    hit_eof = true;
                    break;
                }
                Ok(_) => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    if buf.len() >= max_bytes {
                        // Drain until newline or EOF without retaining the rest.
                        loop {
                            match reader.read(&mut byte) {
                                Ok(0) => break,
                                Ok(_) if byte[0] == b'\n' => break,
                                Ok(_) => continue,
                                Err(e) => return Err(ProtocolError::Io(e.to_string())),
                            }
                        }
                        return Err(ProtocolError::Oversize);
                    }
                    buf.push(byte[0]);
                }
                Err(e) => return Err(ProtocolError::Io(e.to_string())),
            }
        }
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        if buf.is_empty() {
            if hit_eof {
                return Ok(None);
            }
            continue; // blank line
        }
        let line =
            std::str::from_utf8(buf).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
        return parse_event_line(line).map(Some);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn unknown_t_is_skipped() {
        let ev = parse_event_line(r#"{"t":"future_thing","x":1}"#).unwrap();
        match ev {
            Event::Unknown { t } => assert_eq!(t, "future_thing"),
            _ => panic!("expected unknown"),
        }
    }

    #[test]
    fn malformed_known_event_aborts() {
        let err = parse_event_line(r#"{"t":"listing","source":1}"#).unwrap_err();
        assert!(matches!(err, ProtocolError::Malformed(_)));
    }

    #[test]
    fn unknown_field_on_known_event_aborts() {
        let err = parse_event_line(
            r#"{"t":"started","protocol":1,"run_id":"r1","sidecar_version":"1.0.0","sources":1,"extra":true}"#,
        )
        .unwrap_err();
        assert!(matches!(err, ProtocolError::Malformed(_)));
    }

    #[test]
    fn oversize_line_without_buffering_all() {
        let mut oversize = vec![b'a'; MAX_LINE_BYTES + 10];
        oversize.push(b'\n');
        let mut cursor = Cursor::new(oversize);
        let mut buf = Vec::new();
        let err = read_event_line(&mut cursor, &mut buf, MAX_LINE_BYTES).unwrap_err();
        assert!(matches!(err, ProtocolError::Oversize));
        assert!(buf.len() <= MAX_LINE_BYTES);
    }

    #[test]
    fn protocol_started_parses() {
        let ev = parse_event_line(
            r#"{"t":"started","protocol":1,"run_id":"r1","sidecar_version":"1.0.0","sources":1}"#,
        )
        .unwrap();
        match ev {
            Event::Started(s) => assert_eq!(s.protocol, 1),
            _ => panic!("expected started"),
        }
    }
}
