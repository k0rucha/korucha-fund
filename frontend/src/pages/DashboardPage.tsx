import { useEffect, useState, useCallback } from "react";
import { getDashboard, getComposition, getTimeseries } from "../api/portfolio";
import type { DashboardResponse, CompositionItem, TimeseriesResponse } from "../api/portfolio";
import KpiTiles from "../components/KpiTiles";
import CompositionChart from "../components/CompositionChart";
import TimeseriesChart from "../components/TimeseriesChart";
import HoldingsTable from "../components/HoldingsTable";
import RefreshButton from "../components/RefreshButton";
import { createShareCard } from "../api/shareCards";
import { useNavigate } from "react-router-dom";

type Span = "all" | "7d" | "30d";

export default function DashboardPage() {
  const [dashboard, setDashboard] = useState<DashboardResponse | null>(null);
  const [composition, setComposition] = useState<CompositionItem[]>([]);
  const [timeseries, setTimeseries] = useState<TimeseriesResponse | null>(null);
  const [span, setSpan] = useState<Span>("all");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sharing, setSharing] = useState(false);
  const navigate = useNavigate();

  const load = useCallback(async () => {
    try {
      const [d, c, t] = await Promise.all([getDashboard(), getComposition(), getTimeseries()]);
      setDashboard(d);
      setComposition(c);
      setTimeseries(t);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "読み込みエラー");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleShare = async () => {
    setSharing(true);
    try {
      const res = await createShareCard(span);
      navigate(res.url);
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "エラー");
    } finally {
      setSharing(false);
    }
  };

  if (loading) return <div className="p-8 text-gray-400">読み込み中…</div>;
  if (error) return <div className="p-8 text-red-400">{error}</div>;
  if (!dashboard || !timeseries) return null;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-gray-100">ポートフォリオ</h1>
        <div className="flex items-center gap-3">
          <RefreshButton onRefreshed={load} />
          <button
            onClick={handleShare}
            disabled={sharing}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-sm text-gray-200 rounded-lg transition-colors"
          >
            {sharing ? "…" : "共有カード作成"}
          </button>
        </div>
      </div>

      <KpiTiles data={dashboard} />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="bg-gray-800 rounded-xl p-4">
          <h2 className="text-sm font-medium text-gray-400 mb-4">ポートフォリオ構成</h2>
          <CompositionChart data={composition} />
        </div>
        <div className="lg:col-span-2 bg-gray-800 rounded-xl p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-medium text-gray-400">推移</h2>
            <div className="flex gap-1">
              {(["all", "30d", "7d"] as Span[]).map((s) => (
                <button
                  key={s}
                  onClick={() => setSpan(s)}
                  className={`px-2 py-1 text-xs rounded ${span === s ? "bg-violet-600 text-white" : "text-gray-400 hover:text-gray-200"}`}
                >
                  {s === "all" ? "全期間" : s}
                </button>
              ))}
            </div>
          </div>
          <TimeseriesChart data={timeseries} span={span} />
        </div>
      </div>

      <div className="bg-gray-800 rounded-xl p-4">
        <h2 className="text-sm font-medium text-gray-400 mb-4">保有銘柄</h2>
        <HoldingsTable holdings={dashboard.holdings} />
      </div>

      {dashboard.last_updated && (
        <p className="text-xs text-gray-600 text-right">
          最終更新: {new Date(dashboard.last_updated).toLocaleString("ja-JP")}
        </p>
      )}
    </div>
  );
}
