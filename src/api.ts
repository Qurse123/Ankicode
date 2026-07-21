import { invoke } from "@tauri-apps/api/core";

import type {
  AppSettings,
  BackupDocument,
  Bootstrap,
  Difficulty,
  LoopbackStatus,
  PendingCompletion,
  Problem,
  ProblemDetail,
  ProblemListItem,
  ProblemStatus,
  Rating,
  ScheduleState,
  TodayView,
} from "./types";

export function getBootstrap(): Promise<Bootstrap> {
  return invoke("get_bootstrap");
}

export function completeOnboarding(args: {
  timezoneId: string;
  desiredRetention: number;
}): Promise<AppSettings> {
  return invoke("complete_onboarding", { args });
}

export function getToday(): Promise<TodayView> {
  return invoke("get_today");
}

export function listProblemsView(): Promise<ProblemListItem[]> {
  return invoke("list_problems_view");
}

export function addProblemFromUrl(args: {
  url: string;
  title?: string;
  difficulty: Difficulty;
}): Promise<Problem> {
  return invoke("add_problem_from_url", { args });
}

export function setProblemStatus(args: {
  problemId: number;
  status: ProblemStatus;
}): Promise<void> {
  return invoke("set_problem_status_cmd", { args });
}

export function getProblemDetail(problemId: number): Promise<ProblemDetail> {
  return invoke("get_problem_detail", { problemId });
}

export function recordRating(args: {
  problemId: number;
  rating: Rating;
  idempotencyKey: string;
}): Promise<ScheduleState> {
  return invoke("record_rating", { args });
}

export function updateSettings(args: {
  timezoneId: string;
  desiredRetention: number;
}): Promise<AppSettings> {
  return invoke("update_settings", { args });
}

export function regeneratePairingCode(): Promise<AppSettings> {
  return invoke("regenerate_pairing_code");
}

export function listPendingCompletions(): Promise<PendingCompletion[]> {
  return invoke("list_pending_completions");
}

export function getLoopbackStatus(): Promise<LoopbackStatus> {
  return invoke("get_loopback_status");
}

export function exportBackup(): Promise<BackupDocument> {
  return invoke("export_backup");
}

export function importBackup(document: BackupDocument): Promise<AppSettings> {
  return invoke("import_backup", { document });
}

export function openProblemUrl(url: string): Promise<void> {
  return invoke("open_problem_url", { url });
}

export function newIdempotencyKey(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}
