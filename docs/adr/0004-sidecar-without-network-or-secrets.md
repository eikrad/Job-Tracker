# Mail Scan sidecar has no network access and no secrets

**Status:** Accepted · 2026-09-09 · narrows [0002](0002-python-sidecar-for-mail-scan.md)

The Python sidecar parses local mail and emits listings. It does not make HTTP requests, does not receive an API key, and does not know the database path. All network egress — LLM scoring, LLM enrichment, and listing page fetches — happens in Rust, which also owns caching, retries, rate limiting, and persistence.

**Why:** ADR 0002 chose Python because mbox parsing and the job-board digest extractors port fastest from Jobmails. That reasoning covers parsing only. Scoring, fetching, and enrichment are HTTP plus prompt plus JSON — work Rust already does in `listing_check.rs` and `job_search.rs` — and putting them in the sidecar would mean a second HTTP stack, a second retry policy, and a second cache, plus an API key crossing an extra process boundary. Keeping the key out of Python also lets it move from `localStorage` into the OS keyring (ADR 0005), because no process outside Rust needs to read it.

The sidecar becomes a pure function of `(config, mail files) → event stream`: deterministic, testable against fixtures with no network mocking at all.

**Considered options:** LLM calls in the sidecar (rev. 1 of the design — lifts more Jobmails code, but duplicates the HTTP layer and spreads secrets); LLM calls in the frontend as the job form does today (keys stay in the webview, which also renders fetched remote content); LLM calls in Rust. Chose Rust.

**Consequences:** Scoring and enrichment prompts are ported to Rust rather than lifted from Jobmails — prompt templates live in `src-tauri/prompts/` as `include_str!` assets with a dev-only override directory so iteration does not require a recompile. The sidecar contract is a versioned NDJSON event stream on stdout with its config on stdin; nothing sensitive is ever passed in argv. Python keeps exactly one job, and gaining a new job later requires revisiting this ADR.
