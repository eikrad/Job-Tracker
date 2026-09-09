"""Scan orchestration: config → NDJSON events."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, TextIO

from mail_scan import __version__
from mail_scan.events import emit_event, log_warn
from mail_scan.exit_codes import EXIT_CANCELLED, EXIT_OK
from mail_scan.extractors.base import DEFAULT_EXTRACTORS, extract_listings
from mail_scan.fingerprint import fingerprint
from mail_scan.sources import SourceCursor, open_source


def _cancel_requested(cancel_file: str | None) -> bool:
    if not cancel_file:
        return False
    return Path(cancel_file).exists()


def _require_config(config: dict[str, Any]) -> dict[str, Any]:
    if config.get("protocol") != 1:
        raise ValueError(
            f"config protocol mismatch: got {config.get('protocol')!r}, expected 1"
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
            "protocol": 1,
            "run_id": cfg["run_id"],
            "sidecar_version": __version__,
            "sources": len(sources),
        },
    )

    listings_total = 0
    messages_total = 0
    cancelled = False

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
        last_message_id: str | None = None

        for mail in messages:
            if _cancel_requested(cancel_file):
                cancelled = True
                break

            messages_read += 1
            messages_total += 1
            last_message_id = mail.message_id or last_message_id

            if since and mail.message_date and mail.message_date < since:
                skipped += 1
                continue

            if listings_total >= int(limits["max_listings_per_run"]):
                skipped += 1
                continue

            extracted = extract_listings(mail, extractors)
            if not extracted:
                skipped += 1
                continue

            for item in extracted:
                if listings_total >= int(limits["max_listings_per_run"]):
                    skipped += 1
                    break
                fp = fingerprint(
                    board=item.board,
                    external_id=item.external_id,
                    url=item.url,
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
                    "url": item.url,
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
        # Prefer last message id observed in this run.
        if last_message_id:
            cursor = SourceCursor(
                size=cursor.size,
                mtime_ns=cursor.mtime_ns,
                offset=cursor.offset,
                last_message_id=last_message_id,
                sentinel_hash=cursor.sentinel_hash,
            )

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

        if cancelled:
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
    emit_event(emit, finished)

    if cancelled:
        log_warn("scan cancelled via cancel_file")
        return EXIT_CANCELLED
    return EXIT_OK
