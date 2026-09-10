import { useEffect, useMemo, useState } from "react";
import { WORKSPACE_VIEWS } from "@/app/navigation";
import { ImportProgress } from "@/components/import/ImportProgress";
import { RtcLogoMark } from "@/components/RtcLogoMark";
import { WorkspaceSidebar } from "@/components/workspace/WorkspaceSidebar";
import { WorkspaceToolbar } from "@/components/workspace/WorkspaceToolbar";
import {
  cancelImport,
  closeSession as closeStoredSession,
  getRuntimeStatus,
  isImportJobActive,
  listSessions,
  parseDesktopError,
  selectDumpFiles,
  startImport,
  waitForImport,
} from "@/services/desktopApi";
import type {
  ImportDumpError,
  ImportJobSnapshot,
  RuntimeStatus,
  WorkspaceMode,
  WorkspaceSession,
  WorkspaceView,
} from "@/types";
import { ComparisonWorkspace } from "@/views/comparison/ComparisonWorkspace";
import { EmptyWorkspace } from "@/views/EmptyWorkspace";
import { SessionWorkspace } from "@/views/session/SessionWorkspace";

function displayName(path: string) {
  return path.split(/[\\/]/).at(-1) || path;
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
  const [importJob, setImportJob] = useState<ImportJobSnapshot | null>(null);

  useEffect(() => {
    getRuntimeStatus()
      .then(setRuntime)
      .catch(() => setRuntime({ appName: "RTC Inspector", version: "web-preview", platform: "browser" }));
    listSessions()
      .then((loaded) => {
        setSessions(loaded);
        setActiveSessionId(loaded[0]?.id ?? null);
        setBaselineId(loaded[0]?.id ?? "");
        setTargetId(loaded[1]?.id ?? "");
      })
      .catch(() => undefined);
  }, []);

  const activeLabel = useMemo(() => WORKSPACE_VIEWS.find((view) => view.id === activeView)?.label ?? "概览", [activeView]);
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

  async function refreshSessions() {
    const loaded = await listSessions();
    setSessions(loaded);
    return loaded;
  }

  async function importFiles(targetSessionId?: string) {
    setImportError(null);
    try {
      const paths = await selectDumpFiles();
      if (!paths.length) return;
      setPendingNames(paths.map(displayName));
      setIsImporting(true);
      const started = await startImport(paths, targetSessionId);
      setImportJob(started);
      const completed = await waitForImport(started, setImportJob);
      if (!completed.sessionId) {
        const failure = completed.errors[0]?.error;
        setImportError(failure ?? {
          code: completed.status === "cancelled" ? "import.cancelled" : "dump.import-failed",
          message: completed.status === "cancelled" ? "导入已取消" : "所有文件均导入失败",
        });
        return;
      }
      const loaded = await refreshSessions();
      const session = loaded.find((candidate) => candidate.id === completed.sessionId);
      setActiveSessionId(completed.sessionId);
      if (session) {
        setSelectedConnectionIds((current) => ({
          ...current,
          [session.id]: current[session.id] ?? session.analysis.connections[0]?.id ?? "",
        }));
      }
      if (!baselineId) setBaselineId(completed.sessionId);
      else if (!targetId && completed.sessionId !== baselineId) setTargetId(completed.sessionId);
      setActiveView("overview");
      setMode("single");
    } catch (error) {
      setImportError(parseDesktopError(error));
    } finally {
      setPendingNames([]);
      setIsImporting(false);
      setImportJob(null);
    }
  }

  async function cancelCurrentImport() {
    if (!importJob) return;
    try {
      const cancelled = await cancelImport(importJob.id);
      setImportJob(cancelled);
    } catch (error) {
      setImportError(parseDesktopError(error));
    }
  }

  async function closeSession(sessionId: string) {
    try {
      await closeStoredSession(sessionId);
      const remaining = sessions.filter((session) => session.id !== sessionId);
      setSessions(remaining);
      if (activeSession?.id === sessionId) setActiveSessionId(remaining[0]?.id ?? null);
      if (remaining.length < 2) setMode("single");
      if (baselineId === sessionId) setBaselineId(remaining[0]?.id ?? "");
      if (targetId === sessionId) setTargetId(remaining.find((session) => session.id !== remaining[0]?.id)?.id ?? "");
    } catch (error) {
      setImportError(parseDesktopError(error));
    }
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
        <WorkspaceSidebar
          sessions={sessions}
          activeSessionId={activeSession?.id ?? null}
          activeView={activeView}
          isImporting={isImporting}
          onCreateSession={() => importFiles()}
          onSelectSession={(sessionId) => { setActiveSessionId(sessionId); setMode("single"); }}
          onCloseSession={closeSession}
          onSelectView={setActiveView}
        />

        <main className="content">
          <WorkspaceToolbar
            mode={mode}
            activeLabel={activeLabel}
            canCompare={canCompare}
            isImporting={isImporting}
            onChangeMode={setMode}
            onCreateSession={() => importFiles()}
          />

          {isImportJobActive(importJob) && importJob && <ImportProgress job={importJob} pendingNames={pendingNames} onCancel={cancelCurrentImport} />}

          {mode === "compare" && baseline && target ? (
            <ComparisonWorkspace sessions={sessions} baseline={baseline} target={target} activeView={activeView} onSelectBaseline={setBaselineId} onSelectTarget={setTargetId} />
          ) : activeSession ? (
            <SessionWorkspace
              session={activeSession}
              selectedConnection={selectedConnection}
              activeView={activeView}
              isImporting={isImporting}
              onAddSource={() => importFiles(activeSession.id)}
              onSelectConnection={(connectionId) => setSelectedConnectionIds((current) => ({ ...current, [activeSession.id]: connectionId }))}
            />
          ) : (
            <EmptyWorkspace isImporting={isImporting} pendingNames={pendingNames} onCreateSession={() => importFiles()} />
          )}
          {importError && <div className="global-error error-message" role="alert"><strong>{importError.message}</strong><code>{importError.code}</code></div>}
        </main>
      </div>
    </div>
  );
}
