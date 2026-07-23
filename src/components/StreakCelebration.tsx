import { useEffect, type CSSProperties } from "react";

type StreakCelebrationProps = {
  streakDays: number;
  onDone: () => void;
};

const BALLOONS = Array.from({ length: 14 }, (_, index) => index);
const SPARKS = Array.from({ length: 28 }, (_, index) => index);

export function StreakCelebration({
  streakDays,
  onDone,
}: StreakCelebrationProps) {
  useEffect(() => {
    const timer = window.setTimeout(onDone, 3200);
    return () => window.clearTimeout(timer);
    // Intentionally keyed on streakDays so remounted parent callbacks do not reset.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [streakDays]);

  return (
    <div
      className="streak-celebration"
      role="status"
      aria-live="polite"
      aria-label={`Streak increased to ${streakDays} days`}
    >
      <div className="streak-celebration-burst" aria-hidden="true">
        {SPARKS.map((spark) => (
          <span
            key={`spark-${spark}`}
            className={`streak-spark streak-spark-${spark % 7}`}
            style={
              {
                "--angle": `${(360 / SPARKS.length) * spark}deg`,
                "--delay": `${(spark % 6) * 40}ms`,
              } as CSSProperties
            }
          />
        ))}
        {BALLOONS.map((balloon) => (
          <span
            key={`balloon-${balloon}`}
            className={`streak-balloon streak-balloon-${balloon % 5}`}
            style={
              {
                "--x": `${6 + ((balloon * 7) % 88)}%`,
                "--delay": `${balloon * 90}ms`,
                "--drift": `${(balloon % 2 === 0 ? -1 : 1) * (12 + (balloon % 5) * 4)}px`,
              } as CSSProperties
            }
          />
        ))}
      </div>
      <div className="streak-celebration-card">
        <p className="eyebrow">Streak up</p>
        <h2>
          {streakDays} day{streakDays === 1 ? "" : "s"}
        </h2>
        <p className="panel-copy">Nice work — keep the chain going tomorrow.</p>
      </div>
    </div>
  );
}
