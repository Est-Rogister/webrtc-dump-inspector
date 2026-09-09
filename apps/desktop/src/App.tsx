import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity,
  AlertTriangle,
  Braces,
  ChartNoAxesCombined,
  FileJson,
  FilePlus2,
  FolderOpen,
  Gauge,
  GitCompareArrows,
  LoaderCircle,
  Plus,
  Radio,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { AnalysisViews } from "./AnalysisViews";
import { ComparisonViews } from "./ComparisonViews";
import { RtcLogoMark } from "./RtcLogoMark";
import type {
  ImportDumpError,
  ImportDumpResult,
  RuntimeStatus,
  SessionSource,
  SessionSourceError,
  WorkspaceMode,
  WorkspaceSession,
  WorkspaceView,
} from "./types";
import { appendSessionSources, createWorkspaceSession, displayName, sourceFromResult } from "./workspace";

const views: Array<{ id: WorkspaceView; label: string; icon: typeof Gauge }> = [
  { id: "overview", label: "概览", icon: Gauge },
  { id: "events", label: "事件", icon: Activity },
  { id: "sdp", label: "SDP", icon: Braces },
  { id: "ice", label: "ICE", icon: Radio },
  { id: "stats", label: "Stats", icon: ChartNoAxesCombined },
];

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDumpType(format: ImportDumpResult["summary"]["format"]) {
  return format === "rtc-stats" ? "RTCStats" : "WebRTC Internals";
}

function parseImportError(error: unknown): ImportDumpError {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<ImportDumpError>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code, message: candidate.message };
    }
  }
  return { code: "dump.import-failed", message: typeof error === "string" ? error : "日志导入失败" };
}

function normalizeSelection(selection: string | string[] | null): string[] {
  if (!selection) return [];
  return Array.isArray(selection) ? selection : [selection];
}

