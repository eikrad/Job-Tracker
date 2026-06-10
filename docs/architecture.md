# Architecture

How Job Tracker is put together — from the desktop window down to SQLite.

---

## System overview

Job Tracker is a **fully local** desktop app. There is no server, no cloud account required, and all data stays on your machine.

```mermaid
graph TD
    USER([You]) --> WINDOW[Tauri Desktop Window]

    subgraph APP["Desktop App"]
        WINDOW --> REACT[React UI\nTypeScript + Vite]
        WINDOW --> RUST[Rust Backend\nTauri commands]
    end

    RUST --> SQLITE[(SQLite\nOS app data dir)]
    RUST --> FILES[PDF files\nOS app data dir]

    REACT -.->|optional| AI[AI Extraction\nGemini · Mistral]
    REACT -.->|optional| SEARCH[Job Search\nSerpAPI · Brave]
    REACT -.->|optional| GCAL[Google Calendar API\nOAuth 2.0 PKCE]
```

The React UI communicates with the Rust backend exclusively through Tauri's `invoke()` IPC bridge — there is no HTTP server running.

---

## Layer overview

| Layer | Technology | Responsibility |
|-------|-----------|---------------|
| Desktop shell | Tauri 2 (Rust) | Native window, OS integration, IPC bridge |
| UI | React + TypeScript + Vite | All screens, routing, state management |
| Backend | Rust (rusqlite) | SQLite queries, file I/O, system commands |
| Storage | SQLite | Job records, status history, deadlines, reminders |
| File storage | OS app data dir | Uploaded application PDFs |

---

## Source layout

```
src/                    — React frontend (TypeScript)
  features/
    jobs/               — Job CRUD and state
    capture/            — Quick-add / job capture
    extraction/         — AI-assisted field extraction
    jobSearch/          — Job search integration
    deadlines/          — Deadline tracking
    reminders/          — Reminder logic
  pages/                — Page components (Dashboard, AddJob, JobDetail, JobSearch)
  components/           — Shared UI components
  hooks/                — Custom React hooks
  context/              — React context providers
  i18n/                 — Internationalisation strings
  lib/                  — Shared utilities
src-tauri/              — Rust backend (Tauri)
  src/                  — Tauri commands, SQLite queries, file helpers
  tauri.conf.json       — App metadata, bundle config, permissions
docs/                   — Project documentation
```

---

## Job lifecycle

```mermaid
stateDiagram-v2
    [*] --> Interesting : discovered / imported
    Interesting --> Applied : apply action
    Applied --> Interview : interview scheduled
    Interview --> Offer : offer received
    Interview --> Rejected : rejected after interview
    Applied --> Rejected : rejected before interview
    Offer --> Accepted : accepted
    Offer --> Rejected : declined
    Accepted --> [*]
    Rejected --> [*]
```

Each status change is written to a history log in SQLite, so the full timeline of an application is preserved.

---

## AI extraction flow

Paste job description text and AI parses it into pre-filled form fields.

```mermaid
flowchart LR
    TEXT([Paste job\ndescription]) --> CHOICE{Provider set\nin Settings?}
    CHOICE -->|Gemini key| GEM[Google Gemini API]
    CHOICE -->|Mistral key| MIS[Mistral API]
    GEM --> FIELDS[Structured fields\ntitle · company · location\nsalary · deadline · ...]
    MIS --> FIELDS
    FIELDS --> FORM[Pre-filled Add Job form]
```

API keys are stored in the browser's local storage for the app profile. Extraction calls go directly from the React UI to the provider — the Rust backend is not involved.

---

## Job search flow

```mermaid
flowchart TD
    Q([Search query]) --> PLATFORM{Platform}
    PLATFORM -->|Jobindex / Indeed| PROVIDER{Search provider}
    PLATFORM -->|LinkedIn| BROWSER([Open in browser])
    PROVIDER -->|SerpAPI key set| SERP[SerpAPI]
    PROVIDER -->|Brave key set| BRAVE[Brave Search API]
    PROVIDER -->|no keys| ERR([Config error +\nmanual link])
    SERP --> CARDS[Result cards]
    BRAVE --> CARDS
    CARDS -->|one-click| SAVE([Add as Interesting])
    CARDS -->|open form| DETAIL([Edit before saving])
```

---

## Google Calendar integration

The Calendar tab shows job dates (apply-by, interview, start) from SQLite — no Google account is needed for the local month view. The Google integration is only required to push events to your own calendar.

```mermaid
flowchart LR
    SETUP[Settings:\npaste OAuth Client ID] --> PKCE[PKCE flow\nbrowser consent]
    PKCE --> TOKEN[Refresh token\nOS credential store]
    TOKEN --> CREATE[Create event\nGoogle Calendar API]
```

---

## Storage locations

All data lives in the OS application data directory — nothing is stored in the repo.

| Location | Contents |
|----------|----------|
| `{os_data_dir}/com.jobtracker.app/jobs.db` | SQLite: jobs, history, deadlines, reminders |
| `{os_data_dir}/com.jobtracker.app/files/` | Uploaded application PDFs |

---

## CI

Three independent GitHub Actions workflows run on every push:

| Workflow | Checks |
|----------|--------|
| **Frontend** | ESLint → Vitest → Vite build |
| **Rust** | `cargo clippy` → `cargo test` |
| **Python** | ruff · black · isort → pytest |
