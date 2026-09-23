"""Shared extractor result type."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ExtractedListing:
    title: str
    company: str
    location: str
    url: str
    snippet: str
    posted_at: str | None
    board: str | None
    external_id: str | None
    extractor: str
    extractor_confidence: float
    # False when the URL changes from mail to mail for the same job (a sponsored
    # click link): the listing is then keyed by what it says, not by its URL.
    url_is_identity: bool = True
