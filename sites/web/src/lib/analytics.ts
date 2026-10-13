import posthog from "posthog-js";

const key = import.meta.env.POSTHOG_KEY;

export function initAnalytics(): void {
  if (key === undefined) return;
  posthog.init(key, {
    api_host: import.meta.env.POSTHOG_HOST,
    ui_host: "https://us.posthog.com",
    defaults: "2025-05-24",
  });
}

export function identify(subject: string, name: string): void {
  if (key === undefined) return;
  posthog.identify(subject, { name });
}

export function resetAnalytics(): void {
  if (key === undefined) return;
  posthog.reset();
}
