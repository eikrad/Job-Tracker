import { describe, expect, it } from "vitest";
import { DEFAULT_STATUSES } from "./types";
import { effectiveStatuses } from "./statusUtils";

describe("effectiveStatuses", () => {
  it("uses defaults when empty", () => {
    expect(effectiveStatuses([])).toEqual(DEFAULT_STATUSES);
  });

  it("uses custom list when non-empty", () => {
    expect(effectiveStatuses(["X", "Y"])).toEqual(["X", "Y"]);
  });
});
