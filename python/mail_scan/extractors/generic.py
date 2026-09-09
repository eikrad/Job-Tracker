"""Generic URL-based fallback extractor."""

from __future__ import annotations

import re

from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

_URL = re.compile(r"https?://[^\s\"'<>]+", re.IGNORECASE)


def extract_generic(message: MailMessage) -> list[ExtractedListing]:
    urls = _URL.findall(message.body_text)
    if not urls:
        return []

    # Prefer the subject as title; company/location unknown.
    title = message.subject or "Job listing"
    listings: list[ExtractedListing] = []
    seen: set[str] = set()
    for url in urls:
        if url in seen:
            continue
        seen.add(url)
        listings.append(
            ExtractedListing(
                title=title,
                company="",
                location="",
                url=url.rstrip(").,;"),
                snippet=message.body_text[:500],
                posted_at=None,
                board=None,
                external_id=None,
                extractor="generic",
                extractor_confidence=0.4,
            )
        )
    return listings
