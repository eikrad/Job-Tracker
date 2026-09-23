/** `https://www.jobindex.dk/c?t=…` → `jobindex.dk`; the raw string if it is not a URL. */
export function boardName(boardUrl: string): string {
  try {
    return new URL(boardUrl).hostname.replace(/^www\./, "");
  } catch {
    return boardUrl;
  }
}
