import {
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";
import { build, mergeConfig } from "vite";

import contentConfig from "../vite.content.config";
import manifest from "../public/manifest.json";

describe("extension manifest", () => {
  it("wires the MV3 background and content bundles", () => {
    expect(manifest.manifest_version).toBe(3);
    expect(manifest.background.service_worker).toBe("background.js");
    expect(manifest.content_scripts[0]?.js).toContain("content.js");
    expect(manifest.permissions).toEqual(
      expect.arrayContaining(["storage", "alarms"]),
    );
    expect(manifest.host_permissions).toContain("http://127.0.0.1:17342/*");
    expect(manifest.action.default_popup).toBe("popup.html");
  });

  it("builds content scripts as one classic-script artifact", async () => {
    const workspace = await mkdtemp(join(tmpdir(), "ankicode-extension-"));

    try {
      const fixtures = join(workspace, "fixtures");
      const outDir = join(workspace, "dist");
      await mkdir(fixtures);
      await writeFile(
        join(fixtures, "shared.ts"),
        'export const marker = "shared-marker";',
      );
      await writeFile(
        join(fixtures, "content.ts"),
        [
          'import { marker } from "./shared";',
          "document.documentElement.dataset.ankicodeBuild = marker;",
        ].join("\n"),
      );

      await build(
        mergeConfig(contentConfig, {
          publicDir: false,
          build: {
            outDir,
            lib: {
              entry: join(fixtures, "content.ts"),
              fileName: () => "content.js",
              formats: ["iife"],
              name: "AnkicodeContentArtifactTest",
            },
          },
        }),
      );

      const files = await readdir(outDir);
      const artifact = await readFile(join(outDir, "content.js"), "utf8");

      expect(files).toEqual(["content.js"]);
      expect(artifact).toContain("shared-marker");
      expect(artifact).not.toMatch(/^\s*(?:import|export)\s/m);
    } finally {
      await rm(workspace, { force: true, recursive: true });
    }
  });

  it("emits the real content entrypoint as a non-empty classic script", async () => {
    const workspace = await mkdtemp(join(tmpdir(), "ankicode-content-"));
    const outDir = join(workspace, "dist");

    try {
      await build(
        mergeConfig(contentConfig, {
          build: {
            outDir,
          },
        }),
      );

      const artifact = await readFile(join(outDir, "content.js"), "utf8");

      expect(artifact).toContain("ankicode-content-ready");
      expect(artifact).not.toMatch(/^\s*(?:import|export)\s/m);
    } finally {
      await rm(workspace, { force: true, recursive: true });
    }
  });
});
