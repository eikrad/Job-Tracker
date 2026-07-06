import type { Job } from "../../lib/types";

type Status = Job["listing_status"];

const CONFIG: Record<
  NonNullable<Status>,
  { color: string; label: string }
> = {
  active: { color: "#22c55e", label: "Listing active" },
  closed: { color: "#ef4444", label: "Listing closed" },
  archived: { color: "#f97316", label: "Listing archived" },
  unreachable: { color: "#a1a1aa", label: "Listing unreachable" },
};

type Props = {
  status?: Status;
  size?: number;
};

export function ListingStatusDot({ status, size = 8 }: Props) {
  if (!status) {
    return (
      <span
        title="Listing not yet checked"
        style={{
          display: "inline-block",
          width: size,
          height: size,
          borderRadius: "50%",
          background: "transparent",
          border: "1.5px solid #a1a1aa",
          flexShrink: 0,
        }}
      />
    );
  }

  const { color, label } = CONFIG[status];
  return (
    <span
      title={label}
      style={{
        display: "inline-block",
        width: size,
        height: size,
        borderRadius: "50%",
        background: color,
        flexShrink: 0,
      }}
    />
  );
}
