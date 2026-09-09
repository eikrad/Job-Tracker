# Mail Scan — Plan Index (rev. 2)

**Status:** Active index
**Spec:** [`../specs/2026-09-09-mail-scan-integration-design.md`](../specs/2026-09-09-mail-scan-integration-design.md) (rev. 2)
**Glossary:** [`../../../CONTEXT.md`](../../../CONTEXT.md)
**ADRs:** [`0001`](../../adr/0001-no-imap-local-mail-folders-only.md) · [`0002`](../../adr/0002-python-sidecar-for-mail-scan.md) · [`0003`](../../adr/0003-mail-match-inbox-separate-from-capture.md) · [`0004`](../../adr/0004-sidecar-without-network-or-secrets.md) · [`0005`](../../adr/0005-api-keys-in-os-keyring.md)

The rev. 1 implementation plan is **withdrawn**. Execution is **three** PRs, not two.

| PR | Plan | Spec phases | Ships | User-visible on merge |
|----|------|-------------|-------|------------------------|
| **A — Foundation** | [`pr-a-foundation`](2026-09-09-mail-scan-pr-a-foundation.md) | 1–3 | Migration runner, keyring secrets, Rust LLM client, `fetch_untrusted` | Yes — keys move to the keyring, extraction runs in Rust |
| **B — Scan skeleton** | [`pr-b-skeleton`](2026-09-09-mail-scan-pr-b-skeleton.md) | 4–6 | Sidecar, NDJSON protocol, mail tables, fingerprints, cursors | No — behind `mailScanEnabled` flag, no LLM spend |
| **C — Scoring & inbox** | [`pr-c-scoring-inbox`](2026-09-09-mail-scan-pr-c-scoring-inbox.md) | 7–10 | Two-pass scoring, enrichment, Mail Match Inbox, settings, packaging | Yes — the feature ships, flag removed |

### Why three and not two

The previous split put spec phases 4–10 into one PR. That is the sidecar, six tables, the protocol, fingerprint clustering, both LLM passes, enrichment, a three-tab UI and packaging in a single reviewable unit — nobody reviews that honestly. The B/C seam is chosen so that **B contains no LLM call and no user-facing surface**: it is verifiable entirely against fixtures, costs nothing to run, and can sit on `main` behind a flag while C is built. If the project stops after B, nothing is half-exposed.

### Dependency graph

```
A1 migration runner ──┬─► A2 secrets ──► A3 settings UX ──┐
                      │                                    ├─► A6 docs/verify
A5 fetch_untrusted ───┴─► A4 provider registry + LLM ──────┘
                                    │
                                    ▼
              B1 sidecar protocol ─► B2 tables/persistence ─► B3 fingerprints/cursors
                                                                        │
                                                                        ▼
                                    C1 scoring ─► C2 enrichment ─► C3 inbox UI ─► C4 settings/packaging
```

A5 has no dependency on A2–A4 and can be done first or in parallel; A4 depends on A2 (keys) and A5 (its HTTP client shape).

### Pre-flight baseline (record before starting, compare at every PR)

| Check | Baseline to capture |
|-------|---------------------|
| `npm run test` | pass count |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass count |
| `uv run pytest -q` | pass count |
| `npm run build` | clean |

> **`npm run verify:rust` silently prints "Skipping Rust checks" when GTK/WebKit system libs are absent.** A green `npm run verify` therefore does **not** mean the Rust half compiled. Every PR here touches Rust: run the Rust checks in an environment with Tauri prerequisites, or rely on the `rust.yml` CI job, and say which in the PR description.

### Conventions (from `CONTRIBUTING.md`)

Conventional commits (`feat(scope): …`); one logical change per commit; Husky pre-commit runs `npm run verify`; tests alongside behaviour changes; no secrets or personal data in git.

**Do not implement until the user approves the PR A plan.** B and C are refined after the PR before them lands.
