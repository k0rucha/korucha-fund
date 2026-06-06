import { yen, signedYen, signedPct } from "@/lib/format";
import type { DashboardData } from "@/lib/dashboard";

function pnlClass(n: number): string {
  return n >= 0 ? "text-[rgb(var(--da-pos))]" : "text-[rgb(var(--da-neg))]";
}

function Tile({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="border border-da-gray-200 bg-da-gray-50 p-4">
      <div className="text-xs text-da-gray-400">{label}</div>
      <div className="mt-1 text-lg font-bold tabular-nums">{children}</div>
    </div>
  );
}

export default function KpiTiles({ data }: { data: DashboardData }) {
  return (
    <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
      <Tile label="総評価額">
        <span className="text-da-blue-1200">{yen(data.totalValueJpy)}</span>
      </Tile>
      <Tile label="含み損益">
        <span className={pnlClass(data.totalUnrealizedPnlJpy)}>
          {signedYen(data.totalUnrealizedPnlJpy)}{" "}
          <span className="text-sm">({signedPct(data.totalUnrealizedPnlPct)})</span>
        </span>
      </Tile>
      <Tile label="投資元本">{yen(data.totalCostJpy)}</Tile>
      <Tile label="実現損益">
        <span className={pnlClass(data.realizedPnlJpy)}>
          {signedYen(data.realizedPnlJpy)}
        </span>
      </Tile>
      <Tile label={data.dod ? `前日比 (${data.dod.refDate})` : "前日比"}>
        {data.dod ? (
          <span className={pnlClass(data.dod.valueDelta)}>
            {signedYen(data.dod.valueDelta)}{" "}
            <span className="text-sm">({signedPct(data.dod.valuePct)})</span>
          </span>
        ) : (
          <span className="text-da-gray-400">—</span>
        )}
      </Tile>
      <Tile label={data.mom ? `前月比 (${data.mom.refDate})` : "前月比"}>
        {data.mom ? (
          <span className={pnlClass(data.mom.valueDelta)}>
            {signedYen(data.mom.valueDelta)}{" "}
            <span className="text-sm">({signedPct(data.mom.valuePct)})</span>
          </span>
        ) : (
          <span className="text-da-gray-400">—</span>
        )}
      </Tile>
      <Tile label="累計損益（実現+含み）">
        <span className={pnlClass(data.cumulativePnlJpy)}>
          {signedYen(data.cumulativePnlJpy)}
        </span>
      </Tile>
    </div>
  );
}
