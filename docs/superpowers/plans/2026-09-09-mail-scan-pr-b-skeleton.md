# Mail Scan PR B — Scan Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. **Depends on PR A.** Do not start until PR A has landed and the user approves this plan.

**Status:** Active · PR 2 of 3 · [index](2026-09-09-mail-scan-integration.md)
**Goal:** Everything the mail scan needs *before* the first LLM token is spent (spec §12 phases 4–6): a network-less Python sidecar, the versioned NDJSON protocol, the mail tables, per-item persistence with working cancel, tiered fingerprints, dismissals, and incremental mail cursors.
**Source:** Spec §4, §5, §7.2–§7.3, §9.1–§9.3 · ADR 0001, 0002, 0004

**Architecture:** The sidecar reads local mail and streams listing events (ADR 0004: no network, no secrets, no DB path). Rust validates the stream, gates each listing, and commits it in its own transaction. Scoring and enrichment are **stubbed** in this PR — the orchestrator has the seam, but it returns a fixed score and no enrichment.

**The whole PR is invisible to the user.** The scan entry point sits behind a `mailScanEnabled` flag (default off, dev-only toggle in Settings). Nothing costs an LLM call. If the project pauses after B, `main` carries dormant, tested infrastructure and no half-exposed feature.

---

## Global constraints

- Sidecar: no network, no secrets, no DB path; config on stdin; NDJSON on stdout; logs on stderr.
- No personal mail in the repo — every fixture is synthetic and hand-authored.
- One transaction per item (spec §7.3): a crash at listing 60 of 118 keeps 59.
- Domain terms from `CONTEXT.md` in code, tests, and any UI copy.

---

## File map

| Path | Role |
|------|------|
| `python/mail_scan/__main__.py`, `cli.py` | `probe` and `scan` subcommands, exit codes (§4.1) |
| `python/mail_scan/sources.py` | mbox / maildir readers, cursor resume |
| `python/mail_scan/extractors/` | Indeed, Jobindex, LinkedIn, generic |
| `python/mail_scan/fingerprint.py` | Strong/weak key normalization (§5.1) |
| `python/mail_scan/events.py` | Event construction + schema validation |
| `schemas/mail_scan_events.schema.json` | The contract, shared by both languages |
| `tests/fixtures/mail_scan/*.mbox` | Synthetic corpora per extractor |
| `tests/fixtures/fingerprints.json` | Cross-language conformance cases (§9.3) |
| `src-tauri/src/mail_scan/spawn.rs` | Process spawn, isolated env, cancel, kill escalation |
| `src-tauri/src/mail_scan/protocol.rs` | Line-bounded NDJSON reader, strict `serde` |
| `src-tauri/src/mail_scan/persist.rs` | Fingerprints, inbox, sightings, runs, cursors, dismissals |
| `src-tauri/src/migrations.rs` | `m0002_mail_scan_core`, `m0003_mail_scan_indices` |

---

## Task B1 — Sidecar: probe, protocol, one extractor 🟢

**Commit:** `feat(mail-scan): python sidecar with versioned NDJSON protocol`

- [ ] **Step 1 — red (pytest):** `probe --protocol 1` prints one capabilities object and exits `0`; `scan` with an invalid config exits `20`; an unreadable source exits `30`; a present cancel file makes it stop between messages and exit `10`.
- [ ] **Step 2 — red:** every emitted event validates against `schemas/mail_scan_events.schema.json`; no line exceeds 256 KiB; the stream is byte-identical across two runs on the same fixture once `run_id` and timings are excluded (determinism is what makes the whole thing testable).
- [ ] **Step 3 — red (H: enforcement, not intention):** ADR 0004's "no network" must be **mechanically enforced**, not documented. Add a ruff `flake8-tidy-imports` banned-api rule for `socket`, `http`, `urllib`, `requests`, `httpx`, `smtplib`, `ftplib` in `python/mail_scan/`, so a future contributor importing `requests` fails lint rather than quietly re-opening the trust boundary. A grep test is the weaker fallback.
- [ ] **Step 4 — implement:** streaming parse — never accumulate the corpus in memory. Apply `since` and the `limits` caps *before* parsing bodies.
- [ ] **Step 5:** one extractor (Indeed) plus the generic fallback; the rest land in B3.
- [ ] **Step 6:** commit.

---

## Task B2 — Spawn, protocol reader, tables, per-item persistence 🔴

**Commit:** `feat(mail-scan): persist listings per item with cancel and crash safety`

