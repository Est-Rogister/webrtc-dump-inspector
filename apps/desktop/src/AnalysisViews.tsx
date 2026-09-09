import { lazy, Suspense } from "react";
import { AlertTriangle } from "lucide-react";
import type { ConnectionAnalysis, WorkspaceSession, WorkspaceView } from "./types";

const StatsCharts = lazy(() => import("./StatsCharts"));

interface AnalysisViewsProps {
  activeView: WorkspaceView;
  connection: ConnectionAnalysis;
  result: WorkspaceSession;
}

function formatTime(timestamp: number) {
  if (!Number.isFinite(timestamp)) return "-";
  const date = new Date(timestamp);
  if (timestamp > 1_000_000_000_000 && !Number.isNaN(date.getTime())) {
    return `${date.toLocaleTimeString([], { hour12: false })}.${String(date.getMilliseconds()).padStart(3, "0")}`;
  }
  return `${timestamp.toFixed(1)} ms`;
}

function formatValue(value: unknown) {
  if (value === null || value === undefined) return "-";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function compactEventValue(value: unknown) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return value;
  const record = value as Record<string, unknown>;
  if (typeof record.sdp !== "string") return value;
  return { ...record, sdp: `[SDP ${record.sdp.length} chars, see SDP view]` };
}

function stateTone(value: string | null) {
  if (!value) return "neutral";
  if (["connected", "completed", "stable", "complete"].includes(value)) return "healthy";
  if (["failed", "closed", "disconnected"].includes(value)) return "danger";
  return "pending";
}

function EmptyView({ children }: { children: string }) {
  return <div className="view-empty">{children}</div>;
}

function OverviewView({ connection, result }: Omit<AnalysisViewsProps, "activeView">) {
  const selectedPair = connection.candidatePairs.find((pair) => pair.selected)
    ?? connection.candidatePairs.find((pair) => pair.nominated);
  const duration = result.summary.startTime !== null && result.summary.endTime !== null
    ? Math.max(0, result.summary.endTime - result.summary.startTime)
    : null;
  const media = connection.media;

  return (
    <div className="analysis-view">
      <div className="state-grid">
        {[
          ["连接状态", connection.states.connection],
          ["ICE 状态", connection.states.iceConnection],
          ["信令状态", connection.states.signaling],
          ["ICE 收集", connection.states.iceGatheringInferred ? `${connection.states.iceGathering}（推断）` : connection.states.iceGathering],
        ].map(([label, value]) => (
          <div className="state-item" key={label}>
            <span>{label}</span>
            <strong className={stateTone(value)}>{value ?? "无记录"}</strong>
          </div>
        ))}
      </div>

      {connection.findings.length > 0 && (
        <section className="findings-section">
          <h3>发现问题</h3>
          {connection.findings.map((finding, index) => (
            <div className={`finding ${finding.severity}`} key={`${finding.title}-${index}`}>
              <AlertTriangle size={16} />
              <div><strong>{finding.title}</strong><span>{finding.message}</span></div>
              {finding.timestamp !== null && <time>{formatTime(finding.timestamp)}</time>}
            </div>
          ))}
        </section>
      )}

      <div className="overview-columns">
        <section className="info-section">
          <h3>会话信息</h3>
          <dl className="key-values">
            <div><dt>持续时间</dt><dd>{duration === null ? "-" : `${(duration / 1000).toFixed(1)} s`}</dd></div>
            <div><dt>页面</dt><dd title={connection.url ?? undefined}>{connection.url ?? "未记录"}</dd></div>
            <div><dt>浏览器</dt><dd>{result.summary.userAgent ?? "未记录"}</dd></div>
            <div><dt>编解码器</dt><dd>{media.codecs.join(", ") || "未识别"}</dd></div>
          </dl>
        </section>
        <section className="info-section">
          <h3>媒体与网络</h3>
          <dl className="key-values">
            <div><dt>接收媒体</dt><dd>音频 {media.inboundAudio} / 视频 {media.inboundVideo}</dd></div>
            <div><dt>发送媒体</dt><dd>音频 {media.outboundAudio} / 视频 {media.outboundVideo}</dd></div>
            <div><dt>候选数量</dt><dd>{connection.iceCandidates.length}</dd></div>
            <div><dt>选中候选对</dt><dd>{selectedPair?.id ?? "未识别"}</dd></div>
            <div><dt>当前 RTT</dt><dd>{selectedPair?.currentRoundTripTimeMs == null ? "-" : `${selectedPair.currentRoundTripTimeMs.toFixed(1)} ms`}</dd></div>
          </dl>
        </section>
      </div>
    </div>
  );
}

