import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import type { ConnectionAnalysis, WorkspaceSession, WorkspaceView } from "@/types";

const ComparisonStatsCharts = lazy(() => import("./ComparisonStatsCharts"));

interface ComparisonViewsProps {
  activeView: WorkspaceView;
  baseline: WorkspaceSession;
  target: WorkspaceSession;
}

function duration(session: WorkspaceSession) {
  const { startTime, endTime } = session.summary;
  return startTime == null || endTime == null ? null : Math.max(0, endTime - startTime);
}

function selectedPair(connection: ConnectionAnalysis | null) {
  return connection?.candidatePairs.find((pair) => pair.selected)
    ?? connection?.candidatePairs.find((pair) => pair.nominated)
    ?? null;
}

function valueText(value: unknown) {
  if (value === null || value === undefined || value === "") return "-";
  if (Array.isArray(value)) return value.join(", ") || "-";
  return String(value);
}

function OverviewComparison({ baseline, target, left, right }: ComparisonViewsProps & { left: ConnectionAnalysis | null; right: ConnectionAnalysis | null }) {
  const leftPair = selectedPair(left);
  const rightPair = selectedPair(right);
  const rows: Array<[string, unknown, unknown]> = [
    ["数据源", baseline.summary.sourceCount, target.summary.sourceCount],
    ["持续时间", duration(baseline) == null ? null : `${(duration(baseline)! / 1000).toFixed(1)} s`, duration(target) == null ? null : `${(duration(target)! / 1000).toFixed(1)} s`],
    ["PeerConnection", baseline.summary.peerConnectionCount, target.summary.peerConnectionCount],
    ["事件", baseline.summary.eventCount, target.summary.eventCount],
    ["Stats 点", baseline.summary.statsSampleCount, target.summary.statsSampleCount],
    ["连接状态", left?.states.connection, right?.states.connection],
    ["ICE 状态", left?.states.iceConnection, right?.states.iceConnection],
    ["ICE 收集", left?.states.iceGathering, right?.states.iceGathering],
    ["候选数量", left?.iceCandidates.length, right?.iceCandidates.length],
    ["选中候选对", leftPair?.id, rightPair?.id],
    ["当前 RTT", leftPair?.currentRoundTripTimeMs == null ? null : `${leftPair.currentRoundTripTimeMs.toFixed(1)} ms`, rightPair?.currentRoundTripTimeMs == null ? null : `${rightPair.currentRoundTripTimeMs.toFixed(1)} ms`],
    ["编解码器", left?.media.codecs, right?.media.codecs],
  ];
  return (
    <div className="comparison-table">
      <div className="comparison-row comparison-head"><span>指标</span><strong>{baseline.name}</strong><strong>{target.name}</strong></div>
      {rows.map(([label, leftValue, rightValue]) => (
        <div className="comparison-row" key={label}>
          <span>{label}</span><strong>{valueText(leftValue)}</strong><strong>{valueText(rightValue)}</strong>
        </div>
      ))}
    </div>
  );
}

function EventComparison({ baseline, target, left, right }: ComparisonViewsProps & { left: ConnectionAnalysis | null; right: ConnectionAnalysis | null }) {
  return (
    <div className="comparison-columns">
      {[[baseline, left], [target, right]].map(([session, connection]) => {
        const typedSession = session as WorkspaceSession;
        const typedConnection = connection as ConnectionAnalysis | null;
        return (
          <section className="comparison-pane" key={typedSession.id}>
            <h3><i style={{ background: typedSession.color }} />{typedSession.name}<span>{typedConnection?.events.length ?? 0} 条</span></h3>
            <div className="compact-events">
              {typedConnection?.events.slice(0, 120).map((event, index) => (
                <div key={`${event.timestamp}-${event.eventType}-${index}`}><time>{new Date(event.timestamp).toLocaleTimeString([], { hour12: false })}</time><strong>{event.eventType}</strong></div>
              )) ?? <p>无事件</p>}
            </div>
          </section>
        );
      })}
    </div>
  );
}

function SdpComparison({ baseline, target, left, right }: ComparisonViewsProps & { left: ConnectionAnalysis | null; right: ConnectionAnalysis | null }) {
  return (
    <div className="comparison-columns">
      {[[baseline, left], [target, right]].map(([session, connection]) => {
        const typedSession = session as WorkspaceSession;
        const typedConnection = connection as ConnectionAnalysis | null;
        const description = typedConnection?.descriptions.at(-1);
        return (
          <section className="comparison-pane" key={typedSession.id}>
            <h3><i style={{ background: typedSession.color }} />{typedSession.name}<span>{description?.descriptionType ?? "无 SDP"}</span></h3>
            {description ? <pre className="compare-sdp">{description.sdp}</pre> : <div className="view-empty compact">没有可用 SDP。</div>}
          </section>
        );
      })}
    </div>
  );
}

