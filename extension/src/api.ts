export const LOOPBACK_BASE = "http://127.0.0.1:17342";

export type PairResponse = {
  token: string;
  clientId: number;
};

export type AddProblemPayload = {
  slug: string;
  title: string;
  difficulty: string;
  url?: string;
};

export type AcceptedPayload = {
  slug: string;
  acceptedAt?: number;
};

export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

export function isRetryableStatus(status: number): boolean {
  return status === 0 || status === 408 || status === 429 || status >= 500;
}

async function readError(response: Response): Promise<string> {
  const text = await response.text();
  return text || `Request failed (${response.status})`;
}

export async function healthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${LOOPBACK_BASE}/v1/health`);
    if (!response.ok) {
      return false;
    }
    const body = (await response.json()) as { ok?: boolean };
    return body.ok === true;
  } catch {
    return false;
  }
}

export async function pairWithApp(
  code: string,
  origin: string,
): Promise<PairResponse> {
  const response = await fetch(`${LOOPBACK_BASE}/v1/pair`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, origin }),
  });
  if (!response.ok) {
    throw new ApiError(await readError(response), response.status);
  }
  return (await response.json()) as PairResponse;
}

export async function addProblem(
  token: string,
  origin: string,
  payload: AddProblemPayload,
  idempotencyKey: string,
): Promise<void> {
  const response = await fetch(`${LOOPBACK_BASE}/v1/problems/add`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
      Origin: origin,
      "X-Ankicode-Origin": origin,
      "Idempotency-Key": idempotencyKey,
    },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new ApiError(await readError(response), response.status);
  }
}

export async function reportAccepted(
  token: string,
  origin: string,
  payload: AcceptedPayload,
  idempotencyKey: string,
): Promise<void> {
  const response = await fetch(`${LOOPBACK_BASE}/v1/submissions/accepted`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
      Origin: origin,
      "X-Ankicode-Origin": origin,
      "Idempotency-Key": idempotencyKey,
    },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new ApiError(await readError(response), response.status);
  }
}
