export type WorkspaceView = "overview" | "events" | "sdp" | "ice" | "stats";

export interface RuntimeStatus {
  appName: string;
  version: string;
  platform: string;
}

export interface PendingDump {
  path: string;
  name: string;
}

export type DumpFormat = "rtc-stats" | "web-rtc-internals";

export interface SessionSummary {
  format: DumpFormat;
  fileName: string;
  fileSize: number;
  compressed: boolean;
  userAgent: string | null;
  peerConnectionCount: number;
  eventCount: number;
  statsSampleCount: number;
  startTime: number | null;
  endTime: number | null;
  warningCount: number;
}

export interface PeerConnectionSummary {
  id: string;
  url: string | null;
  eventCount: number;
  statsSampleCount: number;
}

export interface ImportIssue {
  code: string;
  severity: "fatal" | "error" | "warning" | "info";
  message: string;
  line: number | null;
  jsonPath: string | null;
}

export interface ImportDumpResult {
  sessionId: string;
  summary: SessionSummary;
  peerConnections: PeerConnectionSummary[];
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

export interface ImportDumpError {
  code: string;
  message: string;
}

export interface SessionAnalysis {
  connections: ConnectionAnalysis[];
}

export interface ConnectionAnalysis {
  id: string;
  url: string | null;
  configuration: unknown;
  states: {
    signaling: string | null;
    connection: string | null;
    iceConnection: string | null;
    iceGathering: string | null;
    iceGatheringInferred?: boolean;
  };
  events: Array<{ timestamp: number; eventType: string; value: unknown }>;
  descriptions: Array<{
    timestamp: number;
    eventType: string;
    descriptionType: string;
    sdp: string;
  }>;
  iceCandidates: Array<{
    id: string;
    side: string;
    address: string | null;
    port: number | null;
    protocol: string | null;
    candidateType: string | null;
    networkType: string | null;
    relayProtocol: string | null;
    priority: number | null;
  }>;
  candidatePairs: Array<{
    id: string;
    state: string | null;
    nominated: boolean;
    selected: boolean;
    localCandidateId: string | null;
    remoteCandidateId: string | null;
    currentRoundTripTimeMs: number | null;
    availableOutgoingBitrateKbps: number | null;
    bytesSent: number | null;
    bytesReceived: number | null;
  }>;
  stats: StatsPoint[];
  media: {
    inboundAudio: number;
    inboundVideo: number;
    outboundAudio: number;
    outboundVideo: number;
    codecs: string[];
  };
  findings: Array<{
    severity: "fatal" | "error" | "warning" | "info";
    title: string;
    message: string;
    timestamp: number | null;
  }>;
}

export interface StatsPoint {
  timestamp: number;
  outgoingBitrateKbps: number | null;
  incomingBitrateKbps: number | null;
  packetsLost: number | null;
  currentRoundTripTimeMs: number | null;
  availableOutgoingBitrateKbps: number | null;
  framesPerSecond: number | null;
}

export type WorkspaceMode = "single" | "compare";

export interface SessionSource {
  path: string;
  name: string;
  result: ImportDumpResult;
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
  capabilities: ImportDumpResult["capabilities"];
  issues: ImportIssue[];
  analysis: SessionAnalysis;
}
