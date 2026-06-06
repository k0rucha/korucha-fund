//! Manual price refresh. Ports OLD/src/handlers/refresh.rs.
import { listTransactions } from "@/db/queries/transactions";
import { getLatestPrices } from "@/db/queries/prices";
import { getLatestUsdJpy } from "@/db/queries/fx";
import { upsertSnapshot } from "@/db/queries/snapshots";
import { canRequestApi, requestsRemaining } from "@/db/queries/apiStats";
import { calculateHoldings } from "@/lib/portfolio";
import { tryUpdatePricesFromApi } from "@/lib/yfinance";
import { jstToday } from "@/lib/format";

export const dynamic = "force-dynamic";

// Single-flight guard: concurrent refreshes would double-count the limiter.
let refreshing = false;

export async function POST() {
  if (refreshing) {
    return Response.json({ error: "refresh already in progress" }, { status: 429 });
  }
  refreshing = true;
  try {
    const txs = listTransactions();
    const holdings = calculateHoldings(txs);
    const symbols = holdings.map((h) => h.symbol);

    // Try external refresh first; fall back to existing cache if blocked/failed.
    let updatedFromApi = false;
    if (canRequestApi().canRequest) {
      const { success, updatedFromApi: u } = await tryUpdatePricesFromApi(symbols);
      if (success) updatedFromApi = u;
    }

    // Write today's snapshot exactly once from the (possibly refreshed) cache.
    const priceMap = new Map(getLatestPrices().map((p) => [p.symbol, p.closePrice]));
    const usdjpy = getLatestUsdJpy();
    const needsFx = holdings.some((h) => h.currency === "USD");
    if (!(needsFx && usdjpy === null)) {
      const fx = usdjpy ?? 1.0;
      let totalValue = 0;
      let totalCost = 0;
      for (const h of holdings) {
        const price = priceMap.get(h.symbol) ?? 0;
        totalValue += h.quantity * price * (h.currency === "USD" ? fx : 1);
        totalCost += h.totalCostJpy;
      }
      upsertSnapshot(jstToday(), totalValue, totalCost, totalValue - totalCost);
    }

    return Response.json({
      ok: true,
      updated_from_api: updatedFromApi,
      remaining_api_requests: requestsRemaining(),
    });
  } finally {
    refreshing = false;
  }
}
