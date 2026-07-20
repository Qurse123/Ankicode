import type { TodayItem, TodayView } from "../types";

type TodayProps = {
  today: TodayView | null;
  loading: boolean;
  error: string | null;
  onStart: (item: TodayItem) => void;
  onRate: (item: TodayItem) => void;
};

export function Today({ today, loading, error, onStart, onRate }: TodayProps) {
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

  return (
    <section className="page-section" aria-labelledby="today-title">
      <div className="page-heading">
        <h1 id="today-title">Today</h1>
        <p className="muted">{today?.localDate ?? "—"}</p>
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
            <li key={item.problemId} className="today-row">
              <div>
                <h2>{item.title}</h2>
                <p className="meta-line">
                  <span className={`pill difficulty-${item.difficulty}`}>
                    {item.difficulty}
                  </span>
                  <span>cost {item.cost}</span>
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
                  Rate
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
