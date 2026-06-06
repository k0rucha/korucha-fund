//! Dashboard data computation. Faithful port of OLD/src/handlers/dashboard.rs.
//! Returns raw numbers; formatting happens client-side (see src/lib/format.ts).
import { listTransactions } from "@/db/queries/transactions";
import { getLatestPrices, getPriceOnOrBefore } from "@/db/queries/prices";
import { getLatestUsdJpy, getUsdJpyOnOrBefore } from "@/db/queries/fx";
import { getSymbolNames } from "@/db/queries/symbols";
import { getSnapshotOnOrBefore } from "@/db/queries/snapshots";
import { calculateHoldings, calculateRealizedPnl } from "@/lib/portfolio";
import { jstToday, addDays } from "@/lib/format";

export interface DashboardHolding {
  symbol: string;
  name: string;
  quantity: number;
  averageCostNative: number;
  currentPriceNative: number;
  totalCostJpy: number;
  currentValueJpy: number;
  unrealizedPnlJpy: number;
  unrealizedPnlPct: number;
  dodAvailable: boolean;
  dodPnlJpy: number;
  dodPnlPct: number;
  momAvailable: boolean;
  momPnlJpy: number;
  momPnlPct: number;
}

export interface DashboardDelta {
  refDate: string;
  valueDelta: number;
  valuePct: number;
  costDelta: number;
  pnlDelta: number;
  pnlPctDelta: number;
}

export interface DashboardData {
  holdings: DashboardHolding[];
  totalCostJpy: number;
  totalValueJpy: number;
  totalUnrealizedPnlJpy: number;
  totalUnrealizedPnlPct: number;
  realizedPnlJpy: number;
  cumulativePnlJpy: number;
  dod: DashboardDelta | null;
  mom: DashboardDelta | null;
}

