"""Contract tests for Job Tracker JSON export shape (mirrors frontend export)."""

import json
from pathlib import Path

FIXTURE = Path(__file__).resolve().parent / "fixtures" / "sample_export.json"

REQUIRED_TOP_LEVEL_KEYS = {
    "id",
    "company",
    "title",
    "url",
    "raw_text",
    "status",
    "deadline",
    "tags",
    "detected_language",
    "notes",
    "pdf_path",
    "created_at",
    "updated_at",
}


def test_sample_export_is_array_of_objects():
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert isinstance(data, list)
    assert len(data) >= 1
    assert isinstance(data[0], dict)


def test_sample_export_row_has_required_keys():
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    row = data[0]
    missing = REQUIRED_TOP_LEVEL_KEYS - row.keys()
    assert not missing, f"Missing keys: {missing}"


def test_sample_export_mandatory_fields_non_empty_strings():
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    row = data[0]
    assert str(row["company"]).strip()
    assert str(row["status"]).strip()
