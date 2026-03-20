# Job Tracker

[![CI](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/eikrad/Job-Tracker/actions/workflows/ci.yml)

Local desktop app (Tauri + React + SQLite) to track job applications, deadlines, and PDFs.

## Development

```bash
npm ci
npm run dev          # Vite only (browser)
npm run tauri:dev    # Desktop shell + Vite
```

## Environment

1. Copy `fake.env` to `.env` (optional; Gemini key can also be stored in-app).
2. Add `GEMINI_API_KEY` if you use **Extract with Gemini**.

## Google Calendar (API)

- **Template link**: opens Google Calendar compose (no OAuth).
- **Create via API**: requires an OAuth **access token** with scope `https://www.googleapis.com/auth/calendar.events` (paste in the app). Tokens expire; refresh via your OAuth client or Google OAuth Playground for testing.

## Local storage

- App data (SQLite + uploaded PDFs) lives under the OS app data directory for Tauri.
- Repo `storage/` is for optional manual assets; see `.gitignore`.

## Import / export

- **Export**: JSON or CSV from the header actions.
- **Import**: JSON array (same shape as export) or CSV from export; creates new rows (IDs are not preserved).

## CI & tests

- **GitHub Actions**: frontend build + lint, Rust `cargo check`, **pytest** (export JSON contract).
- Local Python tests:

```bash
pip install -r requirements-dev.txt
pytest
```

> **Forks:** Replace `eikrad/Job-Tracker` in the badge URL with your `user/repo`.

## Tech stack

- React + TypeScript + Vite
- Tauri 2
- SQLite (rusqlite) in the Rust backend
