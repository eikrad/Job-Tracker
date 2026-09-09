"""Cross-language fingerprint fixture (spec §9.3)."""

from __future__ import annotations

import json
from pathlib import Path

from mail_scan.fingerprint import fingerprint

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "fingerprints.json"


def test_fingerprints_fixture_matches_python():
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    for case in data["cases"]:
        inp = case["input"]
        fp = fingerprint(
            board=inp.get("board"),
            external_id=inp.get("external_id"),
            url=inp.get("url") or "",
            company=inp["company"],
            title=inp["title"],
            location=inp["location"],
        )
        assert fp["weak"] == case["expected_weak"], case["id"]
        assert fp["strong"] == case["expected_strong"], case["id"]
        cluster = fp["strong"] if fp["strong"] else f"weak:{fp['weak']}"
        assert cluster == case["expected_cluster"], case["id"]
