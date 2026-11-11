const AP_MONTHS = [
  "Jan.",
  "Feb.",
  "March",
  "April",
  "May",
  "June",
  "July",
  "Aug.",
  "Sept.",
  "Oct.",
  "Nov.",
  "Dec.",
];

export function apMonthDay(date: Date): string {
  return `${AP_MONTHS[date.getMonth()]} ${date.getDate()}`;
}

export function apDate(date: Date): string {
  return `${apMonthDay(date)}, ${date.getFullYear()}`;
}

export function shortUrl(slug: string): string {
  return `${location.origin}/${slug}`;
}

export function shortLabel(slug: string): string {
  return `${location.host}/${slug}`;
}

export function errorMessage(error: unknown, fallback: string): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    typeof error.error === "string"
  ) {
    return error.error;
  }
  return fallback;
}
