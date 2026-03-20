import { describe, expect, it } from "vitest";
import {
  NEW_JOB_KEYS,
  parseJobsImportCsv,
  parseJobsImportJson,
  validateExportJobShape,
} from "./exportBundle";

describe("parseJobsImportJson", () => {
  it("parses export-shaped array and maps to NewJob", () => {
    const json = JSON.stringify([
      {
        id: 99,
        company: "Acme",
        title: "Engineer",
        url: "https://x.com",
        raw_text: "body",
        status: "Interesting",
        deadline: "2026-06-01",
        tags: "a,b",
        detected_language: "da",
        notes: "n",
        pdf_path: null,
        created_at: "x",
        updated_at: "y",
      },
    ]);
    const rows = parseJobsImportJson(json);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      company: "Acme",
      title: "Engineer",
      url: "https://x.com",
      raw_text: "body",
      status: "Interesting",
      deadline: "2026-06-01",
      tags: "a,b",
      detected_language: "da",
      notes: "n",
    });
  });

  it("throws when not an array", () => {
    expect(() => parseJobsImportJson("{}")).toThrow(/array/i);
  });

  it("defaults status when missing", () => {
    const rows = parseJobsImportJson(JSON.stringify([{ company: "Solo" }]));
    expect(rows[0].status).toBe("Interesting");
  });

  it("uses Unknown company when empty", () => {
    const rows = parseJobsImportJson(JSON.stringify([{ company: "", status: "Done" }]));
    expect(rows[0].company).toBe("Unknown");
  });
});

describe("parseJobsImportCsv", () => {
  it("parses header + quoted row matching export columns", () => {
    const csv = [
      'id,company,title,status,deadline,url,tags,detected_language',
      '"1","Beta AS","Dev","Application Sent","2026-05-10","https://j.dev","t","en"',
    ].join("\n");
    const rows = parseJobsImportCsv(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      company: "Beta AS",
      title: "Dev",
      status: "Application Sent",
      deadline: "2026-05-10",
      url: "https://j.dev",
      tags: "t",
      detected_language: "en",
    });
  });

  it("returns empty for header-only or empty", () => {
    expect(parseJobsImportCsv("a,b")).toEqual([]);
    expect(parseJobsImportCsv("")).toEqual([]);
  });

  it("filters rows with empty company", () => {
    const csv = [
      'id,company,title,status,deadline,url,tags,detected_language',
      '"1","","X","Interesting","","","",""',
    ].join("\n");
    expect(parseJobsImportCsv(csv)).toEqual([]);
  });
});

describe("validateExportJobShape", () => {
  it("requires company and status", () => {
    expect(validateExportJobShape({ company: "A", status: "Interesting" })).toEqual([]);
    expect(validateExportJobShape({ company: "", status: "x" })).toContain("company");
    expect(validateExportJobShape({ company: "A", status: "" })).toContain("status");
    expect(validateExportJobShape({})).toEqual(expect.arrayContaining(["company", "status"]));
  });
});

describe("NEW_JOB_KEYS", () => {
  it("lists expected import field names", () => {
    expect(NEW_JOB_KEYS).toContain("company");
    expect(NEW_JOB_KEYS).toContain("status");
    expect(NEW_JOB_KEYS).toContain("deadline");
  });
});
