import { AlertTriangle, FileJson, FilePlus2 } from "lucide-react";
import type { ConnectionAnalysis, WorkspaceSession, WorkspaceView } from "@/types";
import { AnalysisViews } from "../analysis/AnalysisViews";

interface SessionWorkspaceProps {
  session: WorkspaceSession;
  selectedConnection: ConnectionAnalysis | null;
  activeView: WorkspaceView;
  isImporting: boolean;
  onAddSource: () => void;
  onSelectConnection: (connectionId: string) => void;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDumpType(format: WorkspaceSession["summary"]["formats"][number]) {
  return format === "rtc-stats" ? "RTCStats" : "WebRTC Internals";
}

export function SessionWorkspace({
  session,
  selectedConnection,
  activeView,
  isImporting,
  onAddSource,
  onSelectConnection,
}: SessionWorkspaceProps) {
  return (
    <section className="result-workspace" aria-live="polite">
      <div className="result-heading">
        <div><h2>{session.name}</h2><p>{session.summary.formats.map(formatDumpType).join(" + ")} · {session.summary.sourceCount} 个数据源 · {formatBytes(session.summary.fileSize)}</p></div>
        <div className="heading-actions">
          <button className="secondary-button" type="button" onClick={onAddSource} disabled={isImporting}><FilePlus2 size={16} />添加数据源</button>
          {session.analysis.connections.length > 1 ? (
            <label className="connection-picker"><span>PeerConnection</span><select value={selectedConnection?.id ?? ""} onChange={(event) => onSelectConnection(event.target.value)}>{session.analysis.connections.map((connection) => <option key={connection.id} value={connection.id}>{connection.id}</option>)}</select></label>
          ) : <span className="success-badge">连接 {selectedConnection?.id ?? "无"}</span>}
        </div>
      </div>
      <div className="source-strip">
        {session.sources.map((source) => <span key={source.id}><FileJson size={13} />{source.name}</span>)}
        {session.sourceErrors.map((source) => <span className="failed" key={source.path}><AlertTriangle size={13} />{source.name}</span>)}
      </div>
      <div className="session-facts"><span>{session.summary.peerConnectionCount} 个连接</span><span>{session.summary.eventCount} 条事件</span><span>{session.summary.statsSampleCount} 份 Stats</span>{session.summary.warningCount > 0 && <span>{session.summary.warningCount} 个警告</span>}</div>
      {selectedConnection ? <AnalysisViews activeView={activeView} connection={selectedConnection} result={session} /> : <div className="view-empty">该 Session 只包含客户端事件，没有 PeerConnection。</div>}
      {session.issues.length > 0 && <div className="issue-list">{session.issues.map((issue, index) => <div className="issue-item" key={`${issue.code}-${index}`}><AlertTriangle size={15} /><span>{issue.message}</span><code>{issue.code}</code></div>)}</div>}
    </section>
  );
}
