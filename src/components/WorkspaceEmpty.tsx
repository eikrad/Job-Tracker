import { Link } from "react-router-dom";

type Props = {
  title: string;
  body: string;
  cta: string;
  to?: string;
};

/** Simple inline illustration + copy + optional CTA link. */
export function WorkspaceEmpty({ title, body, cta, to = "/jobs/new" }: Props) {
  return (
    <div className="workspaceEmpty">
      <div className="workspaceEmptyIcon" aria-hidden>
        <svg viewBox="0 0 120 120" width="120" height="120" fill="none">
          <rect x="16" y="24" width="88" height="72" rx="10" stroke="currentColor" strokeWidth="2" />
          <path d="M32 40h56M32 52h40M32 64h48" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          <circle cx="78" cy="78" r="18" fill="var(--color-primary-muted)" stroke="currentColor" strokeWidth="2" />
          <path d="M72 78h12M78 72v12" stroke="var(--color-primary)" strokeWidth="2.5" strokeLinecap="round" />
        </svg>
      </div>
      <h3 className="workspaceEmptyTitle">{title}</h3>
      <p className="workspaceEmptyBody">{body}</p>
      <Link className="btn btnPrimary workspaceEmptyCta" to={to}>
        {cta}
      </Link>
    </div>
  );
}
