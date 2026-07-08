// Pure SPA build. The UI loads the user and links entirely on the client, so
// SSR has nothing to do. The static adapter writes one `index.html` fallback
// that bootstraps the SvelteKit runtime; routing happens client-side.
export const ssr = false;
export const prerender = false;
