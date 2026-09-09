# No IMAP — local Thunderbird mail folders only

**Status:** Accepted · 2026-09-09

Mail intake must never authenticate to or connect to a mail server. The Mail Scan reads only local Thunderbird mbox/folder paths (configurable in Settings). Credentials and live IMAP are explicit non-goals so the app cannot access the mail account beyond what Thunderbird already caches on disk.

**Considered options:** Live IMAP/Proton bridge from the app; local folder reads only. Chose local-only for privacy boundary clarity.
