"""Extractor dispatch.

Board extractors run first and return ``None`` for mail that is not their board's.
A board that claims a message but finds no listings in it (a marketing mail, a
"nothing new" digest) returns ``[]``, which stops dispatch.

Mail no board claims is not guessed at here: ``extract_listings`` returns ``None``
and the scan emits it as a digest for the model to split (``mail_scan.digest``).
``DIGEST`` names that fallback in the configured extractor list.
"""

from __future__ import annotations

from collections.abc import Callable

from mail_scan.extractors.indeed import extract_indeed
from mail_scan.extractors.jobindex import extract_jobindex
from mail_scan.extractors.linkedin import extract_linkedin
from mail_scan.extractors.types import ExtractedListing
from mail_scan.sources import MailMessage

__all__ = ["DEFAULT_EXTRACTORS", "DIGEST", "ExtractedListing", "extract_listings"]

Extractor = Callable[[MailMessage], list[ExtractedListing] | None]

_EXTRACTORS: dict[str, Extractor] = {
    "jobindex": extract_jobindex,
    "linkedin": extract_linkedin,
    "indeed": extract_indeed,
}

# Not an extractor function: the scan's fallback for mail no board claims.
DIGEST = "digest"

DEFAULT_EXTRACTORS = [*_EXTRACTORS, DIGEST]


def extract_listings(
    message: MailMessage,
    enabled: list[str],
) -> list[ExtractedListing] | None:
    """Listings from the first board that claims ``message``; ``None`` if none does."""
    for name in enabled or DEFAULT_EXTRACTORS:
        extractor = _EXTRACTORS.get(name)
        if extractor is None:
            continue
        found = extractor(message)
        if found is not None:
            return found
    return None
