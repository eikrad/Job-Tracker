"""Mail-scan sidecar CLI tests (PR B1)."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

import jsonschema
import pytest

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "mail_scan_events.schema.json"
FIXTURE_MBOX = ROOT / "tests" / "fixtures" / "mail_scan" / "indeed_sample.mbox"
MAX_LINE = 256 * 1024


def _run_sidecar(
    args: list[str], stdin: str | None = None
) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "PYTHONPATH": str(ROOT / "python")}
    return subprocess.run(
        [sys.executable, "-m", "mail_scan", *args],
        input=stdin,
        text=True,
        capture_output=True,
        cwd=ROOT,
        env=env,
        check=False,
    )


@pytest.fixture(scope="module")
def event_schema():
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


def test_probe_prints_capabilities_and_exits_zero():
    proc = _run_sidecar(["probe", "--protocol", "1"])
    assert proc.returncode == 0, proc.stderr
    data = json.loads(proc.stdout.strip())
    assert data["t"] == "capabilities"
    assert data["protocol"] == 1
    assert "sidecar_version" in data
    assert "indeed" in data.get("extractors", [])


def test_scan_invalid_config_exits_20():
    proc = _run_sidecar(["scan", "--protocol", "1"], stdin="{not json")
    assert proc.returncode == 20


def test_scan_unreadable_source_exits_30(tmp_path: Path):
    missing = tmp_path / "missing.mbox"
    cfg = {
        "protocol": 1,
        "run_id": "test-run",
        "sources": [
            {
                "id": "indeed",
                "kind": "mbox",
                "path": str(missing),
                "cursor": None,
            }
        ],
        "extractors": ["indeed", "generic"],
        "limits": {
            "max_messages_per_source": 100,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 100,
            "max_body_chars": 20_000,
        },
        "since": None,
        "cancel_file": str(tmp_path / "cancel"),
    }
    proc = _run_sidecar(["scan", "--protocol", "1"], stdin=json.dumps(cfg))
    assert proc.returncode == 30


def test_cancel_file_stops_between_messages(tmp_path: Path, event_schema):
    cancel = tmp_path / "run.cancel"
    # Pre-create cancel so the first between-message check aborts quickly after started.
    cancel.write_text("1", encoding="utf-8")
    cfg = {
        "protocol": 1,
        "run_id": "cancel-run",
        "sources": [
            {
                "id": "indeed",
                "kind": "mbox",
                "path": str(FIXTURE_MBOX),
                "cursor": None,
            }
        ],
        "extractors": ["indeed", "generic"],
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 2000,
            "max_body_chars": 20_000,
        },
        "since": None,
        "cancel_file": str(cancel),
    }
    proc = _run_sidecar(["scan", "--protocol", "1"], stdin=json.dumps(cfg))
    assert proc.returncode == 10, proc.stderr
    lines = [ln for ln in proc.stdout.splitlines() if ln.strip()]
    assert lines, proc.stdout
    events = [json.loads(ln) for ln in lines]
    for ev in events:
        jsonschema.validate(ev, event_schema)
    finished = events[-1]
    assert finished["t"] == "finished"
    assert finished.get("cancelled") is True


def test_fixture_scan_events_validate_and_are_deterministic(
    tmp_path: Path, event_schema
):
    cfg = {
        "protocol": 1,
        "run_id": "det-run",
        "sources": [
            {
                "id": "indeed",
                "kind": "mbox",
                "path": str(FIXTURE_MBOX),
                "cursor": None,
            }
        ],
        "extractors": ["indeed", "generic"],
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 2000,
            "max_body_chars": 20_000,
        },
        "since": None,
        "cancel_file": str(tmp_path / "no-cancel"),
    }

    def normalize(stdout: str) -> list[dict]:
        events = []
        for ln in stdout.splitlines():
            assert len(ln.encode("utf-8")) <= MAX_LINE
            ev = json.loads(ln)
            jsonschema.validate(ev, event_schema)
            if ev["t"] == "started":
                ev = {**ev, "run_id": "<run>"}
            if ev["t"] == "finished":
                ev = {**ev, "duration_ms": 0}
            events.append(ev)
        return events

    a = _run_sidecar(["scan", "--protocol", "1"], stdin=json.dumps(cfg))
    b = _run_sidecar(
        ["scan", "--protocol", "1"],
        stdin=json.dumps({**cfg, "run_id": "det-run-2"}),
    )
    assert a.returncode == 0, a.stderr
    assert b.returncode == 0, b.stderr
    assert normalize(a.stdout) == normalize(b.stdout)
    listings = [e for e in normalize(a.stdout) if e["t"] == "listing"]
    assert len(listings) >= 2
    assert any(e.get("external_ref", {}).get("board") == "indeed" for e in listings)


def test_mail_scan_package_bans_network_imports():
    """ADR 0004: mechanical ban — also covered by ruff TID251."""
    banned = ("socket", "http", "urllib", "requests", "httpx", "smtplib", "ftplib")
    root = ROOT / "python" / "mail_scan"
    offenders: list[str] = []
    for path in root.rglob("*.py"):
        text = path.read_text(encoding="utf-8")
        for name in banned:
            # Exact module only — allow urllib.parse for local URL string work.
            pattern = (
                rf"(?m)^\s*(?:import {re.escape(name)}\b|"
                rf"from {re.escape(name)} import\b)"
            )
            if re.search(pattern, text):
                offenders.append(f"{path.relative_to(ROOT)}:{name}")
    assert offenders == []
