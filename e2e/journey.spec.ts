import { expect, test } from "@playwright/test";

import {
  getInvokeCalls,
  injectAcceptedSubmission,
  installTauriInvokeMock,
} from "./fixtures/mock-invoke";

test.describe("Ankicode product journey", () => {
  test("add → assignment → accepted → rating → reschedule", async ({
    page,
  }) => {
    await installTauriInvokeMock(page);
    await page.goto("/");

    await expect(page.getByRole("heading", { name: "Ankicode" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
    await expect(
      page.getByText("No problems assigned for today."),
    ).toBeVisible();

    await page.getByRole("button", { name: "My List" }).click();
    await expect(page.getByRole("heading", { name: "My List" })).toBeVisible();

    await page
      .getByPlaceholder("https://leetcode.com/problems/two-sum/")
      .fill("https://leetcode.com/problems/two-sum/");
    await page.getByPlaceholder("two sum").fill("Two Sum");
    await page.locator(".add-form select").selectOption("easy");
    await page.getByRole("button", { name: "Add problem" }).click();

    await expect(
      page.locator(".problem-row").getByText("Two Sum"),
    ).toBeVisible();
    await expect(page.locator(".pill.difficulty-easy")).toBeVisible();

    await page.getByRole("button", { name: "Today" }).click();
    await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Two Sum" })).toBeVisible();
    await expect(page.getByText("2026-07-19")).toBeVisible();

    await injectAcceptedSubmission(page);
    await page.getByRole("button", { name: "My List" }).click();
    await page.getByRole("button", { name: /Today/ }).click();

    const ratingDialog = page.getByRole("dialog");
    await expect(ratingDialog).toBeVisible();
    await expect(
      ratingDialog.getByRole("heading", { name: "Two Sum" }),
    ).toBeVisible();
    await expect(page.getByLabel("pending ratings")).toHaveText("1");

    await ratingDialog.getByRole("button", { name: /medium/i }).click();

    await expect(ratingDialog.getByText(/Next review:/i)).toBeVisible();
    await ratingDialog.getByRole("button", { name: "Done" }).click();
    await expect(ratingDialog).toHaveCount(0);
    await expect(page.getByText("All done for today.")).toBeVisible();
    await expect(page.getByText("1 day")).toBeVisible();
    await expect(page.getByLabel("pending ratings")).toHaveCount(0);

    const calls = await getInvokeCalls(page);
    expect(calls.some((call) => call.cmd === "add_problem_from_url")).toBe(
      true,
    );
    expect(calls.some((call) => call.cmd === "get_today")).toBe(true);
    expect(calls.some((call) => call.cmd === "list_pending_completions")).toBe(
      true,
    );

    const ratingCalls = calls.filter((call) => call.cmd === "record_rating");
    expect(ratingCalls).toHaveLength(1);
    expect(ratingCalls[0]?.args).toMatchObject({
      args: {
        problemId: 1,
        rating: "medium",
      },
    });
  });
});
