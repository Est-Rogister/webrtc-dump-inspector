import { Gauge, GitCompareArrows, LoaderCircle, Plus } from "lucide-react";
import type { WorkspaceMode } from "@/types";

interface WorkspaceToolbarProps {
  mode: WorkspaceMode;
  activeLabel: string;
  canCompare: boolean;
  isImporting: boolean;
  onChangeMode: (mode: WorkspaceMode) => void;
  onCreateSession: () => void;
}

export function WorkspaceToolbar({
  mode,
  activeLabel,
  canCompare,
  isImporting,
  onChangeMode,
  onCreateSession,
}: WorkspaceToolbarProps) {
  return (
    <div className="content-toolbar">
      <div><span className="breadcrumb">{mode === "single" ? "Session 分析" : "Session 对比"}</span><h1>{activeLabel}</h1></div>
      <div className="toolbar-actions">
        <div className="mode-switch" aria-label="工作模式">
          <button className={mode === "single" ? "active" : ""} type="button" onClick={() => onChangeMode("single")}><Gauge size={15} />单会话</button>
          <button className={mode === "compare" ? "active" : ""} type="button" disabled={!canCompare} onClick={() => onChangeMode("compare")} title={canCompare ? "对比 Session" : "至少需要两个 Session"}><GitCompareArrows size={15} />对比</button>
        </div>
        <button className="primary-button" type="button" onClick={onCreateSession} disabled={isImporting}>{isImporting ? <LoaderCircle className="spin" size={17} /> : <Plus size={17} />}{isImporting ? "正在导入" : "新建 Session"}</button>
      </div>
    </div>
  );
}
