"""Indeed job-alert extractor.

Job alerts (``jobalert.indeed.com``) are multipart; the text/plain part has one
block per listing, separated by blank lines::

    Title
    Company - Location
    [salary, "Nem ansøgning gennem Indeed", teaser … — any number of lines]
    (2 dage siden) | Lige opslået
    https://dk.indeed.com/rc/clk/dl?jk=<16 hex>&tk=…&alid=…

The HTML part (and HTML-only mail such as ``match.indeed.com`` invitations) has
``h2 > a.strong-text-link`` titles followed by company and location paragraphs.
Match mail wraps every link in ``cts.indeed.com/v3/<base64url(gzip(JSON))>/<sig>``;
the JSON names the target and a ``clickType``, and only ``viewjob`` is a job.

Links carry the recipient's ``tk``/``alid`` tokens. A listing URL is rebuilt as
``https://<country>.indeed.com/viewjob?jk=<jk>``. A sponsored ``/pagead/clk`` link
has no job key; it keeps only its ``ad`` parameter and is keyed by what the listing
says (weak key), not by a URL that changes per mail.
"""

from __future__ import annotations

import base64
import binascii
import json
import re
import zlib
from dataclasses import dataclass
from datetime import datetime, timedelta

from mail_scan.extractors.types import ExtractedListing
from mail_scan.html_dom import parse_html, squash
from mail_scan.sources import MailMessage

BOARD = "indeed"

_HOST = r"(?P<host>(?:[\w-]+\.)*indeed\.[a-z]{2,}(?:\.[a-z]{2})?)"
_JOB_URL = re.compile(
    rf"^https?://{_HOST}/(?:rc/clk(?:/dl)?|viewjob)\?(?:\S*?[&;])?"
    r"jk=(?P<jk>[0-9a-fA-F]+)(?![0-9a-zA-Z])",
    re.IGNORECASE,
)
_AD_URL = re.compile(
    rf"^https?://{_HOST}/pagead/clk(?:/dl)?\?(?:\S*?[&;])?ad=(?P<ad>[^&\s]+)",
    re.IGNORECASE,
)
_CTS_URL = re.compile(r"^https?://cts\.indeed\.com/v3/(?P<blob>[\w-]+)/", re.IGNORECASE)
_URL_LINE = re.compile(r"^https?://\S+$")

# "(2 dage siden)", "Lige opslået", "vor 3 Tagen", "Just posted", "30+ days ago"
_AGE_DAYS = re.compile(
    r"^\(?\s*(?:vor\s+)?(?P<n>\d+)\+?\s*"
    r"(?:dag|dage|tag|tagen|day|days)\b(?:\s+(?:siden|ago))?\s*\)?$",
    re.IGNORECASE,
)
_AGE_TODAY = re.compile(
    r"^\(?\s*(?:lige opslået|i dag|gerade veröffentlicht|heute|just posted|today)"
    r"\s*\)?$",
    re.IGNORECASE,
)
_MAX_HTML_DETAIL_LINES = 6
# A click-link payload is a few hundred bytes of JSON; the cap keeps a hostile
# gzip bomb in a mail from expanding without bound.
_MAX_CTS_JSON_BYTES = 64 * 1024


@dataclass(frozen=True)
class _Link:
    url: str
    jk: str | None


def _claims(message: MailMessage) -> bool:
    return "indeed." in message.from_addr.lower()


def _decode_cts(url: str) -> str | None:
    """Target of a ``cts.indeed.com`` click link, if it is a view-job click."""
    m = _CTS_URL.match(url)
    if m is None:
        return None
    blob = m.group("blob")
    try:
        packed = base64.urlsafe_b64decode(blob + "=" * (-len(blob) % 4))
        inflater = zlib.decompressobj(16 + zlib.MAX_WBITS)  # gzip framing
        raw = inflater.decompress(packed, _MAX_CTS_JSON_BYTES)
        if inflater.unconsumed_tail:
            return None
        data = json.loads(raw)
    except (binascii.Error, zlib.error, ValueError):
        return None
    if not isinstance(data, dict):
        return None
    meta = data.get("m")
    target = data.get("u")
    if not isinstance(meta, dict) or meta.get("clickType") != "viewjob":
        return None
    return target if isinstance(target, str) else None


