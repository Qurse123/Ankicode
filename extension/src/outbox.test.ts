import { beforeEach, describe, expect, it, vi } from "vitest";

import * as api from "./api";
import { ApiError } from "./api";
import {
  backoffMs,
  enqueue,
  flushOutbox,
  loadOutbox,
  newIdempotencyKey,
  setPairing,
} from "./outbox";

vi.mock("./api", async () => {
  const actual = await vi.importActual<typeof import("./api")>("./api");
  return {
    ...actual,
    addProblem: vi.fn(),
    reportAccepted: vi.fn(),
  };
});

type Store = Record<string, unknown>;

function mockStorage(initial: Store = {}) {
  const store: Store = { ...initial };
  const local = {
    get: vi.fn(async (keys?: string | string[] | Store | null) => {
      if (typeof keys === "string") {
        return { [keys]: store[keys] };
      }
      return { ...store };
    }),
    set: vi.fn(async (values: Store) => {
      Object.assign(store, values);
    }),
    remove: vi.fn(async (keys: string | string[]) => {
      for (const key of Array.isArray(keys) ? keys : [keys]) {
        delete store[key];
      }
    }),
  };
  Object.assign(globalThis, {
    chrome: {
      storage: { local },
    },
  });
  return store;
}

describe("outbox", () => {
  beforeEach(() => {
    mockStorage();
    vi.mocked(api.addProblem).mockReset();
    vi.mocked(api.reportAccepted).mockReset();
  });

  it("computes exponential backoff with a cap", () => {
    expect(backoffMs(0)).toBe(1000);
    expect(backoffMs(1)).toBe(2000);
    expect(backoffMs(2)).toBe(4000);
    expect(backoffMs(10)).toBe(60_000);
  });

  it("uses stable idempotency keys per slug", () => {
    expect(newIdempotencyKey("accepted", "two-sum")).toBe("accepted:two-sum");
    expect(newIdempotencyKey("add", "two-sum")).toBe("add:two-sum");
  });

  it("enqueues and retries failed sends with backoff", async () => {
    await setPairing({
      token: "tok",
      clientId: 1,
      origin: "chrome-extension://abc",
    });
    vi.mocked(api.addProblem)
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(undefined);

    await enqueue(
      "add",
      {
        slug: "two-sum",
        title: "Two Sum",
        difficulty: "Easy",
      },
      "add:two-sum",
    );

    const now = Date.now();
    const first = await flushOutbox(now);
    expect(first.sent).toBe(0);
    expect(first.remaining).toBe(1);
    const waiting = await loadOutbox();
    expect(waiting[0]?.attempts).toBe(1);
    expect(waiting[0]?.nextAttemptAt).toBe(now + 2000);

    const second = await flushOutbox(waiting[0]!.nextAttemptAt);
    expect(second.sent).toBe(1);
    expect(second.remaining).toBe(0);
    expect(api.addProblem).toHaveBeenCalledTimes(2);
  });

  it("retries accepted 404 until the problem exists", async () => {
    await setPairing({
      token: "tok",
      clientId: 1,
      origin: "chrome-extension://abc",
    });
    vi.mocked(api.reportAccepted).mockRejectedValueOnce(
      new ApiError("problem slug missing-problem was not found", 404),
    );

    await enqueue(
      "accepted",
      { slug: "missing-problem" },
      "accepted:missing-problem",
    );
    const result = await flushOutbox(Date.now());
    expect(result.sent).toBe(0);
    expect(result.dropped).toBe(0);
    expect(result.remaining).toBe(1);
  });

  it("drops permanent auth failures instead of retrying forever", async () => {
    await setPairing({
      token: "tok",
      clientId: 1,
      origin: "chrome-extension://abc",
    });
    vi.mocked(api.addProblem).mockRejectedValueOnce(
      new ApiError("unauthorized", 401),
    );

    await enqueue(
      "add",
      {
        slug: "two-sum",
        title: "Two Sum",
        difficulty: "Easy",
      },
      "add:two-sum",
    );
    const result = await flushOutbox(Date.now());
    expect(result.sent).toBe(0);
    expect(result.dropped).toBe(1);
    expect(result.remaining).toBe(0);
    expect(await loadOutbox()).toEqual([]);
  });
});
