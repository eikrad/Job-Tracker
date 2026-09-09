/** Tiered fingerprint helpers (spec §5.1) — shared fixture with Python + Rust. */

const TRACKING = new Set([
  "utm_source",
  "utm_medium",
  "utm_campaign",
  "utm_term",
  "utm_content",
  "gclid",
  "fbclid",
  "from",
  "vjk",
  "trk",
  "refid",
  "refId",
]);

const DIACRITICS: Record<string, string> = {
  ø: "o",
  Ø: "o",
  å: "a",
  Å: "a",
  ä: "a",
  Ä: "a",
  ö: "o",
  Ö: "o",
  ü: "u",
  Ü: "u",
  æ: "ae",
  Æ: "ae",
  ß: "ss",
};

function stripDiacritics(value: string): string {
  let out = "";
  for (const ch of value) {
    out += DIACRITICS[ch] ?? ch;
  }
  return out.normalize("NFKD").replace(/\p{M}/gu, "");
}

export function normalizeText(value: string): string {
  let text = stripDiacritics(value).toLowerCase();
  text = text.replace(/\((?:m\/w\/d|m\/f\/d|m\/w|w\/m\/d|f\/m\/d)\)/gi, "");
  text = text.replace(/[\u2013-]\s*remote\b.*$/i, "");
  for (;;) {
    const stripped = text
      .replace(/(?:^|\s)(?:a\/s|aps|gmbh|ivs|ab|as|ltd|inc)\.?$/i, "")
      .replace(/[ .,]+$/g, "");
    if (stripped === text) break;
    text = stripped;
  }
  text = text.replace(/[^\w\s|]+/gu, " ").replace(/\s+/g, " ").trim();
  return text;
}

export function weakKey(company: string, title: string, location: string): string {
  return `${normalizeText(company)}|${normalizeText(title)}|${normalizeText(location)}`;
}

export function canonicalUrl(url: string): string {
  const raw = url.trim();
  const hashless = raw.split("#")[0] ?? raw;
  const [schemeHostPath, query = ""] = hashless.split("?");
  let scheme = "https";
  let rest = schemeHostPath;
  const schemeIdx = schemeHostPath.indexOf("://");
  if (schemeIdx !== -1) {
    scheme = schemeHostPath.slice(0, schemeIdx).toLowerCase();
    rest = schemeHostPath.slice(schemeIdx + 3);
  }
  const slash = rest.indexOf("/");
  let hostPart = slash === -1 ? rest : rest.slice(0, slash);
  let path = slash === -1 ? "" : rest.slice(slash);
  if (hostPart.includes("@")) hostPart = hostPart.split("@").pop() ?? hostPart;
  let host = hostPart.toLowerCase();
  let port: string | null = null;
  if (!host.startsWith("[") && host.includes(":")) {
    const [h, p] = host.split(":");
    if (p && /^\d+$/.test(p)) {
      host = h ?? host;
      port = p;
    }
  }
  if (host.startsWith("www.")) host = host.slice(4);
  if (path.length > 1 && path.endsWith("/")) path = path.slice(0, -1);

  const pairs = (query ? query.split("&") : [])
    .filter(Boolean)
    .map((part) => {
      const eq = part.indexOf("=");
      return eq === -1 ? [part, ""] as const : [part.slice(0, eq), part.slice(eq + 1)] as const;
    });

  if (host.includes("indeed.") && path.replace(/\/$/, "").endsWith("/rc/clk")) {
    const jk = pairs.find(([k, v]) => k.toLowerCase() === "jk" && v)?.[1];
    if (jk) return `https://${host}/viewjob?jk=${jk}`;
  }

  const kept = pairs
    .filter(([k]) => !TRACKING.has(k) && ![...TRACKING].some((t) => t.toLowerCase() === k.toLowerCase()))
    .map(([k, v]) => `${k}=${v}`);
  const netloc = port ? `${host}:${port}` : host;
  const base = `${scheme}://${netloc}${path}`;
  return kept.length ? `${base}?${kept.join("&")}` : base;
}

export function fingerprint(input: {
  company: string;
  title: string;
  location: string;
  url: string;
  board?: string | null;
  external_id?: string | null;
}): { strong: string | null; weak: string } {
  let strong: string | null = null;
  if (input.board && input.external_id) {
    strong = `${input.board}:${input.external_id}`;
  } else if (input.url.trim()) {
    strong = `url:${canonicalUrl(input.url)}`;
  }
  return {
    strong,
    weak: weakKey(input.company, input.title, input.location),
  };
}

export function clusterId(strong: string | null, weak: string): string {
  return strong && strong.length > 0 ? strong : `weak:${weak}`;
}