def _job_link(url: str) -> _Link | None:
    url = url.strip()
    url = _decode_cts(url) or url
    m = _JOB_URL.match(url)
    if m:
        host = m.group("host").lower()
        jk = m.group("jk").lower()
        return _Link(f"https://{host}/viewjob?jk={jk}", jk)
    m = _AD_URL.match(url)
    if m:
        return _Link(
            f"https://{m.group('host').lower()}/pagead/clk/dl?ad={m.group('ad')}", None
        )
    return None


def _posted_at(age: str, message_date: str) -> str | None:
    days: int | None = None
    m = _AGE_DAYS.match(age.strip())
    if m:
        days = int(m.group("n"))
    elif _AGE_TODAY.match(age.strip()):
        days = 0
    if days is None:
        return None
    try:
        sent = datetime.strptime(message_date[:10], "%Y-%m-%d")
    except ValueError:
        return None
    return (sent - timedelta(days=days)).strftime("%Y-%m-%d")


def _is_age(line: str) -> bool:
    return bool(_AGE_DAYS.match(line.strip()) or _AGE_TODAY.match(line.strip()))


def _split_company_location(line: str) -> tuple[str, str]:
    # Company names contain " - " ("A.P. Moller - Maersk"); locations rarely do.
    company, sep, location = line.rpartition(" - ")
    if not sep:
        return line.strip(), ""
    return company.strip(), location.strip()


def _listing(
    link: _Link,
    title: str,
    company: str,
    location: str,
    details: list[str],
    message_date: str,
) -> ExtractedListing:
    ages = [d for d in details if _is_age(d)]
    return ExtractedListing(
        title=title,
        company=company,
        location=location,
        url=link.url,
        snippet="\n".join(d for d in details if not _is_age(d)),
        posted_at=_posted_at(ages[0], message_date) if ages else None,
        board=BOARD if link.jk else None,
        external_id=link.jk,
        extractor=BOARD,
        extractor_confidence=0.92,
        url_is_identity=link.jk is not None,
    )


def _from_plain(message: MailMessage) -> list[ExtractedListing]:
    listings: list[ExtractedListing] = []
    blocks = re.split(r"\n\s*\n", message.plain.replace("\r\n", "\n"))
    for block in blocks:
        lines = [ln.strip() for ln in block.split("\n") if ln.strip()]
        if len(lines) < 3 or not _URL_LINE.match(lines[-1]):
            continue
        link = _job_link(lines[-1])
        if link is None:
            continue
        company, location = _split_company_location(lines[1])
        listings.append(
            _listing(
                link, lines[0], company, location, lines[2:-1], message.message_date
            )
        )
    return listings


def _from_html(message: MailMessage) -> list[ExtractedListing]:
    root = parse_html(message.html)
    order = list(root.iter())
    listings: list[ExtractedListing] = []
    for i, h2 in enumerate(order):
        if h2.tag != "h2":
            continue
        anchor = h2.find("a", where=lambda a: "strong-text-link" in a.classes)
        link = _job_link(anchor.get("href")) if anchor is not None else None
        if anchor is None or link is None:
            continue
        lines: list[str] = []
        for el in order[i + 1 :]:
            if el.tag == "h2" or len(lines) >= _MAX_HTML_DETAIL_LINES:
                break
            if el.tag == "p" and el.visible:
                text = squash(el.text())
                if text:
                    lines.append(text)
        company = lines[0] if lines else ""
        location = lines[1] if len(lines) > 1 else ""
        listings.append(
            _listing(
                link,
                squash(anchor.text()),
                company,
                location,
                lines[2:],
                message.message_date,
            )
        )
    return listings


def extract_indeed(message: MailMessage) -> list[ExtractedListing] | None:
    """Listings in an Indeed mail; ``None`` when the mail is not Indeed's."""
    if not _claims(message):
        return None
    found = _from_plain(message) or _from_html(message)
    listings: list[ExtractedListing] = []
    seen: set[str] = set()
    for item in found:
        # A match mail links the same job from its button, title and footer.
        if item.url in seen:
            continue
        seen.add(item.url)
        listings.append(item)
    return listings
