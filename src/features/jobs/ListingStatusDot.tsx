import type { Job } from "../../lib/types";

type Status = Job["listing_status"];

const STATUS_CLASS: Record<NonNullable<Status>, string> = {
  active: "listingDot listingDotActive",
  closed: "listingDot listingDotClosed",
  archived: "listingDot listingDotArchived",
  unreachable: "listingDot listingDotUnreachable",
};

const STATUS_LABEL: Record<NonNullable<Status>, string> = {
  active: "Listing active",
  closed: "Listing closed",
  archived: "Listing archived",
  unreachable: "Listing unreachable",
};

type Props = {
  status?: Status;
};

export function ListingStatusDot({ status }: Props) {
  if (!status) {
    return <span className="listingDot listingDotUnchecked" title="Listing not yet checked" />;
  }
  return <span className={STATUS_CLASS[status]} title={STATUS_LABEL[status]} />;
}
