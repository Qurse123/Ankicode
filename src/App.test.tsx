import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

describe("App", () => {
  it("identifies the Ankicode desktop shell", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Ankicode" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Your local coding review queue"),
    ).toBeInTheDocument();
  });
});
