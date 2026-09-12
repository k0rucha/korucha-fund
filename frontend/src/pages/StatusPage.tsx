import { useEffect, useState } from "react";
import { getStatus } from "../api/shareCards";
import type { StatusResponse } from "../api/shareCards";

export default function StatusPage() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getStatus()
      .then(setStatus)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "エラー"));
  }, []);

  if (error) return <div className="p-8 text-red-400">{error}</div>;
  if (!status) return <div className="p-8 text-gray-400">読み込み中…</div>;

  const pct = (status.request_count / status.max_requests_per_day) * 100;

  return (
    <div className="p-6 max-w-lg space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">API ステータス</h1>

      <div className="bg-gray-800 rounded-xl p-4 space-y-3">
        <div className="flex justify-between text-sm">
          <span className="text-gray-400">今日のリクエスト数</span>
          <span className="text-gray-100">{status.request_count} / {status.max_requests_per_day}</span>
        </div>
        <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
          <div className="h-full bg-violet-600 transition-all" style={{ width: `${pct}%` }} />
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-gray-400">残り</span>
          <span className={status.remaining_requests === 0 ? "text-red-400" : "text-emerald-400"}>
            {status.remaining_requests} 回
          </span>
        </div>
        {status.last_request_time && (
          <div className="flex justify-between text-sm">
            <span className="text-gray-400">最終リクエスト</span>
            <span className="text-gray-300">{new Date(status.last_request_time).toLocaleString("ja-JP")}</span>
          </div>
        )}
        {status.next_scheduler_run && (
          <div className="flex justify-between text-sm">
            <span className="text-gray-400">次回スケジューラ実行</span>
            <span className="text-gray-300">{new Date(status.next_scheduler_run).toLocaleString("ja-JP")}</span>
          </div>
        )}
      </div>
    </div>
  );
}
