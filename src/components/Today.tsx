import { formatDueLabel } from "../scheduleLabel";
import type { TodayItem, TodayView } from "../types";

type TodayProps = {
  today: TodayView | null;
  loading: boolean;
  error: string | null;
  streakDays: number;
  onStart: (item: TodayItem) => void;
  onRate: (item: TodayItem) => void;
};

export function Today({
  today,
  loading,
  error,
  streakDays,
  onStart,
  onRate,
}: TodayProps) {
  if (loading) {
    return (
      <section className="page-section" aria-labelledby="today-title">
        <h1 id="today-title">Today</h1>
        <p className="muted">Loading today’s assignment…</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="page-section" aria-labelledby="today-title">
        <h1 id="today-title">Today</h1>
        <p className="error-text">{error}</p>
      </section>
    );
  }

  const items = today?.items ?? [];
  const remaining = items.filter((item) => !item.reviewedToday).length;
  const reviewed = items.length - remaining;
  return (
    <section className="page-section" aria-labelledby="today-title">
      <div className="page-heading">
        <h1 id="today-title">Today</h1>
        <p className="muted">{today?.localDate ?? "—"}</p>
      </div>

      <div className="stats-row" aria-label="Today stats">
        <article className="stat-card">
          <p className="stat-label">Remaining</p>
          <p className="stat-value">{remaining}</p>
        </article>
        <article className="stat-card">
          <p className="stat-label">Reviewed</p>
          <p className="stat-value">{reviewed}</p>
        </article>
        <article className="stat-card">
          <p className="stat-label">Streak</p>
          <p className="stat-value accent">
            {streakDays} {streakDays === 1 ? "day" : "days"}
          </p>
        </article>
      </div>

      {items.length === 0 ? (
        <div className="empty-state">
          <p>No problems assigned for today.</p>
          <p className="muted">
            Add active Easy or Medium problems in My List for future days.
          </p>
        </div>
      ) : (
        <ul className="today-list">
          {items.map((item) => (
            <li
              key={item.problemId}
              className={
                item.reviewedToday ? "today-row today-row-reviewed" : "today-row"
              }
            >
              <div>
                <h2>{item.title}</h2>
                <p className="meta-line">
                  <span className={`pill difficulty-${item.difficulty}`}>
                    {item.difficulty}
                  </span>
                  <span>cost {item.cost}</span>
                  {item.reviewedToday ? (
                    <>
                      <span className="pill status-reviewed">
                        rated {item.lastRating ?? "today"}
                      </span>
                      <span>{formatDueLabel(item.dueAt)}</span>
                    </>
                  ) : (
                    <span className="pill status-pending">not rated</span>
                  )}
                </p>
              </div>
              <div className="row-actions">
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => onStart(item)}
                >
                  Start
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => onRate(item)}
                >
                  {item.reviewedToday ? "Re-rate" : "Rate"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
