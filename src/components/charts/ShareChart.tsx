"use client";

import { useState } from "react";
import { Line } from "react-chartjs-2";
import type { ChartOptions } from "chart.js";
import "./chartSetup";
import { useTheme } from "@/components/ThemeProvider";
import { formatWithCommas } from "@/lib/format";
import SpanToggle, { type Span } from "./SpanToggle";

/** Keep only indices whose date is within `span` of the last (issuance) date. */
function sliceByDays(
  dates: string[],
  cols: number[][],
  span: Span,
): { d: string[]; cols: number[][] } {
  if (span === "all" || dates.length === 0) return { d: dates, cols };
  const lastMs = new Date(dates[dates.length - 1] + "T00:00:00+09:00").getTime();
  const cutoff = lastMs - (span === "7d" ? 7 : 30) * 86400000;
  const idx: number[] = [];
  for (let i = 0; i < dates.length; i++) {
    if (new Date(dates[i] + "T00:00:00+09:00").getTime() >= cutoff) idx.push(i);
  }
  return { d: idx.map((i) => dates[i]), cols: cols.map((c) => idx.map((i) => c[i])) };
}

export default function ShareChart({
  dates,
  values,
  pnls,
  defaultSpan,
}: {
  dates: string[];
  values: number[];
  pnls: number[];
  defaultSpan: string;
}) {
  const { theme } = useTheme();
  const win95 = theme === "win95";
  const [span, setSpan] = useState<Span>(
    defaultSpan === "7d" || defaultSpan === "30d" ? defaultSpan : "all",
  );

  const { d, cols } = sliceByDays(dates, [values, pnls], span);
  const [v, p] = cols;

  const valueColor = win95 ? "#000080" : "#000060";
  const pnlColor = win95 ? "#808000" : "#FB5801";

  const data = {
    labels: d,
    datasets: [
      {
        label: "評価額",
        data: v,
        borderColor: valueColor,
        backgroundColor: win95 ? "transparent" : "rgba(0,0,96,0.05)",
        fill: !win95,
        tension: win95 ? 0 : 0.2,
        pointRadius: 0,
        borderWidth: win95 ? 1 : 3,
        yAxisID: "y",
      },
      {
        label: "損益",
        data: p,
        borderColor: pnlColor,
        fill: false,
        tension: win95 ? 0 : 0.2,
        pointRadius: 0,
        borderWidth: win95 ? 1 : 2,
        yAxisID: "y1",
      },
    ],
  };

  const options: ChartOptions<"line"> = {
    responsive: true,
    maintainAspectRatio: false,
    interaction: { mode: "index", intersect: false },
    scales: {
      x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
      y: {
        position: "left",
        ticks: { font: { size: 11 }, callback: (val) => "¥" + formatWithCommas(Number(val)) },
      },
      y1: {
        position: "right",
        grid: { drawOnChartArea: false },
        ticks: { color: pnlColor, font: { size: 11 }, callback: (val) => "¥" + formatWithCommas(Number(val)) },
      },
    },
    plugins: {
      legend: { labels: { usePointStyle: true, font: { size: 12 } } },
      tooltip: {
        callbacks: {
          label: (ctx) => ` ${ctx.dataset.label}: ¥${formatWithCommas(ctx.parsed.y ?? 0)}`,
        },
      },
    },
  };

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div className="text-xs font-bold uppercase tracking-widest text-da-gray-600">
          発行時までの推移
        </div>
        <SpanToggle span={span} onChange={setSpan} />
      </div>
      <div style={{ height: 280 }}>
        {d.length === 0 ? (
          <p className="py-12 text-center text-sm text-da-gray-400">
            この期間のスナップショットがありません
          </p>
        ) : (
          <Line data={data} options={options} />
        )}
      </div>
    </div>
  );
}
