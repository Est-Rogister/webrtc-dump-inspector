import { useEffect, useMemo, useRef } from "react";
import * as echarts from "echarts/core";
import { LineChart } from "echarts/charts";
import { DataZoomComponent, GridComponent, LegendComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { StatsPoint, WorkspaceSession } from "./types";

echarts.use([LineChart, DataZoomComponent, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

type CompareSeries = {
  session: WorkspaceSession;
  points: StatsPoint[];
};

type MetricKey = keyof Pick<
  StatsPoint,
  "outgoingBitrateKbps" | "incomingBitrateKbps" | "availableOutgoingBitrateKbps" | "currentRoundTripTimeMs" | "packetsLost"
>;

function CompareMetricChart({ series, metric, unit }: { series: CompareSeries[]; metric: MetricKey; unit: string }) {
  const elementRef = useRef<HTMLDivElement>(null);
  const available = useMemo(
    () => series.filter((item) => item.points.some((point) => point[metric] !== null)),
    [metric, series],
  );

  useEffect(() => {
    if (!elementRef.current || !available.length) return;
    const chart = echarts.init(elementRef.current);
    chart.setOption({
      animation: false,
      color: available.map((item) => item.session.color),
      grid: { left: 58, right: 24, top: 42, bottom: 54 },
      legend: { top: 4, textStyle: { color: "#53615a", fontSize: 11 } },
      tooltip: {
        trigger: "axis",
        valueFormatter: (value: unknown) => `${Number(value).toFixed(1)} ${unit}`.trim(),
      },
      xAxis: {
        type: "value",
        name: "相对时间 (s)",
        axisLabel: { color: "#718078", formatter: (value: number) => `${(value / 1000).toFixed(0)}s` },
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
      series: available.map(({ session, points }) => {
        const origin = session.summary.startTime ?? points[0]?.timestamp ?? 0;
        return {
          name: session.name,
          type: "line",
          showSymbol: false,
          connectNulls: false,
          lineStyle: { width: 1.8 },
          data: points.map((point) => [point.timestamp - origin, point[metric]]),
        };
      }),
    });
    const observer = new ResizeObserver(() => chart.resize());
    observer.observe(elementRef.current);
    return () => {
      observer.disconnect();
      chart.dispose();
    };
  }, [available, metric, unit]);

  if (!available.length) return <div className="view-empty compact">所选 Session 没有该指标。</div>;
  return <div className="metrics-chart" ref={elementRef} />;
}

export default function ComparisonStatsCharts({ series }: { series: CompareSeries[] }) {
  return (
    <div className="comparison-charts">
      <div className="view-title"><h3>发送码率</h3><span>按 Session 起点对齐</span></div>
      <CompareMetricChart series={series} metric="outgoingBitrateKbps" unit="kbps" />
      <div className="view-title secondary-title"><h3>接收码率</h3></div>
      <CompareMetricChart series={series} metric="incomingBitrateKbps" unit="kbps" />
      <div className="view-title secondary-title"><h3>往返时延</h3></div>
      <CompareMetricChart series={series} metric="currentRoundTripTimeMs" unit="ms" />
      <div className="view-title secondary-title"><h3>估算可用上行</h3></div>
      <CompareMetricChart series={series} metric="availableOutgoingBitrateKbps" unit="kbps" />
    </div>
  );
}
