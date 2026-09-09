# API keys live in the OS keyring, not localStorage

**Status:** Accepted · 2026-09-09

All provider API keys (Gemini, Mistral, Scaleway, SerpAPI, Brave) are stored in the OS keyring and read only by Rust. The frontend can set, replace, clear, and query *whether* a key is configured; it never receives the key value back. Settings shows "configured / not configured", not the secret.

**Why:** Keys currently sit in `localStorage` (`useJobTrackerState.ts`), which every script in the webview can read — and the app renders content fetched from remote job listings. One injection reads every key. The repo already stores the Google OAuth refresh token in the keyring (`google_oauth.rs`), so this is applying an existing pattern consistently rather than adding a mechanism. With ADR 0004, the mail scan needs no key in the webview at all, which removes the last reason to keep them there.

**Considered options:** Keep `localStorage` (status quo, simplest); encrypt in `localStorage` (the decryption key would have to live next to it, so no real gain); OS keyring. Chose the keyring.

**Consequences:** A one-time migration on launch moves existing `localStorage` keys into the keyring and deletes the originals. Where no secret service is available (headless Linux), the fallback is a `0600` file in app data, and Settings says so rather than downgrading silently. Job-form extraction moves its provider calls behind a Rust command, since the frontend can no longer hold the key. Keys are unreadable by the user after entry — Settings offers Replace and Remove, not reveal. A single `redact()` covers every log sink, error string, and run summary.
