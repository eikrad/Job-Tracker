# Job Tracker — Architecture

This document covers how the app is built: the Tauri process model, the Rust command layer, SQLite data model, React feature structure, and external integrations.

---

## System overview

Job Tracker is a desktop app. The React UI runs inside a Tauri WebView and communicates with a Rust backend via Tauri's type-safe IPC commands. All persistent data lives in SQLite on your machine.

```mermaid
graph LR
    USER([You]) --> UI[React UI\nTypeScript + Vite]
    UI <-->|Tauri IPC commands| RUST[Rust Backend\nTauri 2]
    RUST <-->|rusqlite| DB[(SQLite\nOS app data dir)]
    UI -.->|AI extraction\ncalled from renderer| AI[Gemini\nor Mistral API]
    RUST -.->|Job search\nSerpAPI / Brave| SEARCH[Search APIs]
    RUST -.->|Google Calendar API| GCAL[Google Calendar]
    RUST -.->|OAuth 2.0 PKCE| GAUTH[Google OAuth]
```

The UI never reads from or writes to SQLite directly — all persistence goes through Rust IPC commands. AI extraction calls are made from the renderer directly to the LLM API (keys are stored in browser local storage, scoped to the app WebView).

---

## Process model

```mermaid
flowchart LR
    subgraph OS [Operating System]
        direction TB
        TAURI[Tauri Shell\nRust process]
        WEBVIEW[WebView\nReact app]
        DB[(SQLite\n~/.local/share/...)]
        CRED[OS credential store\nSecretService / Keychain]
    end

    TAURI <-->|WebView bridge / IPC| WEBVIEW
    TAURI <--> DB
    TAURI <--> CRED
```

- The Tauri shell owns the SQLite connection (one connection per process, WAL mode).
- The Google OAuth refresh token is stored in the OS credential store — never in SQLite or the filesystem.
- API keys for AI providers are stored in browser local storage, scoped to the app WebView profile.

---

## Rust command layer (`src-tauri/src/`)

| File | Responsibility |
|------|---------------|
| `lib.rs` | Tauri app setup — registers all IPC commands, opens the DB connection, runs migrations |
| `db.rs` | All SQLite operations — CRUD for jobs, history, reminders, documents, tags |
| `job_search.rs` | Job search against SerpAPI and Brave Search; parses results into typed structs |
| `calendar.rs` | Google Calendar API calls — create, update, delete events |
| `google_oauth.rs` | OAuth 2.0 PKCE flow, token refresh, and OS credential store integration |

---

## Data model

All tables live in a single SQLite file in the OS app data directory.

```mermaid
erDiagram
    JOBS {
        text id PK
        text title
        text company
        text status
        text url
        text location
        text employment_type
        text notes
        text apply_by_date
        text interview_date
        text start_date
        integer created_at
        integer updated_at
    }
    JOB_HISTORY {
        text id PK
        text job_id FK
        text from_status
        text to_status
        integer changed_at
        text note
    }
    REMINDERS {
        text id PK
        text job_id FK
        text message
        integer remind_at
        integer dismissed
    }
    JOB_DOCUMENTS {
        text id PK
        text job_id FK
        text filename
        text file_path
        integer uploaded_at
    }
    JOBS ||--o{ JOB_HISTORY : "has"
    JOBS ||--o{ REMINDERS : "has"
    JOBS ||--o{ JOB_DOCUMENTS : "has"
```

---

## React feature modules (`src/features/`)

| Module | What it handles |
|--------|----------------|
| `jobs/` | Core job CRUD — list, add, edit, status transitions |
| `deadlines/` | Deadline dates (apply-by, interview, start) — reading and editing |
| `extraction/` | AI-assisted field extraction (Gemini / Mistral) |
| `jobSearch/` | Job search UI and result handling |
| `capture/` | Quick-capture flow for saving a job directly from search results |
| `reminders/` | Reminder display and dismissal |

Pages in `src/pages/`:

| Page | Route | Purpose |
|------|-------|---------|
| `DashboardPage` | `/` | Kanban / Table / Calendar view of all jobs |
| `AddJobPage` | `/add` | Form to add a new job manually |
| `JobDetailPage` | `/jobs/:id` | Full detail view — edit, history, documents, reminders |
| `JobSearchPage` | `/search` | In-app job search across platforms |

---

## Job search flow

```mermaid
flowchart TD
    USER([User enters search]) --> UI[JobSearchPage]
    UI --> CMD[Tauri command\nsearch_jobs]
    CMD --> SERP{SerpAPI\nreturns results?}
    SERP -->|yes| PARSE[Parse into result cards]
    SERP -->|no / empty| BRAVE[Brave Search\nfallback]
    BRAVE --> PARSE
    PARSE --> CARDS[Result cards in UI]
    CARDS -->|Add as Interesting| SAVE[Save to SQLite]
    CARDS -->|Open in browser| BROWSER[Default browser]
```

LinkedIn always opens in the browser — no API integration. Jobindex and Indeed use the provider-based search path above.

---

## Google Calendar flow

```mermaid
flowchart TD
    USER([User clicks\nConnect with Google]) --> PKCE[Tauri initiates\nOAuth PKCE flow]
    PKCE --> BROWSER[System browser\nGoogle consent screen]
    BROWSER -->|Authorization code| TAURI[Tauri captures\nlocal redirect]
    TAURI --> TOKEN[Exchange code\nfor access + refresh token]
    TOKEN --> STORE[Store refresh token\nin OS credential store]

    USER2([User clicks\nCreate in Google]) --> CHECK{Refresh token\npresent?}
    CHECK -->|yes| REFRESH[Get new access token\nvia refresh]
    CHECK -->|no| PKCE
    REFRESH --> CREATE[POST to\nGoogle Calendar API]
    CREATE --> DONE([Event created\nin primary calendar])
```

Scope: `https://www.googleapis.com/auth/calendar.events` (create events in the primary calendar only). No Client Secret is required — the PKCE flow is safe for native desktop apps.

---

## Adding a feature

1. Create a directory under `src/features/<name>/` with your components, hooks, and types.
2. For any data that needs to persist, add a Rust command in `src-tauri/src/db.rs` and register it in `lib.rs`.
3. If the feature needs a full-page view, add a page component to `src/pages/` and a route entry in `src/App.tsx`.
4. Update `docs/architecture.md` if the feature adds a new external integration or changes the data model.
