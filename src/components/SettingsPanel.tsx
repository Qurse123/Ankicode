import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";

import {
  exportBackup,
  getLoopbackStatus,
  getPairingStatus,
  importBackup,
  regeneratePairingCode,
  updateSettings,
} from "../api";
import { timezoneOptions } from "../timezones";
import type {
  AppSettings,
  BackupDocument,
  LoopbackStatus,
  PairingStatus,
} from "../types";

type SettingsPanelProps = {
  settings: AppSettings;
  onSettingsChange: (settings: AppSettings) => void;
};

export function SettingsPanel({
  settings,
  onSettingsChange,
}: SettingsPanelProps) {
  const zones = useMemo(
    () => timezoneOptions(settings.timezoneId),
    [settings.timezoneId],
  );
  const [timezoneId, setTimezoneId] = useState(settings.timezoneId);
  const [desiredRetention, setDesiredRetention] = useState(
    settings.desiredRetention,
  );
  const [pairingCode, setPairingCode] = useState(settings.pairingCode);
  const [loopback, setLoopback] = useState<LoopbackStatus | null>(null);
  const [activeClients, setActiveClients] = useState(0);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void getLoopbackStatus()
      .then(setLoopback)
      .catch(() => setLoopback(null));
    void getPairingStatus()
      .then((status: PairingStatus) => {
        setPairingCode(status.pairingCode);
        setActiveClients(status.activeClients);
      })
      .catch(() => setActiveClients(0));
  }, []);

  async function handleSave(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const next = await updateSettings({ timezoneId, desiredRetention });
      setPairingCode(next.pairingCode);
      onSettingsChange(next);
      setMessage("Settings saved.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function handleRegenerate() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const next = await regeneratePairingCode();
      setPairingCode(next.pairingCode);
      onSettingsChange(next);
      const status = await getPairingStatus();
      setActiveClients(status.activeClients);
      setMessage(
        "New pairing code ready. Already-paired extensions keep working until you Unpair them.",
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function handleExport() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const backup = await exportBackup();
      const blob = new Blob([JSON.stringify(backup, null, 2)], {
        type: "application/json",
      });
      const href = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = href;
      anchor.download = `ankicode-backup-${new Date().toISOString().slice(0, 10)}.json`;
      anchor.click();
      URL.revokeObjectURL(href);
      setMessage("Backup downloaded.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function handleImportFile(file: File) {
    const confirmed = window.confirm(
      "Import will replace all local learning data. Continue?",
    );
    if (!confirmed) {
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const text = await file.text();
      const parsed = JSON.parse(text) as BackupDocument;
      const next = await importBackup(parsed);
      setTimezoneId(next.timezoneId);
      setDesiredRetention(next.desiredRetention);
      setPairingCode(next.pairingCode);
      onSettingsChange(next);
      setMessage("Backup imported.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page-section" aria-labelledby="settings-title">
      <div className="page-heading">
        <h1 id="settings-title">Settings</h1>
        <p className="muted">Local preferences and backup.</p>
      </div>

      <form className="settings-form" onSubmit={handleSave}>
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
        <button type="submit" className="primary-button" disabled={busy}>
          Save settings
        </button>
      </form>

      <div className="settings-block">
        <h2>Extension loopback API</h2>
        <p className="muted">
          Local API bound to {loopback?.url ?? "http://127.0.0.1:17342"} for the
          Chromium extension only.
        </p>
        <h2>Pairing</h2>
        <p className="muted">
          Pair once in the extension popup, then forget it. The extension stores
          a long-lived token; this code stays valid until you regenerate it.
        </p>
        <p className="meta-line">
          <span className="pill status-reviewed">
            {activeClients === 0
              ? "No extension paired yet"
              : activeClients === 1
                ? "1 extension paired"
                : `${activeClients} extensions paired`}
          </span>
        </p>
        <code className="pairing-code">{pairingCode}</code>
        <button
          type="button"
          className="secondary-button"
          disabled={busy}
          onClick={handleRegenerate}
        >
          Regenerate code (optional)
        </button>
      </div>

      <div className="settings-block">
        <h2>Backup</h2>
        <div className="row-actions">
          <button
            type="button"
            className="secondary-button"
            disabled={busy}
            onClick={handleExport}
          >
            Export JSON
          </button>
          <button
            type="button"
            className="secondary-button"
            disabled={busy}
            onClick={() => fileInputRef.current?.click()}
          >
            Import JSON
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) {
                void handleImportFile(file);
              }
              event.target.value = "";
            }}
          />
        </div>
      </div>

      {message ? <p className="success-text">{message}</p> : null}
      {error ? <p className="error-text">{error}</p> : null}
    </section>
  );
}
