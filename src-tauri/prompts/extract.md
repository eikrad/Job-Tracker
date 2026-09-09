Extract structured job information from the text below.
Input can be Danish, German, or English.
Return strict JSON with keys:
company, title, url, deadline, interview_date, start_date, tags, detected_language, notes,
contact_name, contact_email, contact_phone,
workplace_street, workplace_city, workplace_postal_code,
work_mode (one of: Remote, Hybrid, On-site, or omit if unknown),
salary_range (free text as stated in the ad, or omit if not mentioned),
contract_type (one of: Permanent, Fixed-term, Freelance, Internship, or omit if unknown),
reference_number (job reference/ID if mentioned, else omit),
source (where the job was posted if mentioned, else omit)
Dates as YYYY-MM-DD when known.
Use English field values when possible for normalized output.
Do not include a priority field.

Text:
{{RAW_TEXT}}
