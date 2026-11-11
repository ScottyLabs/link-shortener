import type { IncomingMessage, ServerResponse } from "node:http";
import { defineConfig, type Plugin } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const BACKEND = "http://localhost:3000";
const SLUG_PATH = /^\/[^/?]+(\?|$)/u;

async function forwardSlug(
  backend: string,
  req: IncomingMessage,
  res: ServerResponse,
  next: (error?: unknown) => void,
) {
  const response = await fetch(new URL(req.url ?? "/", backend), {
    headers: { "user-agent": req.headers["user-agent"] ?? "" },
    redirect: "manual",
  });
  if (response.status === 404) {
    next();
    return;
  }

  res.writeHead(response.status, Object.fromEntries(response.headers));
  res.end(Buffer.from(await response.arrayBuffer()));
}

/** Sends slug redirects through the backend and leaves missing slugs to the app, as the backend does in production. */
export function slugRedirects(backend: string): Plugin {
  return {
    name: "slug-redirects",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (SLUG_PATH.test(req.url ?? "")) {
          forwardSlug(backend, req, res, next).catch(next);
        } else {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [slugRedirects(BACKEND), sveltekit()],
  server: {
    proxy: {
      "/api": BACKEND,
      "/auth": BACKEND,
    },
  },
});
