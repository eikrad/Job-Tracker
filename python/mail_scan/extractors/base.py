"""Extractor dispatch."""

from __future__ import annotations

from mail_scan.extractors.generic import extract_generic
from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

__all__ = ["ExtractedListing", "extract_listings"]


def extract_listings(
    message: MailMessage,
    enabled: list[str],
) -> list[ExtractedListing]:
    order = enabled or ["indeed", "generic"]
    for name in order:
        if name == "indeed":
            found = extract_indeed(message)
            if found:
                return found
        elif name == "generic":
            found = extract_generic(message)
            if found:
                return found
        # jobindex / linkedin land in B3
    return []
