import { X } from "lucide-react";
import type { ImportJobSnapshot } from "@/types";

interface ImportProgressProps {
  job: ImportJobSnapshot;
  pendingNames: string[];
  onCancel: () => void;
}

export function ImportProgress({ job, pendingNames, onCancel }: ImportProgressProps) {
  return (
    <div className="import-progress" role="status">
      <div>
        <strong>{job.status === "queued" ? "等待解析" : "正在解析"}</strong>
        <span>{job.currentFile ?? pendingNames.join("、")}</span>
      </div>
      <progress value={job.completed} max={job.total || 1} />
      <span>{job.completed}/{job.total}</span>
      <button className="small-icon-button" type="button" onClick={onCancel} aria-label="取消导入" title="取消导入"><X size={15} /></button>
    </div>
  );
}
