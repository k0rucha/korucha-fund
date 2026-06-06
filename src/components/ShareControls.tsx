"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

export default function ShareControls() {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [symbol, setSymbol] = useState("");
  const [err, setErr] = useState<string | null>(null);

  async function createShare() {
    setBusy(true);
    setErr(null);
    try {
      const res = await fetch("/api/share", { method: "POST" });
      const j = await res.json().catch(() => ({}));
      if (res.ok && j.url) router.push(j.url);
      else setErr(j.error ?? "発行に失敗しました");
    } catch {
      setErr("発行に失敗しました");
    } finally {
      setBusy(false);
    }
  }

  async function createTicker(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const s = symbol.trim();
    if (!s) return;
    setBusy(true);
    setErr(null);
    try {
      const res = await fetch(`/api/ticker-share?symbol=${encodeURIComponent(s)}`, {
        method: "POST",
      });
      const j = await res.json().catch(() => ({}));
      if (res.ok && j.url) router.push(j.url);
      else setErr(j.error ?? "発行に失敗しました");
    } catch {
      setErr("発行に失敗しました");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-3 border border-da-gray-200 p-4">
      <button
        type="button"
        onClick={createShare}
        disabled={busy}
        className="bg-da-blue-1200 px-4 py-2 text-sm font-bold text-white hover:bg-da-blue-900 disabled:opacity-50"
      >
        戦績カードを発行
      </button>
      <form onSubmit={createTicker} className="flex items-center gap-2">
        <input
          value={symbol}
          onChange={(e) => setSymbol(e.target.value)}
          placeholder="銘柄 (例: AAPL, 7203.T)"
          aria-label="銘柄カードのシンボル"
          className="border border-da-gray-200 bg-white px-3 py-2 text-sm text-da-gray-800"
        />
        <button
          type="submit"
          disabled={busy}
          className="border border-da-blue-900 px-4 py-2 text-sm font-bold text-da-blue-900 hover:bg-da-blue-50 disabled:opacity-50"
        >
          銘柄カード発行
        </button>
      </form>
      {busy && <span className="text-xs text-da-gray-400">発行中…</span>}
      {err && <span className="text-xs text-red-600">{err}</span>}
    </div>
  );
}
