import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { ProblemListItem } from "../types";
import { MyList } from "./MyList";

const problems: ProblemListItem[] = [
  {
    id: 1,
    slug: "two-sum",
    title: "two sum",
    url: "https://leetcode.com/problems/two-sum/",
    difficulty: "easy",
    status: "active",
    addedAt: 1,
    updatedAt: 1,
    dueAt: null,
  },
  {
    id: 2,
    slug: "median-of-two",
    title: "median of two",
    url: "https://leetcode.com/problems/median-of-two/",
    difficulty: "hard",
    status: "paused",
    addedAt: 2,
    updatedAt: 2,
    dueAt: 100,
  },
];

describe("MyList", () => {
  it("filters by difficulty", () => {
    render(
      <MyList
        problems={problems}
        loading={false}
        error={null}
        onAdd={vi.fn()}
        onOpen={vi.fn()}
        onStatus={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.getByText("two sum")).toBeInTheDocument();
    expect(screen.getByText("median of two")).toBeInTheDocument();

    const difficultySelect = screen.getAllByRole("combobox")[1];
    fireEvent.change(difficultySelect, { target: { value: "hard" } });

    expect(screen.queryByText("two sum")).not.toBeInTheDocument();
    expect(screen.getByText("median of two")).toBeInTheDocument();
  });

  it("calls onDelete after in-app confirm", async () => {
    const onDelete = vi.fn().mockResolvedValue(undefined);
    render(
      <MyList
        problems={problems}
        loading={false}
        error={null}
        onAdd={vi.fn()}
        onOpen={vi.fn()}
        onStatus={vi.fn()}
        onDelete={onDelete}
      />,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Delete" })[0]);
    expect(screen.getByRole("dialog", { name: "two sum" })).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Delete permanently" }),
    );
    expect(onDelete).toHaveBeenCalledWith(1);
  });
});
