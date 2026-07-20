import { useMemo, useState, type FormEvent } from "react";

import type { Difficulty, ProblemListItem, ProblemStatus } from "../types";

type MyListProps = {
  problems: ProblemListItem[];
  loading: boolean;
  error: string | null;
  onAdd: (input: {
    url: string;
    title?: string;
    difficulty: Difficulty;
  }) => Promise<void>;
  onOpen: (problemId: number) => void;
  onStatus: (problemId: number, status: ProblemStatus) => Promise<void>;
};

function dueLabel(dueAt: number | null): string {
  if (dueAt == null) {
    return "new";
  }
  const due = new Date(dueAt * 1000);
  const now = Date.now();
  if (due.getTime() <= now) {
    return "due";
  }
  return `due ${due.toLocaleDateString()}`;
}

export function MyList({
  problems,
  loading,
  error,
  onAdd,
  onOpen,
  onStatus,
}: MyListProps) {
  const [url, setUrl] = useState("");
  const [title, setTitle] = useState("");
  const [difficulty, setDifficulty] = useState<Difficulty>("easy");
  const [search, setSearch] = useState("");
  const [difficultyFilter, setDifficultyFilter] = useState<"all" | Difficulty>(
    "all",
  );
  const [statusFilter, setStatusFilter] = useState<"all" | ProblemStatus>(
    "all",
  );
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return problems.filter((problem) => {
      if (
        difficultyFilter !== "all" &&
        problem.difficulty !== difficultyFilter
      ) {
        return false;
      }
      if (statusFilter !== "all" && problem.status !== statusFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      return (
        problem.title.toLowerCase().includes(query) ||
        problem.slug.toLowerCase().includes(query)
      );
    });
  }, [problems, search, difficultyFilter, statusFilter]);

  async function handleAdd(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setFormError(null);
    try {
      await onAdd({
        url,
        title: title.trim() || undefined,
        difficulty,
      });
      setUrl("");
      setTitle("");
      setDifficulty("easy");
    } catch (cause) {
      setFormError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page-section" aria-labelledby="list-title">
      <div className="page-heading">
        <h1 id="list-title">My List</h1>
        <p className="muted">Paste a LeetCode URL to track a problem.</p>
      </div>

      <form className="add-form" onSubmit={handleAdd}>
        <label className="field">
          <span>Problem URL</span>
          <input
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="https://leetcode.com/problems/two-sum/"
            required
          />
        </label>
        <label className="field">
          <span>Title (optional)</span>
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="two sum"
          />
        </label>
        <label className="field">
          <span>Difficulty</span>
          <select
            value={difficulty}
            onChange={(event) =>
              setDifficulty(event.target.value as Difficulty)
            }
          >
            <option value="easy">Easy</option>
            <option value="medium">Medium</option>
            <option value="hard">Hard</option>
          </select>
        </label>
        <button type="submit" className="primary-button" disabled={busy}>
          Add problem
        </button>
        {formError ? <p className="error-text">{formError}</p> : null}
      </form>

      <div className="filter-bar">
        <label className="field inline">
          <span>Search</span>
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Filter by title"
          />
        </label>
        <label className="field inline">
          <span>Difficulty</span>
          <select
            value={difficultyFilter}
            onChange={(event) =>
              setDifficultyFilter(event.target.value as "all" | Difficulty)
            }
          >
            <option value="all">All</option>
            <option value="easy">Easy</option>
            <option value="medium">Medium</option>
            <option value="hard">Hard</option>
          </select>
        </label>
        <label className="field inline">
          <span>Status</span>
          <select
            value={statusFilter}
            onChange={(event) =>
              setStatusFilter(event.target.value as "all" | ProblemStatus)
            }
          >
            <option value="all">All</option>
            <option value="active">Active</option>
            <option value="paused">Paused</option>
            <option value="archived">Archived</option>
            <option value="removed">Removed</option>
          </select>
        </label>
      </div>

      {loading ? <p className="muted">Loading problems…</p> : null}
      {error ? <p className="error-text">{error}</p> : null}

      <ul className="problem-list">
        {filtered.map((problem) => (
          <li key={problem.id} className="problem-row">
            <button
              type="button"
              className="problem-main"
              onClick={() => onOpen(problem.id)}
            >
              <span className="problem-title">{problem.title}</span>
              <span className="meta-line">
                <span className={`pill difficulty-${problem.difficulty}`}>
                  {problem.difficulty}
                </span>
                <span className="pill">{problem.status}</span>
                <span>{dueLabel(problem.dueAt)}</span>
              </span>
            </button>
            <div className="row-actions">
              {problem.status === "active" ? (
                <button
                  type="button"
                  className="ghost-button"
                  onClick={() => onStatus(problem.id, "paused")}
                >
                  Pause
                </button>
              ) : (
                <button
                  type="button"
                  className="ghost-button"
                  onClick={() => onStatus(problem.id, "active")}
                >
                  Activate
                </button>
              )}
              <button
                type="button"
                className="ghost-button"
                onClick={() => onStatus(problem.id, "archived")}
              >
                Archive
              </button>
              <button
                type="button"
                className="ghost-button"
                onClick={() => onStatus(problem.id, "removed")}
              >
                Remove
              </button>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
