import { useEffect } from "react";
import { UrlCaptureCard } from "../features/capture/UrlCaptureCard";
import { useJobTracker } from "../context/JobTrackerContext";
import { en } from "../i18n/en";

type Props = {
  open: boolean;
  onClose: () => void;
};

export function QuickCaptureDrawer({ open, onClose }: Props) {
  const { statuses, onExtract, onSubmit } = useJobTracker();

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="captureDrawer" role="dialog" aria-modal="true" aria-label={en.capture.drawerTitle}>
      <button
        type="button"
        aria-label={en.capture.closeDrawer}
        className="captureDrawerBackdrop"
        onClick={onClose}
      />
      <section className="captureDrawerPanel">
        <div className="captureDrawerHeader">
          <h2>{en.capture.drawerTitle}</h2>
          <button type="button" className="btn btnGhost btnSm" onClick={onClose}>
            {en.capture.closeDrawer}
          </button>
        </div>
        <UrlCaptureCard statuses={statuses} onExtract={onExtract} onSubmit={onSubmit} autoFocusInput />
      </section>
    </div>
  );
}
