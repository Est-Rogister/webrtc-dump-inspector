import {
  Activity,
  Braces,
  ChartNoAxesCombined,
  Gauge,
  Radio,
} from "lucide-react";
import type { WorkspaceView } from "@/types";

export const WORKSPACE_VIEWS: Array<{
  id: WorkspaceView;
  label: string;
  icon: typeof Gauge;
}> = [
  { id: "overview", label: "概览", icon: Gauge },
  { id: "events", label: "事件", icon: Activity },
  { id: "sdp", label: "SDP", icon: Braces },
  { id: "ice", label: "ICE", icon: Radio },
  { id: "stats", label: "Stats", icon: ChartNoAxesCombined },
];