- [ ] **Step 1 — red:** `m0002` creates the spec §7.2 tables; migrating twice is a no-op; the **partial** unique index (`WHERE status = 'pending'`) permits one pending plus one accepted row for the same fingerprint and rejects a second pending one. A plain `UNIQUE(fingerprint_id, kind, status)` passes a naive test while allowing exactly the duplicate it was meant to stop — assert the real behaviour.
- [ ] **Step 2 — red (crash safety):** feed a **truncated** event stream and a stream ending in a malformed line; assert every item committed before the break survives, the run is `failed`, and no partially written item exists.
- [ ] **Step 3 — red (protocol):** unknown `t` is skipped and logged (forward compatible); a malformed *known* event aborts; a line over the cap aborts with `E_PROTOCOL_OVERSIZE` **without** buffering it; a `protocol` mismatch aborts naming both versions.
- [ ] **Step 4 — red (cancel, §4.4):** cancel mid-stream ⇒ committed items stay, run is `cancelled`, the child exits within the escalation window. Assert **no orphan process** survives the parent — spawn into a process group (Unix) or job object (Windows) and test that killing the parent takes the child with it. An orphaned sidecar holding a mail file open is the kind of bug that only shows up on a user's machine.
- [ ] **Step 5 — red (spawn hardening, §6.4):** the child's environment is *constructed*, not inherited — assert `PYTHONPATH`, `PYTHONHOME`, `PYTHONSTARTUP` are absent and `PYTHONNOUSERSITE=1` is set; assert nothing sensitive appears in the child's argv (config goes on stdin).
- [ ] **Step 6 — implement:** the orchestrator with **stubbed** scoring and enrichment behind the trait that C1/C2 will fill. Emit `mail-scan://progress` coalesced at ≤ 4/s.
- [ ] **Step 7:** the `mailScanEnabled` flag gates the entry point; default off.
- [ ] **Step 8:** commit.

---

## Task B3 — Fingerprints, dismissals, sightings, cursors 🟡

**Commit:** `feat(mail-scan): tiered fingerprints, revocable dismissals, incremental cursors`

- [ ] **Step 1 — red (the highest-value test asset in the feature):** author `tests/fixtures/fingerprints.json` — roughly 60 cases with expected strong key, weak key, and cluster id. Cover Danish and German diacritics (`ø å ä`), legal suffixes (`A/S`, `ApS`, `GmbH`), `(m/w/d)` markers, tracking parameters, redirect wrappers, and the near-duplicate pair (same title and company, two cities). Run the **same file** from pytest, cargo test, and vitest. Three implementations that agree only by inspection will drift within a month.
- [ ] **Step 2 — red:** the §5.1 clustering rules as a table test, explicitly including: two different strong keys sharing a weak key stay **separate** and are flagged `near_duplicate`; a weak-only cluster that later gains a strong key is promoted in place with an alias row, and an existing dismissal follows the alias.
- [ ] **Step 3 — red:** dismiss then rescan ⇒ suppressed and counted in `suppressed_by_dismissal`; restore ⇒ re-admitted on the next scan. A dismissal must never silently bury a *different* listing (rule 3 above) — assert that case directly.
- [ ] **Step 4 — red (cursors, §5.5):** resume from a stored offset; a size or mtime regression, or a failed sentinel match, forces a full re-read with `W_CURSOR_RESET`; a message whose terminating `From ` line is not yet written is **not** consumed. Thunderbird writes these files while the app reads them — this is a live-file race, not a theoretical one. Open read-only throughout.
- [ ] **Step 5 — implement**, then measure: a large fixture folder with a handful of new messages must read only the new tail. Record the number of messages parsed in the run stats so the claim stays checkable later.
- [ ] **Step 6 — scope guard:** `src/lib/jobs/duplicateCheck.ts` uses the same `url || company+title` shape. Adopting the shared normalization there changes **existing job dedup for every user** — a user-visible behaviour change that does not belong in a mail-scan PR. Note the alignment as follow-up work and leave it alone here.
- [ ] **Step 7:** commit.

---

## Spec coverage

| Spec §12 phase | Task |
|----------------|------|
| 4 Sidecar skeleton + protocol | B1 |
| 5 Tables, per-item persistence, progress, cancel | B2 |
| 6 Fingerprints, dismissals, cursors | B3 |

## Deliberately deferred to PR C

Scoring, profiles, the score cache, budget, enrichment, provenance, the inbox UI, settings, packaging, `Delete all mail scan data`.

## Success criteria

1. A fixture scan fills the inbox tables with stubbed scores and **no network access from any process**, enforced by lint.
2. Killing the app mid-scan leaves committed items intact, no orphan child, and a `failed` run row.
3. Cancel is honoured between messages and keeps completed work.
4. The fingerprint fixture file passes identically in all three languages.
5. A rescan of an unchanged mail folder parses ~0 messages.
6. With the flag off, the app is byte-for-byte unchanged in behaviour for a user.
