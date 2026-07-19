import { describe, expect, it } from "vitest";

import manifest from "../public/manifest.json";

describe("extension manifest", () => {
  it("wires the MV3 background and content bundles", () => {
    expect(manifest.manifest_version).toBe(3);
    expect(manifest.background.service_worker).toBe("background.js");
    expect(manifest.content_scripts[0]?.js).toContain("content.js");
  });
});
