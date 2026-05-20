import { describe, expect, it } from "vitest";
import { DEFAULT_STATUSES } from "./types";
import { effectiveStatuses, isNextStatus, migrateStatusesV2 } from "./statusUtils";

describe("effectiveStatuses", () => {
  it("uses defaults when empty", () => {
    expect(effectiveStatuses([])).toEqual(DEFAULT_STATUSES);
  });

  it("uses custom list when non-empty", () => {
    expect(effectiveStatuses(["X", "Y"])).toEqual(["X", "Y"]);
  });
});

it("DEFAULT_STATUSES contains 'Plan to Apply' at index 1", () => {
  expect(DEFAULT_STATUSES[1]).toBe("Plan to Apply");
  expect(DEFAULT_STATUSES).toEqual([
    "Interesting",
    "Plan to Apply",
    "Application Sent",
    "Feedback",
    "Done",
  ]);
});

describe("migrateStatusesV2", () => {
  it("splices 'Plan to Apply' between Interesting and Application Sent", () => {
    const old = ["Interesting", "Application Sent", "Feedback", "Done"];
    expect(migrateStatusesV2(old)).toEqual([
      "Interesting",
      "Plan to Apply",
      "Application Sent",
      "Feedback",
      "Done",
    ]);
  });

  it("does not modify list if 'Plan to Apply' already present", () => {
    const current = ["Interesting", "Plan to Apply", "Application Sent", "Feedback", "Done"];
    expect(migrateStatusesV2(current)).toBe(current);
  });

  it("does not modify list if 'Interesting' has been renamed", () => {
    const custom = ["Candidates", "Application Sent", "Feedback", "Done"];
    expect(migrateStatusesV2(custom)).toEqual(custom);
  });
});

describe("isNextStatus", () => {
  const lanes = ["Interesting", "Plan to Apply", "Application Sent", "Done"];

  it("returns true for the immediate next lane", () => {
    expect(isNextStatus(lanes, "Interesting", "Plan to Apply")).toBe(true);
    expect(isNextStatus(lanes, "Plan to Apply", "Application Sent")).toBe(true);
  });

  it("returns false for non-next targets", () => {
    expect(isNextStatus(lanes, "Interesting", "Application Sent")).toBe(false);
    expect(isNextStatus(lanes, "Interesting", "Done")).toBe(false);
  });

  it("returns false when status not found in lanes", () => {
    expect(isNextStatus(lanes, "Unknown", "Plan to Apply")).toBe(false);
  });

  it("returns false for the last status (no next)", () => {
    expect(isNextStatus(lanes, "Done", "")).toBe(false);
  });
});
