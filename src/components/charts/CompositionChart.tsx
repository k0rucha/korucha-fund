"use client";

import { Doughnut } from "react-chartjs-2";
import type { ChartOptions } from "chart.js";
import "./chartSetup";
import { useTheme } from "@/components/ThemeProvider";
import { formatWithCommas } from "@/lib/format";
import type { CompositionData } from "@/lib/composition";

const DEFAULT_PALETTE = [
  "#000060", "#004BB1", "#0017C1", "#FB5801", "#FFBD44",
  "#34D399", "#7799FF", "#F87171", "#8899CC", "#6677AA",
];
const WIN95_PALETTE = [
  "#000080", "#808080", "#008080", "#800000", "#808000",
  "#800080", "#C0C0C0", "#000000", "#008000", "#404040",
];

export default function CompositionChart({ data }: { data: CompositionData }) {
  const { theme } = useTheme();
  const palette = theme === "win95" ? WIN95_PALETTE : DEFAULT_PALETTE;

  if (data.values.length === 0) {
    return (
      <p className="py-12 text-center text-sm text-da-gray-400">
        保有データがありません
      </p>
    );
  }

  const chartData = {
    labels: data.labels,
    datasets: [
      {
        data: data.values,
        backgroundColor: data.values.map((_, i) => palette[i % palette.length]),
        borderColor: theme === "win95" ? "#000000" : "#ffffff",
        borderWidth: 1,
      },
    ],
  };

  const options: ChartOptions<"doughnut"> = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { position: "right", labels: { boxWidth: 12, font: { size: 11 } } },
      tooltip: {
        callbacks: {
          label: (ctx) =>
            ` ${data.fullNames[ctx.dataIndex]}: ¥${formatWithCommas(ctx.parsed)}`,
        },
      },
    },
  };

  return <Doughnut data={chartData} options={options} />;
}
