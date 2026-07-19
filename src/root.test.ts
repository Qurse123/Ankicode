import { describe, expect, it } from "vitest";

import { requireRootElement } from "./root";

describe("requireRootElement", () => {
  it("throws a clear error when the React root is missing", () => {
    expect(() => requireRootElement(document)).toThrow(
      'Missing required element "#root".',
    );
  });
});
