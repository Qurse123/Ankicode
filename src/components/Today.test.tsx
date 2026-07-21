import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Today } from "./Today";

describe("Today", () => {
  it("shows an empty assignment state", () => {
    render(
      <Today
        today={{ localDate: "2024-06-01", items: [] }}
        loading={false}
        error={null}
        retentionTarget={0.91}
        streakDays={12}
        onStart={vi.fn()}
        onRate={vi.fn()}
      />,
    );

    expect(
      screen.getByText("No problems assigned for today."),
    ).toBeInTheDocument();
  });

  it("renders assigned items with start and rate actions", () => {
    const onStart = vi.fn();
    const onRate = vi.fn();
    const item = {
      problemId: 1,
      slug: "two-sum",
      title: "two sum",
      url: "https://leetcode.com/problems/two-sum/",
      difficulty: "easy" as const,
      cost: 1,
      position: 0,
    };

    render(
      <Today
        today={{ localDate: "2024-06-01", items: [item] }}
        loading={false}
        error={null}
        retentionTarget={0.91}
        streakDays={12}
        onStart={onStart}
        onRate={onRate}
      />,
    );

    expect(screen.getByText("two sum")).toBeInTheDocument();
    expect(screen.getByText("cost 1")).toBeInTheDocument();
    expect(screen.getByText("Due today")).toBeInTheDocument();
    expect(screen.getByText("12 days")).toBeInTheDocument();
    expect(screen.getByText("91%")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Start" }));
    fireEvent.click(screen.getByRole("button", { name: "Rate" }));
    expect(onStart).toHaveBeenCalledWith(item);
    expect(onRate).toHaveBeenCalledWith(item);
  });
});
