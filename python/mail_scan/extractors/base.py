"""Extractor dispatch."""

from __future__ import annotations

from mail_scan.extractors.generic import extract_generic
from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

__all__ = ["DEFAULT_EXTRACTORS", "ExtractedListing", "extract_listings"]

DEFAULT_EXTRACTORS = ["indeed", "generic"]


def extract_listings(
    message: MailMessage,
    enabled: list[str],
) -> list[ExtractedListing]:
    order = enabled or list(DEFAULT_EXTRACTORS)
    for name in order:
        if name == "indeed":
            found = extract_indeed(message)
            if found:
                return found
        elif name == "generic":
            found = extract_generic(message)
            if found:
                return found
        # jobindex / linkedin land in later PRs
    return []
