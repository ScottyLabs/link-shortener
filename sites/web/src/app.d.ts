// See https://svelte.dev/docs/kit/types#app.d.ts for information about these interfaces
declare namespace App {
  // interface Error {} interface Locals {} interface PageData {} interface Platform {}
  interface PageState {
    flash?: { kind: "success" | "error"; text: string };
  }
}
