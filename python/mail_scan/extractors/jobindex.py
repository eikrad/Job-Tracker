"""Jobindex job-agent digest extractor (also it-jobbank.dk, same markup).

A digest is HTML-only. Each job is a block keyed by its Jobindex id (``r…`` for a
crawled ad, ``h…`` for one hosted on Jobindex)::

    div#jix_toolbar_<ID>_top  → company link
    h4 > a[href=…/c?t=<ID>…]  → title
    table td (first)          → location
    p…                        → teaser; a paid ad carries several paragraphs
    div#jix_toolbar_<ID>      → INDRYKKET: <time datetime="YYYY-MM-DD">

Every link carries the recipient's ``uid`` and an A/B id; the listing URL is rebuilt
from the id alone. ``t=e…`` (apply), ``t=m…``/``c…`` (marketing, view-online) links
are not jobs. A digest with no job blocks ("N nye job siden sidst") yields nothing.
"""

from __future__ import annotations

import re

from mail_scan.extractors.types import ExtractedListing
from mail_scan.html_dom import Element, parse_html, render_visible, squash
from mail_scan.sources import MailMessage

BOARD = "jobindex"
_HOSTS = ("jobindex.dk", "it-jobbank.dk")

_TOOLBAR_TOP = re.compile(r"^jix_toolbar_([rh]\d+)_top$")
_JOB_LINK = re.compile(
    r"https?://(?:www\.)?(?P<host>jobindex\.dk|it-jobbank\.dk)/c\?"
    r"(?:[^\"'\s]*?[&;])?t=(?P<id>[rh]\d+)(?![\w])",
    re.IGNORECASE,
)


def _claims(message: MailMessage) -> bool:
    sender = message.from_addr.lower()
    return any(host in sender for host in _HOSTS) or "jix_toolbar_" in message.html


def _title_link(root: Element, job_id: str) -> tuple[Element, str] | None:
    """The ``h4`` holding the job's title link, and the host that link points at."""
    for h4 in root.find_all("h4"):
        for a in h4.find_all("a"):
            m = _JOB_LINK.match(a.get("href"))
            if m and m.group("id") == job_id:
                return h4, m.group("host").lower()
    return None


def _location_and_snippet(h4: Element) -> tuple[str, str]:
    """Location is the first cell of the table after the title; the rest is teaser."""
    container = h4.parent
    if container is None:
        return "", ""
    siblings = container.children
    after = siblings[siblings.index(h4) + 1 :]
    location = ""
    for i, node in enumerate(after):
        if isinstance(node, Element) and node.tag == "table":
            cell = node.find("td")
            location = squash(cell.text()) if cell is not None else ""
            after = after[i + 1 :]
            break
    return location, render_visible(after)


def _posted_at(root: Element, job_id: str) -> str | None:
    toolbar = root.find(where=lambda el: el.get("id") == f"jix_toolbar_{job_id}")
    if toolbar is None:
        return None
    stamp = toolbar.find("time")
    return (stamp.get("datetime") or None) if stamp is not None else None


def extract_jobindex(message: MailMessage) -> list[ExtractedListing] | None:
    """Listings in a Jobindex digest; ``None`` when the mail is not Jobindex's."""
    if not _claims(message):
        return None
    root = parse_html(message.html)
    listings: list[ExtractedListing] = []
    seen: set[str] = set()
    for top in root.find_all("div", where=lambda el: "jix_toolbar_" in el.get("id")):
        m = _TOOLBAR_TOP.match(top.get("id"))
        if not m or m.group(1) in seen:
            continue
        job_id = m.group(1)
        found = _title_link(root, job_id)
        if found is None:
            continue
        h4, host = found
        seen.add(job_id)
        location, snippet = _location_and_snippet(h4)
        listings.append(
            ExtractedListing(
                title=squash(h4.text()),
                company=squash(top.text()),
                location=location,
                url=f"https://www.{host}/c?t={job_id}",
                snippet=snippet,
                posted_at=_posted_at(root, job_id),
                board=BOARD,
                external_id=job_id,
                extractor=BOARD,
                extractor_confidence=0.95,
            )
        )
    return listings
