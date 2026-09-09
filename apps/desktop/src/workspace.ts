import type {
  ConnectionAnalysis,
  ImportDumpResult,
  ImportIssue,
  SessionSource,
  SessionSourceError,
  WorkspaceSession,
} from "./types";

export const SESSION_COLORS = ["#147a4b", "#2676b8", "#c07818", "#a34f78", "#5f6db3", "#a6533d"];

export function displayName(path: string) {
  return path.split(/[\\/]/).at(-1) || path;
}

function stableValue(value: unknown) {
  try {
    const normalize = (candidate: unknown): unknown => {
      if (Array.isArray(candidate)) return candidate.map(normalize);
      if (candidate && typeof candidate === "object") {
        return Object.fromEntries(
          Object.entries(candidate as Record<string, unknown>)
            .sort(([left], [right]) => left.localeCompare(right))
            .map(([key, nested]) => [key, normalize(nested)]),
        );
      }
      return candidate;
    };
    return JSON.stringify(normalize(value)) ?? String(value);
  } catch {
    return String(value);
  }
}

function uniqueBy<T>(values: T[], key: (value: T) => string) {
  const seen = new Set<string>();
  return values.filter((value) => {
    const identity = key(value);
    if (seen.has(identity)) return false;
    seen.add(identity);
    return true;
  });
}

function mergeConnection(current: ConnectionAnalysis, incoming: ConnectionAnalysis): ConnectionAnalysis {
  const events = uniqueBy(
    [...current.events, ...incoming.events].sort((a, b) => a.timestamp - b.timestamp),
    (event) => `${event.eventType}:${stableValue(event.value)}:${Math.round(event.timestamp)}`,
  );
  const descriptions = uniqueBy(
    [...current.descriptions, ...incoming.descriptions].sort((a, b) => a.timestamp - b.timestamp),
    (description) => `${description.descriptionType}:${description.sdp}`,
  );
  const iceCandidates = uniqueBy(
    [...current.iceCandidates, ...incoming.iceCandidates],
    (candidate) => [candidate.side, candidate.address, candidate.port, candidate.protocol, candidate.candidateType].join(":"),
  );
  const pairMap = new Map(current.candidatePairs.map((pair) => [pair.id, pair]));
  for (const pair of incoming.candidatePairs) {
    const existing = pairMap.get(pair.id);
    pairMap.set(pair.id, !existing || pair.selected || pair.nominated ? pair : existing);
  }
  const stats = uniqueBy(
    [...current.stats, ...incoming.stats].sort((a, b) => a.timestamp - b.timestamp),
    (point) => `${point.timestamp}:${stableValue(point)}`,
  );
  const findings = uniqueBy(
    [...current.findings, ...incoming.findings].sort((a, b) => (a.timestamp ?? 0) - (b.timestamp ?? 0)),
    (finding) => `${finding.severity}:${finding.title}:${finding.message}:${finding.timestamp ?? ""}`,
  );
  const iceGathering = incoming.states.iceGathering ?? current.states.iceGathering;
  const inferredGathering = !iceGathering
    && iceCandidates.length > 0
    && [...pairMap.values()].some((pair) => pair.selected || pair.nominated);

  return {
    id: current.id,
    url: incoming.url ?? current.url,
    configuration: incoming.configuration ?? current.configuration,
    states: {
      signaling: incoming.states.signaling ?? current.states.signaling,
      connection: incoming.states.connection ?? current.states.connection,
      iceConnection: incoming.states.iceConnection ?? current.states.iceConnection,
      iceGathering: iceGathering ?? (inferredGathering ? "complete" : null),
      iceGatheringInferred: iceGathering ? false : inferredGathering || current.states.iceGatheringInferred,
    },
    events,
    descriptions,
    iceCandidates,
    candidatePairs: [...pairMap.values()],
    stats,
    media: {
      inboundAudio: Math.max(current.media.inboundAudio, incoming.media.inboundAudio),
      inboundVideo: Math.max(current.media.inboundVideo, incoming.media.inboundVideo),
      outboundAudio: Math.max(current.media.outboundAudio, incoming.media.outboundAudio),
      outboundVideo: Math.max(current.media.outboundVideo, incoming.media.outboundVideo),
      codecs: [...new Set([...current.media.codecs, ...incoming.media.codecs])].sort(),
    },
    findings,
  };
}

