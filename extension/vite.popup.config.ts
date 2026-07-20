import { resolve } from "node:path";

import { defineConfig } from "vite";

export default defineConfig({
  publicDir: false,
  build: {
    emptyOutDir: false,
    rollupOptions: {
      input: {
        popup: resolve(import.meta.dirname, "src/popup.ts"),
      },
      output: {
        entryFileNames: "popup.js",
      },
    },
  },
});
