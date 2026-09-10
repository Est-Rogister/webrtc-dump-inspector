import { GitCompareArrows } from "lucide-react";
import type { WorkspaceSession, WorkspaceView } from "@/types";
import { ComparisonViews } from "./ComparisonViews";

interface ComparisonWorkspaceProps {
  sessions: WorkspaceSession[];
  baseline: WorkspaceSession;
  target: WorkspaceSession;
  activeView: WorkspaceView;
  onSelectBaseline: (sessionId: string) => void;
  onSelectTarget: (sessionId: string) => void;
}

export function ComparisonWorkspace({
  sessions,
  baseline,
  target,
  activeView,
  onSelectBaseline,
  onSelectTarget,
}: ComparisonWorkspaceProps) {
  return (
    <section className="result-workspace" aria-live="polite">
      <div className="compare-session-picker">
        <label><span>Baseline</span><select value={baseline.id} onChange={(event) => onSelectBaseline(event.target.value)}>{sessions.map((session) => <option key={session.id} value={session.id}>{session.name}</option>)}</select></label>
        <GitCompareArrows size={18} />
        <label><span>Target</span><select value={target.id} onChange={(event) => onSelectTarget(event.target.value)}>{sessions.filter((session) => session.id !== baseline.id).map((session) => <option key={session.id} value={session.id}>{session.name}</option>)}</select></label>
      </div>
      <ComparisonViews activeView={activeView} baseline={baseline} target={target} />
    </section>
  );
}
