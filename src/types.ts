export type Difficulty = "easy" | "medium" | "hard";
export type ProblemStatus = "active" | "paused" | "archived" | "removed";
export type Rating = "again" | "hard" | "medium" | "easy";

export type AppSettings = {
  timezoneId: string;
  desiredRetention: number;
  onboardingCompleted: boolean;
  pairingCode: string;
  updatedAt: number;
};

export type Bootstrap = {
  settings: AppSettings;
};

export type PendingCompletion = {
  id: number;
  problemId: number;
  slug: string;
  title: string;
  difficulty: Difficulty;
  url: string;
  idempotencyKey: string;
  acceptedAt: number;
  createdAt: number;
};

export type LoopbackStatus = {
  address: string;
  port: number;
  url: string;
};

export type PairingStatus = {
  pairingCode: string;
  activeClients: number;
};

export type TodayItem = {
  problemId: number;
  slug: string;
  title: string;
  url: string;
  difficulty: Difficulty;
  cost: number;
  position: number;
  reviewedToday: boolean;
  lastRating: Rating | null;
  dueAt: number | null;
};

export type TodayView = {
  localDate: string;
  items: TodayItem[];
  streakDays: number;
};

export type ProblemListItem = {
  id: number;
  slug: string;
  title: string;
  url: string;
  difficulty: Difficulty;
  status: ProblemStatus;
  addedAt: number;
  updatedAt: number;
  dueAt: number | null;
};

export type Problem = {
  id: number;
  slug: string;
  title: string;
  url: string;
  difficulty: Difficulty;
  status: ProblemStatus;
  added_at: number;
  updated_at: number;
};

export type ScheduleState = {
  stability: number;
  difficulty: number;
  due_at: number;
  last_review_at: number;
};

export type ReviewEvent = {
  idempotency_key: string;
  rating: Rating;
  reviewed_at: number;
};

export type ProblemDetail = {
  problem: Problem;
  schedule: ScheduleState | null;
  history: ReviewEvent[];
};

export type BackupDocument = {
  version: number;
  settings: {
    timezone_id: string;
    desired_retention: number;
    onboarding_completed: boolean;
    pairing_code: string;
  };
  problems: Array<{
    slug: string;
    title: string;
    url: string;
    difficulty: Difficulty;
    status: ProblemStatus;
    added_at: number;
    updated_at: number;
  }>;
  review_events: Array<{
    problem_slug: string;
    idempotency_key: string;
    rating: Rating;
    reviewed_at: number;
  }>;
  schedules?: Array<{
    problem_slug: string;
    stability: number;
    difficulty: number;
    due_at: number;
    last_review_at: number;
  }> | null;
};
