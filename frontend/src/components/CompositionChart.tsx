import { Doughnut } from "react-chartjs-2";
import { Chart as ChartJS, ArcElement, Tooltip, Legend } from "chart.js";
import type { CompositionItem } from "../api/portfolio";

ChartJS.register(ArcElement, Tooltip, Legend);

const COLORS = [
  "#8b5cf6", "#6366f1", "#3b82f6", "#0ea5e9",
  "#10b981", "#f59e0b", "#ef4444", "#ec4899",
];

export default function CompositionChart({ data }: { data: CompositionItem[] }) {
  if (!data.length) return <div className="text-gray-500 text-sm">データなし</div>;

  const chartData = {
    labels: data.map((d) => d.label),
    datasets: [
      {
        data: data.map((d) => d.value_jpy),
        backgroundColor: data.map((_, i) => COLORS[i % COLORS.length]),
        borderWidth: 1,
        borderColor: "#1f2028",
      },
    ],
  };

  return (
    <div className="w-full max-w-sm mx-auto">
      <Doughnut
        data={chartData}
        options={{
          plugins: {
            legend: { position: "right", labels: { color: "#9ca3af", boxWidth: 12 } },
            tooltip: {
              callbacks: {
                label: (ctx) =>
                  ` ¥${(ctx.raw as number).toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`,
              },
            },
          },
        }}
      />
    </div>
  );
}
