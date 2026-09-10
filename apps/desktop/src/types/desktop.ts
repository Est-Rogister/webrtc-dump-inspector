import type { SessionSourceError } from "./workspace";

export interface RuntimeStatus {
  appName: string;
  version: string;
  platform: string;
}

export type ImportJobStatus = "queued" | "running" | "completed" | "cancelled" | "failed";

export interface ImportJobSnapshot {
  id: string;
  status: ImportJobStatus;
  completed: number;
  total: number;
  currentFile: string | null;
  sessionId: string | null;
  errors: SessionSourceError[];
}
