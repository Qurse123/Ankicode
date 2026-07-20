import { useEffect } from "react";

import type { Rating } from "../types";

const RATINGS: Rating[] = ["again", "hard", "good", "easy"];

type RatingDialogProps = {
  title: string;
  busy?: boolean;
  error?: string | null;
  onRate: (rating: Rating) => void;
  onClose: () => void;
};

export function RatingDialog({
  title,
  busy = false,
  error = null,
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

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <div
        className="modal-panel rating-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="rating-title"
        onClick={(event) => event.stopPropagation()}
      >
        <p className="eyebrow">Manual rating</p>
        <h2 id="rating-title">{title}</h2>
        <p className="panel-copy">How did this review feel?</p>
        {error ? (
          <p className="error-text" role="alert">
            {error}
          </p>
        ) : null}
        <div className="rating-grid">
          {RATINGS.map((rating) => (
            <button
              key={rating}
              type="button"
              className={`rating-button rating-${rating}`}
              disabled={busy}
              onClick={() => onRate(rating)}
            >
              {rating}
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
