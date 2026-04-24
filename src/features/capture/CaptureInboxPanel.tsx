import { useMemo, useState } from "react";
import type { NewJob } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";
import { buildCaptureDraft } from "./capturePipeline";
import {
  listCaptureInboxItems,
  updateCaptureInboxItem,
  type CaptureInboxItem,
} from "./captureInbox";

type Props = {
  statuses: string[];
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
  onSubmit: (payload: NewJob) => Promise<boolean>;
};

export function CaptureInboxPanel({ statuses, onExtract, onSubmit }: Props) {
  const [items, setItems] = useState<CaptureInboxItem[]>(() => listCaptureInboxItems());
  const defaultStatus = useMemo(() => statuses[0] ?? "Interesting", [statuses]);

  function refresh() {
    setItems(listCaptureInboxItems());
  }

  async function prepareItem(item: CaptureInboxItem) {
    updateCaptureInboxItem(item.id, { status: "pending", warning: undefined });
    refresh();
    const { draft, warning } = await buildCaptureDraft({
      url: item.url,
      defaultStatus,
      onExtract,
    });
    updateCaptureInboxItem(item.id, {
      draft,
      warning,
      status: warning ? "failed" : "ready",
    });
    refresh();
  }

  async function acceptItem(item: CaptureInboxItem) {
    if (!item.draft?.company?.trim()) return;
    const ok = await onSubmit(item.draft);
    if (!ok) return;
    updateCaptureInboxItem(item.id, { status: "accepted" });
    refresh();
  }

  function dismissItem(item: CaptureInboxItem) {
    updateCaptureInboxItem(item.id, { status: "dismissed" });
    refresh();
  }

  function editDraft(item: CaptureInboxItem, patch: Partial<NewJob>) {
    if (!item.draft) return;
    updateCaptureInboxItem(item.id, { draft: { ...item.draft, ...patch } });
    refresh();
  }

  const activeItems = items.filter((item) => item.status === "pending" || item.status === "ready" || item.status === "failed");

  return (
    <section className="card captureInboxPanel">
      <h3>{en.capture.inboxTitle}</h3>
      <p className="muted">{en.capture.inboxSubtitle}</p>
      {activeItems.length === 0 && <p className="muted">{en.capture.inboxEmpty}</p>}
      {activeItems.map((item) => (
        <article key={item.id} className="captureInboxItem">
          <p className="captureInboxUrl">{item.url}</p>
          {!item.draft && (
            <div className="row">
              <button type="button" className="btn btnGhost btnSm" onClick={() => void prepareItem(item)}>
                {en.capture.prepare}
              </button>
              <button type="button" className="btn btnGhost btnSm" onClick={() => dismissItem(item)}>
                {en.capture.dismiss}
              </button>
            </div>
          )}
          {item.warning && <p className="muted">{item.warning}</p>}
          {item.draft && (
            <>
              <div className="grid">
                <input
                  placeholder={en.capture.companyLabel}
                  value={item.draft.company}
                  onChange={(e) => editDraft(item, { company: e.target.value })}
                />
                <input
                  placeholder={en.capture.titleLabel}
                  value={item.draft.title ?? ""}
                  onChange={(e) => editDraft(item, { title: e.target.value })}
                />
              </div>
              <div className="row">
                <button
                  type="button"
                  className="btn btnPrimary btnSm"
                  disabled={!item.draft.company.trim()}
                  onClick={() => void acceptItem(item)}
                >
                  {en.capture.accept}
                </button>
                <button type="button" className="btn btnGhost btnSm" onClick={() => dismissItem(item)}>
                  {en.capture.dismiss}
                </button>
              </div>
            </>
          )}
        </article>
      ))}
    </section>
  );
}
