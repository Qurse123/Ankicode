import { healthCheck, pairWithApp } from "./api";
import {
  clearPairing,
  enqueue,
  flushOutbox,
  getPairing,
  newIdempotencyKey,
  setPairing,
  type PairingState,
} from "./outbox";
import type { ProblemMetadata } from "./metadata";

type ExtensionMessage =
  | { type: "ADD_PROBLEM"; metadata: ProblemMetadata }
  | { type: "ACCEPTED"; slug: string }
  | { type: "PAIR"; code: string }
  | { type: "UNPAIR" }
  | { type: "STATUS" };

const FLUSH_ALARM = "ankicode-outbox-flush";

async function ensureFlushAlarm(): Promise<void> {
  const existing = await chrome.alarms.get(FLUSH_ALARM);
  if (!existing) {
    await chrome.alarms.create(FLUSH_ALARM, { periodInMinutes: 1 });
  }
}

async function statusPayload(): Promise<{
  paired: boolean;
  online: boolean;
  clientId: number | null;
}> {
  const pairing = await getPairing();
  const online = await healthCheck();
  return {
    paired: Boolean(pairing),
    online,
    clientId: pairing?.clientId ?? null,
  };
}

chrome.runtime.onInstalled.addListener(() => {
  void ensureFlushAlarm();
  void flushOutbox();
});

chrome.runtime.onStartup.addListener(() => {
  void ensureFlushAlarm();
  void flushOutbox();
});

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === FLUSH_ALARM) {
    void flushOutbox();
  }
});

chrome.runtime.onMessage.addListener(
  (message: ExtensionMessage, _sender, sendResponse) => {
    void (async () => {
      try {
        if (message.type === "STATUS") {
          sendResponse({ ok: true, ...(await statusPayload()) });
          return;
        }
        if (message.type === "UNPAIR") {
          await clearPairing();
          sendResponse({ ok: true, ...(await statusPayload()) });
          return;
        }
        if (message.type === "PAIR") {
          const origin = `chrome-extension://${chrome.runtime.id}`;
          const result = await pairWithApp(message.code.trim(), origin);
          const pairing: PairingState = {
            token: result.token,
            clientId: result.clientId,
            origin,
          };
          await setPairing(pairing);
          await flushOutbox();
          sendResponse({ ok: true, ...(await statusPayload()) });
          return;
        }
        if (message.type === "ADD_PROBLEM") {
          const key = newIdempotencyKey("add", message.metadata.slug);
          await enqueue(
            "add",
            {
              slug: message.metadata.slug,
              title: message.metadata.title,
              difficulty: message.metadata.difficulty,
              url: message.metadata.url,
            },
            key,
          );
          await flushOutbox();
          sendResponse({ ok: true });
          return;
        }
        if (message.type === "ACCEPTED") {
          const key = newIdempotencyKey("accepted", message.slug);
          await enqueue("accepted", { slug: message.slug }, key);
          await flushOutbox();
          sendResponse({ ok: true });
          return;
        }
        sendResponse({ ok: false, error: "unknown message" });
      } catch (cause) {
        sendResponse({
          ok: false,
          error: cause instanceof Error ? cause.message : String(cause),
        });
      }
    })();
    return true;
  },
);

void ensureFlushAlarm();
void flushOutbox();
