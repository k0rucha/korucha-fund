"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

export default function RefreshButton() {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  async function refresh() {
    setBusy(true);
    setNote(null);
    try {
      const res = await fetch("/api/refresh", { method: "POST" });
      const j = await res.json().catch(() => ({}));
      if (res.ok) {
        setNote(
          j.updated_from_api
            ? `更新しました（残り ${j.remaining_api_requests} 回）`
            : `レート制限中（残り ${j.remaining_api_requests ?? "?"} 回）`,
        );
        router.refresh();
      } else {
        setNote(res.status === 429 ? "更新処理が進行中です" : "更新に失敗しました");
      }
    } catch {
      setNote("更新に失敗しました");
    } finally {
      setBusy(false);
      setTimeout(() => setNote(null), 4000);
    }
  }

  return (
    <div className="flex items-center gap-2">
      {note && <span className="text-xs text-da-gray-400">{note}</span>}
      <button
        type="button"
        onClick={refresh}
        disabled={busy}
        className="border border-da-blue-900 px-3 py-1 text-sm font-bold text-da-blue-900 hover:bg-da-blue-50 disabled:opacity-50"
      >
        {busy ? "更新中…" : "価格更新"}
      </button>
    </div>
  );
}
