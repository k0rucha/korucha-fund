"use client";

import { useRef, useState } from "react";
import { useRouter } from "next/navigation";
import type { Transaction } from "@/lib/portfolio";

const inputCls =
  "w-full border border-da-gray-200 bg-white px-3 py-2 text-sm text-da-gray-800 focus:border-da-blue-900 focus:outline-none";
const labelCls =
  "mb-1 block text-[10px] font-bold uppercase tracking-widest text-da-gray-600";

export default function AdminClient({
  transactions,
  symbolNames,
}: {
  transactions: Transaction[];
  symbolNames: Record<string, string>;
}) {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const importRef = useRef<HTMLInputElement>(null);

  async function addTransaction(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = e.currentTarget;
    const fd = new FormData(form);
    const body = {
      symbol: String(fd.get("symbol") ?? "").trim(),
      txnType: String(fd.get("txn_type") ?? "BUY"),
      quantity: Number(fd.get("quantity")),
      price: Number(fd.get("price")),
      currency: String(fd.get("currency") ?? "JPY"),
      fee: Number(fd.get("fee") ?? 0),
      txnDate: String(fd.get("txn_date") ?? ""),
      fxRateToJpy: fd.get("fx_rate_to_jpy") ? Number(fd.get("fx_rate_to_jpy")) : null,
      notes: fd.get("notes") ? String(fd.get("notes")) : null,
    };
    setBusy(true);
    setMsg(null);
    const res = await fetch("/api/admin/transactions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    setBusy(false);
    if (res.ok) {
      form.reset();
      setMsg("取引を追加しました");
      router.refresh();
    } else {
      const j = await res.json().catch(() => ({}));
      setMsg("追加に失敗: " + (j.error ?? res.status));
    }
  }

  async function del(id: number) {
    if (!confirm("この取引記録を削除してもよろしいですか？")) return;
    setBusy(true);
    const res = await fetch(`/api/admin/transactions/${id}`, { method: "DELETE" });
    setBusy(false);
    if (res.ok) {
      setMsg("削除しました");
      router.refresh();
    } else {
      setMsg("削除に失敗");
    }
  }

  async function onImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!confirm("JSONから取引データをインポートしますか？同じ取引はスキップされます。")) {
      e.target.value = "";
      return;
    }
    const fd = new FormData();
    fd.append("file", file);
    setBusy(true);
    setMsg(null);
    const res = await fetch("/api/admin/import", { method: "POST", body: fd });
    setBusy(false);
    e.target.value = "";
    if (res.ok) {
      const j = await res.json();
      setMsg(`インポート完了: ${j.imported} 件追加 / ${j.skipped} 件スキップ`);
      router.refresh();
    } else {
      const j = await res.json().catch(() => ({}));
      setMsg("インポートに失敗: " + (j.error ?? res.status));
    }
  }

  return (
    <div>
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-2xl font-black text-da-blue-1200">取引管理</h2>
        <div className="flex gap-2">
          <a
            href="/api/admin/export"
            className="border border-da-gray-200 px-3 py-2 text-xs font-bold text-da-blue-900 hover:bg-da-gray-50"
          >
            エクスポート (JSON)
          </a>
          <button
            type="button"
            onClick={() => importRef.current?.click()}
            className="border border-da-gray-200 px-3 py-2 text-xs font-bold text-da-gray-600 hover:bg-da-gray-50"
          >
            インポート (JSON)
          </button>
          <input
            ref={importRef}
            type="file"
            accept=".json"
            className="hidden"
            onChange={onImport}
          />
        </div>
      </div>

      {msg && (
        <div className="mb-4 border-l-4 border-da-blue-900 bg-da-blue-50 px-4 py-2 text-sm text-da-blue-900">
          {msg}
        </div>
      )}

      {/* Add transaction */}
      <form
        onSubmit={addTransaction}
        className="mb-10 grid grid-cols-1 gap-4 border border-da-gray-200 p-6 md:grid-cols-2 lg:grid-cols-4"
      >
        <div>
          <label className={labelCls} htmlFor="symbol">銘柄シンボル</label>
          <input id="symbol" name="symbol" required placeholder="例: 7203.T, AAPL" className={inputCls} />
        </div>
        <div>
          <label className={labelCls} htmlFor="txn_type">取引タイプ</label>
          <select id="txn_type" name="txn_type" required className={inputCls}>
            <option value="BUY">BUY</option>
            <option value="SELL">SELL</option>
          </select>
        </div>
        <div>
          <label className={labelCls} htmlFor="quantity">数量</label>
          <input id="quantity" name="quantity" type="number" step="any" required className={inputCls} />
        </div>
        <div>
          <label className={labelCls} htmlFor="price">単価 / 通貨</label>
          <div className="flex gap-2">
            <input id="price" name="price" type="number" step="any" required className={inputCls} />
            <select name="currency" required aria-label="通貨" className={`${inputCls} w-24`}>
              <option value="JPY">JPY</option>
              <option value="USD">USD</option>
            </select>
          </div>
        </div>
        <div>
          <label className={labelCls} htmlFor="fee">手数料</label>
          <input id="fee" name="fee" type="number" step="any" defaultValue="0" required className={inputCls} />
        </div>
        <div>
          <label className={labelCls} htmlFor="txn_date">約定日</label>
          <input id="txn_date" name="txn_date" type="date" required className={inputCls} />
        </div>
        <div className="md:col-span-2">
          <label className={labelCls} htmlFor="fx_rate_to_jpy">USD/JPY換算レート (USD取引のみ)</label>
          <input id="fx_rate_to_jpy" name="fx_rate_to_jpy" type="number" step="any" placeholder="150.0" className={inputCls} />
        </div>
        <div className="md:col-span-4">
          <label className={labelCls} htmlFor="notes">メモ</label>
          <input id="notes" name="notes" placeholder="任意" className={inputCls} />
        </div>
        <div className="md:col-span-4">
          <button
            type="submit"
            disabled={busy}
            className="bg-da-blue-1200 px-8 py-3 text-sm font-black uppercase tracking-widest text-white hover:bg-da-blue-900 disabled:opacity-50"
          >
            取引を記録する
          </button>
        </div>
      </form>

      {/* History */}
      <div className="border border-da-gray-200">
        <div className="flex items-center justify-between border-b border-da-gray-200 bg-da-gray-50 px-4 py-3">
          <h3 className="text-xs font-bold uppercase tracking-widest text-da-gray-600">取引履歴</h3>
          <span className="text-xs text-da-gray-400">{transactions.length} 件</span>
        </div>
        {transactions.length === 0 ? (
          <p className="px-4 py-12 text-center text-sm text-da-gray-400">取引履歴がありません</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="border-b border-da-gray-200 text-left text-xs text-da-gray-400">
                  <th className="px-4 py-3">ID</th>
                  <th className="px-4 py-3">約定日</th>
                  <th className="px-4 py-3">銘柄</th>
                  <th className="px-4 py-3 text-center">タイプ</th>
                  <th className="px-4 py-3 text-right">数量</th>
                  <th className="px-4 py-3 text-right">単価 / 通貨</th>
                  <th className="px-4 py-3 text-right">操作</th>
                </tr>
              </thead>
              <tbody>
                {transactions.map((t) => (
                  <tr key={t.id} className="border-b border-da-gray-200/60">
                    <td className="px-4 py-3 font-mono text-xs text-da-gray-400">#{t.id}</td>
                    <td className="px-4 py-3 tabular-nums text-da-gray-600">{t.txnDate}</td>
                    <td className="px-4 py-3 font-bold text-da-blue-1200" title={symbolNames[t.symbol] ?? ""}>
                      {t.symbol}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span
                        className={`inline-block border px-2 py-0.5 text-[10px] font-black ${
                          t.txnType === "BUY"
                            ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                            : "border-rose-200 bg-rose-50 text-rose-700"
                        }`}
                      >
                        {t.txnType}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-right tabular-nums text-da-gray-600">{t.quantity}</td>
                    <td className="px-4 py-3 text-right tabular-nums">
                      <span className="font-bold text-da-gray-800">{t.price}</span>
                      <span className="ml-1 text-xs text-da-gray-400">{t.currency}</span>
                    </td>
                    <td className="px-4 py-3 text-right">
                      <button
                        type="button"
                        onClick={() => del(t.id)}
                        disabled={busy}
                        aria-label={`取引 #${t.id} を削除`}
                        className="text-da-gray-400 hover:text-red-600 disabled:opacity-50"
                      >
                        削除
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
