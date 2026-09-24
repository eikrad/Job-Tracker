import { describe, expect, it } from "vitest";
import { formatBlocklist, parseBlocklist } from "./titleBlocklist";

describe("parseBlocklist", () => {
  it("splits on commas, semicolons and new lines", () => {
    expect(parseBlocklist("lead, head of\nmanager; principal")).toEqual([
      "lead",
      "head of",
      "manager",
      "principal",
    ]);
  });

  it("trims, collapses spaces, drops blanks and duplicates ignoring case", () => {
    expect(parseBlocklist("  Lead ,, head   of , LEAD ,\n\n")).toEqual(["Lead", "head of"]);
  });

  it("round-trips through the display text", () => {
    const list = ["lead", "head of", "*leder"];
    expect(parseBlocklist(formatBlocklist(list))).toEqual(list);
  });

  it("is empty for blank text", () => {
    expect(parseBlocklist("  \n , ")).toEqual([]);
  });
});
