import { FolderOpen, LoaderCircle, Plus } from "lucide-react";

interface EmptyWorkspaceProps {
  isImporting: boolean;
  pendingNames: string[];
  onCreateSession: () => void;
}

export function EmptyWorkspace({ isImporting, pendingNames, onCreateSession }: EmptyWorkspaceProps) {
  return (
    <section className="empty-workspace" aria-live="polite">
      <div className="empty-icon">{isImporting ? <LoaderCircle className="spin" size={29} strokeWidth={1.7} /> : <FolderOpen size={29} strokeWidth={1.7} />}</div>
      <h2>{isImporting ? "正在创建 Session" : "新建分析 Session"}</h2>
      <p>{isImporting ? pendingNames.join("、") : "一次可导入一份或多份同通话日志。"}</p>
      {!isImporting && <button className="secondary-button" type="button" onClick={onCreateSession}><Plus size={17} />新建 Session</button>}
    </section>
  );
}
