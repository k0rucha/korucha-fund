//! Shared helpers for share cards. Ports the `compute_current` + id/span logic
//! from OLD/src/handlers/share.rs.
import { listTransactions } from "@/db/queries/transactions";
import { getLatestPrices } from "@/db/queries/prices";
import { getLatestUsdJpy } from "@/db/queries/fx";
import { getSymbolNames } from "@/db/queries/symbols";
import { calculateHoldings } from "@/lib/portfolio";

export interface CardHolding {
  symbol: string;
  name: string;
  currentValueJpy: number;
  unrealizedPnlJpy: number;
  unrealizedPnlPct: number;
}

export interface CurrentPortfolio {
  totalValue: number;
  totalCost: number;
  pnl: number;
  holdings: CardHolding[];
}

export function computeCurrentPortfolio(): CurrentPortfolio {
  const holdings = calculateHoldings(listTransactions());
  const priceMap = new Map(getLatestPrices().map((p) => [p.symbol, p.closePrice]));
  const usdjpy = getLatestUsdJpy() ?? 150.0; // display-only fallback
  const nameMap = new Map(
    getSymbolNames()
      .filter((s) => s.name)
      .map((s) => [s.symbol, s.name as string]),
  );

  let totalValue = 0;
  let totalCost = 0;
  const cards: CardHolding[] = [];
  for (const h of holdings) {
    const price = priceMap.get(h.symbol) ?? 0;
    const fx = h.currency === "USD" ? usdjpy : 1;
    const value = h.quantity * price * fx;
    const pnl = value - h.totalCostJpy;
    totalValue += value;
    totalCost += h.totalCostJpy;
    cards.push({
      symbol: h.symbol,
      name: nameMap.get(h.symbol) ?? "",
      currentValueJpy: value,
      unrealizedPnlJpy: pnl,
      unrealizedPnlPct: h.totalCostJpy > 0 ? (pnl / h.totalCostJpy) * 100 : 0,
    });
  }
  cards.sort((a, b) => b.currentValueJpy - a.currentValueJpy);
  return { totalValue, totalCost, pnl: totalValue - totalCost, holdings: cards };
}

/** Short, time-ordered hex id with a random salt (collision-retried by caller). */
export function generateId(): string {
  const t = (BigInt(Date.now()) * 1000n + BigInt(Math.floor(Math.random() * 1000))).toString(16);
  const salt = Math.floor(Math.random() * 0x10000)
    .toString(16)
    .padStart(4, "0");
  return t + salt;
}

export function normalizeSpan(s: string | null | undefined): "all" | "7d" | "30d" {
  return s === "7d" ? "7d" : s === "30d" ? "30d" : "all";
}
