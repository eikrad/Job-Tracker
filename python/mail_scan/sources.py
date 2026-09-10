"""Local mbox / maildir readers (read-only, streaming)."""

from __future__ import annotations

import email
import hashlib
import mailbox
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from datetime import UTC
from email.message import Message
from email.utils import parsedate_to_datetime
from pathlib import Path

from mail_scan.html_text import html_to_visible_text, looks_like_html


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
    sentinel_hash: str | None = None

    def as_dict(self) -> dict[str, int | str | None]:
        return {
            "size": self.size,
            "mtime_ns": self.mtime_ns,
            "offset": self.offset,
            "last_message_id": self.last_message_id,
            "sentinel_hash": self.sentinel_hash,
        }


@dataclass(frozen=True)
class OpenResult:
    messages: Iterator[MailMessage]
    finalize: Callable[[], SourceCursor]
    cursor_reset: bool
    reset_reason: str | None


def _file_meta(path: Path) -> tuple[int, int]:
    st = path.stat()
    mtime_ns = getattr(st, "st_mtime_ns", int(st.st_mtime * 1_000_000_000))
    return st.st_size, mtime_ns


def _from_line_hash(from_line: bytes) -> str:
    return hashlib.sha256(from_line).hexdigest()


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


def _decode_part(part: Message) -> str:
    payload = part.get_payload(decode=True)
    if isinstance(payload, bytes):
        charset = part.get_content_charset() or "utf-8"
        try:
            return payload.decode(charset, errors="replace")
        except LookupError:
            # Mail in the wild names charsets Python has never heard of.
            return payload.decode("utf-8", errors="replace")
    if isinstance(payload, str):
        return payload
    return ""


def _body_text(msg: Message, max_chars: int) -> str:
    """Best available plain text for a message.

    Prefers `text/plain`. Falls back to the *visible* text of `text/html` — never the
    markup, and never the parts of it a human reader cannot see (spec §6.2): a hidden
    block contradicting the visible ad is a prompt-injection vector, not content.
    """
    plain: list[str] = []
    html: list[str] = []

    if msg.is_multipart():
        for part in msg.walk():
            if part.get_content_maintype() == "multipart":
                continue
            # Attachments are not body text, whatever they claim to be.
            if (part.get_content_disposition() or "") == "attachment":
                continue
            ctype = part.get_content_type()
            if ctype == "text/plain":
                plain.append(_decode_part(part))
            elif ctype == "text/html":
                html.append(_decode_part(part))
    else:
        decoded = _decode_part(msg)
        if msg.get_content_type() == "text/html":
            html.append(decoded)
        else:
            plain.append(decoded)

    text = "\n".join(p for p in plain if p.strip()).strip()
    if not text and html:
        text = html_to_visible_text("\n".join(html)).strip()
    elif text and looks_like_html(text):
        # A digest sent as text/plain that is really markup.
        text = html_to_visible_text(text).strip()

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


def _parse_stored_cursor(raw: dict | None) -> SourceCursor | None:
    if not isinstance(raw, dict):
        return None
    try:
        return SourceCursor(
            size=int(raw["size"]),
            mtime_ns=int(raw["mtime_ns"]),
            offset=int(raw["offset"]),
            last_message_id=raw.get("last_message_id"),
            sentinel_hash=raw.get("sentinel_hash"),
        )
    except (KeyError, TypeError, ValueError):
        return None


def _last_from_line_before(data: bytes) -> bytes | None:
    idx = data.rfind(b"\nFrom ")
    if idx != -1:
        line = data[idx + 1 :]
    elif data.startswith(b"From "):
        line = data
    else:
        return None
    nl = line.find(b"\n")
    return line[: nl + 1] if nl != -1 else line


def _iter_mbox_bytes(
    path: Path, *, start_offset: int = 0
) -> Iterator[tuple[bytes, int, bytes]]:
    """Yield (raw_message, end_offset, from_line).

    A message is emitted only when closed by the next ``From `` line. Trailing
    bytes at EOF are never consumed — Thunderbird may still be writing them
    (spec §5.5). Static fixtures should end with a sentinel ``From `` line so the
    last real message is closed.
    """
    with path.open("rb") as handle:
        if start_offset:
            handle.seek(start_offset)
        buf = bytearray()
        start = start_offset
        from_line = b""
        while True:
            line = handle.readline()
            if not line:
                return
            if line.startswith(b"From "):
                if buf:
                    end_offset = start + len(buf)
                    yield bytes(buf), end_offset, from_line
                    start = end_offset
                buf.clear()
                from_line = line
                buf.extend(line)
            elif buf:
                buf.extend(line)
            # else: skip preamble before the first From (e.g. after resume)


def _resolve_resume(
    path: Path, stored: SourceCursor | None
) -> tuple[int, bool, str | None]:
    if stored is None:
        return 0, False, None
    size, mtime_ns = _file_meta(path)
    if size < stored.size or mtime_ns < stored.mtime_ns:
        return 0, True, "size_or_mtime_regression"
    if stored.offset > size:
        return 0, True, "offset_past_eof"
    if stored.offset <= 0:
        return 0, False, None
    if stored.sentinel_hash:
        window = min(stored.offset, 16384)
        with path.open("rb") as handle:
            handle.seek(stored.offset - window)
            chunk = handle.read(window)
        last_from = _last_from_line_before(chunk)
        if last_from is None or _from_line_hash(last_from) != stored.sentinel_hash:
            return 0, True, "sentinel_mismatch"
    return stored.offset, False, None


