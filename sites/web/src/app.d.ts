// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare namespace App {
  // interface Error {}
  // interface Locals {}
  // interface PageData {}
  // interface PageState {}
  // interface Platform {}
}

interface ImportMetaEnv {
  readonly POSTHOG_KEY?: string;
  readonly POSTHOG_HOST?: string;
}
