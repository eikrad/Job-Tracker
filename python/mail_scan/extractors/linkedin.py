"""LinkedIn job-alert extractor.

Alerts are HTML-only; each job is a ``td[data-test-id="job-card"]``::

    a[href=…/comm/jobs/view/<id>/?…]  → title (innermost job link with text)
    p  "Company · Location (Mode)"     → company, location, work mode
    p  "45.000 DKK-55.000 DKK/Monat"   → salary, when LinkedIn shows one

Recommendation mails ("Jobs für Sie", "<Company> hat eine offene Stelle") use a
``*-job-cards`` section instead, where each card is one job link wrapping a
``.font-bold`` title and the same ``Company · Location`` line.

Every job link's query string carries member tokens (``midToken``, ``otpToken``,
``eid``, ``trackingId``); the listing URL is rebuilt from the job id alone. Mail
from linkedin.com without job cards (messages, invitations, the feed) yields nothing.
"""

from __future__ import annotations

import re

from mail_scan.extractors.types import ExtractedListing
from mail_scan.html_dom import Element, parse_html, squash
from mail_scan.sources import MailMessage

BOARD = "linkedin"

_JOB_LINK = re.compile(
    r"https?://(?:[\w-]+\.)?linkedin\.com/(?:comm/)?jobs/view/(\d+)", re.IGNORECASE
)
_META_SEPARATOR = " · "
# "Kopenhagen (Hybrid)" → location "Kopenhagen"; the mode stays in the snippet.
_MODE_SUFFIX = re.compile(r"\s*\([^()]*\)\s*$")


def _claims(message: MailMessage) -> bool:
    return "linkedin.com" in message.from_addr.lower() or (
        'data-test-id="job-card"' in message.html
    )


def _cards(root: Element) -> list[Element]:
    cards: list[Element] = []
    for el in root.iter():
        test_id = el.get("data-test-id")
        if test_id == "job-card":
            cards.append(el)
        elif test_id.endswith("-job-cards"):
            cards.extend(
                a
                for a in el.find_all("a")
                if _JOB_LINK.match(a.get("href")) and a.find("a") is None
            )
    return cards


def _title_and_id(card: Element) -> tuple[str, str] | None:
    if card.tag == "a":
        m = _JOB_LINK.match(card.get("href"))
        heading = card.find(where=lambda el: "font-bold" in el.classes)
        title = squash(heading.text()) if heading is not None else ""
        return (title, m.group(1)) if m and title else None
    for a in card.find_all("a"):
        m = _JOB_LINK.match(a.get("href"))
        # A card-wide link wraps the title link; the title is the innermost one.
        if m is None or a.find("a") is not None:
            continue
        title = squash(a.text())
        if title:
            return title, m.group(1)
    return None


def _company_and_location(card: Element) -> tuple[str, str]:
    for p in card.find_all("p"):
        line = squash(p.text())
        if _META_SEPARATOR in line:
            company, _, place = line.partition(_META_SEPARATOR)
            return company.strip(), _MODE_SUFFIX.sub("", place).strip()
    return "", ""


def extract_linkedin(message: MailMessage) -> list[ExtractedListing] | None:
    """Listings in a LinkedIn alert; ``None`` when the mail is not LinkedIn's."""
    if not _claims(message):
        return None
    listings: list[ExtractedListing] = []
    seen: set[str] = set()
    for card in _cards(parse_html(message.html)):
        found = _title_and_id(card)
        if found is None:
            continue
        title, job_id = found
        # The same job can appear twice in one alert (top pick and list entry).
        if job_id in seen:
            continue
        seen.add(job_id)
        company, location = _company_and_location(card)
        listings.append(
            ExtractedListing(
                title=title,
                company=company,
                location=location,
                url=f"https://www.linkedin.com/jobs/view/{job_id}",
                snippet=card.text(),
                posted_at=None,
                board=BOARD,
                external_id=job_id,
                extractor=BOARD,
                extractor_confidence=0.95,
            )
        )
    return listings
