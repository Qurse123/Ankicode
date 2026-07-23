import type { Page } from "@playwright/test";

export type InvokeCall = {
  cmd: string;
  args?: Record<string, unknown>;
};

declare global {
  interface Window {
    __ANKICODE_E2E__?: {
      calls: InvokeCall[];
      injectAcceptedSubmission: () => void;
    };
    __ANKICODE_E2E_INVOKE__?: (
      cmd: string,
      args?: Record<string, unknown>,
    ) => Promise<unknown>;
  }
}

/**
 * Install a stateful Tauri `invoke` mock before the React shell boots.
 * Covers the product journey: add → today → accepted → rate → reschedule.
 */
export async function installTauriInvokeMock(page: Page): Promise<void> {
  await page.addInitScript(() => {
    type Difficulty = "easy" | "medium" | "hard";
    type ProblemStatus = "active" | "paused" | "archived" | "removed";
    type Rating = "again" | "hard" | "medium" | "easy";

    type Settings = {
      timezoneId: string;
      desiredRetention: number;
      onboardingCompleted: boolean;
      pairingCode: string;
      updatedAt: number;
    };

    type ProblemListItem = {
      id: number;
      slug: string;
      title: string;
      url: string;
      difficulty: Difficulty;
      status: ProblemStatus;
      addedAt: number;
      updatedAt: number;
      dueAt: number | null;
    };

    type Problem = {
      id: number;
      slug: string;
      title: string;
      url: string;
      difficulty: Difficulty;
      status: ProblemStatus;
      added_at: number;
      updated_at: number;
    };

    type TodayItem = {
      problemId: number;
      slug: string;
      title: string;
      url: string;
      difficulty: Difficulty;
      cost: number;
      position: number;
      reviewedToday: boolean;
      lastRating: Rating | null;
      dueAt: number | null;
    };

    type PendingCompletion = {
      id: number;
      problemId: number;
      slug: string;
      title: string;
      difficulty: Difficulty;
      url: string;
      idempotencyKey: string;
      acceptedAt: number;
      createdAt: number;
    };

    const settings: Settings = {
      timezoneId: "America/New_York",
      desiredRetention: 0.9,
      onboardingCompleted: true,
      pairingCode: "TESTCODE",
      updatedAt: 1,
    };

    let nextProblemId = 1;
    let nextPendingId = 1;
    let problems: ProblemListItem[] = [];
    let todayItems: TodayItem[] = [];
    let pending: PendingCompletion[] = [];
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];

    function nowSeconds(): number {
      return Math.floor(Date.now() / 1000);
    }

    function slugFromUrl(url: string): string {
      const match = url.match(/\/problems\/([^/?#]+)/i);
      return match?.[1] ?? "untitled-problem";
    }

    function titleFromSlug(slug: string): string {
      return slug.replace(/-/g, " ");
    }

    window.__ANKICODE_E2E__ = {
      calls,
      injectAcceptedSubmission() {
        const today = todayItems[0];
        const problem = problems[0];
        const source = today
          ? {
              problemId: today.problemId,
              slug: today.slug,
              title: today.title,
              difficulty: today.difficulty,
              url: today.url,
            }
          : problem
            ? {
                problemId: problem.id,
                slug: problem.slug,
                title: problem.title,
                difficulty: problem.difficulty,
                url: problem.url,
              }
            : null;
        if (!source) {
          throw new Error("No problem available for accepted submission");
        }
        pending = [
          {
            id: nextPendingId++,
            problemId: source.problemId,
            slug: source.slug,
            title: source.title,
            difficulty: source.difficulty,
            url: source.url,
            idempotencyKey: `accepted-${source.problemId}`,
            acceptedAt: nowSeconds(),
            createdAt: nowSeconds(),
          },
        ];
      },
    };

    window.__ANKICODE_E2E_INVOKE__ = async (cmd, args) => {
      calls.push({ cmd, args });

      switch (cmd) {
        case "get_bootstrap":
          return { settings: { ...settings } };

        case "complete_onboarding": {
          const payload = (args?.args ?? {}) as {
            timezoneId?: string;
            desiredRetention?: number;
          };
          settings.onboardingCompleted = true;
          settings.timezoneId = payload.timezoneId ?? settings.timezoneId;
          settings.desiredRetention =
            payload.desiredRetention ?? settings.desiredRetention;
          settings.updatedAt = nowSeconds();
          return { ...settings };
        }

        case "list_problems_view":
          return problems.map((problem) => ({ ...problem }));

        case "add_problem_from_url": {
          const payload = (args?.args ?? {}) as {
            url: string;
            title?: string;
            difficulty: Difficulty;
          };
          const slug = slugFromUrl(payload.url);
          const title = payload.title?.trim() || titleFromSlug(slug);
          const timestamp = nowSeconds();
          const problem: Problem = {
            id: nextProblemId++,
            slug,
            title,
            url: payload.url,
            difficulty: payload.difficulty,
            status: "active",
            added_at: timestamp,
            updated_at: timestamp,
          };
          const listItem: ProblemListItem = {
            id: problem.id,
            slug: problem.slug,
            title: problem.title,
            url: problem.url,
            difficulty: problem.difficulty,
            status: problem.status,
            addedAt: timestamp,
            updatedAt: timestamp,
            dueAt: null,
          };
          problems = [...problems, listItem];
          todayItems = [
            {
              problemId: problem.id,
              slug: problem.slug,
              title: problem.title,
              url: problem.url,
              difficulty: problem.difficulty,
              cost: 1,
              position: 0,
              reviewedToday: false,
              lastRating: null,
              dueAt: null,
            },
          ];
          return problem;
        }

        case "get_today":
          return {
            localDate: "2026-07-19",
            streakDays: todayItems.some((item) => item.reviewedToday) ? 1 : 0,
            items: todayItems.map((item) => ({ ...item })),
          };

        case "list_pending_completions":
          return pending.map((item) => ({ ...item }));

        case "record_rating": {
          const payload = (args?.args ?? {}) as {
            problemId: number;
            rating: Rating;
            idempotencyKey: string;
          };
          pending = pending.filter(
            (item) => item.problemId !== payload.problemId,
          );
          const dueAt = nowSeconds() + 86_400;
          todayItems = todayItems.map((item) =>
            item.problemId === payload.problemId
              ? {
                  ...item,
                  reviewedToday: true,
                  lastRating: payload.rating,
                  dueAt,
                }
              : item,
          );
          problems = problems.map((problem) =>
            problem.id === payload.problemId
              ? { ...problem, dueAt, updatedAt: nowSeconds() }
              : problem,
          );
          return {
            stability: 2.5,
            difficulty: 5,
            due_at: dueAt,
            last_review_at: nowSeconds(),
          };
        }

        case "delete_problem": {
          const problemId = Number(args?.problemId ?? args?.problem_id);
          problems = problems.filter((item) => item.id !== problemId);
          todayItems = todayItems.filter((item) => item.problemId !== problemId);
          pending = pending.filter((item) => item.problemId !== problemId);
          return;
        }

        case "set_problem_status_cmd": {
          const payload = (args?.args ?? {}) as {
            problemId: number;
            status: ProblemStatus;
          };
          problems = problems.map((problem) =>
            problem.id === payload.problemId
              ? { ...problem, status: payload.status, updatedAt: nowSeconds() }
              : problem,
          );
          if (payload.status !== "active") {
            todayItems = todayItems.filter(
              (item) => item.problemId !== payload.problemId,
            );
          }
          return;
        }

        case "get_problem_detail": {
          const problemId = Number(args?.problemId ?? args?.problem_id);
          const problem = problems.find((item) => item.id === problemId);
          if (!problem) {
            throw new Error(`Problem ${problemId} not found`);
          }
          return {
            problem: {
              id: problem.id,
              slug: problem.slug,
              title: problem.title,
              url: problem.url,
              difficulty: problem.difficulty,
              status: problem.status,
              added_at: problem.addedAt,
              updated_at: problem.updatedAt,
            },
            schedule:
              problem.dueAt == null
                ? null
                : {
                    stability: 2.5,
                    difficulty: 5,
                    due_at: problem.dueAt,
                    last_review_at: nowSeconds(),
                  },
            history: [],
          };
        }

        case "open_problem_url":
          return;

        case "get_pairing_status":
          return {
            pairingCode: settings.pairingCode,
            activeClients: 0,
          };

        case "get_loopback_status":
          return {
            address: "127.0.0.1",
            port: 17342,
            url: "http://127.0.0.1:17342",
          };

        case "update_settings": {
          const payload = (args?.args ?? {}) as {
            timezoneId: string;
            desiredRetention: number;
          };
          settings.timezoneId = payload.timezoneId;
          settings.desiredRetention = payload.desiredRetention;
          settings.updatedAt = nowSeconds();
          return { ...settings };
        }

        case "regenerate_pairing_code":
          settings.pairingCode = "NEWCODE1";
          settings.updatedAt = nowSeconds();
          return { ...settings };

        case "export_backup":
          return {
            version: 1,
            settings: {
              timezone_id: settings.timezoneId,
              desired_retention: settings.desiredRetention,
              onboarding_completed: settings.onboardingCompleted,
              pairing_code: settings.pairingCode,
            },
            problems: [],
            review_events: [],
            schedules: null,
          };

        case "import_backup":
          return { ...settings };

        default:
          throw new Error(`Unhandled mocked invoke: ${cmd}`);
      }
    };
  });
}

export async function injectAcceptedSubmission(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.__ANKICODE_E2E__?.injectAcceptedSubmission();
  });
}

export async function getInvokeCalls(page: Page): Promise<InvokeCall[]> {
  return page.evaluate(() => window.__ANKICODE_E2E__?.calls ?? []);
}