function aggregateSources(sources: SessionSource[], sourceErrors: SessionSourceError[], id: string, name: string, color: string): WorkspaceSession {
  const connections = new Map<string, ConnectionAnalysis>();
  for (const source of sources) {
    for (const connection of source.result.analysis.connections) {
      const current = connections.get(connection.id);
      connections.set(connection.id, current ? mergeConnection(current, connection) : structuredClone(connection));
    }
  }

  for (const [connectionId, connection] of connections) {
    if (!connection.states.iceGathering
      && connection.iceCandidates.length > 0
      && connection.candidatePairs.some((pair) => pair.selected || pair.nominated)) {
      connections.set(connectionId, {
        ...connection,
        states: { ...connection.states, iceGathering: "complete", iceGatheringInferred: true },
      });
    }
  }

  const issues = uniqueBy(
    sources.flatMap((source) => source.result.issues),
    (issue: ImportIssue) => `${issue.code}:${issue.message}:${issue.line ?? ""}:${issue.jsonPath ?? ""}`,
  );
  const startTimes = sources.flatMap((source) => source.result.summary.startTime == null ? [] : [source.result.summary.startTime]);
  const endTimes = sources.flatMap((source) => source.result.summary.endTime == null ? [] : [source.result.summary.endTime]);

  return {
    id,
    name,
    color,
    sources,
    sourceErrors,
    summary: {
      sourceCount: sources.length,
      fileNames: sources.map((source) => source.name),
      formats: [...new Set(sources.map((source) => source.result.summary.format))],
      fileSize: sources.reduce((total, source) => total + source.result.summary.fileSize, 0),
      userAgent: sources.find((source) => source.result.summary.userAgent)?.result.summary.userAgent ?? null,
      peerConnectionCount: connections.size,
      eventCount: [...connections.values()].reduce((total, connection) => total + connection.events.length, 0),
      statsSampleCount: [...connections.values()].reduce((total, connection) => total + connection.stats.length, 0),
      startTime: startTimes.length ? Math.min(...startTimes) : null,
      endTime: endTimes.length ? Math.max(...endTimes) : null,
      warningCount: issues.filter((issue) => issue.severity === "warning").length + sourceErrors.length,
    },
    capabilities: {
      hasApiTrace: sources.some((source) => source.result.capabilities.hasApiTrace),
      hasRawSdp: sources.some((source) => source.result.capabilities.hasRawSdp),
      hasStats: sources.some((source) => source.result.capabilities.hasStats),
      hasCandidateAddress: sources.some((source) => source.result.capabilities.hasCandidateAddress),
      isPossiblyTruncated: sources.some((source) => source.result.capabilities.isPossiblyTruncated),
    },
    issues,
    analysis: { connections: [...connections.values()] },
  };
}

export function createWorkspaceSession(
  sources: SessionSource[],
  sourceErrors: SessionSourceError[],
  index: number,
): WorkspaceSession {
  const id = sources[0]?.result.sessionId ?? `session-${Date.now()}-${index}`;
  const name = sources.length === 1
    ? sources[0].name
    : `Session ${index + 1}`;
  return aggregateSources(sources, sourceErrors, id, name, SESSION_COLORS[index % SESSION_COLORS.length]);
}

export function appendSessionSources(
  session: WorkspaceSession,
  sources: SessionSource[],
  sourceErrors: SessionSourceError[],
): WorkspaceSession {
  return aggregateSources(
    [...session.sources, ...sources],
    [...session.sourceErrors, ...sourceErrors],
    session.id,
    session.name,
    session.color,
  );
}

export function sourceFromResult(path: string, result: ImportDumpResult): SessionSource {
  return { path, name: displayName(path), result };
}
