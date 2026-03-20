import type { Job } from "../types";

export function googleCalendarEventLink(job: Job): string | null {
  if (!job.deadline) return null;
  const date = job.deadline.replaceAll("-", "");
  const title = encodeURIComponent(`Application Deadline: ${job.company}`);
  const details = encodeURIComponent(job.title ?? "Job application");
  return `https://calendar.google.com/calendar/render?action=TEMPLATE&text=${title}&dates=${date}/${date}&details=${details}`;
}
