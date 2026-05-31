import type { DashboardResponse } from "../api/portfolio";

function fmt(v: number | null, prefix = "¥"): string {
  if (v === null) return "—";
  return `${prefix}${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`;
}

function fmtDelta(v: number | null): { text: string; cls: string } {
  if (v === null) return { text: "—", cls: "text-gray-400" };
  const sign = v >= 0 ? "+" : "";
  return {
    text: `${sign}¥${v.toLocaleString("ja-JP", { maximumFractionDigits: 0 })}`,
    cls: v >= 0 ? "text-emerald-400" : "text-red-400",
  };
}

function fmtPct(v: number | null): string {
  if (v === null) return "";
  return ` (${v >= 0 ? "+" : ""}${v.toFixed(2)}%)`;
}

interface TileProps {
  label: string;
  value: string;
  sub?: string;
  subCls?: string;
}

function Tile({ label, value, sub, subCls = "text-gray-400" }: TileProps) {
  return (
    <div className="bg-gray-800 rounded-xl p-4 flex flex-col gap-1">
      <span className="text-xs text-gray-400 uppercase tracking-wide">{label}</span>
      <span className="text-2xl font-semibold text-gray-100">{value}</span>
      {sub && <span className={`text-sm ${subCls}`}>{sub}</span>}
    </div>
  );
}

export default function KpiTiles({ data }: { data: DashboardResponse }) {
  const dod = fmtDelta(data.dod_delta_jpy);
  const mom = fmtDelta(data.mom_delta_jpy);
  const pnl = data.total_unrealized_pnl_jpy;
  const pnlCls = pnl === null ? "text-gray-400" : pnl >= 0 ? "text-emerald-400" : "text-red-400";

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
      <Tile label="評価額" value={fmt(data.total_value_jpy)} sub={`前日: ${dod.text}`} subCls={dod.cls} />
      <Tile
        label="含み損益"
        value={fmtDelta(pnl).text}
        sub={fmtPct(data.total_pnl_pct)}
        subCls={pnlCls}
      />
      <Tile label="実現損益" value={fmtDelta(data.realized_pnl_jpy).text} subCls={data.realized_pnl_jpy >= 0 ? "text-emerald-400" : "text-red-400"} />
      <Tile label="累計損益" value={fmtDelta(data.cumulative_pnl_jpy).text} subCls={data.cumulative_pnl_jpy !== null && data.cumulative_pnl_jpy >= 0 ? "text-emerald-400" : "text-red-400"} />
      <Tile label="取得原価" value={fmt(data.total_cost_jpy)} sub={`前月比: ${mom.text}`} subCls={mom.cls} />
    </div>
  );
}
