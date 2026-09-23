Find every job listing in this job-alert email.

A listing is one open position the email advertises: its job title, the hiring
company, its location, and the link id of the reference that opens that position.
Skip everything that is not an open position: unsubscribe and settings links, the
sender's homepage, app-store badges, "see all jobs" and search links, newsletters,
course or event adverts, and marketing. An email with no open positions is a valid
answer: return an empty `listings` array.

For each listing return `title`, `company` (empty if not stated), `location` (empty
if not stated), `link_id` (one of the ids below), and `snippet`: at most two
sentences from the email describing that position, or empty. Copy names as the email
writes them. List each position once.

Subject: {{SUBJECT}}
Sender: {{SENDER}}

Link ids you may use, with the site each one leads to:
{{LINKS}}

<<<DIGEST
{{BODY}}
>>>
