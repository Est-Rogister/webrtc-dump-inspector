import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { App } from "./App";
import type { DumpFormat, ImportJobSnapshot, WorkspaceSession } from "@/types";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));

function session(id: string, fileNames: string[], formats: DumpFormat[] = ["rtc-stats"]): WorkspaceSession {
  return {
    id,
    name: fileNames.length === 1 ? fileNames[0] : `Session ${id.split("-").at(-1)}`,
    color: "#147a4b",
    sources: fileNames.map((name, index) => ({
      id: `${id}-source-${index}`,
      path: `/tmp/${name}`,
      name,
      format: formats[index] ?? formats[0],
      fileSize: 2048,
      compressed: true,
    })),
    sourceErrors: [],
    summary: {
      sourceCount: fileNames.length,
      fileNames,
      formats,
      fileSize: fileNames.length * 2048,
      userAgent: "Chrome",
      peerConnectionCount: 1,
      eventCount: 1,
      statsSampleCount: 3,
      startTime: 1000,
      endTime: 2000,
      warningCount: 0,
    },
    capabilities: {
      hasApiTrace: true,
      hasRawSdp: true,
      hasStats: true,
      hasCandidateAddress: true,
      isPossiblyTruncated: false,
    },
    issues: [],
    analysis: {
      connections: [{
        id: "pc-1",
        url: "https://example.test/call",
        configuration: {},
        states: {
          signaling: "stable",
          connection: "connected",
          iceConnection: "connected",
          iceGathering: "complete",
          iceGatheringInferred: true,
        },
        events: [{ timestamp: 1000, eventType: "createOffer", value: {} }],
        descriptions: [],
        iceCandidates: [{ id: "local", side: "local", address: "10.0.0.1", port: 5000, protocol: "udp", candidateType: "host", networkType: "wifi", relayProtocol: null, priority: 1 }],
        candidatePairs: [{ id: "pair", state: "succeeded", nominated: true, selected: true, localCandidateId: "local", remoteCandidateId: "remote", currentRoundTripTimeMs: 12, availableOutgoingBitrateKbps: 1500, bytesSent: 100, bytesReceived: 200 }],
        stats: [],
        media: { inboundAudio: 1, inboundVideo: 0, outboundAudio: 1, outboundVideo: 0, codecs: ["audio/opus"] },
        findings: [],
      }],
    },
  };
}

function completedJob(id: string): ImportJobSnapshot {
  return {
    id: `job-${id}`,
    status: "completed",
    completed: 1,
    total: 1,
    currentFile: null,
    sessionId: id,
    errors: [],
  };
}

function installBackend() {
  const sessions: WorkspaceSession[] = [];
  mocks.invoke.mockImplementation((command: string, args?: { paths?: string[]; targetSessionId?: string }) => {
    if (command === "runtime_status") {
      return Promise.resolve({ appName: "RTC Inspector", version: "0.2.0", platform: "macos" });
    }
    if (command === "list_sessions") return Promise.resolve([...sessions]);
    if (command === "start_import") {
      const paths = args?.paths ?? [];
      const names = paths.map((path) => path.split("/").at(-1) ?? path);
      const target = sessions.find((candidate) => candidate.id === args?.targetSessionId);
      if (target) {
        target.sources.push(...names.map((name, index) => ({ id: `added-${index}`, path: paths[index], name, format: "rtc-stats" as const, fileSize: 2048, compressed: true })));
        target.summary.sourceCount = target.sources.length;
        target.summary.fileSize = target.sources.length * 2048;
        return Promise.resolve(completedJob(target.id));
      }
      const id = `session-${sessions.length + 1}`;
      sessions.push(session(id, names, names.map((name) => name.includes("internals") ? "web-rtc-internals" : "rtc-stats")));
      return Promise.resolve({ ...completedJob(id), completed: paths.length, total: paths.length });
    }
    if (command === "close_session") return Promise.resolve(true);
    return Promise.reject(new Error(`unexpected command: ${command}`));
  });
}

describe("App", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.open.mockReset();
    installBackend();
  });

  it("renders an empty Rust-backed workspace", async () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "概览" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "新建 Session" }).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "对比" })).toBeDisabled();
    expect(await screen.findByText("macos · 0.2.0")).toBeInTheDocument();
  });

  it("groups files selected together into one Rust session", async () => {
    mocks.open.mockResolvedValue(["/tmp/rtc.gz", "/tmp/internals.gz"]);
    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);

    expect(await screen.findByRole("heading", { name: "Session 1" })).toBeInTheDocument();
    expect(screen.getAllByText(/2 个数据源/).length).toBeGreaterThan(0);
    expect(screen.getByText("RTCStats + WebRTC Internals · 2 个数据源 · 4.0 KB")).toBeInTheDocument();
    expect(screen.getByText("complete（推断）")).toBeInTheDocument();
  });

  it("creates separate sessions on later imports and compares them", async () => {
    mocks.open.mockResolvedValueOnce("/tmp/call-a.gz").mockResolvedValueOnce("/tmp/call-b.gz");
    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);
    expect(await screen.findByRole("heading", { name: "call-a.gz" })).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);
    expect(await screen.findByRole("heading", { name: "call-b.gz" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "对比" }));
    expect(await screen.findByText("Baseline")).toBeInTheDocument();
    expect(screen.getByText("Target")).toBeInTheDocument();
    expect(screen.getAllByText("call-a.gz").length).toBeGreaterThan(0);
    expect(screen.getAllByText("call-b.gz").length).toBeGreaterThan(0);
  });

  it("renders structured parser errors when every source fails", async () => {
    mocks.open.mockResolvedValue("/tmp/broken.txt");
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.2.0", platform: "macos" });
      if (command === "list_sessions") return Promise.resolve([]);
      if (command === "start_import") {
        return Promise.resolve({
          id: "job-failed",
          status: "failed",
          completed: 1,
          total: 1,
          currentFile: null,
          sessionId: null,
          errors: [{ path: "/tmp/broken.txt", name: "broken.txt", error: { code: "dump.unknown-format", message: "unrecognized dump format" } }],
        });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);
    expect(await screen.findByRole("alert")).toHaveTextContent("unrecognized dump format");
    expect(screen.getByRole("alert")).toHaveTextContent("dump.unknown-format");
  });

  it("polls a running import job before refreshing sessions", async () => {
    mocks.open.mockResolvedValue("/tmp/call.gz");
    const ready = session("session-1", ["call.gz"]);
    let polls = 0;
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.2.0", platform: "macos" });
      if (command === "start_import") return Promise.resolve({ ...completedJob("session-1"), status: "running", completed: 0 });
      if (command === "get_import_job") {
        polls += 1;
        return Promise.resolve(completedJob("session-1"));
      }
      if (command === "list_sessions") return Promise.resolve(polls ? [ready] : []);
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);
    expect(await screen.findByRole("heading", { name: "call.gz" })).toBeInTheDocument();
    await waitFor(() => expect(polls).toBe(1));
  });
});
