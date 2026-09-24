/**
 * The title blocklist is edited as one piece of text (commas or new lines between
 * entries) and stored as a list. Matching itself happens in the backend.
 */

export function parseBlocklist(raw: string): string[] {
  const seen = new Set<string>();
  const entries: string[] = [];
  for (const part of raw.split(/[\n,;]/)) {
    const entry = part.trim().replace(/\s+/g, " ");
    const key = entry.toLowerCase();
    if (!entry || seen.has(key)) continue;
    seen.add(key);
    entries.push(entry);
  }
  return entries;
}

export function formatBlocklist(entries: string[]): string {
  return entries.join(", ");
}
