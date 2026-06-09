# Job Tracker Architecture

## Overview

Job Tracker is a local-first desktop application. All data lives on the user's machine in a SQLite database. There is no server and no cloud dependency by default — the only optional network calls are AI extraction and job search.

```mermaid
graph LR
    USER([User]) --> UI[React UI\nVite + TypeScript]
    UI <-->|Tauri IPC\ncommands / events| RUST[Rust Backend\nTauri + rusqlite]
    RUST <--> DB[(SQLite\napp data dir)]
    RUST <--> FS[File System\nPDF storage]
    UI -->|HTTPS| AI[AI Extraction\nGemini / Mistral]
    UI -->|HTTPS| SEARCH[Job Search\nSerpAPI + Brave]
    UI -->|OAuth PKCE| GCAL[Google Calendar API]
```

---

## Source structure

```
src/                  — React frontend (TypeScript)
├── components/       — Shared UI components
├── features/         — Feature modules (dashboard, job-detail, search, calendar, settings)
├── pages/            — Top-level route pages
├── hooks/            — Custom React hooks
├── context/          — App-wide React context (jobs, settings, auth)
├── lib/              — Utility functions and API wrappers
└── i18n/             — Internationalisation strings

src-tauri/            — Tauri Rust backend
├── src/
│   ├── main.rs       — Entry point
│   ├── commands/     — Tauri command handlers (CRUD, PDF, export, calendar)
│   ├── db/           — SQLite migrations and query helpers
│   └── storage/      — File path helpers and PDF management
└── tauri.conf.json   — App config (version, permissions, bundle targets)
```

---

## Data flows

### Adding or editing a job

```mermaid
sequenceDiagram
    participant UI as React UI
    participant IPC as Tauri IPC
    participant DB as SQLite

    UI->>IPC: invoke("create_job", { title, company, ... })
    IPC->>DB: INSERT INTO jobs ...
    DB-->>IPC: new job row
    IPC-->>UI: job object
    UI->>UI: update local state, navigate to detail
```

### AI extraction from text or PDF

```mermaid
flowchart TD
    U([User pastes job text\nor uploads PDF]) --> EXTRACT[UI calls AI provider\nGemini or Mistral]
    EXTRACT --> RESP[Structured JSON\ntitle · company · deadline · ...]
    RESP --> FORM[Pre-fills Add Job form]
    FORM --> SAVE[User reviews and saves]
    SAVE --> DB[(SQLite)]
```

### Job search

```mermaid
flowchart TD
    U([User enters search query]) --> SP1{SerpAPI key\nconfigured?}
    SP1 -->|yes| SERP[SerpAPI\nweb results]
    SP1 -->|no| SP2{Brave key\nconfigured?}
    SP2 -->|yes| BRAVE[Brave Search API\nweb results]
    SP2 -->|no| ERR([Config error])
    SERP --> RESULTS[Result cards\nwith Add / Open actions]
    BRAVE --> RESULTS
```

---

## Dashboard views

The Dashboard renders the same SQLite data in three views:

```mermaid
graph LR
    DB[(SQLite jobs)] --> KANBAN[Kanban\ncolumns by status]
    DB --> TABLE[Table\nsortable / filterable]
    DB --> CAL[Calendar\napply-by · interview · start dates]
```

---

## Google Calendar integration

Events are created via the Google Calendar API using a desktop OAuth PKCE flow. No client secret is stored.

```mermaid
sequenceDiagram
    participant APP as Desktop App
    participant GC as Google Calendar API

    APP->>APP: open system browser\nOAuth consent screen
    APP->>APP: local redirect captures auth code
    APP->>GC: exchange code for tokens\n(PKCE — no client secret)
    GC-->>APP: access token + refresh token
    APP->>APP: store refresh token\nin OS credential store
    APP->>GC: POST /events
    GC-->>APP: event created
```

---

## Storage

All data lives locally on the user's machine:

| Store | Location | Contents |
|-------|----------|----------|
| SQLite database | OS app data dir | Jobs, status history, notes, deadlines |
| PDF files | OS app data dir | Uploaded application PDFs |
| API keys | Browser local storage | Gemini / Mistral / SerpAPI / Brave keys |
| Google refresh token | OS credential store | Google Calendar OAuth token |
| Google Client ID | App settings | Configured by user in Settings |

The `storage/` folder in the repo is for optional manual files only; it is not used by the running app.

---

## Tech stack

| Layer | Technology |
|-------|------------|
| UI framework | React 19 + TypeScript |
| Build tool | Vite 8 |
| Routing | React Router v7 |
| Drag & drop | @dnd-kit/core |
| Desktop shell | Tauri 2 (Rust) |
| Database | SQLite via rusqlite |
| Testing — frontend | Vitest + Testing Library |
| Testing — Rust | cargo test |
| Linting | ESLint 9 · Ruff (Python) |
| Formatting | Black + isort (Python) |
| CI | GitHub Actions (3 workflows) |

---

## CI workflows

Three independent GitHub Actions workflows run on every push and pull request:

```mermaid
flowchart LR
    PUSH([git push]) --> FE[frontend.yml\nlint → test → build]
    PUSH --> RUST[rust.yml\ncargo clippy → cargo test]
    PUSH --> PY[python.yml\nruff → black → isort → pytest]
```

The Python workflow covers the `tests/` scripts (linting helpers and smoke tests), not a Python backend.
