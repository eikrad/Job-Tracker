"""Tiered fingerprint helpers (spec §5.1).

The sidecar is the only place fingerprints are computed; Rust stores the keys it
receives. URL canonicalization lives in ``urls`` with the rest of URL hygiene.
"""

from __future__ import annotations

import re
import unicodedata

from mail_scan.urls import canonical_url

__all__ = ["canonical_url", "fingerprint", "normalize_text", "weak_key"]

_LEGAL_SUFFIX = re.compile(
    r"(?:^|\s)(?:a/s|aps|gmbh|ivs|ab|as|ltd|inc)\.?$",
    re.IGNORECASE,
)
_GENDER_MARK = re.compile(
    r"\((?:m/w/d|m/f/d|m/w|w/m/d|f/m/d)\)",
    re.IGNORECASE,
)
# EN DASH (\u2013) and hyphen-minus before "remote".
_REMOTE_SUFFIX = re.compile(r"[\u2013\-]\s*remote\b.*$", re.IGNORECASE)
_PUNCT = re.compile(r"[^\w\s|]+", re.UNICODE)
_WS = re.compile(r"\s+")
_DIACRITICS = str.maketrans(
    {
        "ø": "o",
        "Ø": "o",
        "å": "a",
        "Å": "a",
        "ä": "a",
        "Ä": "a",
        "ö": "o",
        "Ö": "o",
        "ü": "u",
        "Ü": "u",
        "æ": "ae",
        "Æ": "ae",
        "ß": "ss",
    }
)


def normalize_text(value: str) -> str:
    text = value.translate(_DIACRITICS)
    text = unicodedata.normalize("NFKD", text)
    text = "".join(c for c in text if not unicodedata.combining(c))
    text = text.lower()
    text = _GENDER_MARK.sub("", text)
    text = _REMOTE_SUFFIX.sub("", text)
    # Strip legal suffixes repeatedly from the end.
    while True:
        stripped = _LEGAL_SUFFIX.sub("", text).rstrip(" .,")
        if stripped == text:
            break
        text = stripped
    text = _PUNCT.sub(" ", text)
    text = _WS.sub(" ", text).strip()
    return text


def weak_key(company: str, title: str, location: str) -> str:
    return "|".join(
        (
            normalize_text(company),
            normalize_text(title),
            normalize_text(location),
        )
    )


def strong_key_from_external(board: str, external_id: str) -> str:
    return f"{board}:{external_id}"


def strong_key_from_url(url: str) -> str:
    return f"url:{canonical_url(url)}"


def fingerprint(
    *,
    board: str | None,
    external_id: str | None,
    url: str,
    company: str,
    title: str,
    location: str,
) -> dict[str, str | None]:
    strong: str | None = None
    if board and external_id:
        strong = strong_key_from_external(board, external_id)
    elif url.strip():
        strong = strong_key_from_url(url)
    return {"strong": strong, "weak": weak_key(company, title, location)}
