import { useEffect } from "react";

import type { ProblemDetail as ProblemDetailData } from "../types";

type ProblemDetailProps = {
  detail: ProblemDetailData;
  error?: string | null;
  onClose: () => void;
  onRate: () => void;
  onStart: () => void;
};

export function ProblemDetailPanel({
  detail,
  error = null,
  onClose,
  onRate,
  onStart,
}: ProblemDetailProps) {
  const { problem, schedule, history } = detail;

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <div
        className="modal-panel detail-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="detail-title"
        onClick={(event) => event.stopPropagation()}
      >
        <p className="eyebrow">Problem</p>
        <h2 id="detail-title">{problem.title}</h2>
        <p className="meta-line">
          <span className={`pill difficulty-${problem.difficulty}`}>
            {problem.difficulty}
          </span>
          <span className="pill">{problem.status}</span>
          {schedule ? (
            <span>due {new Date(schedule.due_at * 1000).toLocaleString()}</span>
          ) : (
            <span>not scheduled yet</span>
          )}
        </p>

        {error ? (
          <p className="error-text" role="alert">
            {error}
          </p>
        ) : null}

        <div className="row-actions">
          <button type="button" className="primary-button" onClick={onStart}>
            Start
          </button>
          <button type="button" className="secondary-button" onClick={onRate}>
            Rate
          </button>
          <button type="button" className="ghost-button" onClick={onClose}>
            Close
          </button>
        </div>

        <h3>History</h3>
        {history.length === 0 ? (
          <p className="muted">No reviews yet.</p>
        ) : (
          <ul className="history-list">
            {history.map((event) => (
              <li key={`${event.idempotency_key}-${event.reviewed_at}`}>
                <strong>{event.rating}</strong>
                <span>
                  {new Date(event.reviewed_at * 1000).toLocaleString()}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
