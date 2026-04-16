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

describe("parseJobsImportJson – additional edge cases", () => {
  it("preserves interview_date, start_date, and notes", () => {
    const rows = parseJobsImportJson(
      JSON.stringify([
        {
          company: "Corp",
          status: "Feedback",
          interview_date: "2026-05-20",
          start_date: "2026-06-01",
          notes: "Looked promising",
        },
      ]),
    );
    expect(rows[0].interview_date).toBe("2026-05-20");
    expect(rows[0].start_date).toBe("2026-06-01");
    expect(rows[0].notes).toBe("Looked promising");
  });

  it("treats whitespace-only company as Unknown", () => {
    const rows = parseJobsImportJson(JSON.stringify([{ company: "   ", status: "Done" }]));
    expect(rows[0].company).toBe("Unknown");
  });

  it("maps null optional fields to undefined, not the string 'null'", () => {
    const rows = parseJobsImportJson(
      JSON.stringify([{ company: "X", status: "Interesting", url: null, title: null }]),
    );
    expect(rows[0].url).toBeUndefined();
    expect(rows[0].title).toBeUndefined();
  });
});

describe("parseJobsImportCsv – additional edge cases", () => {
  it("unescapes doubled double-quotes inside fields", () => {
    // CSV: "say ""hello""" → field value: say "hello"
    const csv = [
      "id,company,title,status,deadline,interview_date,start_date,url,tags,detected_language",
      '"1","Firm","say ""hello""","Interesting","","","","","",""',
    ].join("\n");
    const rows = parseJobsImportCsv(csv);
    expect(rows[0].title).toBe('say "hello"');
  });

  it("handles CRLF line endings", () => {
    const csv =
      "id,company,title,status,deadline,interview_date,start_date,url,tags,detected_language\r\n" +
      '"1","CRLF Corp","Dev","Interesting","","","","","",""\r\n';
    const rows = parseJobsImportCsv(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0].company).toBe("CRLF Corp");
  });

  it("maps interview_date and start_date columns", () => {
    const csv = [
      "id,company,title,status,deadline,interview_date,start_date,url,tags,detected_language",
      '"1","Acme","Dev","Interesting","2026-05-01","2026-05-20","2026-06-01","","",""',
    ].join("\n");
    const rows = parseJobsImportCsv(csv);
    expect(rows[0].deadline).toBe("2026-05-01");
    expect(rows[0].interview_date).toBe("2026-05-20");
    expect(rows[0].start_date).toBe("2026-06-01");
  });
});
