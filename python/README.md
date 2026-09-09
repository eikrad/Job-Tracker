# Mail-scan Python sidecar (`python -m mail_scan`).
#
# Contract: local mbox/maildir only — no network, no secrets, no DB path
# (ADR 0004). Config on stdin; NDJSON events on stdout; logs on stderr.
#
# Dev: `PYTHONPATH=python python -m mail_scan probe --protocol 1`
