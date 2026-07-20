type InvokeHandler = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

declare global {
  interface Window {
    __ANKICODE_E2E_INVOKE__?: InvokeHandler;
  }
}

/**
 * Vite alias target for `@tauri-apps/api/core` during Playwright runs.
 * The real handler is injected via `page.addInitScript` before the app loads.
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const handler = window.__ANKICODE_E2E_INVOKE__;
  if (!handler) {
    throw new Error(
      `Tauri invoke mock missing for "${cmd}". Load the e2e init script first.`,
    );
  }
  return (await handler(cmd, args)) as T;
}