def iter_mbox(
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
    stored_cursor: SourceCursor | None = None,
) -> OpenResult:
    if not path.is_file():
        raise FileNotFoundError(f"mbox not found: {path}")

    start_offset, cursor_reset, reset_reason = _resolve_resume(path, stored_cursor)
    last_id: list[str | None] = [None]
    last_offset: list[int] = [start_offset]
    last_sentinel: list[str | None] = [None]

    def generator() -> Iterator[MailMessage]:
        count = 0
        for raw, end_offset, from_line in _iter_mbox_bytes(
            path, start_offset=start_offset
        ):
            if count >= max_messages:
                break
            if len(raw) > max_message_bytes:
                last_offset[0] = end_offset
                if from_line:
                    last_sentinel[0] = _from_line_hash(from_line)
                continue
            body = raw
            nl = raw.find(b"\n")
            if nl != -1 and raw.startswith(b"From "):
                body = raw[nl + 1 :]
            msg = email.message_from_bytes(body)
            mail = _to_mail_message(msg, len(raw), max_body_chars)
            last_id[0] = mail.message_id or last_id[0]
            last_offset[0] = end_offset
            if from_line:
                last_sentinel[0] = _from_line_hash(from_line)
            count += 1
            yield mail

    def finalize() -> SourceCursor:
        size_now, mtime_now = _file_meta(path)
        return SourceCursor(
            size=size_now,
            mtime_ns=mtime_now,
            offset=last_offset[0],
            last_message_id=last_id[0],
            sentinel_hash=last_sentinel[0],
        )

    return OpenResult(
        messages=generator(),
        finalize=finalize,
        cursor_reset=cursor_reset,
        reset_reason=reset_reason,
    )


def iter_maildir(
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
    stored_cursor: SourceCursor | None = None,
) -> OpenResult:
    if not path.is_dir():
        raise FileNotFoundError(f"maildir not found: {path}")

    def folder_meta() -> tuple[int, int]:
        st = path.stat()
        mtime_ns = getattr(st, "st_mtime_ns", int(st.st_mtime * 1_000_000_000))
        total = 0
        for sub in ("cur", "new", "tmp"):
            d = path / sub
            if d.is_dir():
                for child in d.iterdir():
                    if child.is_file():
                        total += child.stat().st_size
        return total, mtime_ns

    cursor_reset = False
    reset_reason: str | None = None
    skip_after_id: str | None = None

    if stored_cursor is not None:
        total, mtime_ns = folder_meta()
        if total < stored_cursor.size or mtime_ns < stored_cursor.mtime_ns:
            cursor_reset = True
            reset_reason = "size_or_mtime_regression"
        elif stored_cursor.last_message_id:
            skip_after_id = stored_cursor.last_message_id

    box = mailbox.Maildir(path, create=False)
    if skip_after_id and not cursor_reset:
        try:
            present = False
            for key in box:
                msg = box.get_message(key)
                mid = (msg.get("Message-ID") or msg.get("Message-Id") or "").strip()
                if mid == skip_after_id:
                    present = True
                    break
            if not present:
                cursor_reset = True
                reset_reason = "maildir_anchor_missing"
                skip_after_id = None
        except (OSError, mailbox.Error):
            cursor_reset = True
            reset_reason = "maildir_unreadable"
            skip_after_id = None

    last_id: list[str | None] = [None]
    resume_id = None if cursor_reset else skip_after_id

    def generator() -> Iterator[MailMessage]:
        count = 0
        past_cursor = resume_id is None
        try:
            for key in sorted(box):
                if count >= max_messages:
                    break
                msg = box.get_message(key)
                raw = msg.as_bytes()
                if len(raw) > max_message_bytes:
                    continue
                mail = _to_mail_message(msg, len(raw), max_body_chars)
                if not past_cursor:
                    if mail.message_id == resume_id:
                        past_cursor = True
                    continue
                last_id[0] = mail.message_id or last_id[0]
                count += 1
                yield mail
        finally:
            box.close()

    def finalize() -> SourceCursor:
        total, mtime_ns = folder_meta()
        return SourceCursor(
            size=total,
            mtime_ns=mtime_ns,
            offset=total,
            last_message_id=last_id[0],
            sentinel_hash=None,
        )

    return OpenResult(
        messages=generator(),
        finalize=finalize,
        cursor_reset=cursor_reset,
        reset_reason=reset_reason,
    )


def open_source(
    kind: str,
    path: Path,
    *,
    max_messages: int,
    max_message_bytes: int,
    max_body_chars: int,
    stored_cursor: dict | None = None,
) -> OpenResult:
    cursor = _parse_stored_cursor(stored_cursor)
    if kind == "mbox":
        return iter_mbox(
            path,
            max_messages=max_messages,
            max_message_bytes=max_message_bytes,
            max_body_chars=max_body_chars,
            stored_cursor=cursor,
        )
    if kind == "maildir":
        return iter_maildir(
            path,
            max_messages=max_messages,
            max_message_bytes=max_message_bytes,
            max_body_chars=max_body_chars,
            stored_cursor=cursor,
        )
    raise ValueError(f"unknown source kind: {kind}")
