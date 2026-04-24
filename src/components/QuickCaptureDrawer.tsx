import { useEffect, useMemo, useState } from "react";
import { UrlCaptureCard } from "../features/capture/UrlCaptureCard";
import { CaptureInboxPanel } from "../features/capture/CaptureInboxPanel";
import { normalizeCaptureUrl } from "../features/capture/urlCapture";
import { useJobTracker } from "../context/JobTrackerContext";
import { en } from "../i18n/en";

type Props = {
  open: boolean;
  onClose: () => void;
};

export function QuickCaptureDrawer({ open, onClose }: Props) {
  const { statuses, onExtract, onSubmit } = useJobTracker();
  const [handoffUrl, setHandoffUrl] = useState("");
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">("idle");

  const handoffLink = useMemo(() => {
    const normalized = normalizeCaptureUrl(handoffUrl);
    if (!normalized) return "";
    const appUrl = new URL(window.location.href);
    appUrl.searchParams.set("capture_url", normalized);
    return appUrl.toString();
  }, [handoffUrl]);

  function handleClose() {
    setHandoffUrl("");
    setCopyState("idle");
    onClose();
  }

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") handleClose();
    };
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
    // handleClose is stable enough for our purposes; excluding it keeps
    // the listener from rebinding on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, onClose]);

  if (!open) return null;

  async function copyHandoffLink() {
    if (!handoffLink) return;
    try {
      await navigator.clipboard.writeText(handoffLink);
      setCopyState("copied");
    } catch {
      setCopyState("error");
    }
  }

  return (
    <div className="captureDrawer" role="dialog" aria-modal="true" aria-label={en.capture.drawerTitle}>
      <button
        type="button"
        aria-label={en.capture.closeDrawer}
        className="captureDrawerBackdrop"
        onClick={handleClose}
      />
      <section className="captureDrawerPanel">
        <div className="captureDrawerHeader">
          <h2>{en.capture.drawerTitle}</h2>
          <button type="button" className="btn btnGhost btnSm" onClick={handleClose}>
            {en.capture.closeDrawer}
          </button>
        </div>
        <UrlCaptureCard statuses={statuses} onExtract={onExtract} onSubmit={onSubmit} autoFocusInput />
        <section className="card captureHandoffCard">
          <h3>{en.capture.handoffTitle}</h3>
          <p className="muted">{en.capture.handoffSubtitle}</p>
          <div className="row captureInputRow">
            <input
              value={handoffUrl}
              onChange={(e) => {
                setHandoffUrl(e.target.value);
                setCopyState("idle");
              }}
              placeholder={en.capture.urlPlaceholder}
              aria-label={en.capture.handoffInputAriaLabel}
            />
            <button
              type="button"
              className="btn btnGhost"
              onClick={() => void copyHandoffLink()}
              disabled={!handoffLink}
            >
              {en.capture.copyHandoffLink}
            </button>
          </div>
          {copyState === "copied" && <p className="muted">{en.capture.handoffCopied}</p>}
          {copyState === "error" && <p className="error">{en.capture.handoffCopyFailed}</p>}
        </section>
        <CaptureInboxPanel statuses={statuses} onExtract={onExtract} onSubmit={onSubmit} />
      </section>
    </div>
  );
}
