"""CLI for probe / scan (spec §4.1)."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any

from mail_scan import __version__
from mail_scan.exit_codes import (
    EXIT_CONFIG,
    EXIT_INTERNAL,
    EXIT_OK,
    EXIT_UNREADABLE,
)
from mail_scan.extractors.base import DEFAULT_EXTRACTORS
from mail_scan.scan import run_scan

SUPPORTED_PROTOCOL = 1


def probe(protocol: int) -> int:
    if protocol != SUPPORTED_PROTOCOL:
        print(
            f"unsupported protocol {protocol}; expected {SUPPORTED_PROTOCOL}",
            file=sys.stderr,
        )
        return EXIT_CONFIG
    payload = {
        "t": "capabilities",
        "protocol": SUPPORTED_PROTOCOL,
        "sidecar_version": __version__,
        "extractors": list(DEFAULT_EXTRACTORS),
        "source_kinds": ["mbox", "maildir"],
    }
    sys.stdout.write(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n"
    )
    sys.stdout.flush()
    return EXIT_OK


def _load_config(raw: str) -> dict[str, Any]:
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError(f"config is not JSON: {exc}") from exc
    if not isinstance(data, dict):
        raise ValueError("config must be a JSON object")
    return data


def scan(protocol: int) -> int:
    if protocol != SUPPORTED_PROTOCOL:
        print(
            f"unsupported protocol {protocol}; expected {SUPPORTED_PROTOCOL}",
            file=sys.stderr,
        )
        return EXIT_CONFIG
    raw = sys.stdin.read()
    try:
        config = _load_config(raw)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return EXIT_CONFIG
    try:
        return run_scan(config, emit=sys.stdout)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return EXIT_CONFIG
    except FileNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return EXIT_UNREADABLE
    except OSError as exc:
        print(str(exc), file=sys.stderr)
        return EXIT_UNREADABLE
    except Exception as exc:
        print(f"internal error: {exc}", file=sys.stderr)
        return EXIT_INTERNAL


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="mail_scan")
    sub = parser.add_subparsers(dest="command", required=True)

    probe_p = sub.add_parser("probe", help="Print capabilities and exit")
    probe_p.add_argument("--protocol", type=int, required=True)

    scan_p = sub.add_parser("scan", help="Scan mail sources; config on stdin")
    scan_p.add_argument("--protocol", type=int, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "probe":
        return probe(args.protocol)
    if args.command == "scan":
        return scan(args.protocol)
    parser.error(f"unknown command {args.command}")
    return EXIT_INTERNAL


if __name__ == "__main__":
    raise SystemExit(main())
