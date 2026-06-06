//! Portfolio composition (for the donut chart). Ports OLD/src/handlers/fragments.rs.
import { listTransactions } from "@/db/queries/transactions";
import { getLatestPrices } from "@/db/queries/prices";
import { getLatestUsdJpy } from "@/db/queries/fx";
import { getSymbolNames } from "@/db/queries/symbols";
import { calculateHoldings } from "@/lib/portfolio";

const NAME_TRUNCATE_CHARS = 12;

function truncateName(name: string): string {
  const chars = [...name];
  return chars.length <= NAME_TRUNCATE_CHARS
    ? name
    : chars.slice(0, NAME_TRUNCATE_CHARS).join("") + "…";
}

export interface CompositionData {
  labels: string[];
  fullNames: string[];
  values: number[];
}

export function computeComposition(): CompositionData {
  const holdings = calculateHoldings(listTransactions());
  const priceMap = new Map(getLatestPrices().map((p) => [p.symbol, p.closePrice]));
  const usdjpy = getLatestUsdJpy() ?? 150.0;
  const nameMap = new Map(
    getSymbolNames()
      .filter((s) => s.name)
      .map((s) => [s.symbol, s.name as string]),
  );

  const entries = holdings
    .map((h) => {
      const price = priceMap.get(h.symbol) ?? 0;
      const fx = h.currency === "USD" ? usdjpy : 1.0;
      const value = h.quantity * price * fx;
      return {
        symbol: h.symbol,
        name: nameMap.get(h.symbol) ?? "",
        value: Math.round(value * 100) / 100,
      };
    })
    .filter((e) => e.value > 0)
    // Largest slice first — deterministic legend order across reloads.
    .sort((a, b) => b.value - a.value);

  return {
    labels: entries.map((e) =>
      e.name ? `${e.symbol} ${truncateName(e.name)}` : e.symbol,
    ),
    fullNames: entries.map((e) => e.name || e.symbol),
    values: entries.map((e) => e.value),
  };
}
