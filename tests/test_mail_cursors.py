"""Incremental mbox cursor behaviour (spec §5.5)."""

from __future__ import annotations

import hashlib
from pathlib import Path

from mail_scan.sources import SourceCursor, open_source

FIXTURE = (
    Path(__file__).resolve().parents[1]
    / "tests"
    / "fixtures"
    / "mail_scan"
    / "indeed_sample.mbox"
)


def _limits() -> dict:
    return {
        "max_messages": 5000,
        "max_message_bytes": 2_097_152,
        "max_body_chars": 20_000,
    }


def test_incomplete_trailing_message_not_consumed(tmp_path: Path):
    # Open message at EOF (no following From) is not consumed — live-tail safe.
    path = tmp_path / "live.mbox"
    path.write_bytes(
        b"From a@x Sat Sep 07 06:12:00 2026\n"
        b"From: a@x\nSubject: hi\nMessage-ID: <a@x>\n\n"
        b"body with trailing newline\n"
    )
    opened = open_source("mbox", path, stored_cursor=None, **_limits())
    msgs = list(opened.messages)
    assert msgs == []
    cursor = opened.finalize()
    assert cursor.offset == 0


def test_resume_reads_only_new_tail(tmp_path: Path):
    base = FIXTURE.read_bytes()
    path = tmp_path / "growing.mbox"
    path.write_bytes(base)

    first = open_source("mbox", path, stored_cursor=None, **_limits())
    first_msgs = list(first.messages)
    assert len(first_msgs) >= 2
    cursor = first.finalize()
    assert cursor.offset > 0
    assert cursor.sentinel_hash

    # Unchanged file → resume from offset yields nothing new.
    second = open_source(
        "mbox",
        path,
        stored_cursor=cursor.as_dict(),
        **_limits(),
    )
    assert second.cursor_reset is False
    assert list(second.messages) == []
    assert second.finalize().offset == cursor.offset

    # Drop unconsumed sentinel bytes, then append one complete message + closer.
    # Refresh size/mtime on the stored cursor so truncate isn't treated as regression.
    path.write_bytes(path.read_bytes()[: cursor.offset])
    st = path.stat()
    mtime_ns = getattr(st, "st_mtime_ns", int(st.st_mtime * 1_000_000_000))
    stored = {
        **cursor.as_dict(),
        "size": st.st_size,
        "mtime_ns": mtime_ns,
    }
    extra = (
        b"From new@x Mon Sep 09 08:00:00 2026\n"
        b"From: new@x\nTo: seeker@example.com\n"
        b"Subject: 1 new job\n"
        b"Date: Mon, 09 Sep 2026 08:00:00 +0000\n"
        b"Message-ID: <new@x>\n"
        b"MIME-Version: 1.0\n"
        b"Content-Type: text/plain; charset=utf-8\n\n"
        b"Platform Engineer - NewCo - Oslo\n"
        b"https://no.indeed.com/viewjob?jk=newjob00112233\n"
        b"\nFrom mail-scan-sentinel@localhost Mon Sep 09 08:00:01 2026\n"
    )
    with path.open("ab") as handle:
        handle.write(extra)

    third = open_source(
        "mbox",
        path,
        stored_cursor=stored,
        **_limits(),
    )
    assert third.cursor_reset is False
    new_msgs = list(third.messages)
    assert len(new_msgs) == 1
    assert new_msgs[0].message_id == "<new@x>"


def test_size_regression_forces_cursor_reset(tmp_path: Path):
    path = tmp_path / "shrink.mbox"
    path.write_bytes(FIXTURE.read_bytes())
    first = open_source("mbox", path, stored_cursor=None, **_limits())
    list(first.messages)
    cursor = first.finalize()

    # Compaction / rebuild: size regresses.
    path.write_bytes(FIXTURE.read_bytes()[: len(FIXTURE.read_bytes()) // 2])
    # Ensure file ends with newline so any complete message can parse.
    if not path.read_bytes().endswith(b"\n"):
        with path.open("ab") as handle:
            handle.write(b"\n")

    stored = SourceCursor(
        size=cursor.size,
        mtime_ns=cursor.mtime_ns + 10_000_000_000,  # newer mtime but smaller size
        offset=cursor.offset,
        last_message_id=cursor.last_message_id,
        sentinel_hash=cursor.sentinel_hash,
    )
    # Force size regression relative to stored.size
    opened = open_source(
        "mbox",
        path,
        stored_cursor={
            **stored.as_dict(),
            "size": path.stat().st_size + 1000,
        },
        **_limits(),
    )
    assert opened.cursor_reset is True
    assert opened.reset_reason == "size_or_mtime_regression"


def test_sentinel_mismatch_forces_reset(tmp_path: Path):
    path = tmp_path / "sent.mbox"
    path.write_bytes(FIXTURE.read_bytes())
    first = open_source("mbox", path, stored_cursor=None, **_limits())
    list(first.messages)
    cursor = first.finalize()

    bad = {
        **cursor.as_dict(),
        "sentinel_hash": hashlib.sha256(b"From forged\n").hexdigest(),
    }
    opened = open_source("mbox", path, stored_cursor=bad, **_limits())
    assert opened.cursor_reset is True
    assert opened.reset_reason == "sentinel_mismatch"
