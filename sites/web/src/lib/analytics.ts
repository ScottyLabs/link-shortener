import { env } from "$env/dynamic/public";
import posthog from "posthog-js";

const key = env.PUBLIC_POSTHOG_KEY;

export function initAnalytics(): void {
  if (key === undefined) return;
  posthog.init(key, {
    api_host: env.PUBLIC_POSTHOG_HOST,
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
