//! Normalize messy LLM JSON into job field maps (mirrors TS `normalizeLlmJobPartial`).

use serde_json::{Map, Value};
use std::collections::HashMap;

fn str_field(raw: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(Value::String(s)) = raw.get(*k) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn normalize_work_mode(raw: Option<String>) -> Option<String> {
    let raw = raw?;
    let key = raw.to_lowercase();
    let key = key.trim();
    match key {
        "remote" => Some("Remote".into()),
        "hybrid" => Some("Hybrid".into()),
        "on-site" | "onsite" | "on site" | "office" => Some("On-site".into()),
        _ => None,
    }
}

pub fn normalize_llm_job_partial(raw: &Map<String, Value>) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    let put = |out: &mut HashMap<String, Value>, key: &str, val: Option<String>| {
        if let Some(v) = val {
            out.insert(key.to_string(), Value::String(v));
        }
    };

    put(
        &mut out,
        "company",
        str_field(raw, &["company", "Company", "employer", "Employer", "organization"]),
    );
    put(
        &mut out,
        "title",
        str_field(raw, &["title", "Title", "role", "Role", "job_title", "position"]),
    );
    put(
        &mut out,
        "url",
        str_field(raw, &["url", "URL", "link", "job_url", "apply_url"]),
    );
    put(
        &mut out,
        "deadline",
        str_field(raw, &["deadline", "Deadline", "closing_date", "apply_by"]),
    );
    put(
        &mut out,
        "interview_date",
        str_field(
            raw,
            &[
                "interview_date",
                "interviewDate",
                "InterviewDate",
                "interviews",
                "interview",
                "assessment_date",
            ],
        ),
    );
    put(
        &mut out,
        "start_date",
        str_field(
            raw,
            &[
                "start_date",
                "startDate",
                "StartDate",
                "position_start",
                "role_start",
                "contract_start",
            ],
        ),
    );

    let mut tags = str_field(raw, &["tags", "Tags", "keywords"]);
    if tags.is_none() {
        if let Some(Value::Array(arr)) = raw.get("tags") {
            let joined: Vec<&str> = arr
                .iter()
                .filter_map(|x| x.as_str())
                .collect();
            let s = joined.join(", ");
            if !s.trim().is_empty() {
                tags = Some(s);
            }
        }
    }
    put(&mut out, "tags", tags);

    put(
        &mut out,
        "detected_language",
        str_field(raw, &["detected_language", "DetectedLanguage", "language", "Language"]),
    );
    put(&mut out, "notes", str_field(raw, &["notes", "Notes", "summary"]));
    put(
        &mut out,
        "contact_name",
        str_field(raw, &["contact_name", "contactName", "contact"]),
    );
    put(
        &mut out,
        "contact_email",
        str_field(raw, &["contact_email", "contactEmail", "email"]),
    );
    put(
        &mut out,
        "contact_phone",
        str_field(raw, &["contact_phone", "contactPhone", "phone"]),
    );
    put(
        &mut out,
        "workplace_street",
        str_field(raw, &["workplace_street", "street", "address"]),
    );
    put(
        &mut out,
        "workplace_city",
        str_field(raw, &["workplace_city", "city"]),
    );
    put(
        &mut out,
        "workplace_postal_code",
        str_field(raw, &["workplace_postal_code", "postalCode", "postal_code", "zip"]),
    );
    put(
        &mut out,
        "work_mode",
        normalize_work_mode(str_field(raw, &["work_mode", "workMode", "location_type"])),
    );
    put(
        &mut out,
        "salary_range",
        str_field(raw, &["salary_range", "salaryRange", "salary", "compensation"]),
    );
    put(
        &mut out,
        "contract_type",
        str_field(raw, &["contract_type", "contractType", "employment_type", "contract"]),
    );
    // priority intentionally excluded
    put(
        &mut out,
        "reference_number",
        str_field(raw, &["reference_number", "referenceNumber", "ref", "job_ref", "job_id"]),
    );
    put(
        &mut out,
        "source",
        str_field(raw, &["source", "Source", "job_source", "platform"]),
    );
    out
}

fn parse_raw_json_object(text: &str) -> Option<Map<String, Value>> {
    let trimmed = text.trim();
    let without_think = strip_think_blocks(trimmed);
    let unfenced = without_think
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    for candidate in [unfenced, without_think.as_str(), trimmed] {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(candidate) {
            return Some(map);
        }
        if let Some(extracted) = extract_first_json_object(candidate) {
            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&extracted) {
                return Some(map);
            }
        }
    }
    None
}

/// Scaleway/DeepSeek reasoning models often wrap the answer as
/// `<think>…</think>{…json…}`. A known bug can omit the opening tag.
fn strip_think_blocks(text: &str) -> String {
    let mut out = text.to_string();
    while let Some(start) = out.find("<think>") {
        let after = start + "<think>".len();
        if let Some(rel_end) = out[after..].find("</think>") {
            let end = after + rel_end + "</think>".len();
            out.replace_range(start..end, "");
        } else {
            // Unclosed think block — drop from the tag to the end.
            out.truncate(start);
            break;
        }
    }
    // Missing opening tag: content starts with reasoning and ends with </think>.
    if let Some(end) = out.find("</think>") {
        out = out[end + "</think>".len()..].to_string();
    }
    out
}

/// Pull the first top-level `{ … }` object out of surrounding prose / fences.
fn extract_first_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn parse_partial_new_job_from_llm_text(text: &str) -> HashMap<String, Value> {
    match parse_raw_json_object(text) {
        Some(raw) => normalize_llm_job_partial(&raw),
        None => HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_fences_and_maps_company() {
        let out = parse_partial_new_job_from_llm_text("```json\n{\"company\":\"X\"}\n```");
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("X"));
    }

    #[test]
    fn strips_scaleway_think_tags_around_json() {
        // User-visible failure: DeepSeek/Scaleway returns reasoning then JSON;
        // treating the whole string as JSON yields "Could not parse JSON…".
        let text = "<think>\nLet me extract fields carefully.\n</think>\n{\"company\":\"Acme\",\"title\":\"Rust engineer\"}";
        let out = parse_partial_new_job_from_llm_text(text);
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
        assert_eq!(out.get("title").and_then(|v| v.as_str()), Some("Rust engineer"));
    }

    #[test]
    fn strips_think_block_when_opening_tag_is_missing() {
        let text = "reasoning about the ad…</think>\n{\"company\":\"Beta\"}";
        let out = parse_partial_new_job_from_llm_text(text);
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Beta"));
    }

    #[test]
    fn extracts_json_object_from_prose_preamble() {
        let text = "Here is the result:\n\n{\"company\":\"Gamma\",\"title\":\"SRE\"}\nThanks.";
        let out = parse_partial_new_job_from_llm_text(text);
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Gamma"));
    }

    #[test]
    fn drops_priority() {
        let raw = json!({"company":"Acme","priority":3})
            .as_object()
            .unwrap()
            .clone();
        let out = normalize_llm_job_partial(&raw);
        assert!(!out.contains_key("priority"));
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }

    #[test]
    fn maps_employer_alias() {
        let raw = json!({"employer":"Y"}).as_object().unwrap().clone();
        let out = normalize_llm_job_partial(&raw);
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Y"));
    }
}
