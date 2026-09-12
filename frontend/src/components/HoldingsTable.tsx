import type { HoldingView } from "../api/portfolio";

function fmt(v: number | null, prefix = "¥"): string {
  if (v === null) return "—";
  return `${prefix}${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`;
}

function fmtDelta(v: number | null): { text: string; cls: string } {
  if (v === null) return { text: "—", cls: "text-gray-400" };
  const sign = v >= 0 ? "+" : "";
  return { text: `${sign}¥${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`, cls: v >= 0 ? "text-emerald-400" : "text-red-400" };
}

export default function HoldingsTable({ holdings }: { holdings: HoldingView[] }) {
  if (!holdings.length) return <div className="text-gray-500 text-sm py-4">保有銘柄なし</div>;

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm text-gray-300">
        <thead>
          <tr className="border-b border-gray-700 text-gray-500 text-xs uppercase">
            <th className="text-left py-2 pr-4">銘柄</th>
            <th className="text-right py-2 pr-4">数量</th>
            <th className="text-right py-2 pr-4">平均取得単価</th>
            <th className="text-right py-2 pr-4">現在値</th>
            <th className="text-right py-2 pr-4">評価額</th>
            <th className="text-right py-2 pr-4">含み損益</th>
            <th className="text-right py-2 pr-4">前日比</th>
            <th className="text-right py-2">前月比</th>
          </tr>
        </thead>
        <tbody>
          {holdings.map((h) => {
            const pnl = fmtDelta(h.unrealized_pnl_jpy);
            const dod = fmtDelta(h.dod_delta_jpy);
            const mom = fmtDelta(h.mom_delta_jpy);
            const curSym = h.currency === "USD" ? "$" : "¥";
            return (
              <tr key={h.symbol} className="border-b border-gray-800 hover:bg-gray-800/50">
                <td className="py-3 pr-4">
                  <div className="font-medium text-gray-100">{h.symbol}</div>
                  {h.name && <div className="text-xs text-gray-500 truncate max-w-[140px]">{h.name}</div>}
                </td>
                <td className="text-right pr-4">{h.quantity.toLocaleString("ja-JP")}</td>
                <td className="text-right pr-4">{`${curSym}${h.avg_cost_native.toLocaleString("ja-JP", { maximumFractionDigits: 2 })}`}</td>
                <td className="text-right pr-4">
                  {h.current_price !== null ? `${curSym}${h.current_price.toLocaleString("ja-JP", { maximumFractionDigits: 2 })}` : "—"}
                </td>
                <td className="text-right pr-4">{fmt(h.current_value_jpy)}</td>
                <td className={`text-right pr-4 ${pnl.cls}`}>{pnl.text}</td>
                <td className={`text-right pr-4 ${dod.cls}`}>{dod.text}</td>
                <td className={`text-right ${mom.cls}`}>{mom.text}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
