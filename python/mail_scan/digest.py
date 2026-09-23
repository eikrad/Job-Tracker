"""Digests: alert mail no board extractor recognises, prepared for the model.

An unknown board's markup gives no reliable listing boundaries, so guessing listings
here (the old subject-as-title fallback) produced one junk listing per footer link.
Instead the sidecar emits the mail's *visible* text with every admissible link
replaced by a numbered reference ``[anchor text][L3]``, plus a table from reference
to token-free URL and its precomputed fingerprint keys. Rust asks the model to split
the text into listings; the model can only name an id from the table, so it can never
introduce a URL (spec §6.2), and fingerprints stay computed here (spec §5.1).
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass

from mail_scan.fingerprint import strong_key_from_url
from mail_scan.html_dom import Element, parse_html
from mail_scan.html_text import BLOCK_ELEMENTS, normalize_rendered_text
from mail_scan.sources import MailMessage
from mail_scan.urls import canonical_url, clean_url, is_public_http_url

# Bounds that keep one digest event far below the 256 KiB line cap: a 20 000-char
# body plus at most this many links of at most this length (each carried three times:
# URL, strong key, weak key).
MAX_LINKS = 50
MAX_URL_CHARS = 1_000

_URL = re.compile(r"https?://[^\s\"'<>]+", re.IGNORECASE)
_REF = re.compile(r"\[(L\d+)\]")


@dataclass(frozen=True)
class Digest:
    body: str
    # id -> {"url": ..., "fingerprint": {"strong": ..., "weak": ...}}, in id order.
    links: dict[str, dict]
    fingerprint: str


class _LinkTable:
    """Assigns ``L1``, ``L2``, … per distinct canonical URL, in order of appearance."""

    def __init__(self) -> None:
        self._ids: dict[str, str] = {}
        self.entries: dict[str, dict] = {}

    def ref(self, raw_url: str) -> str | None:
        raw = raw_url.strip()
        if not is_public_http_url(raw):
            return None
        url = clean_url(raw)
        if len(url) > MAX_URL_CHARS:
            return None
        key = canonical_url(url)
        if key in self._ids:
            return self._ids[key]
        if len(self._ids) >= MAX_LINKS:
            return None
        link_id = f"L{len(self._ids) + 1}"
        self._ids[key] = link_id
        strong = strong_key_from_url(url)
        # The weak key would need the company and title the model has yet to name. A
        # guessed one (the anchor text) would mark unrelated jobs with the same title
        # as near-duplicates, so a digest listing is keyed by its link alone.
        self.entries[link_id] = {
            "url": url,
            "fingerprint": {"strong": strong, "weak": strong},
        }
        return link_id


def _anchor_label(text: str) -> str:
    # Brackets inside the label would make the reference ambiguous to the model.
    label = " ".join(text.split()).replace("[", "(").replace("]", ")")
    return label


def _render_html(root: Element, links: _LinkTable) -> str:
    chunks: list[str] = []

    def walk(items: list[Element | str]) -> None:
        for item in items:
            if isinstance(item, str):
                # A bare URL in the prose is a link too.
                chunks.append(_link_bare_urls(item, links))
                continue
            if item.hidden:
                continue
            if item.tag == "a":
                link_id = links.ref(item.get("href"))
                if link_id is not None:
                    label = _anchor_label(item.text())
                    chunks.append(f"[{label}][{link_id}]" if label else f"[{link_id}]")
                    continue
            block = item.tag in BLOCK_ELEMENTS
            if block:
                chunks.append("\n")
            walk(item.children)
            if block:
                chunks.append("\n")

    walk(root.children)
    return normalize_rendered_text("".join(chunks))


def _link_bare_urls(text: str, links: _LinkTable) -> str:
    def replace(match: re.Match[str]) -> str:
        raw = match.group(0)
        trimmed = raw.rstrip(").,;")
        link_id = links.ref(trimmed)
        if link_id is None:
            return raw
        return f"[{link_id}]" + raw[len(trimmed) :]

    return _URL.sub(replace, text)


def _render_plain(text: str, links: _LinkTable) -> str:
    return normalize_rendered_text(_link_bare_urls(text, links))


def _truncate(body: str, max_chars: int) -> str:
    if len(body) <= max_chars:
        return body
    cut = body[:max_chars]
    # Never leave half a reference behind: "[Title][L1" reads like an id.
    open_at = cut.rfind("[")
    if open_at != -1 and "]" not in cut[open_at:]:
        cut = cut[:open_at]
    return cut.rstrip()


def build_digest(message: MailMessage, max_chars: int) -> Digest | None:
    """The digest for ``message``, or ``None`` when it has no admissible link."""
    links = _LinkTable()
    if message.html.strip():
        body = _render_html(parse_html(message.html), links)
    else:
        body = _render_plain(message.plain or message.body_text, links)

    body = _truncate(body, max_chars)
    referenced = set(_REF.findall(body))
    kept = {k: v for k, v in links.entries.items() if k in referenced}
    if not kept:
        return None

    digest_hash = hashlib.sha256()
    for part in (message.subject, message.from_addr, body, json.dumps(kept)):
        digest_hash.update(part.encode("utf-8"))
        digest_hash.update(b"\x00")
    return Digest(
        body=body, links=kept, fingerprint="msg:" + digest_hash.hexdigest()[:32]
    )
