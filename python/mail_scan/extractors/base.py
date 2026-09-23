"""Extractor dispatch.

Board extractors run first and return ``None`` for mail that is not their board's.
A board that claims a message but finds no listings in it (a marketing mail, a
"nothing new" digest) returns ``[]``, which stops dispatch: the generic URL
fallback must not turn a board's footer links into listings.
"""

from __future__ import annotations

from collections.abc import Callable

from mail_scan.extractors.generic import extract_generic
from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.jobindex import extract_jobindex
from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

__all__ = ["DEFAULT_EXTRACTORS", "ExtractedListing", "extract_listings"]

Extractor = Callable[[MailMessage], list[ExtractedListing] | None]

_EXTRACTORS: dict[str, Extractor] = {
    "jobindex": extract_jobindex,
    "indeed": extract_indeed,
    "generic": extract_generic,
}

DEFAULT_EXTRACTORS = list(_EXTRACTORS)


def extract_listings(
    message: MailMessage,
    enabled: list[str],
) -> list[ExtractedListing]:
    for name in enabled or DEFAULT_EXTRACTORS:
        extractor = _EXTRACTORS.get(name)
        if extractor is None:
            continue
        found = extractor(message)
        if found is not None:
            return found
    return []
