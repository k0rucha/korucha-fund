"use client";

import { useState } from "react";
import { Line } from "react-chartjs-2";
import type { ChartOptions } from "chart.js";
import "./chartSetup";
import { useTheme } from "@/components/ThemeProvider";
import { formatWithCommas } from "@/lib/format";
import SpanToggle, { type Span } from "./SpanToggle";

export default function TickerChart({
  dates,
  prices,
  defaultSpan,
  currency,
}: {
  dates: string[];
  prices: number[];
  defaultSpan: string;
  currency: string;
}) {
  const { theme } = useTheme();
  const win95 = theme === "win95";
  const unit = currency === "USD" ? "$" : "¥";
  const [span, setSpan] = useState<Span>(
    defaultSpan === "7d" || defaultSpan === "30d" ? defaultSpan : "all",
  );

  let d = dates;
  let p = prices;
  if (span !== "all" && dates.length) {
    const lastMs = new Date(dates[dates.length - 1] + "T00:00:00+09:00").getTime();
    const cutoff = lastMs - (span === "7d" ? 7 : 30) * 86400000;
    d = [];
    p = [];
    for (let i = 0; i < dates.length; i++) {
      if (new Date(dates[i] + "T00:00:00+09:00").getTime() >= cutoff) {
        d.push(dates[i]);
        p.push(prices[i]);
      }
    }
  }

  const lineColor = win95 ? "#000080" : "#000060";
  const data = {
    labels: d,
    datasets: [
      {
        label: "価格",
        data: p,
        borderColor: lineColor,
        backgroundColor: win95 ? "transparent" : "rgba(0,0,96,0.05)",
        fill: !win95,
        tension: win95 ? 0 : 0.2,
        pointRadius: 0,
        borderWidth: win95 ? 1 : 3,
      },
    ],
  };

  const options: ChartOptions<"line"> = {
    responsive: true,
    maintainAspectRatio: false,
    interaction: { mode: "index", intersect: false },
    scales: {
      x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
      y: { ticks: { font: { size: 11 }, callback: (v) => unit + formatWithCommas(Number(v)) } },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        callbacks: { label: (ctx) => ` ${unit}${formatWithCommas(ctx.parsed.y ?? 0)}` },
      },
    },
  };

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div className="text-xs font-bold uppercase tracking-widest text-da-gray-600">
          発行時までの価格推移
        </div>
        <SpanToggle span={span} onChange={setSpan} />
      </div>
      <div style={{ height: 280 }}>
        {d.length === 0 ? (
          <p className="py-12 text-center text-sm text-da-gray-400">
            この期間のデータがありません
          </p>
        ) : (
          <Line data={data} options={options} />
        )}
      </div>
    </div>
  );
}
