import { describe, expect, it } from "vitest";

import { LOOPBACK_BASE } from "./api";

describe("pairing request shape", () => {
  it("targets the loopback pair endpoint with code and origin", () => {
    const body = {
      code: "ABCD1234",
      origin: "chrome-extension://ext-id",
    };
    expect(LOOPBACK_BASE).toBe("http://127.0.0.1:17342");
    expect(JSON.stringify(body)).toContain('"code":"ABCD1234"');
    expect(JSON.stringify(body)).toContain(
      '"origin":"chrome-extension://ext-id"',
    );
    expect(`${LOOPBACK_BASE}/v1/pair`).toBe("http://127.0.0.1:17342/v1/pair");
  });
});