export function computeDashboard(): DashboardData {
  const txs = listTransactions();
  const holdings = calculateHoldings(txs);
  const realizedPnlJpy = calculateRealizedPnl(txs);

  const priceMap = new Map(getLatestPrices().map((p) => [p.symbol, p.closePrice]));
  // Display-only fallback: nominal 150 JPY/USD when no FX cached (matches OLD).
  const usdjpy = getLatestUsdJpy() ?? 150.0;
  const nameMap = new Map(
    getSymbolNames()
      .filter((s) => s.name)
      .map((s) => [s.symbol, s.name as string]),
  );

  const today = jstToday();
  const yesterday = addDays(today, -1);
  const oneMonthAgo = addDays(today, -30);

  let totalCostJpy = 0;
  let totalValueJpy = 0;
  const views: DashboardHolding[] = [];

  for (const h of holdings) {
    const currentPrice = priceMap.get(h.symbol) ?? 0;
    const fxRate = h.currency === "USD" ? usdjpy : 1.0;
    const currentValueJpy = h.quantity * currentPrice * fxRate;
    const unrealizedPnlJpy = currentValueJpy - h.totalCostJpy;
    const unrealizedPnlPct =
      h.totalCostJpy > 0 ? (unrealizedPnlJpy / h.totalCostJpy) * 100 : 0;

    totalCostJpy += h.totalCostJpy;
    totalValueJpy += currentValueJpy;

    // Per-holding dod/mom: current value vs prev price × current quantity.
    const isJpy = h.symbol.endsWith(".T");
    const dodPrevPrice = getPriceOnOrBefore(h.symbol, yesterday);
    const momPrevPrice = getPriceOnOrBefore(h.symbol, oneMonthAgo);

    let dodAvailable = false;
    let dodPnlJpy = 0;
    let dodPnlPct = 0;
    if (dodPrevPrice !== null) {
      const prevFx = isJpy ? 1.0 : (getUsdJpyOnOrBefore(yesterday) ?? usdjpy);
      const prevValue = h.quantity * dodPrevPrice * prevFx;
      const delta = currentValueJpy - prevValue;
      dodAvailable = true;
      dodPnlJpy = delta;
      dodPnlPct = prevValue > 0 ? (delta / prevValue) * 100 : 0;
    }

    let momAvailable = false;
    let momPnlJpy = 0;
    let momPnlPct = 0;
    if (momPrevPrice !== null) {
      const prevFx = isJpy ? 1.0 : (getUsdJpyOnOrBefore(oneMonthAgo) ?? usdjpy);
      const prevValue = h.quantity * momPrevPrice * prevFx;
      const delta = currentValueJpy - prevValue;
      momAvailable = true;
      momPnlJpy = delta;
      momPnlPct = prevValue > 0 ? (delta / prevValue) * 100 : 0;
    }

    views.push({
      symbol: h.symbol,
      name: nameMap.get(h.symbol) ?? "",
      quantity: h.quantity,
      averageCostNative: h.averageCostNative,
      currentPriceNative: currentPrice,
      totalCostJpy: h.totalCostJpy,
      currentValueJpy,
      unrealizedPnlJpy,
      unrealizedPnlPct,
      dodAvailable,
      dodPnlJpy,
      dodPnlPct,
      momAvailable,
      momPnlJpy,
      momPnlPct,
    });
  }

  // Largest holding first.
  views.sort((a, b) => b.currentValueJpy - a.currentValueJpy);

  const totalUnrealizedPnlJpy = totalValueJpy - totalCostJpy;
  const totalUnrealizedPnlPct =
    totalCostJpy > 0 ? (totalUnrealizedPnlJpy / totalCostJpy) * 100 : 0;

  // MoM totals: snapshot closest to ~30d ago, suppressed if older than 60d.
  const momFloor = addDays(today, -60);
  const prevMom = getSnapshotOnOrBefore(oneMonthAgo);
  let mom: DashboardDelta | null = null;
  if (prevMom && prevMom.date >= momFloor) {
    const valueDelta = totalValueJpy - prevMom.totalValueJpy;
    const prevPnlPct =
      prevMom.totalCostJpy > 0
        ? (prevMom.unrealizedPnlJpy / prevMom.totalCostJpy) * 100
        : 0;
    mom = {
      refDate: prevMom.date,
      valueDelta,
      valuePct: prevMom.totalValueJpy > 0 ? (valueDelta / prevMom.totalValueJpy) * 100 : 0,
      costDelta: totalCostJpy - prevMom.totalCostJpy,
      pnlDelta: totalUnrealizedPnlJpy - prevMom.unrealizedPnlJpy,
      pnlPctDelta: totalUnrealizedPnlPct - prevPnlPct,
    };
  }

  // DoD totals: most recent snapshot before today, suppressed if older than 7d.
  const dodFloor = addDays(today, -7);
  const prevDod = getSnapshotOnOrBefore(yesterday);
  let dod: DashboardDelta | null = null;
  if (prevDod && prevDod.date >= dodFloor) {
    const valueDelta = totalValueJpy - prevDod.totalValueJpy;
    const prevPnlPct =
      prevDod.totalCostJpy > 0
        ? (prevDod.unrealizedPnlJpy / prevDod.totalCostJpy) * 100
        : 0;
    dod = {
      refDate: prevDod.date,
      valueDelta,
      valuePct: prevDod.totalValueJpy > 0 ? (valueDelta / prevDod.totalValueJpy) * 100 : 0,
      costDelta: totalCostJpy - prevDod.totalCostJpy,
      pnlDelta: totalUnrealizedPnlJpy - prevDod.unrealizedPnlJpy,
      pnlPctDelta: totalUnrealizedPnlPct - prevPnlPct,
    };
  }

  return {
    holdings: views,
    totalCostJpy,
    totalValueJpy,
    totalUnrealizedPnlJpy,
    totalUnrealizedPnlPct,
    realizedPnlJpy,
    cumulativePnlJpy: realizedPnlJpy + totalUnrealizedPnlJpy,
    dod,
    mom,
  };
}
