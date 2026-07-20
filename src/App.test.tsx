import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import * as api from "./api";

vi.mock("./api", async () => {
  const actual = await vi.importActual<typeof import("./api")>("./api");
  return {
    ...actual,
    getBootstrap: vi.fn(),
    getToday: vi.fn(),
    listProblemsView: vi.fn(),
    completeOnboarding: vi.fn(),
  };
});

describe("App", () => {
  beforeEach(() => {
    vi.mocked(api.getBootstrap).mockReset();
    vi.mocked(api.getToday).mockReset();
    vi.mocked(api.listProblemsView).mockReset();
  });

  it("shows onboarding when the local profile is incomplete", async () => {
    vi.mocked(api.getBootstrap).mockResolvedValue({
      settings: {
        timezoneId: "America/New_York",
        desiredRetention: 0.9,
        onboardingCompleted: false,
        pairingCode: "WXYZ9876",
        updatedAt: 1,
      },
    });

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Ankicode" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Complete onboarding" }),
    ).toBeInTheDocument();
    expect(screen.getByText("WXYZ9876")).toBeInTheDocument();
  });

  it("shows the today shell after onboarding", async () => {
    vi.mocked(api.getBootstrap).mockResolvedValue({
      settings: {
        timezoneId: "America/New_York",
        desiredRetention: 0.9,
        onboardingCompleted: true,
        pairingCode: "WXYZ9876",
        updatedAt: 1,
      },
    });
    vi.mocked(api.getToday).mockResolvedValue({
      localDate: "2024-06-01",
      items: [],
    });

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Today" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Ankicode" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("No problems assigned for today."),
    ).toBeInTheDocument();
  });
});
