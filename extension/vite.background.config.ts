import { resolve } from "node:path";

import { defineConfig } from "vite";

export default defineConfig({
  build: {
    emptyOutDir: true,
    rollupOptions: {
      input: {
        background: resolve(import.meta.dirname, "src/background.ts"),
      },
      output: {
        chunkFileNames: "chunks/[name]-[hash].js",
        entryFileNames: "background.js",
      },
    },
  },
});
