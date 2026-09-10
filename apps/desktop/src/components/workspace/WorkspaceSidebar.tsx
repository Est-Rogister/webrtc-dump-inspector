import { Plus, X } from "lucide-react";
import { WORKSPACE_VIEWS } from "@/app/navigation";
import type { WorkspaceSession, WorkspaceView } from "@/types";

interface WorkspaceSidebarProps {
  sessions: WorkspaceSession[];
  activeSessionId: string | null;
  activeView: WorkspaceView;
  isImporting: boolean;
  onCreateSession: () => void;
  onSelectSession: (sessionId: string) => void;
  onCloseSession: (sessionId: string) => void;
  onSelectView: (view: WorkspaceView) => void;
}

export function WorkspaceSidebar({
  sessions,
  activeSessionId,
  activeView,
  isImporting,
  onCreateSession,
  onSelectSession,
  onCloseSession,
  onSelectView,
}: WorkspaceSidebarProps) {
  return (
    <aside className="sidebar">
      <div className="session-section">
        <div className="section-heading">
          <span className="section-label">Sessions</span>
          <button className="small-icon-button" type="button" onClick={onCreateSession} disabled={isImporting} aria-label="新建 Session" title="新建 Session"><Plus size={15} /></button>
        </div>
        <div className="session-list">
          {sessions.map((session) => (
            <div className={activeSessionId === session.id ? "session-entry active" : "session-entry"} key={session.id}>
              <button className="session-main" type="button" onClick={() => onSelectSession(session.id)}>
                <i style={{ background: session.color }} />
                <span><strong>{session.name}</strong><small>{session.summary.sourceCount} 个数据源 · {session.summary.peerConnectionCount} 个连接</small></span>
              </button>
              <button className="session-close" type="button" onClick={() => onCloseSession(session.id)} aria-label={`关闭 ${session.name}`} title="关闭 Session"><X size={13} /></button>
            </div>
          ))}
          {!sessions.length && <div className="session-empty">尚未创建 Session</div>}
        </div>
      </div>

      <nav className="nav-list" aria-label="分析视图">
        {WORKSPACE_VIEWS.map((view) => {
          const Icon = view.icon;
          return <button className={activeView === view.id ? "nav-item active" : "nav-item"} key={view.id} type="button" disabled={!sessions.length} onClick={() => onSelectView(view.id)}><Icon size={17} /><span>{view.label}</span></button>;
        })}
      </nav>
      <div className="sidebar-footer"><span>本地模式</span><i aria-hidden="true" /></div>
    </aside>
  );
}
