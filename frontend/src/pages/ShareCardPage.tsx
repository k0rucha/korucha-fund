import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { getShareCard } from "../api/shareCards";
import type { ShareCardRead } from "../api/shareCards";
import TimeseriesChart from "../components/TimeseriesChart";
import type { TimeseriesResponse } from "../api/portfolio";

function fmt(v: number | null): string {
  if (v === null) return "—";
  return `¥${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`;
}
function fmtDelta(v: number | null): { text: string; cls: string } {
  if (v === null) return { text: "—", cls: "text-gray-400" };
  const sign = v >= 0 ? "+" : "";
  return { text: `${sign}¥${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`, cls: v >= 0 ? "text-emerald-400" : "text-red-400" };
}

type Span = "all" | "7d" | "30d";

export default function ShareCardPage() {
  const { id } = useParams<{ id: string }>();
  const [card, setCard] = useState<ShareCardRead | null>(null);
  const [span, setSpan] = useState<Span>("all");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    getShareCard(id)
      .then((c) => { setCard(c); setSpan(c.default_span as Span); })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "エラー"));
  }, [id]);

  if (error) return <div className="p-8 text-red-400">{error}</div>;
  if (!card) return <div className="p-8 text-gray-400">読み込み中…</div>;

  const ogpUrl = `${window.location.origin}/api/share-cards/${card.id}/ogp.png`;
  const pnl = fmtDelta(card.unrealized_pnl_jpy);
  const delta = fmtDelta(card.value_delta_jpy);

  const ts: TimeseriesResponse = {
    dates: card.history_dates,
    values: card.history_values,
    costs: [],
    pnls: card.history_pnls,
  };

  return (
    <>
      <head>
        <meta property="og:title" content="korucha-fund ポートフォリオ" />
        <meta property="og:image" content={ogpUrl} />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:image" content={ogpUrl} />
      </head>
      <div className="p-6 space-y-6 max-w-2xl mx-auto">
        <h1 className="text-xl font-semibold text-gray-100">ポートフォリオスナップショット</h1>
        <p className="text-sm text-gray-500">発行日時: {new Date(card.created_at).toLocaleString("ja-JP")}</p>

        <div className="grid grid-cols-2 gap-4">
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">発行時評価額</div>
            <div className="text-2xl font-semibold text-gray-100">{fmt(card.total_value_jpy)}</div>
          </div>
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">現在価値との差</div>
            <div className={`text-2xl font-semibold ${delta.cls}`}>{delta.text}</div>
          </div>
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">含み損益 (発行時)</div>
            <div className={`text-2xl font-semibold ${pnl.cls}`}>{pnl.text}</div>
          </div>
          <div className="bg-gray-800 rounded-xl p-4">
            <div className="text-xs text-gray-400 mb-1">取得原価</div>
            <div className="text-2xl font-semibold text-gray-100">{fmt(card.total_cost_jpy)}</div>
          </div>
        </div>

        <div className="bg-gray-800 rounded-xl p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-medium text-gray-400">推移</h2>
            <div className="flex gap-1">
              {(["all", "30d", "7d"] as Span[]).map((s) => (
                <button key={s} onClick={() => setSpan(s)}
                  className={`px-2 py-1 text-xs rounded ${span === s ? "bg-violet-600 text-white" : "text-gray-400 hover:text-gray-200"}`}>
                  {s === "all" ? "全期間" : s}
                </button>
              ))}
            </div>
          </div>
          <TimeseriesChart data={ts} span={span} />
        </div>

        <div className="bg-gray-800 rounded-xl p-4">
          <h2 className="text-sm font-medium text-gray-400 mb-3">保有銘柄 (発行時)</h2>
          <table className="w-full text-sm text-gray-300">
            <thead>
              <tr className="border-b border-gray-700 text-xs text-gray-500">
                <th className="text-left py-1.5 pr-4">銘柄</th>
                <th className="text-right py-1.5 pr-4">数量</th>
                <th className="text-right py-1.5">評価額</th>
              </tr>
            </thead>
            <tbody>
              {card.holdings.map((h) => (
                <tr key={h.symbol} className="border-b border-gray-800">
                  <td className="py-2 pr-4">{h.symbol}{h.name && <span className="ml-2 text-xs text-gray-500">{h.name}</span>}</td>
                  <td className="text-right pr-4">{h.quantity.toLocaleString("ja-JP")}</td>
                  <td className="text-right">{fmt(h.value_jpy)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
