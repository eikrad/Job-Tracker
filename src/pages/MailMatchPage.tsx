/**
 * Route wrapper for the Mail Match Inbox (spec §11.1, ADR 0003).
 *
 * Accepting a match navigates to the job form with the draft prefilled, so the last
 * step before a Job exists is always a form the user submits.
 */

import { useNavigate } from "react-router-dom";
import type { NewJob } from "../lib/types";
import { MailMatchInboxPanel } from "../features/mailMatch/MailMatchInboxPanel";

export function MailMatchPage() {
  const navigate = useNavigate();

  return (
    <main className="page page--mail-match">
      <MailMatchInboxPanel
        onAcceptDraft={(inboxId, draft: Partial<NewJob>) => {
          navigate("/jobs/new", { state: { draft, mailMatchInboxId: inboxId } });
        }}
      />
    </main>
  );
}
