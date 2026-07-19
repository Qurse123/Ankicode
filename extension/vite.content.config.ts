import { resolve } from "node:path";

import { defineConfig } from "vite";

export default defineConfig({
  publicDir: false,
  build: {
    emptyOutDir: false,
    lib: {
      entry: resolve(import.meta.dirname, "src/content.ts"),
      fileName: () => "content.js",
      formats: ["iife"],
      name: "AnkicodeContent",
    },
  },
});
