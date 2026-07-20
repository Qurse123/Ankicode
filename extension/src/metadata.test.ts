/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";

import {
  extractProblemMetadata,
  pageShowsAccepted,
  parseDifficulty,
} from "./metadata";

describe("metadata parse", () => {
  it("parses difficulty labels", () => {
    expect(parseDifficulty("Easy")).toBe("Easy");
    expect(parseDifficulty("hard mode")).toBe("Hard");
    expect(parseDifficulty("none")).toBeNull();
  });

  it("extracts slug title and difficulty from fixture HTML", () => {
    document.documentElement.innerHTML = `
      <head>
        <meta property="og:url" content="https://leetcode.com/problems/two-sum/" />
        <meta property="og:title" content="1. Two Sum - LeetCode" />
      </head>
      <body>
        <div data-cy="question-title">1. Two Sum</div>
        <div class="text-difficulty-easy">Easy</div>
      </body>
    `;
    Object.defineProperty(window, "location", {
      value: { pathname: "/problems/two-sum/" },
      writable: true,
    });

    const metadata = extractProblemMetadata(document);
    expect(metadata).toEqual({
      slug: "two-sum",
      title: "Two Sum",
      difficulty: "Easy",
      url: "https://leetcode.com/problems/two-sum/",
    });
  });
});

it("requires a submission result panel for Accepted", () => {
  document.body.innerHTML = `
      <div>This page mentions Accepted and Runtime in the description.</div>
    `;
  expect(pageShowsAccepted(document)).toBe(false);

  document.body.innerHTML = `
      <div data-e2e-locator="submission-result">
        <div>Accepted</div>
        <div>Runtime: 12 ms</div>
      </div>
    `;
  expect(pageShowsAccepted(document)).toBe(true);
});
