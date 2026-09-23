"""Scan orchestration: config → NDJSON events."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, TextIO

from mail_scan import __version__
from mail_scan.digest import build_digest
from mail_scan.events import emit_event, log_warn
from mail_scan.exit_codes import EXIT_CANCELLED, EXIT_OK
from mail_scan.extractors.base import DEFAULT_EXTRACTORS, DIGEST, extract_listings
from mail_scan.fingerprint import fingerprint
from mail_scan.sources import open_source
from mail_scan.urls import clean_url

# Protocol 2 added the `digest` event (mail no board extractor recognises).
PROTOCOL = 2


def _cancel_requested(cancel_file: str | None) -> bool:
    if not cancel_file:
        return False
    return Path(cancel_file).exists()


def _require_config(config: dict[str, Any]) -> dict[str, Any]:
    if config.get("protocol") != PROTOCOL:
        raise ValueError(
            f"config protocol mismatch: got {config.get('protocol')!r}, "
            f"expected {PROTOCOL}"
        )
    if not isinstance(config.get("run_id"), str) or not config["run_id"]:
        raise ValueError("run_id is required")
    if not isinstance(config.get("sources"), list):
        raise ValueError("sources must be a list")
    limits = config.get("limits")
    if not isinstance(limits, dict):
        raise ValueError("limits must be an object")
    for key in (
        "max_messages_per_source",
        "max_message_bytes",
        "max_listings_per_run",
        "max_body_chars",
    ):
        if key not in limits:
            raise ValueError(f"limits.{key} is required")
    return config


def run_scan(config: dict[str, Any], *, emit: TextIO) -> int:
    cfg = _require_config(config)
    started = time.monotonic()
    sources = cfg["sources"]
    limits = cfg["limits"]
    extractors = list(cfg.get("extractors") or DEFAULT_EXTRACTORS)
    cancel_file = cfg.get("cancel_file")
    # Hard floor; older mail skipped when message_date is comparable (B3 deepens).
    since = cfg.get("since")

    emit_event(
        emit,
        {
            "t": "started",
            "protocol": PROTOCOL,
            "run_id": cfg["run_id"],
            "sidecar_version": __version__,
            "sources": len(sources),
        },
    )

    listings_total = 0
    messages_total = 0
    cancelled = False
    # Set when max_listings_per_run stops the scan. The source being read keeps a
    # cursor on the last mail it finished; later sources are not touched at all, so
    # the next scan continues where this one stopped instead of losing that mail.
    truncated = False
    # IMAP folders hold copies of one message under the same Message-ID (a label
    # applied twice, a move that left the original behind). One message is one set
    # of listings, however many copies the scan walks past.
    seen_message_ids: set[str] = set()

    for source in sources:
        if _cancel_requested(cancel_file):
            cancelled = True
            break

        if not isinstance(source, dict):
            raise ValueError("each source must be an object")
        source_id = source.get("id")
        kind = source.get("kind")
        path_raw = source.get("path")
        if not source_id or not kind or not path_raw:
            raise ValueError("source requires id, kind, and path")

        path = Path(path_raw)
        opened = open_source(
            kind,
            path,
            max_messages=int(limits["max_messages_per_source"]),
            max_message_bytes=int(limits["max_message_bytes"]),
            max_body_chars=int(limits["max_body_chars"]),
            stored_cursor=source.get("cursor"),
        )
        messages = opened.messages
        finalize = opened.finalize

        if opened.cursor_reset:
            emit_event(
                emit,
                {
                    "t": "warning",
                    "code": "W_CURSOR_RESET",
                    "source": source_id,
                    "detail": opened.reset_reason or "cursor reset",
                },
            )

        # estimated_messages is best-effort; 0 is fine for fixtures.
        emit_event(
            emit,
            {
                "t": "source_started",
                "source": source_id,
                "estimated_messages": 0,
            },
        )

        messages_read = 0
        listings = 0
        skipped = 0

        for mail in messages:
            if _cancel_requested(cancel_file):
                cancelled = True
                break

            messages_read += 1
            messages_total += 1

            if since and mail.message_date and mail.message_date < since:
                skipped += 1
                continue

            if mail.message_id:
                if mail.message_id in seen_message_ids:
                    skipped += 1
                    continue
                seen_message_ids.add(mail.message_id)

            if listings_total >= int(limits["max_listings_per_run"]):
                # This mail stays unread: breaking before asking for the next one
                # keeps the source cursor on the last mail that was finished.
                truncated = True
                break

            extracted = extract_listings(mail, extractors)
            if extracted is None and DIGEST in extractors:
                digest = build_digest(mail, int(limits["max_body_chars"]))
                if digest is None:
                    skipped += 1
                    continue
                emit_event(
                    emit,
                    {
                        "t": "digest",
                        "source": source_id,
                        "message_id": mail.message_id,
                        "message_date": mail.message_date,
                        "subject": mail.subject,
                        "sender": mail.from_addr,
                        "message_fingerprint": digest.fingerprint,
                        "body": digest.body,
                        "links": digest.links,
                    },
                )
                continue
            if not extracted:
                skipped += 1
                continue

            # A started mail is always finished: splitting one would re-send its first
            # listings next run, and a mail larger than the limit would never pass.
            for item in extracted:
                # Whatever an extractor found, no per-user token is emitted.
                url = clean_url(item.url)
                fp = fingerprint(
                    board=item.board,
                    external_id=item.external_id,
                    url=url if item.url_is_identity else "",
                    company=item.company,
                    title=item.title,
                    location=item.location,
                )
                event: dict[str, Any] = {
                    "t": "listing",
                    "source": source_id,
                    "message_id": mail.message_id,
                    "message_date": mail.message_date,
                    "seq": listings,
                    "title": item.title,
                    "company": item.company,
                    "location": item.location,
                    "url": url,
                    "snippet": item.snippet[: int(limits["max_body_chars"])],
                    "posted_at": item.posted_at,
                    "fingerprint": fp,
                    "extractor": item.extractor,
                    "extractor_confidence": item.extractor_confidence,
                }
                if item.board and item.external_id:
                    event["external_ref"] = {
                        "board": item.board,
                        "id": item.external_id,
                    }
                emit_event(emit, event)
                listings += 1
                listings_total += 1

        cursor = finalize()
        emit_event(
            emit,
            {
                "t": "source_finished",
                "source": source_id,
                "messages_read": messages_read,
                "listings": listings,
                "skipped": skipped,
                "cursor": cursor.as_dict(),
            },
        )

        if cancelled or truncated:
            break

    duration_ms = int((time.monotonic() - started) * 1000)
    finished: dict[str, Any] = {
        "t": "finished",
        "listings_total": listings_total,
        "messages_total": messages_total,
        "duration_ms": duration_ms,
    }
    if cancelled:
        finished["cancelled"] = True
    if truncated:
        finished["truncated"] = True
    emit_event(emit, finished)

    if cancelled:
        log_warn("scan cancelled via cancel_file")
        return EXIT_CANCELLED
    return EXIT_OK
