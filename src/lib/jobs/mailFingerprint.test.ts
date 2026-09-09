import { describe, expect, it } from "vitest";
import fixture from "../../../tests/fixtures/fingerprints.json" with { type: "json" };
import { clusterId, fingerprint } from "./mailFingerprint";

type Case = {
  id: string;
  input: {
    company: string;
    title: string;
    location: string;
    url: string;
    board: string | null;
    external_id: string | null;
  };
  expected_strong: string | null;
  expected_weak: string;
  expected_cluster: string;
};

const cases = (fixture as { cases: Case[] }).cases;

describe("mail fingerprint fixture (cross-language)", () => {
  it.each(cases)("$id", (c) => {
    const got = fingerprint({
      company: c.input.company,
      title: c.input.title,
      location: c.input.location,
      url: c.input.url,
      board: c.input.board,
      external_id: c.input.external_id,
    });
    expect(got.weak).toBe(c.expected_weak);
    expect(got.strong).toBe(c.expected_strong);
    expect(clusterId(got.strong, got.weak)).toBe(c.expected_cluster);
  });
});
