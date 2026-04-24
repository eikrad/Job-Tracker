import { useMemo, useState } from "react";
import type { NewJob } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";
import { buildCaptureDraft, captureWarningMessage } from "./capturePipeline";
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
  const [view, setView] = useState<"active" | "history">("active");
  const defaultStatus = useMemo(() => statuses[0] ?? "Interesting", [statuses]);

  function refresh() {
    setItems(listCaptureInboxItems());
  }

  async function prepareItem(item: CaptureInboxItem) {
    updateCaptureInboxItem(item.id, { status: "pending", warning: undefined });
    refresh();
    const { draft, reason } = await buildCaptureDraft({
      url: item.url,
      defaultStatus,
      onExtract,
    });
    updateCaptureInboxItem(item.id, {
      draft,
      warning: reason ? captureWarningMessage(reason) : undefined,
      status: reason ? "failed" : "ready",
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

  const visibleItems = useMemo(() => {
    const isHistory = (item: CaptureInboxItem) =>
      item.status === "accepted" || item.status === "dismissed";
    return items.filter((item) => (view === "history" ? isHistory(item) : !isHistory(item)));
  }, [items, view]);

  return (
    <section className="card captureInboxPanel">
      <h3>{en.capture.inboxTitle}</h3>
      <p className="muted">{en.capture.inboxSubtitle}</p>
      <div className="tabList captureInboxTabs" role="tablist" aria-label={en.capture.inboxFilterAriaLabel}>
        <button
          type="button"
          role="tab"
          className="btnTab"
          aria-selected={view === "active"}
          onClick={() => setView("active")}
        >
          {en.capture.inboxFilterActive}
        </button>
        <button
          type="button"
          role="tab"
          className="btnTab"
          aria-selected={view === "history"}
          onClick={() => setView("history")}
        >
          {en.capture.inboxFilterHistory}
        </button>
      </div>
      {visibleItems.length === 0 && (
        <p className="muted">{view === "active" ? en.capture.inboxEmpty : en.capture.inboxHistoryEmpty}</p>
      )}
      {visibleItems.map((item) => (
        <article key={item.id} className="captureInboxItem">
          <p className="captureInboxUrl">{item.url}</p>
          <p className="muted captureInboxStatus">{en.capture.inboxStatusLabel}: {item.status}</p>
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
                {view === "active" && (
                  <button type="button" className="btn btnGhost btnSm" onClick={() => dismissItem(item)}>
                    {en.capture.dismiss}
                  </button>
                )}
              </div>
            </>
          )}
        </article>
      ))}
    </section>
  );
}
