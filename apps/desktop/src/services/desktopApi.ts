import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ImportDumpError,
  ImportJobSnapshot,
  RuntimeStatus,
  WorkspaceSession,
} from "@/types";

const MAX_STATS_POINTS = 2_000;
const IMPORT_POLL_INTERVAL_MS = 100;

export function getRuntimeStatus() {
  return invoke<RuntimeStatus>("runtime_status");
}

export function listSessions() {
  return invoke<WorkspaceSession[]>("list_sessions", { maxStatsPoints: MAX_STATS_POINTS });
}

export function startImport(paths: string[], targetSessionId?: string) {
  return invoke<ImportJobSnapshot>("start_import", {
    paths,
    targetSessionId: targetSessionId ?? null,
  });
}

export function cancelImport(jobId: string) {
  return invoke<ImportJobSnapshot>("cancel_import", { jobId });
}

export function closeSession(sessionId: string) {
  return invoke<boolean>("close_session", { sessionId });
}

export async function selectDumpFiles() {
  const selection = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "WebRTC dump", extensions: ["json", "jsonl", "txt", "gz"] }],
  });
  if (!selection) return [];
  return Array.isArray(selection) ? selection : [selection];
}

export function isImportJobActive(job: ImportJobSnapshot | null) {
  return job?.status === "queued" || job?.status === "running";
}

export async function waitForImport(
  initial: ImportJobSnapshot,
  onProgress: (job: ImportJobSnapshot) => void,
) {
  let current = initial;
  while (isImportJobActive(current)) {
    current = await invoke<ImportJobSnapshot>("get_import_job", { jobId: current.id });
    onProgress(current);
    if (isImportJobActive(current)) {
      await new Promise((resolve) => window.setTimeout(resolve, IMPORT_POLL_INTERVAL_MS));
    }
  }
  return current;
}

export function parseDesktopError(error: unknown): ImportDumpError {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<ImportDumpError>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return {
    code: "desktop.command-failed",
    message: typeof error === "string" ? error : "桌面操作失败",
  };
}
