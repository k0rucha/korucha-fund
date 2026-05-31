import type { Transaction } from "../api/transactions";

interface Props {
  transactions: Transaction[];
  onDelete: (id: number) => void;
}

export default function TransactionTable({ transactions, onDelete }: Props) {
  if (!transactions.length) return <div className="text-gray-500 text-sm py-4">取引なし</div>;

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm text-gray-300">
        <thead>
          <tr className="border-b border-gray-700 text-gray-500 text-xs uppercase">
            <th className="text-left py-2 pr-4">日付</th>
            <th className="text-left py-2 pr-4">銘柄</th>
            <th className="text-left py-2 pr-4">種別</th>
            <th className="text-right py-2 pr-4">数量</th>
            <th className="text-right py-2 pr-4">単価</th>
            <th className="text-left py-2 pr-4">通貨</th>
            <th className="text-right py-2 pr-4">手数料</th>
            <th className="text-left py-2 pr-4">メモ</th>
            <th className="py-2"></th>
          </tr>
        </thead>
        <tbody>
          {transactions.map((t) => (
            <tr key={t.id} className="border-b border-gray-800 hover:bg-gray-800/50">
              <td className="py-2 pr-4 text-gray-400">{t.txn_date}</td>
              <td className="py-2 pr-4">
                <div className="font-medium">{t.symbol}</div>
                {t.symbol_name && <div className="text-xs text-gray-500 truncate max-w-[120px]">{t.symbol_name}</div>}
              </td>
              <td className="py-2 pr-4">
                <span className={`px-2 py-0.5 rounded text-xs font-medium ${t.txn_type === "BUY" ? "bg-emerald-900 text-emerald-300" : "bg-red-900 text-red-300"}`}>
                  {t.txn_type}
                </span>
              </td>
              <td className="text-right pr-4">{t.quantity.toLocaleString("ja-JP")}</td>
              <td className="text-right pr-4">{t.price.toLocaleString("ja-JP", { maximumFractionDigits: 2 })}</td>
              <td className="pr-4">{t.currency}</td>
              <td className="text-right pr-4">{t.fee.toLocaleString("ja-JP")}</td>
              <td className="pr-4 text-gray-500 truncate max-w-[100px]">{t.notes ?? ""}</td>
              <td>
                <button
                  onClick={() => onDelete(t.id)}
                  className="text-red-500 hover:text-red-400 text-xs px-2 py-1 rounded hover:bg-red-950"
                >
                  削除
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
