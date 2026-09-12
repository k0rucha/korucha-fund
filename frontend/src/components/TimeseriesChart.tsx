import { Line } from "react-chartjs-2";
import {
  Chart as ChartJS, CategoryScale, LinearScale,
  PointElement, LineElement, Tooltip, Legend, Filler,
} from "chart.js";
import type { TimeseriesResponse } from "../api/portfolio";

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Legend, Filler);

type Span = "all" | "7d" | "30d";

function filterBySpan(data: TimeseriesResponse, span: Span) {
  if (span === "all") return data;
  const days = span === "7d" ? 7 : 30;
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - days);
  const idx = data.dates.findIndex((d) => new Date(d) >= cutoff);
  if (idx < 0) return data;
  return {
    dates: data.dates.slice(idx),
    values: data.values.slice(idx),
    costs: data.costs.slice(idx),
    pnls: data.pnls.slice(idx),
  };
}

interface Props {
  data: TimeseriesResponse;
  span?: Span;
}

export default function TimeseriesChart({ data, span = "all" }: Props) {
  const filtered = filterBySpan(data, span);

  if (!filtered.dates.length)
    return <div className="text-gray-500 text-sm">スナップショットなし</div>;

  const chartData = {
    labels: filtered.dates,
    datasets: [
      {
        label: "評価額",
        data: filtered.values,
        borderColor: "#8b5cf6",
        backgroundColor: "rgba(139,92,246,0.1)",
        fill: true,
        tension: 0.3,
        pointRadius: 2,
      },
      {
        label: "取得原価",
        data: filtered.costs,
        borderColor: "#6b7280",
        backgroundColor: "transparent",
        borderDash: [4, 4],
        tension: 0.3,
        pointRadius: 0,
      },
    ],
  };

  return (
    <Line
      data={chartData}
      options={{
        responsive: true,
        plugins: {
          legend: { labels: { color: "#9ca3af" } },
          tooltip: {
            callbacks: {
              label: (ctx) =>
                ` ¥${(ctx.raw as number).toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`,
            },
          },
        },
        scales: {
          x: { ticks: { color: "#6b7280", maxTicksLimit: 8 }, grid: { color: "#2e303a" } },
          y: {
            ticks: {
              color: "#6b7280",
              callback: (v) => `¥${Number(v).toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`,
            },
            grid: { color: "#2e303a" },
          },
        },
      }}
    />
  );
}
