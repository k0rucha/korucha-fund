"use client";

import { Line } from "react-chartjs-2";
import type { ChartOptions } from "chart.js";
import "./chartSetup";
import { useTheme } from "@/components/ThemeProvider";
import { formatWithCommas } from "@/lib/format";
import type { TimeseriesData } from "@/lib/timeseries";

export default function TimeseriesChart({ data }: { data: TimeseriesData }) {
  const { theme } = useTheme();
  const win95 = theme === "win95";
  const valueColor = win95 ? "#000080" : "#004BB1";
  const costColor = win95 ? "#808080" : "#FB5801";
  const gridColor = win95 ? "#808080" : "rgba(0,0,0,0.06)";

  if (data.dates.length === 0) {
    return (
      <p className="py-12 text-center text-sm text-da-gray-400">
        スナップショットがありません
      </p>
    );
  }

  const chartData = {
    labels: data.dates,
    datasets: [
      {
        label: "評価額",
        data: data.values,
        borderColor: valueColor,
        backgroundColor: valueColor + "22",
        fill: true,
        tension: 0.2,
        pointRadius: 0,
        borderWidth: 2,
      },
      {
        label: "投資元本",
        data: data.costs,
        borderColor: costColor,
        borderDash: [4, 4],
        fill: false,
        tension: 0.2,
        pointRadius: 0,
        borderWidth: 1.5,
      },
    ],
  };

  const options: ChartOptions<"line"> = {
    responsive: true,
    maintainAspectRatio: false,
    interaction: { mode: "index", intersect: false },
    plugins: {
      legend: { position: "top", labels: { boxWidth: 14, font: { size: 11 } } },
      tooltip: {
        callbacks: {
          label: (ctx) =>
            ` ${ctx.dataset.label}: ¥${formatWithCommas(ctx.parsed.y ?? 0)}`,
        },
      },
    },
    scales: {
      x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 10 } } },
      y: {
        grid: { color: gridColor },
        ticks: {
          font: { size: 10 },
          callback: (v) => "¥" + formatWithCommas(Number(v)),
        },
      },
    },
  };

  return <Line data={chartData} options={options} />;
}
