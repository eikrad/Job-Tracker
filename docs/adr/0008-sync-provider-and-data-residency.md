# Turso Cloud is the sync provider; self-hosting stays open

**Status:** Proposed · 2026-09-14

Sync uses Turso Cloud (ChiselStrike, Inc., a US company), in an EU region where one is available. Every user supplies their own credentials; the project never operates a shared sync server on anyone else's behalf.

**Why:** Job Tracker is a single-user desktop app, and a private job tracker is squarely a "purely personal or household activity" under GDPR Art. 2(2)(c) and Recital 18 — the Regulation does not apply. There is no controller role, no DPA to execute, no legal basis to establish, and no data subject rights machinery to build. Self-hosting would cost roughly €60–180/year plus ongoing maintenance and would buy no legal protection that the household exemption does not already provide. Provider nationality is therefore not a deciding factor at this scale.

This is a deliberate exception to the data-minimisation line the project otherwise holds (ADR 0001 no IMAP, ADR 0004 a sidecar with no network and no secrets, ADR 0005 keys in the OS keyring), and it is recorded as such rather than arrived at by momentum.

**Considered options:** Self-hosted `sqld` on European infrastructure — Hetzner (DE/FI) at roughly €55–70/year or Scaleway (FR) at roughly €135–180/year once block storage and a flexible IPv4 are added, both of which the design supports unchanged; managed Postgres from an EU company (Aiven in Finland, Scaleway, Clever Cloud, OVHcloud, IONOS, or Exoscale in Switzerland) — rejected because none offers SQLite or libSQL sync, so each would force exactly the Postgres rewrite ADR 0006 exists to avoid; ElectricSQL (UK-registered, headquartered in Croatia), architecturally the closest fit but still requiring a Postgres backend, and the UK holds only an adequacy decision rather than EU membership; Turso Cloud. Chose Turso Cloud.

Cost figures are approximate, gathered September 2026 from secondary sources — `turso.tech` and `scaleway.com` were both unreachable from the environment where this was researched. Verify before relying on them; both providers raised prices during 2026.

**Consequences:** The per-user-credentials rule is load-bearing and not merely a convenience. It is what keeps the household exemption applicable to every user of this Apache-2.0 repository rather than only to the maintainer. **If the project ever operates a central sync server for other people, it becomes their processor and this ADR must be revisited in full** — DPA, legal basis, subprocessors, the lot.

The residual risk is breach, not jurisdiction: a leaked token or a compromised provider exposes the same data wherever the provider is incorporated. That argues for encryption at rest with a user-held key and for token hygiene, not for a different provider. The synced `jobs` table carries `contact_name`, `contact_email`, and `contact_phone` — personal data of recruiters and hiring managers who were never asked. The household exemption covers this legally; encryption is the appropriate care regardless. Candidate Profile documents stay excluded from sync, as `CONTEXT.md` already requires.

The decision is cheap to reverse. Per ADR 0006 the sync target is provider-agnostic, so moving to self-hosted `sqld` on European infrastructure later is an endpoint and a token, with no change to the Rust.
