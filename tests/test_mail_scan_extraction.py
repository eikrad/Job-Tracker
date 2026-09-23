"""Listing extraction from real-shaped alert mail (spec §4.3, §5.1).

Every fixture here is synthetic — fake companies, ids and tokens — but mirrors the
structure of the real Jobindex, LinkedIn and Indeed alert mails: HTML-only or
multipart, quoted-printable, per-user tokens in every link.
"""

from __future__ import annotations

import io
import json
from pathlib import Path

from mail_scan.scan import run_scan
from mail_scan.sources import open_source

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "mail_scan"


def scan_events(mbox: Path, extractors: list[str] | None = None) -> list[dict]:
    config = {
        "protocol": 1,
        "run_id": "extraction-test",
        "sources": [{"id": "src", "kind": "mbox", "path": str(mbox), "cursor": None}],
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 2000,
            "max_body_chars": 20_000,
        },
        "since": None,
        "cancel_file": None,
    }
    if extractors is not None:
        config["extractors"] = extractors
    out = io.StringIO()
    assert run_scan(config, emit=out) == 0
    return [json.loads(line) for line in out.getvalue().splitlines()]


def scan_listings(mbox: Path, extractors: list[str] | None = None) -> list[dict]:
    return [e for e in scan_events(mbox, extractors) if e["t"] == "listing"]


def open_messages(mbox: Path) -> list:
    opened = open_source(
        "mbox",
        mbox,
        max_messages=5000,
        max_message_bytes=2_097_152,
        max_body_chars=20_000,
    )
    return list(opened.messages)


# --- message access -------------------------------------------------------


def test_extractors_see_the_decoded_message_html() -> None:
    [first, _copy] = open_messages(FIXTURES / "imap_copies.mbox")

    # Quoted-printable soft breaks and =3D escapes are gone; the href is whole.
    assert (
        'href="https://careers.example.org/jobs/4711?ref=mail&amp;'
        'campaign=autumn-hiring-2026-example"'
    ) in first.html
    assert first.plain == ""


def test_encoded_subject_headers_are_decoded() -> None:
    [first, _copy] = open_messages(FIXTURES / "imap_copies.mbox")
    assert first.subject == "Ny stilling: Udvikler i København"


def test_imap_copies_of_one_message_are_scanned_once() -> None:
    events = scan_events(FIXTURES / "imap_copies.mbox", ["generic"])
    listings = [e for e in events if e["t"] == "listing"]

    assert [e["url"] for e in listings] == ["https://careers.example.org/jobs/4711"]
    [finished] = [e for e in events if e["t"] == "source_finished"]
    assert finished["messages_read"] == 2
    assert finished["skipped"] == 1


# --- Jobindex --------------------------------------------------------------


def _summary(listing: dict) -> dict:
    return {
        "title": listing["title"],
        "company": listing["company"],
        "location": listing["location"],
        "url": listing["url"],
        "posted_at": listing["posted_at"],
        "strong": listing["fingerprint"]["strong"],
        "extractor": listing["extractor"],
    }


def test_jobindex_digest_yields_one_listing_per_job_block() -> None:
    listings = scan_listings(FIXTURES / "jobindex_alerts.mbox")

    assert [_summary(e) for e in listings] == [
        {
            "title": "Geodatakonsulent til Klima & Miljø",
            "company": "Fjordby Kommune",
            "location": "Roskilde",
            "url": "https://www.jobindex.dk/c?t=h1000001",
            "posted_at": "2026-09-17",
            "strong": "jobindex:h1000001",
            "extractor": "jobindex",
        },
        {
            "title": "GIS-udvikler (Python)",
            "company": "Nordlys Analytics ApS",
            "location": "Aarhus C",
            "url": "https://www.jobindex.dk/c?t=r20000002",
            "posted_at": "2026-09-18",
            "strong": "jobindex:r20000002",
            "extractor": "jobindex",
        },
        {
            "title": "Backend-udvikler",
            "company": "Kystnet A/S",
            "location": "Odense",
            "url": "https://www.it-jobbank.dk/c?t=r30000005",
            "posted_at": "2026-09-19",
            "strong": "jobindex:r30000005",
            "extractor": "jobindex",
        },
    ]


