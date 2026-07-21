/** Human-readable FSRS next-due label from a UTC epoch-second timestamp. */
export function formatDueLabel(
  dueAt: number | null | undefined,
  nowMs: number = Date.now(),
): string {
  if (dueAt == null) {
    return "new";
  }
  const dueMs = dueAt * 1000;
  const deltaMs = dueMs - nowMs;
  if (deltaMs <= 0) {
    return "due now";
  }
  const dayMs = 86_400_000;
  const days = Math.max(1, Math.round(deltaMs / dayMs));
  if (days === 1) {
    return "due tomorrow";
  }
  if (days < 14) {
    return `due in ${days} days`;
  }
  return `due ${new Date(dueMs).toLocaleDateString()}`;
}
