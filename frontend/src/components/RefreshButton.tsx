import { useState } from "react";
import { refreshPrices } from "../api/portfolio";

interface Props {
  onRefreshed?: () => void;
}

export default function RefreshButton({ onRefreshed }: Props) {
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  const handleClick = async () => {
    setLoading(true);
    setMsg(null);
    try {
      const res = await refreshPrices();
      setMsg(
        res.updated_from_api
          ? `更新完了 (残り ${res.remaining_api_requests} 回)`
          : `スキップ (残り ${res.remaining_api_requests} 回)`
      );
      onRefreshed?.();
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "エラー");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex items-center gap-3">
      <button
        onClick={handleClick}
        disabled={loading}
        className="px-4 py-2 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 text-white text-sm rounded-lg transition-colors"
      >
        {loading ? "更新中…" : "価格更新"}
      </button>
      {msg && <span className="text-sm text-gray-400">{msg}</span>}
    </div>
  );
}
