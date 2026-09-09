"""Indeed digest extractor."""

from __future__ import annotations

import re

from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

# Title - Company - Location followed by an Indeed viewjob URL.
_LINE = re.compile(
    r"^(?P<title>.+?)\s+-\s+(?P<company>.+?)\s+-\s+(?P<location>.+?)\s*$"
)
_JK_URL = re.compile(
    r"https?://(?:[\w.-]+\.)?indeed\.[a-z.]+/(?:viewjob|rc/clk)\?[^\s\"'<>]*jk=([a-fA-F0-9]+)",
    re.IGNORECASE,
)


def _looks_like_indeed(message: MailMessage) -> bool:
    blob = f"{message.from_addr}\n{message.subject}\n{message.body_text}".lower()
    return "indeed" in blob


def extract_indeed(message: MailMessage) -> list[ExtractedListing]:
    if not _looks_like_indeed(message):
        return []

    lines = [ln.strip() for ln in message.body_text.splitlines() if ln.strip()]
    listings: list[ExtractedListing] = []
    i = 0
    while i < len(lines):
        m = _LINE.match(lines[i])
        if not m:
            i += 1
            continue
        url = ""
        jk = None
        if i + 1 < len(lines):
            um = _JK_URL.search(lines[i + 1])
            if um:
                jk = um.group(1).lower()
                url = lines[i + 1].strip()
                i += 2
            else:
                i += 1
                continue
        else:
            i += 1
            continue

        title = m.group("title").strip()
        company = m.group("company").strip()
        location = m.group("location").strip()
        snippet = f"{title} at {company} ({location})"
        listings.append(
            ExtractedListing(
                title=title,
                company=company,
                location=location,
                url=url,
                snippet=snippet,
                posted_at=None,
                board="indeed",
                external_id=jk,
                extractor="indeed",
                extractor_confidence=0.92,
            )
        )
    return listings
