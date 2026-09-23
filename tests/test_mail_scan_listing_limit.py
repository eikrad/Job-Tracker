"""The per-run listing limit must never make mail unreachable (spec §5.5).

A scan that stops at `max_listings_per_run` leaves every cursor on the last mail it
fully handled, so the next scan picks up exactly where this one stopped.
"""

from __future__ import annotations

import io
import json
from pathlib import Path

from mail_scan.scan import run_scan

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "mail_scan"
SOURCES = {
    "linkedin": FIXTURES / "linkedin_alerts.mbox",
    "jobindex": FIXTURES / "jobindex_alerts.mbox",
}


def scan(cursors: dict[str, dict | None], max_listings: int) -> list[dict]:
    config = {
        "protocol": 2,
        "run_id": "limit-test",
        "sources": [
            {"id": sid, "kind": "mbox", "path": str(path), "cursor": cursors.get(sid)}
            for sid, path in SOURCES.items()
        ],
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": max_listings,
            "max_body_chars": 20_000,
        },
        "since": None,
        "cancel_file": None,
    }
    out = io.StringIO()
    assert run_scan(config, emit=out) == 0
    return [json.loads(line) for line in out.getvalue().splitlines()]


def listing_keys(events: list[dict]) -> set[str]:
    return {e["fingerprint"]["strong"] for e in events if e["t"] == "listing"}


def finished(events: list[dict]) -> dict:
    return next(e for e in events if e["t"] == "finished")


def test_repeated_scans_under_a_listing_limit_reach_every_listing() -> None:
    everything = listing_keys(scan({}, max_listings=2000))
    assert len(everything) > 3, "fixtures should hold more listings than the limit"

    seen: set[str] = set()
    cursors: dict[str, dict | None] = {}
    for _ in range(len(everything) + 2):
        events = scan(cursors, max_listings=2)
        seen |= listing_keys(events)
        for e in events:
            if e["t"] == "source_finished":
                cursors[e["source"]] = e["cursor"]
        if not finished(events).get("truncated"):
            break

    assert seen == everything


def test_a_scan_that_hits_the_listing_limit_says_so() -> None:
    assert finished(scan({}, max_listings=2))["truncated"] is True
    assert "truncated" not in finished(scan({}, max_listings=2000))


def test_sources_after_the_limit_are_left_untouched() -> None:
    events = scan({}, max_listings=1)
    finished_sources = [e["source"] for e in events if e["t"] == "source_finished"]
    # The limit is hit inside the first source; the second keeps its old cursor.
    assert finished_sources == ["linkedin"]
