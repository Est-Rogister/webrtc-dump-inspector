import { describe, expect, it } from "vitest";
import type { DumpFormat, ImportDumpResult } from "./types";
import { createWorkspaceSession, sourceFromResult } from "./workspace";

function sourceResult(id: string, format: DumpFormat): ImportDumpResult {
  return {
    sessionId: id,
    summary: { format, fileName: `${id}.gz`, fileSize: 100, compressed: true, userAgent: "Chrome", peerConnectionCount: 1, eventCount: 1, statsSampleCount: 1, startTime: 1000, endTime: 2000, warningCount: 0 },
    peerConnections: [{ id: "19-1", url: null, eventCount: 1, statsSampleCount: 1 }],
    capabilities: { hasApiTrace: true, hasRawSdp: true, hasStats: true, hasCandidateAddress: true, isPossiblyTruncated: false },
    issues: [],
    analysis: { connections: [{
      id: "19-1",
      url: "https://example.test",
      configuration: {},
      states: { signaling: "stable", connection: "connected", iceConnection: "connected", iceGathering: null },
      events: [{ timestamp: 1000, eventType: "setLocalDescription", value: { type: "offer" } }],
      descriptions: [{ timestamp: 1000, eventType: "setLocalDescription", descriptionType: "offer", sdp: "v=0" }],
      iceCandidates: [{ id: "local", side: "local", address: "10.0.0.1", port: 5000, protocol: "udp", candidateType: "host", networkType: "wifi", relayProtocol: null, priority: 1 }],
      candidatePairs: [{ id: "pair", state: "succeeded", nominated: true, selected: true, localCandidateId: "local", remoteCandidateId: "remote", currentRoundTripTimeMs: 10, availableOutgoingBitrateKbps: 1000, bytesSent: 10, bytesReceived: 20 }],
      stats: [{ timestamp: 1000, outgoingBitrateKbps: 100, incomingBitrateKbps: 90, packetsLost: 0, currentRoundTripTimeMs: 10, availableOutgoingBitrateKbps: 1000, framesPerSecond: 30 }],
      media: { inboundAudio: 1, inboundVideo: 0, outboundAudio: 1, outboundVideo: 1, codecs: ["audio/opus"] },
      findings: [],
    }] },
  };
}

describe("workspace aggregation", () => {
  it("keeps sources but deduplicates one logical connection", () => {
    const rtc = sourceResult("rtc", "rtc-stats");
    const internals = sourceResult("internals", "web-rtc-internals");
    const session = createWorkspaceSession([
      sourceFromResult("/tmp/rtc.gz", rtc),
      sourceFromResult("/tmp/internals.gz", internals),
    ], [], 0);

    expect(session.sources).toHaveLength(2);
    expect(session.analysis.connections).toHaveLength(1);
    expect(session.analysis.connections[0].descriptions).toHaveLength(1);
    expect(session.analysis.connections[0].iceCandidates).toHaveLength(1);
    expect(session.analysis.connections[0].stats).toHaveLength(1);
    expect(session.analysis.connections[0].states).toMatchObject({ iceGathering: "complete", iceGatheringInferred: true });
    expect(session.summary.formats).toEqual(["rtc-stats", "web-rtc-internals"]);
  });
});