def test_a_paid_jobindex_ad_keeps_every_visible_paragraph_as_snippet() -> None:
    paid = scan_listings(FIXTURES / "jobindex_alerts.mbox")[0]

    assert "geodata i en kommune" in paid["snippet"]
    assert "team på seks kolleger" in paid["snippet"]
    assert "Ansøgningsfrist" in paid["snippet"]
    assert "SKJULT" not in paid["snippet"], "hidden text must not reach the scorer"
    assert "Gem job" not in paid["snippet"]


def test_a_jobindex_mail_without_job_blocks_yields_nothing() -> None:
    events = scan_events(FIXTURES / "jobindex_alerts.mbox")
    by_message = {e["message_id"] for e in events if e["t"] == "listing"}
    assert "<jobagent-2@jobindex.dk>" not in by_message


def test_jobindex_listings_never_carry_the_user_token() -> None:
    for listing in scan_listings(FIXTURES / "jobindex_alerts.mbox"):
        line = json.dumps(listing)
        assert "FAKEUID" not in line
        assert "abtestid" not in line


# --- LinkedIn --------------------------------------------------------------


def test_linkedin_alert_yields_one_listing_per_job_card() -> None:
    listings = [
        e
        for e in scan_listings(FIXTURES / "linkedin_alerts.mbox")
        if e["message_id"] == "<alert-1@linkedin.com>"
    ]

    assert [_summary(e) for e in listings] == [
        {
            "title": "Data Scientist",
            "company": "Acme Analytics",
            "location": "Kopenhagen",
            "url": "https://www.linkedin.com/jobs/view/4000000001",
            "posted_at": None,
            "strong": "linkedin:4000000001",
            "extractor": "linkedin",
        },
        {
            "title": "Senior Data Engineer \u2013 Plattform & Streaming",
            "company": "Blåvand Energi A/S",
            "location": "Metropolregion Kopenhagen",
            "url": "https://www.linkedin.com/jobs/view/4000000002",
            "posted_at": None,
            "strong": "linkedin:4000000002",
            "extractor": "linkedin",
        },
        {
            "title": "Machine Learning Engineer",
            "company": "Nordhavn Robotics",
            "location": "Aarhus",
            "url": "https://www.linkedin.com/jobs/view/4000000003",
            "posted_at": None,
            "strong": "linkedin:4000000003",
            "extractor": "linkedin",
        },
    ]


def test_linkedin_work_mode_and_salary_reach_the_snippet() -> None:
    first, second, *_ = scan_listings(FIXTURES / "linkedin_alerts.mbox")

    assert "Hybrid" in first["snippet"]
    assert "45.000\u00a0DKK-55.000\u00a0DKK/Monat" in first["snippet"]
    assert "Vor Ort" in second["snippet"]


def test_linkedin_mail_without_job_cards_yields_nothing() -> None:
    events = scan_events(FIXTURES / "linkedin_alerts.mbox")
    by_message = {e["message_id"] for e in events if e["t"] == "listing"}
    assert "<msg-2@linkedin.com>" not in by_message


def test_linkedin_job_recommendations_yield_listings() -> None:
    listings = [
        e
        for e in scan_listings(FIXTURES / "linkedin_alerts.mbox")
        if e["message_id"] == "<jobs-3@linkedin.com>"
    ]

    assert [
        (e["title"], e["company"], e["location"], e["fingerprint"]["strong"])
        for e in listings
    ] == [
        ("Geodata Analyst", "Kløverhus Kommune", "Dänemark", "linkedin:4000000011"),
        ("Data Platform Lead", "Acme Analytics", "Kopenhagen", "linkedin:4000000012"),
    ]
    assert "Remote" in listings[0]["snippet"]


def test_linkedin_listings_never_carry_member_tokens() -> None:
    for listing in scan_listings(FIXTURES / "linkedin_alerts.mbox"):
        line = json.dumps(listing)
        for token in ("midToken", "otpToken", "FAKE", "trackingId", "eid="):
            assert token not in line
