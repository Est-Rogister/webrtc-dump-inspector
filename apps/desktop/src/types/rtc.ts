export type DumpFormat = "rtc-stats" | "web-rtc-internals";

export type IssueSeverity = "fatal" | "error" | "warning" | "info";

export interface ImportIssue {
  code: string;
  severity: IssueSeverity;
  message: string;
  line: number | null;
  jsonPath: string | null;
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
    severity: IssueSeverity;
    title: string;
    message: string;
    timestamp: number | null;
  }>;
}

export interface SessionAnalysis {
  connections: ConnectionAnalysis[];
}
