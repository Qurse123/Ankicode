import { useEffect, useState } from "react";

import "./App.css";
import {
  addProblemFromUrl,
  getBootstrap,
  getProblemDetail,
  getToday,
  listProblemsView,
  newIdempotencyKey,
  openProblemUrl,
  recordRating,
  setProblemStatus,
} from "./api";
import { MyList } from "./components/MyList";
import { Onboarding } from "./components/Onboarding";
import { ProblemDetailPanel } from "./components/ProblemDetail";
import { RatingDialog } from "./components/RatingDialog";
import { SettingsPanel } from "./components/SettingsPanel";
import { Today } from "./components/Today";
import type {
  AppSettings,
  Difficulty,
  ProblemDetail,
  ProblemListItem,
  ProblemStatus,
  Rating,
  TodayItem,
  TodayView,
} from "./types";

type Tab = "today" | "list" | "settings";

type RatingTarget = {
  problemId: number;
  title: string;
};

function App() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("today");
  const [today, setToday] = useState<TodayView | null>(null);
  const [todayLoading, setTodayLoading] = useState(false);
  const [todayError, setTodayError] = useState<string | null>(null);
  const [problems, setProblems] = useState<ProblemListItem[]>([]);
  const [listLoading, setListLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [detail, setDetail] = useState<ProblemDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [ratingTarget, setRatingTarget] = useState<RatingTarget | null>(null);
  const [ratingBusy, setRatingBusy] = useState(false);
  const [ratingError, setRatingError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void getBootstrap()
      .then((bootstrap) => {
        if (!cancelled) {
          setSettings(bootstrap.settings);
        }
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setBootError(cause instanceof Error ? cause.message : String(cause));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!settings?.onboardingCompleted) {
      return;
    }
    if (tab === "today") {
      void refreshToday();
    }
    if (tab === "list") {
      void refreshList();
    }
  }, [settings?.onboardingCompleted, tab]);

  async function refreshToday() {
    setTodayLoading(true);
    setTodayError(null);
    try {
      setToday(await getToday());
    } catch (cause) {
      setTodayError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setTodayLoading(false);
    }
  }

  async function refreshList() {
    setListLoading(true);
    setListError(null);
    try {
      setProblems(await listProblemsView());
    } catch (cause) {
      setListError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setListLoading(false);
    }
  }

  async function handleStart(url: string) {
    const message = (cause: unknown) =>
      cause instanceof Error ? cause.message : String(cause);
    try {
      await openProblemUrl(url);
    } catch (cause) {
      if (detail) {
        setDetailError(message(cause));
      } else if (tab === "list") {
        setListError(message(cause));
      } else {
        setTodayError(message(cause));
      }
    }
  }

  async function handleRate(rating: Rating) {
    if (!ratingTarget) {
      return;
    }
    setRatingBusy(true);
    setRatingError(null);
    try {
      await recordRating({
        problemId: ratingTarget.problemId,
        rating,
        idempotencyKey: newIdempotencyKey("manual"),
      });
      setRatingTarget(null);
      if (tab === "today") {
        await refreshToday();
      }
      if (tab === "list" || detail) {
        await refreshList();
      }
      if (detail?.problem.id === ratingTarget.problemId) {
        setDetail(await getProblemDetail(ratingTarget.problemId));
      }
    } catch (cause) {
      setRatingError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRatingBusy(false);
    }
  }

  if (bootError) {
    return (
      <main className="app-shell">
        <p className="error-text">{bootError}</p>
      </main>
    );
  }

  if (!settings) {
    return (
      <main className="app-shell">
        <p className="muted">Opening Ankicode…</p>
      </main>
    );
  }

  if (!settings.onboardingCompleted) {
    return (
      <Onboarding
        pairingCode={settings.pairingCode}
        onComplete={(next) => setSettings(next)}
      />
    );
  }

  return (
    <div className="app-frame">
      <header className="app-header">
        <div className="brand-lockup">
          <p className="eyebrow">Local study</p>
          <h1 className="brand">Ankicode</h1>
        </div>
        <nav className="app-nav" aria-label="Primary">
          <button
            type="button"
            className={tab === "today" ? "nav-button active" : "nav-button"}
            aria-current={tab === "today" ? "page" : undefined}
            onClick={() => setTab("today")}
          >
            Today
          </button>
          <button
            type="button"
            className={tab === "list" ? "nav-button active" : "nav-button"}
            aria-current={tab === "list" ? "page" : undefined}
            onClick={() => setTab("list")}
          >
            My List
          </button>
          <button
            type="button"
            className={tab === "settings" ? "nav-button active" : "nav-button"}
            aria-current={tab === "settings" ? "page" : undefined}
            onClick={() => setTab("settings")}
          >
            Settings
          </button>
        </nav>
      </header>

      <main className="app-main">
        {tab === "today" ? (
          <Today
            today={today}
            loading={todayLoading}
            error={todayError}
            onStart={(item: TodayItem) => void handleStart(item.url)}
            onRate={(item: TodayItem) => {
              setRatingError(null);
              setRatingTarget({ problemId: item.problemId, title: item.title });
            }}
          />
        ) : null}

        {tab === "list" ? (
          <MyList
            problems={problems}
            loading={listLoading}
            error={listError}
            onAdd={async (input: {
              url: string;
              title?: string;
              difficulty: Difficulty;
            }) => {
              await addProblemFromUrl(input);
              await refreshList();
            }}
            onOpen={async (problemId) => {
              try {
                setListError(null);
                setDetailError(null);
                setDetail(await getProblemDetail(problemId));
              } catch (cause) {
                setListError(
                  cause instanceof Error ? cause.message : String(cause),
                );
              }
            }}
            onStatus={async (problemId, status: ProblemStatus) => {
              try {
                setListError(null);
                await setProblemStatus({ problemId, status });
                await refreshList();
              } catch (cause) {
                setListError(
                  cause instanceof Error ? cause.message : String(cause),
                );
              }
            }}
          />
        ) : null}

        {tab === "settings" ? (
          <SettingsPanel
            settings={settings}
            onSettingsChange={(next) => {
              setSettings(next);
              void refreshToday();
              void refreshList();
            }}
          />
        ) : null}
      </main>

      {detail ? (
        <ProblemDetailPanel
          detail={detail}
          error={detailError}
          onClose={() => {
            setDetail(null);
            setDetailError(null);
          }}
          onStart={() => void handleStart(detail.problem.url)}
          onRate={() => {
            setRatingError(null);
            setRatingTarget({
              problemId: detail.problem.id,
              title: detail.problem.title,
            });
          }}
        />
      ) : null}

      {ratingTarget ? (
        <RatingDialog
          title={ratingTarget.title}
          busy={ratingBusy}
          error={ratingError}
          onClose={() => {
            setRatingTarget(null);
            setRatingError(null);
          }}
          onRate={(rating) => void handleRate(rating)}
        />
      ) : null}
    </div>
  );
}

export default App;
