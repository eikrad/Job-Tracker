import type { Job } from "../types";

/** Google Calendar “create event” URL for an all-day date (YYYY-MM-DD). */
export function googleCalendarTemplateUrl(params: {
  date: string;
  summaryLine: string;
  details?: string | null;
}): string {
  const date = params.date.replaceAll("-", "");
  const text = encodeURIComponent(params.summaryLine);
  const details = encodeURIComponent(params.details ?? "Job Tracker");
  return `https://calendar.google.com/calendar/render?action=TEMPLATE&text=${text}&dates=${date}/${date}&details=${details}`;
}

export function googleCalendarEventLink(job: Job): string | null {
  if (!job.deadline) return null;
  return googleCalendarTemplateUrl({
    date: job.deadline,
    summaryLine: `Application Deadline: ${job.company}`,
    details: job.title ?? "Job application",
  });
}
