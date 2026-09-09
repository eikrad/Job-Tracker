"""NDJSON event helpers (spec §4.3)."""

from __future__ import annotations

import json
import sys
from typing import Any, TextIO

MAX_LINE_BYTES = 256 * 1024


def emit_event(stream: TextIO, event: dict[str, Any]) -> None:
    line = json.dumps(event, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    encoded = line.encode("utf-8")
    if len(encoded) > MAX_LINE_BYTES:
        raise RuntimeError(
            f"event line exceeds {MAX_LINE_BYTES} bytes ({len(encoded)})"
        )
    stream.write(line + "\n")
    stream.flush()


def log_warn(message: str) -> None:
    print(message, file=sys.stderr)
