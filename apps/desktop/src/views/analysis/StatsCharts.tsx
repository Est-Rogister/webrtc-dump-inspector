import { useEffect, useMemo, useRef } from "react";
import * as echarts from "echarts/core";
import { LineChart } from "echarts/charts";
import {
  DataZoomComponent,
  GridComponent,
  LegendComponent,
  TooltipComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { StatsPoint } from "@/types";

echarts.use([
  LineChart,
  DataZoomComponent,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  CanvasRenderer,
]);

type Metric = { key: keyof StatsPoint; name: string; color: string };

const bitrateMetrics: Metric[] = [
  { key: "outgoingBitrateKbps", name: "发送码率", color: "#147a4b" },
  { key: "incomingBitrateKbps", name: "接收码率", color: "#2676b8" },
];
const bandwidthMetrics: Metric[] = [
  { key: "availableOutgoingBitrateKbps", name: "可用上行", color: "#c07818" },
];
const qualityMetrics: Metric[] = [
  { key: "currentRoundTripTimeMs", name: "RTT", color: "#b44c43" },
  { key: "framesPerSecond", name: "FPS", color: "#2676b8" },
  { key: "packetsLost", name: "累计丢包", color: "#c07818" },
];

function MetricsChart({ points, metrics, unit }: { points: StatsPoint[]; metrics: Metric[]; unit: string }) {
  const elementRef = useRef<HTMLDivElement>(null);
  const available = useMemo(
    () => metrics.filter((metric) => points.some((point) => point[metric.key] !== null)),
    [metrics, points],
  );

  useEffect(() => {
    if (!elementRef.current || !available.length) return;
    const chart = echarts.init(elementRef.current);
    chart.setOption({
      animation: false,
      color: available.map((metric) => metric.color),
      grid: { left: 54, right: 24, top: 42, bottom: 54 },
      legend: { top: 4, textStyle: { color: "#53615a", fontSize: 11 } },
      tooltip: {
        trigger: "axis",
        valueFormatter: (value: unknown) => `${Number(value).toFixed(1)} ${unit}`,
      },
      xAxis: {
        type: "time",
        axisLabel: { color: "#718078" },
        axisLine: { lineStyle: { color: "#dce2df" } },
      },
      yAxis: {
        type: "value",
        name: unit,
        nameTextStyle: { color: "#718078" },
        axisLabel: { color: "#718078" },
        splitLine: { lineStyle: { color: "#edf0ee" } },
      },
      dataZoom: [{ type: "inside" }, { type: "slider", height: 18, bottom: 8 }],
      series: available.map((metric) => ({
        name: metric.name,
        type: "line",
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 1.7 },
        data: points.map((point) => [point.timestamp, point[metric.key]]),
      })),
    });
    const observer = new ResizeObserver(() => chart.resize());
    observer.observe(elementRef.current);
    return () => {
      observer.disconnect();
      chart.dispose();
    };
  }, [available, metrics, points, unit]);

  if (!available.length) return <div className="view-empty compact">该日志没有记录这组指标。</div>;
  return <div className="metrics-chart" ref={elementRef} />;
}

export default function StatsCharts({ points }: { points: StatsPoint[] }) {
  return (
    <>
      <MetricsChart points={points} metrics={bitrateMetrics} unit="kbps" />
      <div className="view-title secondary-title"><h3>估算可用上行带宽</h3></div>
      <MetricsChart points={points} metrics={bandwidthMetrics} unit="kbps" />
      <div className="view-title secondary-title"><h3>网络与视频质量</h3></div>
      <MetricsChart points={points} metrics={qualityMetrics} unit="" />
    </>
  );
}
