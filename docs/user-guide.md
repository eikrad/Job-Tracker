# User guide

Detailed setup and usage for Job Tracker. For a short overview and quick start, see the [README](../README.md); for internals, see [architecture.md](architecture.md).

| Section | Topic |
|---|---|
| [Configuration](#configuration) | AI provider and job-search API keys |
| [The board](#the-board) | Views, status columns, job details, reminders |
| [Adding jobs](#adding-jobs) | Manual form, AI extraction, Quick capture, Capture Inbox |
| [Job Search](#job-search) | In-app search and listing status check |
| [Google Calendar](#google-calendar) | OAuth setup and smoke test |
| [Mail scan](#mail-scan) | Thunderbird job-alert scanning and the Mail Match Inbox |
| [Data storage](#data-storage) | Where data and secrets live |
| [Backup, import and export](#backup-import-and-export) | Automatic backup folder, JSON/CSV |
| [Building installers](#building-installers) | Bundles, AppImage, release build, Linux launcher |

## Configuration

Everything is configured in **Settings** (top right). The sections are *Appearance*, *AI & calendar*, *Mail scan*, *Board columns* and *Backup & restore*.

1. **AI extraction**: in **Settings → AI & calendar**, pick a provider (**Scaleway DeepSeek** is the default) and paste the matching API key. Keys are stored in the OS keyring (falling back to a `0600` file), never in `.env`, and never sent to the web view. The desktop app does not read `.env` (see [`fake.env`](../fake.env), which is only a placeholder).
   - **Scaleway DeepSeek**: sign up at [Scaleway](https://console.scaleway.com/) and create a secret key for the Generative APIs product; this is also the provider **Mail scan** uses (see below), regardless of which provider is selected for manual extraction.
   - **Mistral**: sign up at [La Plateforme](https://console.mistral.ai/); the free **Experiment** tier is typically enough for occasional job-text extraction (high monthly token allowance; rate limits apply — see [Mistral help center](https://help.mistral.ai/)). Limits can change; check their current docs.
   - **Gemini**: an API key from Google AI Studio.
   - Click **Save key** after pasting. **Test connection** checks the selected provider. The **LLM endpoint overrides (advanced)** fields change only the base URL or model id of the selected provider (leave blank for the built-in defaults); they never store a key.
2. **Job Search providers**: in **Settings**, add one or both:
   - **SerpAPI key** (primary provider)
   - **Brave Search API key** (fallback provider)
   
   Provider order is:
   1. SerpAPI
   2. Brave Search (if SerpAPI returns no usable results)
   
   If both are empty, Job Search will return a configuration error and still allow manual **Open in browser**.

## The board

The dashboard (`/`) has three views, switched in the header: **Kanban** (drag a card to change its status), **Table** (sortable by two columns; **Customize** shows or hides columns and hides chosen statuses) and **Calendar** (month grid of apply-by, interview and start dates). The search box in the header filters all three by company or title. **Settings → Appearance** sets the view shown at startup and the theme (system, light or dark; the header button cycles it too).

- **Status columns**: the default pipeline is `Interesting → Plan to Apply → Application Sent → Feedback → Done`. Rename and reorder the columns in **Settings → Board columns**. Every status change is recorded with a timestamp.
- **Detail panel**: clicking a job shows a summary beside the board: status, priority, dates, work mode, tags, notes and the listing check. **Full details** opens the job page (`/job/:id`) with the full edit form, contact and workplace fields, notes, documents (PDFs tagged CV, Cover Letter or Other, opened in your PDF viewer) and history. A job found through a job board shows the board as a secondary "via …" link.
- **Reminders**: the Reminder Center on the dashboard lists jobs whose apply-by date is overdue or within 14 days, except jobs in `Done`.

## Adding jobs

- **Add job** (header) opens a form. Paste the listing text (or just set **Job URL**, and the page text is fetched first) and press **Extract with AI** to fill the fields with the provider from Settings; review and save. `priority` is never filled by the AI. Saving warns when a job with the same URL or company and title already exists.
- **Quick capture** (header) opens a drawer: paste a listing URL and press **Capture**. The app fetches the page, extracts the fields, and shows a preview you can edit before **Save as Interesting** (the first board column). If fetching or extraction fails, you fill in the fields by hand. The same URL capture card sits at the top of the **Add job** page.
- **Capture Inbox** (in the same drawer) collects URLs handed to the app with a `?capture_url=<url>` link. For each one: **Prepare draft** (fetch and extract), edit, then **Accept** or **Dismiss**; the *History* filter shows handled items. The drawer's **Browser handoff link** builds such a link for you. The Capture Inbox lives in the app's local browser storage, not in the database.
- From **Job search** and the **Mail Match Inbox**, see below.

## Job Search

- Platforms in-app: **Jobindex**, **Indeed**, **LinkedIn**, **The Hub**.
- All four platforms use the same provider-based web search (SerpAPI + Brave fallback), scoped to each platform's domain.
- **Keywords** are suggested from the tags on your saved jobs (bigger chips are more frequent); click to select, or add your own. Optionally set a location and, for Indeed, the country site.
- Results are scored by freshness and keyword match. **Ranking mode** shows either the top 5 across all platforms or the top 5 per platform.
- Search result cards support:
  - **Add as Interesting** (one-click save with status `Interesting`)
  - auto-tags from source platform/location
  - **Open form** if you want to edit details before saving
- **Listing status check**: once a job is saved, the **Check listing** button on its detail page re-checks the original URL and marks it `active`, `closed`, `archived`, or `unreachable` (uses SerpAPI for sources that block direct fetches, e.g. Indeed).

## Google Calendar

### Calendar tab (in the app)

- **Month view** shows dates from your jobs: **apply-by**, **interview**, and **role start** (from SQLite only — no Google account required to view).
- **Template**: opens Google Calendar in the browser with a prefilled all-day event (no sign-in in the app).
- **Create in Google**: creates the event in your **primary** Google calendar via the Calendar API. Use **Settings → Connect with Google** (recommended) or paste a short-lived **access token** under **Advanced**.

### One-time Google Cloud setup

1. In [Google Cloud Console](https://console.cloud.google.com/), create or select a project.
2. Enable **Google Calendar API** (APIs & Services → Library).
3. Configure the **OAuth consent screen** (External is fine for personal use; add yourself as a test user while in "Testing").
4. **Credentials → Create credentials → OAuth client ID → Application type: Desktop app**. Copy the **Client ID**.
5. In Job Tracker **Settings**, paste the Client ID, click **Save Client ID**, then **Connect with Google**. Your browser opens; after you approve, the app stores a **refresh token** in the OS keyring (or a `0600` file when no keyring is available). No Client Secret is required for this desktop PKCE flow.

Scope used: `https://www.googleapis.com/auth/calendar.events`.

If **Create in Google** fails after a long time, use **Disconnect Google** and **Connect with Google** again.

### Smoke test checklist (local, after Google Cloud setup)

1. Start the app: `npm run tauri:dev`.
2. **Settings → Google Calendar**: paste your **Desktop** OAuth **Client ID** → **Save Client ID** → **Connect with Google**. Finish consent in the browser.
3. Open the **Calendar** tab: confirm the month grid lists jobs that have **apply-by**, **interview**, or **start** dates.
4. For one job with each kind of date, use **Create in Google** and confirm events appear in your **primary** Google Calendar.
5. **Disconnect Google** when finished testing if you do not want the refresh token left on this machine.

## Mail scan

Configure mail folders and both Candidate Profiles in **Settings → Mail scan**, then press
**Scan job emails** on the **Mail matches** page. Mail scan always scores through the
**Scaleway DeepSeek** API key from **Settings → AI & calendar**, independent of which provider is selected for manual
AI extraction — add a Scaleway key first if you want to use it. A pre-run sheet shows
the *resolved* folder paths, the model, the cutoff, and an estimate of how many model
calls the run will cost, before it spends anything.

Settings → Mail scan:

- **Mail folders**: **Find Thunderbird folders** lists the mail files under Thunderbird's
  *Local Folders*, or use **Browse** (mbox file or maildir). Point at the mail file, not
  the `.msf` index next to it. **Test read** counts the messages and shows their date range.
- **Candidate profiles**: a *short* and a *full* version of your CV as `.md` or `.txt`
  (not PDF). The short one screens listings (pass 1), the full one scores the survivors
  (pass 2).
- **Scoring**: the cutoff (minimum score to reach the inbox, default 7 of 10), how many
  days back to read (default 90), the model-call budget per scan (default 600; a scan
  that hits it stops cleanly and offers **Continue scan**) and the title blocklist.
- **Scanner** shows how the sidecar runs (development `uv` or frozen binary) and whether
  its signature was checked. **Re-score backlog** scores listings below the cutoff again.

What comes back lands in the **Mail Match Inbox** (`/mail-matches`), separate from the
Capture Inbox:

- **Pending** — matches sorted by score, with badges for incomplete details, "mail
  snippet only" (boards like Indeed that never serve their page), suspicious content,
  near-duplicates, and repeat sightings. Click a row to read the full ad, both score
  reasons, and its links. **Accept** adds it to your jobs as *Interesting* in one
  click; **Dismiss** hides it. Both show a notice with **Undo**.
- **Dismissed** — everything you have suppressed, with its reason, restorable in one
  click.
- **History** — past runs and their counters.

Filters above the list narrow it by board, details (complete, incomplete, failed, not
fetched), minimum score, and a search over title, company and board.

Working through the Pending tab:

- **Keyboard:** `j`/`k` (or ↓/↑) move between rows, `Enter` or `o` opens the detail,
  `a` accepts, `d` dismisses, `u` undoes the last accept or dismiss. Keys are ignored
  while you type in the search box.
- **Bulk:** tick rows (or *Select all shown*), then **Accept N** or **Dismiss N**. The
  app asks once, naming the count, runs them one by one, and tells you which failed;
  those stay selected. Only rows the current filters show are included.

Notes on how it behaves:

- **Scores are advisory.** A score never sets a job's `priority` or `status`, and it
  never creates a Job on its own.
- **Mail is hostile input.** Listing text is passed to the model as data, hidden HTML
  and invisible characters are stripped, and a listing that tries to instruct the model
  is flagged rather than trusted. Fetched pages are never rendered in the app.
- **Skips happen before they cost.** Listings whose title matches your **title blocklist**
  (Settings → Mail scan) and listings whose page says the posting is closed never reach
  the model.
- **Re-runs are cheap.** Scores are cached, so a crashed or cancelled scan costs almost
  nothing to repeat. Editing a Candidate Profile re-scores the backlog; *copying* the
  file does not. A listing you dismissed, or one already on your board, is skipped
  before it costs a model call or a page fetch.
- **Candidate Profiles are CV content.** They live at `$APPDATA/profiles/` mode `0600`,
  are never shown to the web view, and are excluded from export and backup.
- **Settings → Delete all mail scan data** removes every trace of scanning (inbox,
  sightings, dismissals, cursors, runs, cache) and leaves your Jobs untouched.

The external *Jobmails* helper this replaces is retired; day-to-day use is entirely in
the app.

## Data storage

- **SQLite and uploaded PDFs** live in the OS app data directory for the Tauri app (not
  in this repo): `~/.local/share/com.github.eikrad.jobtracker/` on Linux,
  `%APPDATA%\com.github.eikrad.jobtracker\` on Windows,
  `~/Library/Application Support/com.github.eikrad.jobtracker/` on macOS. The database is
  `data/app.db`, PDFs are under `storage/applications/`.
- The repo `storage/` folder is for optional manual files; see `.gitignore`.
- API keys and the Google refresh token are stored in the **OS keyring** (falling back to
  `0600` files when no secret service is available); they are never returned to the web
  view.
- Candidate Profiles live at `$APPDATA/profiles/{short,full}.md`, mode `0600`, and are
  excluded from export and backup.
- Small UI preferences (theme, board columns, default view, table layout, backup folder,
  selected AI provider) and the Capture Inbox are kept in the app's local browser storage.

## Backup, import and export

All three are in **Settings → Backup & restore**.

- **Automatic backup**: after each change to your jobs (add, edit, status change, delete,
  import, PDF upload), the app copies the database and your PDFs to
  `<backup folder>/JobTracker/`. The default folder is `~/Jottacloud`; point it at any
  synced or external folder, or clear the field to switch backups off. Candidate Profiles
  and API keys are never copied.
- **Export**: JSON or CSV of all jobs.
- **Import**: JSON array (same shape as export) or CSV from export; creates **new** rows
  (IDs are not preserved; rows without a company are skipped).

## Building installers

`npm run tauri:build` produces a binary under `src-tauri/target/release/`; packaged installers land under `src-tauri/target/release/bundle/`. Maintainers shipping a release build with the frozen mail-scan sidecar use `npm run tauri:build:release` (see [CONTRIBUTING.md](../CONTRIBUTING.md#mail-scan-sidecar)).

| Platform | Default bundles in this repo | Where to look |
|----------|------------------------------|---------------|
| **Linux** | `.deb`, `.rpm` | `src-tauri/target/release/bundle/deb/`, `…/rpm/` |
| **Windows** | NSIS + MSI | `…/bundle/nsis/`, `…/msi/` |
| **macOS** | `.app` + `.dmg` | `…/bundle/macos/`, `…/dmg/` |

**AppImage is not in the default list.** Tauri's AppImage step invokes [**linuxdeploy**](https://github.com/linuxdeploy/linuxdeploy); if it is missing or not executable, the build fails with **`failed to run linuxdeploy`**. That is a common pain on fresh machines, so we only request formats that do not need it unless you opt in.

**To build an AppImage locally:** follow [Tauri: AppImage](https://v2.tauri.app/distribute/appimage/) (linuxdeploy + AppImage plugin on `PATH`), then add `"appimage"` to `bundle.targets` in [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json). Maintainer-oriented details: [CONTRIBUTING.md — Tauri release bundles](../CONTRIBUTING.md#tauri-release-bundles-linux-appimage).

**Arch / Manjaro:** there is no AUR package generated by these defaults; you can run the binary at `src-tauri/target/release/<app-name>` or install from `.deb`/`.rpm` if your distro supports it.

### Linux desktop shortcut

`npm run tauri:build` installs this automatically on Linux. To refresh the launcher without a full rebuild:

```bash
npm run desktop:shortcut
```

This creates `~/.local/share/applications/JobTracker.desktop` and copies app icons into `~/.local/share/icons/hicolor/`.

Regenerate platform icons from `assets/app-icon-source.png` with `npm run icon:generate`.
