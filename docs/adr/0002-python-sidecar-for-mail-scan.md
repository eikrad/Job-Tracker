# Python sidecar for Mail Scan

**Status:** Accepted · 2026-09-09 · narrowed by [0004](0004-sidecar-without-network-or-secrets.md)

> The sidecar keeps mail parsing and listing extraction; scoring, listing fetch, and enrichment moved to Rust. The choice of Python for parsing stands — read the scope below as parsing only.

The Mail Scan pipeline (parse, extract, two-pass LLM score, listing fetch, enrichment) runs as a vendored Python sidecar invoked by Tauri, not as a full Rust rewrite and not as a separate Jobmails repo workflow.

**Why:** Existing Jobmails logic ports faster and keeps parsing/scoring testable; the app owns persistence (Mail Match Inbox) and UI. A Rust rewrite would delay a working scan for weeks without changing the product boundary.

**Consequences:** Packaging must ship or locate a Python/uv runtime; IPC contract is JSON in/out; the sidecar must not write the `jobs` table directly.
