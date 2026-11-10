import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      "/api": "http://localhost:3000",
      "/auth": "http://localhost:3000",
      // forward slug redirects to the backend
      "^/[^/?]+(\\?|$)": "http://localhost:3000",
    },
  },
});
