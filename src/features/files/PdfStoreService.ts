export function buildStoredPdfName(company: string, title: string, originalName: string): string {
  const date = new Date().toISOString().slice(0, 10);
  const ext = originalName.toLowerCase().endsWith(".pdf") ? ".pdf" : "";
  const slug = `${company}_${title}`
    .replace(/[^a-zA-Z0-9_-]/g, "_")
    .replace(/_+/g, "_")
    .slice(0, 80);
  return `${date}_${slug}${ext || ".pdf"}`;
}
