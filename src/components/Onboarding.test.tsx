import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as api from "../api";
import { Onboarding } from "./Onboarding";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    completeOnboarding: vi.fn(),
  };
});

describe("Onboarding", () => {
  beforeEach(() => {
    vi.mocked(api.completeOnboarding).mockReset();
  });

  it("completes onboarding from the primary CTA", async () => {
    const onComplete = vi.fn();
    vi.mocked(api.completeOnboarding).mockResolvedValue({
      timezoneId: "America/New_York",
      desiredRetention: 0.9,
      onboardingCompleted: true,
      pairingCode: "ABCD1234",
      updatedAt: 1,
    });

    render(<Onboarding pairingCode="ABCD1234" onComplete={onComplete} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Complete onboarding" }),
    );

    await waitFor(() => {
      expect(api.completeOnboarding).toHaveBeenCalled();
      expect(onComplete).toHaveBeenCalledWith(
        expect.objectContaining({ onboardingCompleted: true }),
      );
    });
  });
});
