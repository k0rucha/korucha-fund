import { useState } from "react";
import type { TransactionCreate } from "../api/transactions";
import { addTransaction } from "../api/transactions";

interface Props {
  credentials: { user: string; pass: string };
  onAdded: () => void;
}

const defaultForm: TransactionCreate = {
  symbol: "",
  txn_type: "BUY",
  quantity: 0,
  price: 0,
  currency: "JPY",
  fee: 0,
  txn_date: new Date().toISOString().slice(0, 10),
  fx_rate_to_jpy: null,
  notes: null,
};

export default function TransactionForm({ credentials, onAdded }: Props) {
  const [form, setForm] = useState<TransactionCreate>(defaultForm);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const set = <K extends keyof TransactionCreate>(k: K, v: TransactionCreate[K]) =>
    setForm((prev) => ({ ...prev, [k]: v }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await addTransaction(form, credentials.user, credentials.pass);
      setForm(defaultForm);
      onAdded();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "エラー");
    } finally {
      setLoading(false);
    }
  };

  const inputCls =
    "w-full bg-gray-700 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-100 focus:outline-none focus:border-violet-500";

  return (
    <form onSubmit={handleSubmit} className="grid grid-cols-2 md:grid-cols-4 gap-3">
      <div>
        <label className="text-xs text-gray-400">銘柄コード</label>
        <input className={inputCls} value={form.symbol} onChange={(e) => set("symbol", e.target.value.toUpperCase())} required />
      </div>
      <div>
        <label className="text-xs text-gray-400">種別</label>
        <select className={inputCls} value={form.txn_type} onChange={(e) => set("txn_type", e.target.value as "BUY" | "SELL")}>
          <option value="BUY">BUY</option>
          <option value="SELL">SELL</option>
        </select>
      </div>
      <div>
        <label className="text-xs text-gray-400">数量</label>
        <input type="number" className={inputCls} value={form.quantity || ""} onChange={(e) => set("quantity", Number(e.target.value))} required min={0.0001} step="any" />
      </div>
      <div>
        <label className="text-xs text-gray-400">単価</label>
        <input type="number" className={inputCls} value={form.price || ""} onChange={(e) => set("price", Number(e.target.value))} required min={0.0001} step="any" />
      </div>
      <div>
        <label className="text-xs text-gray-400">通貨</label>
        <select className={inputCls} value={form.currency} onChange={(e) => set("currency", e.target.value as "JPY" | "USD")}>
          <option value="JPY">JPY</option>
          <option value="USD">USD</option>
        </select>
      </div>
      <div>
        <label className="text-xs text-gray-400">手数料</label>
        <input type="number" className={inputCls} value={form.fee || ""} onChange={(e) => set("fee", Number(e.target.value))} min={0} step="any" />
      </div>
      <div>
        <label className="text-xs text-gray-400">約定日</label>
        <input type="date" className={inputCls} value={form.txn_date} onChange={(e) => set("txn_date", e.target.value)} required />
      </div>
      <div>
        <label className="text-xs text-gray-400">FXレート (USD のみ)</label>
        <input type="number" className={inputCls} value={form.fx_rate_to_jpy ?? ""} onChange={(e) => set("fx_rate_to_jpy", e.target.value ? Number(e.target.value) : null)} step="any" placeholder="例: 150.5" />
      </div>
      <div className="col-span-2 md:col-span-4 flex items-center gap-3">
        <button type="submit" disabled={loading} className="px-4 py-2 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 text-white text-sm rounded-lg">
          {loading ? "追加中…" : "追加"}
        </button>
        {error && <span className="text-red-400 text-sm">{error}</span>}
      </div>
    </form>
  );
}
