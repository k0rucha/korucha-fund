import { yen, signedYen, signedPct } from "@/lib/format";
import type { DashboardHolding } from "@/lib/dashboard";

function pnlClass(n: number): string {
  return n >= 0 ? "text-[rgb(var(--da-pos))]" : "text-[rgb(var(--da-neg))]";
}

export default function HoldingsTable({
  holdings,
}: {
  holdings: DashboardHolding[];
}) {
  if (holdings.length === 0) {
    return <p className="text-sm text-da-gray-400">保有銘柄がありません</p>;
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-da-gray-200 text-left text-xs text-da-gray-400">
            <th className="py-2 pr-3">銘柄</th>
            <th className="px-2 py-2 text-right">数量</th>
            <th className="px-2 py-2 text-right">平均取得</th>
            <th className="px-2 py-2 text-right">現在値</th>
            <th className="px-2 py-2 text-right">評価額</th>
            <th className="px-2 py-2 text-right">含み損益</th>
            <th className="px-2 py-2 text-right">前日比</th>
            <th className="py-2 pl-2 text-right">前月比</th>
          </tr>
        </thead>
        <tbody>
          {holdings.map((h) => (
            <tr key={h.symbol} className="border-b border-da-gray-200/60">
              <td className="py-2 pr-3">
                <span className="font-bold text-da-blue-1200">{h.symbol}</span>
                {h.name && (
                  <span className="ml-2 text-xs text-da-gray-400">{h.name}</span>
                )}
              </td>
              <td className="px-2 py-2 text-right tabular-nums">
                {h.quantity.toFixed(2)}
              </td>
              <td className="px-2 py-2 text-right tabular-nums">
                {h.averageCostNative.toFixed(2)}
              </td>
              <td className="px-2 py-2 text-right tabular-nums">
                {h.currentPriceNative.toFixed(2)}
              </td>
              <td className="px-2 py-2 text-right tabular-nums">
                {yen(h.currentValueJpy)}
              </td>
              <td
                className={`px-2 py-2 text-right tabular-nums ${pnlClass(h.unrealizedPnlJpy)}`}
              >
                {signedYen(h.unrealizedPnlJpy)}
                <span className="ml-1 text-xs">
                  ({signedPct(h.unrealizedPnlPct)})
                </span>
              </td>
              <td
                className={`px-2 py-2 text-right tabular-nums ${h.dodAvailable ? pnlClass(h.dodPnlJpy) : "text-da-gray-400"}`}
              >
                {h.dodAvailable ? signedPct(h.dodPnlPct) : "—"}
              </td>
              <td
                className={`py-2 pl-2 text-right tabular-nums ${h.momAvailable ? pnlClass(h.momPnlJpy) : "text-da-gray-400"}`}
              >
                {h.momAvailable ? signedPct(h.momPnlPct) : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
