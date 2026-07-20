export type ProblemMetadata = {
  slug: string;
  title: string;
  difficulty: "Easy" | "Medium" | "Hard";
  url: string;
};

const DIFFICULTY_RE = /\b(Easy|Medium|Hard)\b/i;

export function slugFromPathname(pathname: string): string | null {
  const match = pathname.match(/\/problems\/([^/?#]+)/i);
  if (!match?.[1]) {
    return null;
  }
  return match[1].toLowerCase();
}

export function canonicalUrl(slug: string): string {
  return `https://leetcode.com/problems/${slug}/`;
}

export function titleFromSlug(slug: string): string {
  return slug.replace(/-/g, " ");
}

export function parseDifficulty(
  text: string | null | undefined,
): ProblemMetadata["difficulty"] | null {
  if (!text) {
    return null;
  }
  const match = text.match(DIFFICULTY_RE);
  if (!match?.[1]) {
    return null;
  }
  const value = match[1].toLowerCase();
  if (value === "easy") {
    return "Easy";
  }
  if (value === "medium") {
    return "Medium";
  }
  if (value === "hard") {
    return "Hard";
  }
  return null;
}

export function extractProblemMetadata(doc: Document): ProblemMetadata | null {
  const slug =
    slugFromPathname(doc.defaultView?.location.pathname ?? "") ??
    slugFromPathname(
      doc.querySelector('meta[property="og:url"]')?.getAttribute("content") ??
        "",
    );
  if (!slug) {
    return null;
  }

  const ogTitle = doc
    .querySelector('meta[property="og:title"]')
    ?.getAttribute("content")
    ?.trim();
  const heading =
    doc.querySelector('[data-cy="question-title"]')?.textContent?.trim() ||
    doc.querySelector("div[class*='text-title']")?.textContent?.trim() ||
    doc.querySelector("h1")?.textContent?.trim();
  let title = heading || ogTitle || titleFromSlug(slug);
  title = title.replace(/\s*-\s*LeetCode\s*$/i, "").trim();
  const numbered = title.match(/^\d+\.\s*(.+)$/);
  if (numbered?.[1]) {
    title = numbered[1].trim();
  }

  const difficultyNode =
    doc.querySelector("[diff]") ||
    doc.querySelector('[class*="text-difficulty"]') ||
    doc.querySelector('[class*="difficulty"]') ||
    doc.body;
  const difficulty =
    parseDifficulty(difficultyNode?.textContent) ||
    parseDifficulty(doc.body?.innerText?.slice(0, 4000)) ||
    "Medium";

  return {
    slug,
    title: title || titleFromSlug(slug),
    difficulty,
    url: canonicalUrl(slug),
  };
}

/** True only when a submission result panel reports Accepted. */
export function pageShowsAccepted(root: ParentNode): boolean {
  const result =
    root.querySelector?.(
      '[data-e2e-locator="submission-result"], [data-e2e-locator*="submission-result"], [class*="submission-result"], [class*="result-data-container"]',
    ) ?? null;
  if (!result) {
    return false;
  }
  const text = result.textContent ?? "";
  return (
    /\bAccepted\b/i.test(text) &&
    (/\bRuntime\b/i.test(text) || /\b\d+\s*ms\b/i.test(text))
  );
}