export function App() {
  const [activeView, setActiveView] = useState<WorkspaceView>("overview");
  const [mode, setMode] = useState<WorkspaceMode>("single");
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [sessions, setSessions] = useState<WorkspaceSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [selectedConnectionIds, setSelectedConnectionIds] = useState<Record<string, string>>({});
  const [baselineId, setBaselineId] = useState("");
  const [targetId, setTargetId] = useState("");
  const [importError, setImportError] = useState<ImportDumpError | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [pendingNames, setPendingNames] = useState<string[]>([]);

  useEffect(() => {
    invoke<RuntimeStatus>("runtime_status")
      .then(setRuntime)
      .catch(() => setRuntime({ appName: "RTC Inspector", version: "web-preview", platform: "browser" }));
  }, []);

  const activeLabel = useMemo(() => views.find((view) => view.id === activeView)?.label ?? "概览", [activeView]);
  const activeSession = useMemo(
    () => sessions.find((session) => session.id === activeSessionId) ?? sessions[0] ?? null,
    [activeSessionId, sessions],
  );
  const selectedConnection = useMemo(() => {
    if (!activeSession) return null;
    const selectedId = selectedConnectionIds[activeSession.id];
    return activeSession.analysis.connections.find((connection) => connection.id === selectedId)
      ?? activeSession.analysis.connections[0]
      ?? null;
  }, [activeSession, selectedConnectionIds]);
  const baseline = useMemo(
    () => sessions.find((session) => session.id === baselineId) ?? sessions[0] ?? null,
    [baselineId, sessions],
  );
  const target = useMemo(
    () => sessions.find((session) => session.id === targetId && session.id !== baseline?.id)
      ?? sessions.find((session) => session.id !== baseline?.id)
      ?? null,
    [baseline?.id, sessions, targetId],
  );

  async function selectFiles() {
    const selection = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "WebRTC dump", extensions: ["json", "jsonl", "txt", "gz"] }],
    });
    return normalizeSelection(selection);
  }

  async function parseFiles(paths: string[]) {
    const settled = await Promise.allSettled(paths.map((path) => invoke<ImportDumpResult>("import_dump", { path })));
    const sources: SessionSource[] = [];
    const errors: SessionSourceError[] = [];
    settled.forEach((result, index) => {
      const path = paths[index];
      if (result.status === "fulfilled") sources.push(sourceFromResult(path, result.value));
      else errors.push({ path, name: displayName(path), error: parseImportError(result.reason) });
    });
    return { sources, errors };
  }

  async function importFiles(targetSessionId?: string) {
    setImportError(null);
    try {
      const paths = await selectFiles();
      if (!paths.length) return;
      setPendingNames(paths.map(displayName));
      setIsImporting(true);
      const { sources, errors } = await parseFiles(paths);
      if (!sources.length) {
        setImportError(errors[0]?.error ?? { code: "dump.import-failed", message: "所有文件均导入失败" });
        return;
      }

      if (targetSessionId) {
        setSessions((current) => current.map((session) => (
          session.id === targetSessionId ? appendSessionSources(session, sources, errors) : session
        )));
        setActiveSessionId(targetSessionId);
      } else {
        const session = createWorkspaceSession(sources, errors, sessions.length);
        setSessions((current) => [...current, session]);
        setActiveSessionId(session.id);
        setSelectedConnectionIds((current) => ({ ...current, [session.id]: session.analysis.connections[0]?.id ?? "" }));
        if (!baselineId) setBaselineId(session.id);
        else if (!targetId && session.id !== baselineId) setTargetId(session.id);
      }
      setActiveView("overview");
      setMode("single");
    } catch {
      setImportError({ code: "dialog.open-failed", message: "无法打开文件选择器" });
    } finally {
      setPendingNames([]);
      setIsImporting(false);
    }
  }

  function closeSession(sessionId: string) {
    const remaining = sessions.filter((session) => session.id !== sessionId);
    setSessions(remaining);
    if (activeSession?.id === sessionId) setActiveSessionId(remaining[0]?.id ?? null);
    if (remaining.length < 2) setMode("single");
    if (baselineId === sessionId) setBaselineId(remaining[0]?.id ?? "");
    if (targetId === sessionId) setTargetId(remaining.find((session) => session.id !== remaining[0]?.id)?.id ?? "");
  }

  const canCompare = sessions.length >= 2;

  return (
    <div className="app-shell">
      <header className="titlebar">
        <div className="brand-mark" aria-hidden="true"><RtcLogoMark /></div>
        <div className="brand-copy"><strong>RTC Inspector</strong><span>Dump Analyzer</span></div>
        <div className="titlebar-actions"><span className="runtime-pill">{runtime ? `${runtime.platform} · ${runtime.version}` : "正在连接"}</span></div>
      </header>

      <div className="workspace">
        <aside className="sidebar">
          <div className="session-section">
            <div className="section-heading">
              <span className="section-label">Sessions</span>
              <button className="small-icon-button" type="button" onClick={() => importFiles()} disabled={isImporting} aria-label="新建 Session" title="新建 Session"><Plus size={15} /></button>
            </div>
            <div className="session-list">
              {sessions.map((session) => (
                <div className={activeSession?.id === session.id ? "session-entry active" : "session-entry"} key={session.id}>
                  <button className="session-main" type="button" onClick={() => { setActiveSessionId(session.id); setMode("single"); }}>
                    <i style={{ background: session.color }} />
                    <span><strong>{session.name}</strong><small>{session.summary.sourceCount} 个数据源 · {session.summary.peerConnectionCount} 个连接</small></span>
                  </button>
                  <button className="session-close" type="button" onClick={() => closeSession(session.id)} aria-label={`关闭 ${session.name}`} title="关闭 Session"><X size={13} /></button>
                </div>
              ))}
              {!sessions.length && <div className="session-empty">尚未创建 Session</div>}
            </div>
          </div>

          <nav className="nav-list" aria-label="分析视图">
            {views.map((view) => {
              const Icon = view.icon;
              return <button className={activeView === view.id ? "nav-item active" : "nav-item"} key={view.id} type="button" disabled={!activeSession} onClick={() => setActiveView(view.id)}><Icon size={17} /><span>{view.label}</span></button>;
            })}
          </nav>
          <div className="sidebar-footer"><span>本地模式</span><i aria-hidden="true" /></div>
        </aside>

        <main className="content">
          <div className="content-toolbar">
            <div><span className="breadcrumb">{mode === "single" ? "Session 分析" : "Session 对比"}</span><h1>{activeLabel}</h1></div>
            <div className="toolbar-actions">
              <div className="mode-switch" aria-label="工作模式">
                <button className={mode === "single" ? "active" : ""} type="button" onClick={() => setMode("single")}><Gauge size={15} />单会话</button>
                <button className={mode === "compare" ? "active" : ""} type="button" disabled={!canCompare} onClick={() => setMode("compare")} title={canCompare ? "对比 Session" : "至少需要两个 Session"}><GitCompareArrows size={15} />对比</button>
              </div>
              <button className="primary-button" type="button" onClick={() => importFiles()} disabled={isImporting}>{isImporting ? <LoaderCircle className="spin" size={17} /> : <Plus size={17} />}{isImporting ? "正在导入" : "新建 Session"}</button>
            </div>
          </div>

          {mode === "compare" && baseline && target ? (
            <section className="result-workspace" aria-live="polite">
              <div className="compare-session-picker">
                <label><span>Baseline</span><select value={baseline.id} onChange={(event) => setBaselineId(event.target.value)}>{sessions.map((session) => <option key={session.id} value={session.id}>{session.name}</option>)}</select></label>
                <GitCompareArrows size={18} />
                <label><span>Target</span><select value={target.id} onChange={(event) => setTargetId(event.target.value)}>{sessions.filter((session) => session.id !== baseline.id).map((session) => <option key={session.id} value={session.id}>{session.name}</option>)}</select></label>
              </div>
              <ComparisonViews activeView={activeView} baseline={baseline} target={target} />
            </section>
          ) : activeSession ? (
            <section className="result-workspace" aria-live="polite">
              <div className="result-heading">
                <div><h2>{activeSession.name}</h2><p>{activeSession.summary.formats.map(formatDumpType).join(" + ")} · {activeSession.summary.sourceCount} 个数据源 · {formatBytes(activeSession.summary.fileSize)}</p></div>
                <div className="heading-actions">
                  <button className="secondary-button" type="button" onClick={() => importFiles(activeSession.id)} disabled={isImporting}><FilePlus2 size={16} />添加数据源</button>
                  {activeSession.analysis.connections.length > 1 ? (
                    <label className="connection-picker"><span>PeerConnection</span><select value={selectedConnection?.id ?? ""} onChange={(event) => setSelectedConnectionIds((current) => ({ ...current, [activeSession.id]: event.target.value }))}>{activeSession.analysis.connections.map((connection) => <option key={connection.id} value={connection.id}>{connection.id}</option>)}</select></label>
                  ) : <span className="success-badge">连接 {selectedConnection?.id ?? "无"}</span>}
                </div>
              </div>
              <div className="source-strip">
                {activeSession.sources.map((source) => <span key={source.path}><FileJson size={13} />{source.name}</span>)}
                {activeSession.sourceErrors.map((source) => <span className="failed" key={source.path}><AlertTriangle size={13} />{source.name}</span>)}
              </div>
              <div className="session-facts"><span>{activeSession.summary.peerConnectionCount} 个连接</span><span>{activeSession.summary.eventCount} 条事件</span><span>{activeSession.summary.statsSampleCount} 份 Stats</span>{activeSession.summary.warningCount > 0 && <span>{activeSession.summary.warningCount} 个警告</span>}</div>
              {selectedConnection ? <AnalysisViews activeView={activeView} connection={selectedConnection} result={activeSession} /> : <div className="view-empty">该 Session 只包含客户端事件，没有 PeerConnection。</div>}
              {activeSession.issues.length > 0 && <div className="issue-list">{activeSession.issues.map((issue, index) => <div className="issue-item" key={`${issue.code}-${index}`}><AlertTriangle size={15} /><span>{issue.message}</span><code>{issue.code}</code></div>)}</div>}
            </section>
          ) : (
            <section className="empty-workspace" aria-live="polite">
              <div className="empty-icon">{isImporting ? <LoaderCircle className="spin" size={29} strokeWidth={1.7} /> : <FolderOpen size={29} strokeWidth={1.7} />}</div>
              <h2>{isImporting ? "正在创建 Session" : "新建分析 Session"}</h2>
              <p>{isImporting ? pendingNames.join("、") : "一次可导入一份或多份同通话日志。"}</p>
              {!isImporting && <button className="secondary-button" type="button" onClick={() => importFiles()}><Plus size={17} />新建 Session</button>}
            </section>
          )}
          {importError && <div className="global-error error-message" role="alert"><strong>{importError.message}</strong><code>{importError.code}</code></div>}
        </main>
      </div>
    </div>
  );
}