function IceComparison({ baseline, target, left, right }: ComparisonViewsProps & { left: ConnectionAnalysis | null; right: ConnectionAnalysis | null }) {
  return (
    <div className="comparison-columns">
      {[[baseline, left], [target, right]].map(([session, connection]) => {
        const typedSession = session as WorkspaceSession;
        const typedConnection = connection as ConnectionAnalysis | null;
        const pair = selectedPair(typedConnection);
        const typeCounts = typedConnection?.iceCandidates.reduce<Record<string, number>>((counts, candidate) => {
          const key = candidate.candidateType ?? "unknown";
          counts[key] = (counts[key] ?? 0) + 1;
          return counts;
        }, {}) ?? {};
        return (
          <section className="comparison-pane" key={typedSession.id}>
            <h3><i style={{ background: typedSession.color }} />{typedSession.name}<span>{typedConnection?.iceCandidates.length ?? 0} 条候选</span></h3>
            <dl className="key-values compare-values">
              <div><dt>ICE 状态</dt><dd>{typedConnection?.states.iceConnection ?? "无记录"}</dd></div>
              <div><dt>ICE 收集</dt><dd>{typedConnection?.states.iceGathering ?? "无记录"}{typedConnection?.states.iceGatheringInferred ? "（推断）" : ""}</dd></div>
              <div><dt>候选类型</dt><dd>{Object.entries(typeCounts).map(([type, count]) => `${type} ${count}`).join(" / ") || "-"}</dd></div>
              <div><dt>选中路径</dt><dd>{pair ? `${pair.localCandidateId ?? "?"} → ${pair.remoteCandidateId ?? "?"}` : "未识别"}</dd></div>
              <div><dt>RTT</dt><dd>{pair?.currentRoundTripTimeMs == null ? "-" : `${pair.currentRoundTripTimeMs.toFixed(1)} ms`}</dd></div>
              <div><dt>可用上行</dt><dd>{pair?.availableOutgoingBitrateKbps == null ? "-" : `${pair.availableOutgoingBitrateKbps.toFixed(0)} kbps`}</dd></div>
            </dl>
          </section>
        );
      })}
    </div>
  );
}

export function ComparisonViews({ activeView, baseline, target }: ComparisonViewsProps) {
  const [leftId, setLeftId] = useState(baseline.analysis.connections[0]?.id ?? "");
  const [rightId, setRightId] = useState(target.analysis.connections[0]?.id ?? "");
  useEffect(() => setLeftId(baseline.analysis.connections[0]?.id ?? ""), [baseline]);
  useEffect(() => setRightId(target.analysis.connections[0]?.id ?? ""), [target]);
  const left = useMemo(() => baseline.analysis.connections.find((connection) => connection.id === leftId) ?? null, [baseline, leftId]);
  const right = useMemo(() => target.analysis.connections.find((connection) => connection.id === rightId) ?? null, [rightId, target]);

  return (
    <div className="analysis-view comparison-view">
      <div className="compare-connection-bar">
        <label><span>{baseline.name}</span><select value={leftId} onChange={(event) => setLeftId(event.target.value)}>{baseline.analysis.connections.map((connection) => <option key={connection.id}>{connection.id}</option>)}</select></label>
        <label><span>{target.name}</span><select value={rightId} onChange={(event) => setRightId(event.target.value)}>{target.analysis.connections.map((connection) => <option key={connection.id}>{connection.id}</option>)}</select></label>
      </div>
      {activeView === "events" && <EventComparison activeView={activeView} baseline={baseline} target={target} left={left} right={right} />}
      {activeView === "sdp" && <SdpComparison activeView={activeView} baseline={baseline} target={target} left={left} right={right} />}
      {activeView === "ice" && <IceComparison activeView={activeView} baseline={baseline} target={target} left={left} right={right} />}
      {activeView === "stats" && (
        <Suspense fallback={<div className="view-empty">正在加载对比图表...</div>}>
          <ComparisonStatsCharts series={[{ session: baseline, points: left?.stats ?? [] }, { session: target, points: right?.stats ?? [] }]} />
        </Suspense>
      )}
      {activeView === "overview" && <OverviewComparison activeView={activeView} baseline={baseline} target={target} left={left} right={right} />}
    </div>
  );
}