function EventsView({ connection }: { connection: ConnectionAnalysis }) {
  if (!connection.events.length) return <EmptyView>日志中没有 PeerConnection 事件。</EmptyView>;
  return (
    <div className="analysis-view">
      <div className="view-title"><h3>API 与状态事件</h3><span>{connection.events.length} 条</span></div>
      <div className="data-table event-table">
        <div className="table-head"><span>时间</span><span>事件</span><span>数据</span></div>
        {connection.events.map((event, index) => (
          <div className="table-row" key={`${event.timestamp}-${event.eventType}-${index}`}>
            <time>{formatTime(event.timestamp)}</time>
            <strong>{event.eventType}</strong>
            <pre>{formatValue(compactEventValue(event.value))}</pre>
          </div>
        ))}
      </div>
    </div>
  );
}

function SdpView({ connection }: { connection: ConnectionAnalysis }) {
  if (!connection.descriptions.length) return <EmptyView>日志中没有可用的 SDP offer/answer。</EmptyView>;
  return (
    <div className="analysis-view">
      <div className="view-title"><h3>SDP 协商记录</h3><span>{connection.descriptions.length} 份</span></div>
      <div className="sdp-list">
        {connection.descriptions.map((description, index) => (
          <details key={`${description.timestamp}-${description.eventType}-${index}`} open={index === connection.descriptions.length - 1}>
            <summary>
              <span className={`description-type ${description.descriptionType}`}>{description.descriptionType}</span>
              <strong>{description.eventType}</strong>
              <time>{formatTime(description.timestamp)}</time>
            </summary>
            <pre>{description.sdp}</pre>
          </details>
        ))}
      </div>
    </div>
  );
}

function IceView({ connection }: { connection: ConnectionAnalysis }) {
  const iceEvents = connection.events.filter((event) => event.eventType.toLowerCase().includes("icecandidate"));
  if (!connection.iceCandidates.length && !connection.candidatePairs.length && !iceEvents.length) {
    return <EmptyView>日志中没有可用的 ICE 候选信息。</EmptyView>;
  }
  return (
    <div className="analysis-view">
      <div className="view-title"><h3>候选对</h3><span>{connection.candidatePairs.length} 组</span></div>
      {connection.candidatePairs.length ? (
        <div className="data-table pair-table">
          <div className="table-head"><span>状态</span><span>候选对</span><span>RTT</span><span>可用上行</span></div>
          {connection.candidatePairs.map((pair) => (
            <div className={pair.selected || pair.nominated ? "table-row selected-row" : "table-row"} key={pair.id}>
              <span>{pair.selected ? "选中" : pair.nominated ? "提名" : pair.state ?? "-"}</span>
              <code title={pair.id}>{pair.localCandidateId ?? "?"} → {pair.remoteCandidateId ?? "?"}</code>
              <span>{pair.currentRoundTripTimeMs == null ? "-" : `${pair.currentRoundTripTimeMs.toFixed(1)} ms`}</span>
              <span>{pair.availableOutgoingBitrateKbps == null ? "-" : `${pair.availableOutgoingBitrateKbps.toFixed(0)} kbps`}</span>
            </div>
          ))}
        </div>
      ) : <EmptyView>Stats 中没有候选对报告。</EmptyView>}

      <div className="view-title secondary-title"><h3>ICE 候选</h3><span>{connection.iceCandidates.length} 条</span></div>
      {connection.iceCandidates.length ? (
        <div className="data-table candidate-table">
          <div className="table-head"><span>方向</span><span>地址</span><span>类型</span><span>协议/网络</span></div>
          {connection.iceCandidates.map((candidate) => (
            <div className="table-row" key={candidate.id}>
              <span className={`candidate-side ${candidate.side}`}>{candidate.side === "local" ? "本地" : "远端"}</span>
              <code>{candidate.address ?? "隐藏"}{candidate.port == null ? "" : `:${candidate.port}`}</code>
              <span>{candidate.candidateType ?? "-"}{candidate.relayProtocol ? ` / ${candidate.relayProtocol}` : ""}</span>
              <span>{candidate.protocol ?? "-"}{candidate.networkType ? ` / ${candidate.networkType}` : ""}</span>
            </div>
          ))}
        </div>
      ) : <EmptyView>Stats 中没有候选报告，可在事件视图检查候选信令。</EmptyView>}
    </div>
  );
}

function StatsView({ connection }: { connection: ConnectionAnalysis }) {
  if (!connection.stats.length) return <EmptyView>日志中没有可用的 getStats 数据。</EmptyView>;
  return (
    <div className="analysis-view stats-view">
      <div className="view-title"><h3>实时收发码率</h3><span>{connection.stats.length} 个绘图点</span></div>
      <Suspense fallback={<div className="view-empty">正在加载图表...</div>}>
        <StatsCharts points={connection.stats} />
      </Suspense>
    </div>
  );
}

export function AnalysisViews({ activeView, connection, result }: AnalysisViewsProps) {
  if (activeView === "events") return <EventsView connection={connection} />;
  if (activeView === "sdp") return <SdpView connection={connection} />;
  if (activeView === "ice") return <IceView connection={connection} />;
  if (activeView === "stats") return <StatsView connection={connection} />;
  return <OverviewView connection={connection} result={result} />;
}
