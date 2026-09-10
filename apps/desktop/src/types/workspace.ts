import type { DumpFormat, ImportIssue, SessionAnalysis } from "./rtc";

export type WorkspaceView = "overview" | "events" | "sdp" | "ice" | "stats";

export type WorkspaceMode = "single" | "compare";

export interface SessionSource {
  id: string;
  path: string;
  name: string;
  format: DumpFormat;
  fileSize: number;
  compressed: boolean;
}

export interface ImportDumpError {
  code: string;
  message: string;
}

export interface SessionSourceError {
  path: string;
  name: string;
  error: ImportDumpError;
}

export interface WorkspaceSessionSummary {
  sourceCount: number;
  fileNames: string[];
  formats: DumpFormat[];
  fileSize: number;
  userAgent: string | null;
  peerConnectionCount: number;
  eventCount: number;
  statsSampleCount: number;
  startTime: number | null;
  endTime: number | null;
  warningCount: number;
}

export interface WorkspaceSession {
  id: string;
  name: string;
  color: string;
  sources: SessionSource[];
  sourceErrors: SessionSourceError[];
  summary: WorkspaceSessionSummary;
  capabilities: {
    hasApiTrace: boolean;
    hasRawSdp: boolean;
    hasStats: boolean;
    hasCandidateAddress: boolean;
    isPossiblyTruncated: boolean;
  };
  issues: ImportIssue[];
  analysis: SessionAnalysis;
}
