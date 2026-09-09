"""Tiered fingerprint helpers (spec §5.1) — Python side for listing events."""

from __future__ import annotations

import re
import unicodedata

_TRACKING_PARAMS = {
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "gclid",
    "fbclid",
    "from",
    "vjk",
    "trk",
    "refid",
    "refId",
}

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
_URL_PARTS = re.compile(
    r"^(?:(?P<scheme>[a-zA-Z][a-zA-Z0-9+.-]*)://)?"
    r"(?P<host>[^/?#]+)"
    r"(?P<path>/[^?#]*)?"
    r"(?:\?(?P<query>[^#]*))?"
    r"(?:#.*)?$"
)

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


def _parse_query(query: str) -> list[tuple[str, str]]:
    if not query:
        return []
    pairs: list[tuple[str, str]] = []
    for part in query.split("&"):
        if not part:
            continue
        if "=" in part:
            key, value = part.split("=", 1)
        else:
            key, value = part, ""
        pairs.append((key, value))
    return pairs


def canonical_url(url: str) -> str:
    raw = url.strip()
    match = _URL_PARTS.match(raw)
    if not match:
        return raw.lower()

    scheme = (match.group("scheme") or "https").lower()
    host_raw = match.group("host") or ""
    # Drop userinfo if present.
    if "@" in host_raw:
        host_raw = host_raw.rsplit("@", 1)[-1]
    host = host_raw.lower()
    port = None
    if not (host.startswith("[") and "]" in host) and ":" in host:
        host, port_s = host.rsplit(":", 1)
        if port_s.isdigit():
            port = port_s
    if host.startswith("www."):
        host = host[4:]

    path = match.group("path") or ""
    if path.endswith("/") and len(path) > 1:
        path = path[:-1]

    pairs = _parse_query(match.group("query") or "")

    # Unwrap Indeed click-through.
    if "indeed." in host and path.rstrip("/").endswith("/rc/clk"):
        jk = next((v for k, v in pairs if k.lower() == "jk" and v), None)
        if jk:
            return f"https://{host}/viewjob?jk={jk}"

    tracking = {p.lower() for p in _TRACKING_PARAMS}
    kept = [(k, v) for k, v in pairs if k.lower() not in tracking]
    query_out = "&".join(f"{k}={v}" for k, v in kept)

    netloc = host if port is None else f"{host}:{port}"
    base = f"{scheme}://{netloc}{path}"
    if query_out:
        return f"{base}?{query_out}"
    return base


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
