import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { App } from "./App";
import type { DumpFormat, ImportDumpResult } from "./types";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));

function result(id: string, fileName: string, format: DumpFormat = "rtc-stats"): ImportDumpResult {
  return {
    sessionId: id,
    summary: {
      format,
      fileName,
      fileSize: 2048,
      compressed: true,
      userAgent: "Chrome",
      peerConnectionCount: 1,
      eventCount: 8,
      statsSampleCount: 3,
      startTime: 1000,
      endTime: 2000,
      warningCount: 0,
    },
    peerConnections: [{ id: "pc-1", url: null, eventCount: 8, statsSampleCount: 3 }],
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
        states: { signaling: "stable", connection: "connected", iceConnection: "connected", iceGathering: null },
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

describe("App", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.open.mockReset();
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.1.0", platform: "macos" });
      return Promise.reject(new Error("unexpected command"));
    });
  });

  it("renders an empty multi-session workspace", async () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "概览" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "新建 Session" }).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "对比" })).toBeDisabled();
    expect(await screen.findByText("macos · 0.1.0")).toBeInTheDocument();
  });

  it("groups files selected together into one session", async () => {
    mocks.open.mockResolvedValue(["/tmp/rtc.gz", "/tmp/internals.gz"]);
    mocks.invoke.mockImplementation((command: string, args?: { path: string }) => {
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.1.0", platform: "macos" });
      return Promise.resolve(result(args?.path.includes("internals") ? "source-2" : "source-1", args?.path.split("/").at(-1) ?? "dump", args?.path.includes("internals") ? "web-rtc-internals" : "rtc-stats"));
    });

    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);

    expect(await screen.findByRole("heading", { name: "Session 1" })).toBeInTheDocument();
    expect(screen.getAllByText(/2 个数据源/).length).toBeGreaterThan(0);
    expect(screen.getByText("RTCStats + WebRTC Internals · 2 个数据源 · 4.0 KB")).toBeInTheDocument();
    expect(screen.getByText("complete（推断）")).toBeInTheDocument();
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(3));
  });

  it("creates separate sessions on later imports and compares them", async () => {
    mocks.open.mockResolvedValueOnce("/tmp/call-a.gz").mockResolvedValueOnce("/tmp/call-b.gz");
    let importCount = 0;
    mocks.invoke.mockImplementation((command: string, args?: { path: string }) => {
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.1.0", platform: "macos" });
      importCount += 1;
      return Promise.resolve(result(`session-${importCount}`, args?.path.split("/").at(-1) ?? "dump"));
    });

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
      if (command === "runtime_status") return Promise.resolve({ appName: "RTC Inspector", version: "0.1.0", platform: "macos" });
      return Promise.reject({ code: "dump.unknown-format", message: "unrecognized dump format" });
    });

    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "新建 Session" })[0]);
    expect(await screen.findByRole("alert")).toHaveTextContent("unrecognized dump format");
    expect(screen.getByRole("alert")).toHaveTextContent("dump.unknown-format");
  });
});
