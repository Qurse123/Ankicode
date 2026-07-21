import { useEffect } from "react";

import type { Rating } from "../types";

const RATINGS: Array<{ rating: Rating; hint: string }> = [
  { rating: "again", hint: "Forgot — show again soon" },
  { rating: "hard", hint: "Tough — shorter interval" },
  { rating: "medium", hint: "Solid — normal interval" },
  { rating: "easy", hint: "Easy — longer interval" },
];

type RatingDialogProps = {
  title: string;
  busy?: boolean;
  error?: string | null;
  successDueAt?: number | null;
  onRate: (rating: Rating) => void;
  onClose: () => void;
};

export function RatingDialog({
  title,
  busy = false,
  error = null,
  successDueAt = null,
  onRate,
  onClose,
}: RatingDialogProps) {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) {
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, onClose]);

  if (successDueAt != null) {
    return (
      <div className="modal-backdrop" role="presentation" onClick={onClose}>
        <div
          className="modal-panel rating-panel"
          role="dialog"
          aria-modal="true"
          aria-labelledby="rating-title"
          onClick={(event) => event.stopPropagation()}
        >
          <p className="eyebrow">Scheduled</p>
          <h2 id="rating-title">{title}</h2>
          <p className="panel-copy">
            Rated with FSRS. Next review:{" "}
            {new Date(successDueAt * 1000).toLocaleString()}.
          </p>
          <button type="button" className="primary-button" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <div
        className="modal-panel rating-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="rating-title"
        onClick={(event) => event.stopPropagation()}
      >
        <p className="eyebrow">FSRS rating</p>
        <h2 id="rating-title">{title}</h2>
        <p className="panel-copy">
          Rate anytime — before or after you solve. Again / Hard / Medium / Easy
          sets when this problem returns.
        </p>
        {error ? (
          <p className="error-text" role="alert">
            {error}
          </p>
        ) : null}
        <div className="rating-grid">
          {RATINGS.map(({ rating, hint }) => (
            <button
              key={rating}
              type="button"
              className={`rating-button rating-${rating}`}
              disabled={busy}
              onClick={() => onRate(rating)}
              title={hint}
            >
              <span className="rating-label">{rating}</span>
              <span className="rating-hint">{hint}</span>
            </button>
          ))}
        </div>
        <button
          type="button"
          className="ghost-button"
          disabled={busy}
          onClick={onClose}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
