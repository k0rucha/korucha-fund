import { useEffect, useState, useCallback } from "react";
import {
  getTransactions, deleteTransaction, exportTransactions, importTransactions,
} from "../api/transactions";
import type { Transaction } from "../api/transactions";
import TransactionForm from "../components/TransactionForm";
import TransactionTable from "../components/TransactionTable";

function useCredentials() {
  const [user, setUser] = useState(() => sessionStorage.getItem("admin_user") ?? "");
  const [pass, setPass] = useState(() => sessionStorage.getItem("admin_pass") ?? "");
  const save = (u: string, p: string) => {
    setUser(u); setPass(p);
    sessionStorage.setItem("admin_user", u);
    sessionStorage.setItem("admin_pass", p);
  };
  return { user, pass, save };
}

export default function AdminPage() {
  const { user, pass, save } = useCredentials();
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [authed, setAuthed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [inputUser, setInputUser] = useState(user);
  const [inputPass, setInputPass] = useState("");
  const [importing, setImporting] = useState(false);

  const load = useCallback(async () => {
    if (!user || !pass) return;
    try {
      const data = await getTransactions(user, pass);
      setTransactions(data);
      setAuthed(true);
      setError(null);
    } catch (e: unknown) {
      setAuthed(false);
      setError(e instanceof Error ? e.message : "認証エラー");
    }
  }, [user, pass]);

  useEffect(() => { if (user && pass) load(); }, [load, user, pass]);

  const handleLogin = (e: React.FormEvent) => {
    e.preventDefault();
    save(inputUser, inputPass);
  };

  const handleDelete = async (id: number) => {
    if (!confirm("削除しますか？")) return;
    await deleteTransaction(id, user, pass);
    load();
  };

  const handleExport = async () => {
    const blob = await exportTransactions(user, pass);
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "transactions.json";
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setImporting(true);
    try {
      const result = await importTransactions(file, user, pass);
      alert(`インポート完了: ${result.imported} 件追加, ${result.skipped} 件スキップ`);
      load();
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : "インポートエラー");
    } finally {
      setImporting(false);
      e.target.value = "";
    }
  };

  if (!authed) {
    return (
      <div className="p-6 max-w-sm mx-auto mt-20">
        <h1 className="text-lg font-semibold text-gray-100 mb-6">管理画面ログイン</h1>
        <form onSubmit={handleLogin} className="space-y-4">
          <input
            className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-gray-100 focus:outline-none focus:border-violet-500"
            placeholder="ユーザー名"
            value={inputUser}
            onChange={(e) => setInputUser(e.target.value)}
            required
          />
          <input
            type="password"
            className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-gray-100 focus:outline-none focus:border-violet-500"
            placeholder="パスワード"
            value={inputPass}
            onChange={(e) => setInputPass(e.target.value)}
            required
          />
          {error && <p className="text-red-400 text-sm">{error}</p>}
          <button type="submit" className="w-full py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg">
            ログイン
          </button>
        </form>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-gray-100">取引管理</h1>
        <div className="flex gap-2">
          <button onClick={handleExport} className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg">
            エクスポート
          </button>
          <label className={`px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg cursor-pointer ${importing ? "opacity-50" : ""}`}>
            {importing ? "インポート中…" : "インポート"}
            <input type="file" accept=".json" className="hidden" onChange={handleImport} disabled={importing} />
          </label>
        </div>
      </div>

      <div className="bg-gray-800 rounded-xl p-4">
        <h2 className="text-sm font-medium text-gray-400 mb-4">取引追加</h2>
        <TransactionForm credentials={{ user, pass }} onAdded={load} />
      </div>

      <div className="bg-gray-800 rounded-xl p-4">
        <h2 className="text-sm font-medium text-gray-400 mb-4">取引一覧 ({transactions.length}件)</h2>
        <TransactionTable transactions={transactions} onDelete={handleDelete} />
      </div>
    </div>
  );
}
