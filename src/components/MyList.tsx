import { useEffect, useMemo, useState, type FormEvent } from "react";

import { formatDueLabel } from "../scheduleLabel";
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
  onDelete: (problemId: number) => Promise<void>;
};

export function MyList({
  problems,
  loading,
  error,
  onAdd,
  onOpen,
  onStatus,
  onDelete,
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
  const [deleteTarget, setDeleteTarget] = useState<ProblemListItem | null>(
    null,
  );
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

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

  useEffect(() => {
    if (!deleteTarget) {
      return;
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !deleteBusy) {
        setDeleteTarget(null);
        setDeleteError(null);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [deleteTarget, deleteBusy]);

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

  async function confirmDelete() {
    if (!deleteTarget) {
      return;
    }
    setDeleteBusy(true);
    setDeleteError(null);
    try {
      await onDelete(deleteTarget.id);
      setDeleteTarget(null);
    } catch (cause) {
      setDeleteError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDeleteBusy(false);
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
                <span>{formatDueLabel(problem.dueAt)}</span>
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
                onClick={() => {
                  setDeleteError(null);
                  setDeleteTarget(problem);
                }}
              >
                Delete
              </button>
            </div>
          </li>
        ))}
      </ul>

      {deleteTarget ? (
        <div
          className="modal-backdrop"
          role="presentation"
          onClick={() => {
            if (!deleteBusy) {
              setDeleteTarget(null);
              setDeleteError(null);
            }
          }}
        >
          <div
            className="modal-panel"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-title"
            onClick={(event) => event.stopPropagation()}
          >
            <p className="eyebrow">Delete problem</p>
            <h2 id="delete-title">{deleteTarget.title}</h2>
            <p className="panel-copy">
              Permanently remove this problem and its review history? This cannot
              be undone.
            </p>
            {deleteError ? (
              <p className="error-text" role="alert">
                {deleteError}
              </p>
            ) : null}
            <div className="row-actions">
              <button
                type="button"
                className="primary-button"
                disabled={deleteBusy}
                onClick={() => void confirmDelete()}
              >
                Delete permanently
              </button>
              <button
                type="button"
                className="ghost-button"
                disabled={deleteBusy}
                onClick={() => {
                  setDeleteTarget(null);
                  setDeleteError(null);
                }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
