import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { getTickerShareCard } from "../api/shareCards";
import type { TickerShareCardRead } from "../api/shareCards";
import { Line } from "react-chartjs-2";
import {
  Chart as ChartJS, CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Filler,
} from "chart.js";

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Filler);

type Span = "all" | "7d" | "30d";

function filterHistory(dates: string[], prices: number[], span: Span) {
  if (span === "all") return { dates, prices };
  const days = span === "7d" ? 7 : 30;
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - days);
  const idx = dates.findIndex((d) => new Date(d) >= cutoff);
  if (idx < 0) return { dates, prices };
  return { dates: dates.slice(idx), prices: prices.slice(idx) };
}

export default function TickerShareCardPage() {
  const { id } = useParams<{ id: string }>();
  const [card, setCard] = useState<TickerShareCardRead | null>(null);
  const [span, setSpan] = useState<Span>("30d");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    getTickerShareCard(id)
      .then((c) => { setCard(c); setSpan(c.default_span as Span); })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "エラー"));
  }, [id]);

  if (error) return <div className="p-8 text-red-400">{error}</div>;
  if (!card) return <div className="p-8 text-gray-400">読み込み中…</div>;

  const curSym = card.currency === "JPY" ? "¥" : "$";
  const ogpUrl = `${window.location.origin}/api/ticker-share-cards/${card.id}/ogp.png`;
  const delta = card.price_delta_native;
  const deltaCls = delta === null ? "text-gray-400" : delta >= 0 ? "text-emerald-400" : "text-red-400";
  const deltaText = delta !== null ? `${delta >= 0 ? "+" : ""}${curSym}${delta.toLocaleString("ja-JP", { maximumFractionDigits: 2 })}` : "—";

  const { dates: fd, prices: fp } = filterHistory(card.history_dates, card.history_prices, span);
  const chartData = {
    labels: fd,
    datasets: [{
      data: fp,
      borderColor: "#8b5cf6",
      backgroundColor: "rgba(139,92,246,0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 2,
    }],
  };

  return (
    <>
      <head>
        <meta property="og:title" content={`${card.symbol} - korucha-fund`} />
        <meta property="og:image" content={ogpUrl} />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:image" content={ogpUrl} />
      </head>
      <div className="p-6 space-y-6 max-w-2xl mx-auto">
        <div>
          <h1 className="text-xl font-semibold text-gray-100">{card.symbol}</h1>
          {card.display_name && <p className="text-gray-400">{card.display_name}</p>}
          <p className="text-sm text-gray-500 mt-1">発行日時: {new Date(card.created_at).toLocaleString("ja-JP")}</p>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">発行時価格</div>
            <div className="text-2xl font-semibold text-gray-100">{`${curSym}${card.issue_price_native.toLocaleString("ja-JP", { maximumFractionDigits: 2 })}`}</div>
          </div>
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">現在価格との差</div>
            <div className={`text-2xl font-semibold ${deltaCls}`}>{deltaText}</div>
          </div>
          {card.quantity !== null && (
            <>
              <div className="bg-gray-800 rounded-xl p-4">
                <div className="text-xs text-gray-400 mb-1">保有数量</div>
                <div className="text-2xl font-semibold text-gray-100">{card.quantity.toLocaleString("ja-JP")}</div>
              </div>
              {card.issue_pnl_jpy !== null && (
                <div className="bg-gray-800 rounded-xl p-4">
                  <div className="text-xs text-gray-400 mb-1">含み損益 (発行時)</div>
                  <div className={`text-2xl font-semibold ${card.issue_pnl_jpy >= 0 ? "text-emerald-400" : "text-red-400"}`}>
                    {`${card.issue_pnl_jpy >= 0 ? "+" : ""}¥${card.issue_pnl_jpy.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`}
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        <div className="bg-gray-800 rounded-xl p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-medium text-gray-400">価格推移</h2>
            <div className="flex gap-1">
              {(["all", "30d", "7d"] as Span[]).map((s) => (
                <button key={s} onClick={() => setSpan(s)}
                  className={`px-2 py-1 text-xs rounded ${span === s ? "bg-violet-600 text-white" : "text-gray-400 hover:text-gray-200"}`}>
                  {s === "all" ? "全期間" : s}
                </button>
              ))}
            </div>
          </div>
          {fd.length > 0 ? (
            <Line data={chartData} options={{
              responsive: true,
              plugins: { legend: { display: false }, tooltip: { callbacks: { label: (ctx) => ` ${curSym}${(ctx.raw as number).toLocaleString("ja-JP", { maximumFractionDigits: 2 })}` } } },
              scales: {
                x: { ticks: { color: "#6b7280", maxTicksLimit: 8 }, grid: { color: "#2e303a" } },
                y: { ticks: { color: "#6b7280", callback: (v) => `${curSym}${Number(v).toLocaleString("ja-JP", { maximumFractionDigits: 2 })}` }, grid: { color: "#2e303a" } },
              },
            }} />
          ) : <div className="text-gray-500 text-sm">価格履歴なし</div>}
        </div>
      </div>
    </>
  );
}
