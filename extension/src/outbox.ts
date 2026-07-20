/// <reference types="chrome" />

import {
  addProblem,
  ApiError,
  isRetryableStatus,
  reportAccepted,
  type AcceptedPayload,
  type AddProblemPayload,
} from "./api";

export type OutboxKind = "add" | "accepted";

export type OutboxItem = {
  id: string;
  kind: OutboxKind;
  payload: AddProblemPayload | AcceptedPayload;
  idempotencyKey: string;
  attempts: number;
  nextAttemptAt: number;
};

export type PairingState = {
  token: string;
  clientId: number;
  origin: string;
};

const OUTBOX_KEY = "outbox";
const PAIRING_KEY = "pairing";

const MAX_BACKOFF_MS = 60_000;

export function backoffMs(attempts: number): number {
  const base = 1_000 * 2 ** Math.max(0, attempts);
  return Math.min(base, MAX_BACKOFF_MS);
}

/** Stable keys so retries and reloads do not create duplicate server rows. */
export function newIdempotencyKey(kind: OutboxKind, slug: string): string {
  return `${kind}:${slug}`;
}

export async function getPairing(): Promise<PairingState | null> {
  const stored = await chrome.storage.local.get(PAIRING_KEY);
  const pairing = stored[PAIRING_KEY] as PairingState | undefined;
  if (!pairing?.token || !pairing.origin) {
    return null;
  }
  return pairing;
}

export async function setPairing(pairing: PairingState): Promise<void> {
  await chrome.storage.local.set({ [PAIRING_KEY]: pairing });
}

export async function clearPairing(): Promise<void> {
  await chrome.storage.local.remove(PAIRING_KEY);
}

export async function loadOutbox(): Promise<OutboxItem[]> {
  const stored = await chrome.storage.local.get(OUTBOX_KEY);
  const items = stored[OUTBOX_KEY] as OutboxItem[] | undefined;
  return Array.isArray(items) ? items : [];
}

export async function saveOutbox(items: OutboxItem[]): Promise<void> {
  await chrome.storage.local.set({ [OUTBOX_KEY]: items });
}

export async function enqueue(
  kind: OutboxKind,
  payload: AddProblemPayload | AcceptedPayload,
  idempotencyKey: string,
): Promise<OutboxItem> {
  const items = await loadOutbox();
  const existing = items.find((item) => item.idempotencyKey === idempotencyKey);
  if (existing) {
    return existing;
  }
  const item: OutboxItem = {
    id: `${kind}-${idempotencyKey}`,
    kind,
    payload,
    idempotencyKey,
    attempts: 0,
    nextAttemptAt: Date.now(),
  };
  items.push(item);
  await saveOutbox(items);
  return item;
}

export async function flushOutbox(now = Date.now()): Promise<{
  sent: number;
  remaining: number;
  dropped: number;
}> {
  const pairing = await getPairing();
  if (!pairing) {
    const items = await loadOutbox();
    return { sent: 0, remaining: items.length, dropped: 0 };
  }

  const items = await loadOutbox();
  const remaining: OutboxItem[] = [];
  let sent = 0;
  let dropped = 0;

  for (const item of items) {
    if (item.nextAttemptAt > now) {
      remaining.push(item);
      continue;
    }
    try {
      if (item.kind === "add") {
        await addProblem(
          pairing.token,
          pairing.origin,
          item.payload as AddProblemPayload,
          item.idempotencyKey,
        );
      } else {
        await reportAccepted(
          pairing.token,
          pairing.origin,
          item.payload as AcceptedPayload,
          item.idempotencyKey,
        );
      }
      sent += 1;
    } catch (cause) {
      const status = cause instanceof ApiError ? cause.status : 0;
      // Accepted 404 means the problem is not in My List yet — keep retrying.
      const retryAcceptedNotFound = item.kind === "accepted" && status === 404;
      if (!isRetryableStatus(status) && !retryAcceptedNotFound) {
        dropped += 1;
        continue;
      }
      remaining.push({
        ...item,
        attempts: item.attempts + 1,
        nextAttemptAt: now + backoffMs(item.attempts + 1),
      });
    }
  }

  await saveOutbox(remaining);
  return { sent, remaining: remaining.length, dropped };
}
