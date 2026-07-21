import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";

import { recordRating } from "../api";
import { RatingDialog } from "./RatingDialog";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("RatingDialog", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("rating buttons call invoke through recordRating", async () => {
    vi.mocked(invoke).mockResolvedValue({
      stability: 1,
      difficulty: 1,
      due_at: 2,
      last_review_at: 1,
    });

    render(
      <RatingDialog
        title="two sum"
        onClose={vi.fn()}
        onRate={async (rating) => {
          await recordRating({
            problemId: 7,
            rating,
            idempotencyKey: "test-key",
          });
        }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /medium/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("record_rating", {
        args: {
          problemId: 7,
          rating: "medium",
          idempotencyKey: "test-key",
        },
      });
    });
  });

  it("surfaces rating failures inside the dialog", () => {
    render(
      <RatingDialog
        title="two sum"
        error="could not record rating"
        onClose={vi.fn()}
        onRate={vi.fn()}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "could not record rating",
    );
    expect(screen.getByRole("dialog")).toHaveAttribute("aria-modal", "true");
  });
});
