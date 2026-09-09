"""Local mbox / maildir readers (read-only, streaming)."""

from __future__ import annotations

import email
import mailbox
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from datetime import UTC
from email.message import Message
from email.utils import parsedate_to_datetime
from pathlib import Path


@dataclass(frozen=True)
class MailMessage:
    message_id: str
    message_date: str
    subject: str
    from_addr: str
    body_text: str
    raw_size: int


@dataclass(frozen=True)
class SourceCursor:
    size: int
    mtime_ns: int
    offset: int
    last_message_id: str | None

    def as_dict(self) -> dict[str, int | str | None]:
        return {
            "size": self.size,
            "mtime_ns": self.mtime_ns,
            "offset": self.offset,
            "last_message_id": self.last_message_id,
        }


def _file_cursor(path: Path, last_message_id: str | None, offset: int) -> SourceCursor:
    st = path.stat()
    mtime_ns = getattr(st, "st_mtime_ns", int(st.st_mtime * 1_000_000_000))
    return SourceCursor(
        size=st.st_size,
        mtime_ns=mtime_ns,
        offset=offset,
        last_message_id=last_message_id,
    )


def _message_date(msg: Message) -> str:
    raw = msg.get("Date")
    if not raw:
        return ""
    try:
        dt = parsedate_to_datetime(raw)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=UTC)
        return dt.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    except (TypeError, ValueError, IndexError, OverflowError):
        return raw


def _body_text(msg: Message, max_chars: int) -> str:
    parts: list[str] = []
    if msg.is_multipart():
        for part in msg.walk():
            ctype = part.get_content_type()
            if ctype == "text/plain":
                payload = part.get_payload(decode=True)
                if isinstance(payload, bytes):
                    charset = part.get_content_charset() or "utf-8"
                    parts.append(payload.decode(charset, errors="replace"))
                elif isinstance(payload, str):
                    parts.append(payload)
    else:
        payload = msg.get_payload(decode=True)
        if isinstance(payload, bytes):
            charset = msg.get_content_charset() or "utf-8"
            parts.append(payload.decode(charset, errors="replace"))
        elif isinstance(payload, str):
            parts.append(payload)
    text = "\n".join(parts).strip()
    if len(text) > max_chars:
        return text[:max_chars]
    return text


def _to_mail_message(msg: Message, raw_size: int, max_body_chars: int) -> MailMessage:
    mid = (msg.get("Message-ID") or msg.get("Message-Id") or "").strip()
    return MailMessage(
        message_id=mid,
        message_date=_message_date(msg),
        subject=(msg.get("Subject") or "").strip(),
        from_addr=(msg.get("From") or "").strip(),
        body_text=_body_text(msg, max_body_chars),
        raw_size=raw_size,
    )


def _iter_mbox_bytes(path: Path) -> Iterator[tuple[bytes, int]]:
    """Yield (raw_message_bytes, end_offset) without mutating the file."""
    with path.open("rb") as handle:
        buf = bytearray()
        start_offset = 0
        while True:
            line = handle.readline()
            if not line:
                break
            if line.startswith(b"From ") and buf:
                end_offset = start_offset + len(buf)
                yield bytes(buf), end_offset
                buf.clear()
                start_offset = end_offset
            buf.extend(line)
        if buf:
            end_offset = start_offset + len(buf)
            yield bytes(buf), end_offset


def iter_mbox(
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
) -> tuple[Iterator[MailMessage], Callable[[], SourceCursor]]:
    """Yield messages from an mbox; return (iterator, finalize_cursor_fn)."""

    if not path.is_file():
        raise FileNotFoundError(f"mbox not found: {path}")

    last_id: list[str | None] = [None]
    last_offset: list[int] = [0]

    def generator() -> Iterator[MailMessage]:
        count = 0
        for raw, end_offset in _iter_mbox_bytes(path):
            if count >= max_messages:
                break
            if len(raw) > max_message_bytes:
                last_offset[0] = end_offset
                continue
            # Drop the leading "From " separator line for email parsing.
            body = raw
            nl = raw.find(b"\n")
            if nl != -1 and raw.startswith(b"From "):
                body = raw[nl + 1 :]
            msg = email.message_from_bytes(body)
            mail = _to_mail_message(msg, len(raw), max_body_chars)
            last_id[0] = mail.message_id or last_id[0]
            last_offset[0] = end_offset
            count += 1
            yield mail

    def finalize() -> SourceCursor:
        return _file_cursor(path, last_id[0], last_offset[0] or path.stat().st_size)

    return generator(), finalize


def iter_maildir(
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
) -> tuple[Iterator[MailMessage], Callable[[], SourceCursor]]:
    if not path.is_dir():
        raise FileNotFoundError(f"maildir not found: {path}")

    box = mailbox.Maildir(path, create=False)
    last_id: list[str | None] = [None]

    def generator() -> Iterator[MailMessage]:
        count = 0
        try:
            # Sorted keys keep scans deterministic.
            for key in sorted(box.keys()):
                if count >= max_messages:
                    break
                msg = box.get_message(key)
                raw = msg.as_bytes()
                if len(raw) > max_message_bytes:
                    continue
                mail = _to_mail_message(msg, len(raw), max_body_chars)
                last_id[0] = mail.message_id or last_id[0]
                count += 1
                yield mail
        finally:
            box.close()

    def finalize() -> SourceCursor:
        st = path.stat()
        mtime_ns = getattr(st, "st_mtime_ns", int(st.st_mtime * 1_000_000_000))
        total = 0
        for sub in ("cur", "new", "tmp"):
            d = path / sub
            if d.is_dir():
                for child in d.iterdir():
                    if child.is_file():
                        total += child.stat().st_size
        return SourceCursor(
            size=total,
            mtime_ns=mtime_ns,
            offset=total,
            last_message_id=last_id[0],
        )

    return generator(), finalize


def open_source(
    kind: str,
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
) -> tuple[Iterator[MailMessage], Callable[[], SourceCursor]]:
    if kind == "mbox":
        return iter_mbox(
            path,
            max_messages=max_messages,
            max_message_bytes=max_message_bytes,
            max_body_chars=max_body_chars,
        )
    if kind == "maildir":
        return iter_maildir(
            path,
            max_messages=max_messages,
            max_message_bytes=max_message_bytes,
            max_body_chars=max_body_chars,
        )
    raise ValueError(f"unknown source kind: {kind}")
