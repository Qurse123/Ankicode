import { useMemo, useState, type FormEvent } from "react";

import { completeOnboarding } from "../api";
import { detectSystemTimezone, timezoneOptions } from "../timezones";
import type { AppSettings } from "../types";

type OnboardingProps = {
  pairingCode: string;
  onComplete: (settings: AppSettings) => void;
};

export function Onboarding({ pairingCode, onComplete }: OnboardingProps) {
  const zones = useMemo(() => timezoneOptions(), []);
  const [timezoneId, setTimezoneId] = useState(detectSystemTimezone());
  const [desiredRetention, setDesiredRetention] = useState(0.9);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const settings = await completeOnboarding({
        timezoneId,
        desiredRetention,
      });
      onComplete(settings);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="onboarding-shell">
      <form className="onboarding-panel" onSubmit={handleSubmit}>
        <p className="eyebrow">Local-only</p>
        <h1>Ankicode</h1>
        <p className="lead">
          Spaced review for coding problems, kept on this machine.
        </p>

        <label className="field">
          <span>Timezone</span>
          <select
            value={timezoneId}
            onChange={(event) => setTimezoneId(event.target.value)}
          >
            {zones.map((zone) => (
              <option key={zone} value={zone}>
                {zone}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Retention target</span>
          <input
            type="number"
            min={0.01}
            max={0.99}
            step={0.01}
            value={desiredRetention}
            onChange={(event) =>
              setDesiredRetention(Number(event.target.value))
            }
          />
        </label>

        <div className="pairing-block">
          <span>Extension pairing code</span>
          <code className="pairing-code">{pairingCode}</code>
          <small>Save this for the browser extension later.</small>
        </div>

        {error ? <p className="error-text">{error}</p> : null}

        <button type="submit" className="primary-button" disabled={busy}>
          Complete onboarding
        </button>
      </form>
    </main>
  );
}
