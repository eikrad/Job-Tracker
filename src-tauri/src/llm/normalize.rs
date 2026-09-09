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
    let unfenced = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    for candidate in [unfenced, trimmed] {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(candidate) {
            return Some(map);
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
    fn drops_priority() {
        let raw = json!({"company":"Acme","priority":3})
            .as_object()
            .unwrap()
            .clone();
        let out = normalize_llm_job_partial(&raw);
        assert!(out.get("priority").is_none());
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Acme"));
    }

    #[test]
    fn maps_employer_alias() {
        let raw = json!({"employer":"Y"}).as_object().unwrap().clone();
        let out = normalize_llm_job_partial(&raw);
        assert_eq!(out.get("company").and_then(|v| v.as_str()), Some("Y"));
    }
}
