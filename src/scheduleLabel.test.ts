import { describe, expect, it } from "vitest";

import { formatDueLabel } from "./scheduleLabel";

describe("formatDueLabel", () => {
  const now = Date.parse("2026-07-21T12:00:00Z");

  it("labels unscheduled cards as new", () => {
    expect(formatDueLabel(null, now)).toBe("new");
  });

  it("labels overdue cards as due now", () => {
    expect(formatDueLabel(Math.floor(now / 1000) - 60, now)).toBe("due now");
  });

  it("labels a one-day interval as due tomorrow", () => {
    expect(formatDueLabel(Math.floor(now / 1000) + 86_400, now)).toBe(
      "due tomorrow",
    );
  });
});
