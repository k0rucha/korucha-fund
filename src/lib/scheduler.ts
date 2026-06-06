//! Daily batch + backfill + cron scheduler. Ports OLD/src/services/scheduler.rs.
import cron from "node-cron";
import { listTransactions } from "@/db/queries/transactions";
import { getLatestPrices, getPriceOnOrBefore } from "@/db/queries/prices";
import { getLatestUsdJpy, getUsdJpyOnOrBefore } from "@/db/queries/fx";
import { upsertSnapshot } from "@/db/queries/snapshots";
import { recordApiRequestForced } from "@/db/queries/apiStats";
import {
  calculateHoldings,
  calculateHoldingsAsOf,
  type Transaction,
} from "@/lib/portfolio";
import {
  updatePriceCache,
  updateFxCache,
  backfillPriceHistory,
  backfillFxHistory,
} from "@/lib/yfinance";
import { jstToday, addDays } from "@/lib/format";

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/**
 * Daily batch: refresh latest prices for held symbols + FX, then write today's
 * snapshot. NOT gated by the rate limiter (it runs on schedule) but records one
 * forced API session so the status page reflects scheduler runs too.
 */
export async function runDailyBatch(): Promise<void> {
  console.log("[scheduler] daily batch start");
  const txs = listTransactions();
  const holdings = calculateHoldings(txs);
  if (holdings.length === 0) {
    console.log("[scheduler] no holdings, skipping");
    return;
  }

  for (const h of holdings) {
    try {
      await updatePriceCache(h.symbol);
    } catch (e) {
      console.warn("[scheduler] price update failed", h.symbol, e);
    }
    await sleep(200);
  }
  try {
    await updateFxCache();
  } catch (e) {
    console.warn("[scheduler] fx update failed", e);
  }
  try {
    recordApiRequestForced();
  } catch (e) {
    console.warn("[scheduler] record api request failed", e);
  }

  const priceMap = new Map(getLatestPrices().map((p) => [p.symbol, p.closePrice]));
  const usdjpyOpt = getLatestUsdJpy();
  const needsFx = holdings.some((h) => h.currency === "USD");
  if (needsFx && usdjpyOpt === null) {
    console.warn("[scheduler] USD holdings but no USDJPY — skipping snapshot");
    return;
  }
  const usdjpy = usdjpyOpt ?? 1.0;

  let totalValue = 0;
  let totalCost = 0;
  for (const h of holdings) {
    const price = priceMap.get(h.symbol) ?? 0;
    totalValue += h.quantity * price * (h.currency === "USD" ? usdjpy : 1);
    totalCost += h.totalCostJpy;
  }
  upsertSnapshot(jstToday(), totalValue, totalCost, totalValue - totalCost);
  console.log("[scheduler] daily batch done");
}

/**
 * Backfill historical prices/FX for all held symbols from the earliest tx date,
 * then regenerate every daily snapshot. Idempotent (existing rows preserved).
 */
export async function backfillAndRegenerate(): Promise<void> {
  const txs = listTransactions();
  if (txs.length === 0) return;

  const earliest = txs.reduce((m, t) => (t.txnDate < m ? t.txnDate : m), txs[0].txnDate);
  const today = jstToday();
  const symbols = [...new Set(txs.map((t) => t.symbol))].sort();

  for (const symbol of symbols) {
    try {
      const n = await backfillPriceHistory(symbol, earliest);
      console.log(`[backfill] ${symbol} → ${n} new rows`);
    } catch (e) {
      console.warn("[backfill] failed", symbol, e);
    }
    await sleep(200);
  }
  if (txs.some((t) => t.currency === "USD")) {
    try {
      const n = await backfillFxHistory(earliest);
      console.log(`[backfill] USDJPY → ${n} new rows`);
    } catch (e) {
      console.warn("[backfill] USDJPY failed", e);
    }
  }
  regenerateSnapshots(txs, earliest, today);
}

function regenerateSnapshots(txs: Transaction[], start: string, end: string): void {
  let date = start;
  let written = 0;
  while (date <= end) {
    const holdings = calculateHoldingsAsOf(txs, date);
    if (holdings.length === 0) {
      date = addDays(date, 1);
      continue;
    }
    const usdjpy = getUsdJpyOnOrBefore(date);
    const needsFx = holdings.some((h) => h.currency === "USD");
    // Skip rather than fabricate 150 JPY/USD — would corrupt the record.
    if (needsFx && usdjpy === null) {
      date = addDays(date, 1);
      continue;
    }

    let totalValue = 0;
    let totalCost = 0;
    let haveAnyPrice = false;
    for (const h of holdings) {
      const p = getPriceOnOrBefore(h.symbol, date);
      if (p !== null) haveAnyPrice = true;
      const price = p ?? 0;
      const fx = h.currency === "USD" ? (usdjpy as number) : 1;
      totalValue += h.quantity * price * fx;
      totalCost += h.totalCostJpy;
    }
    if (!haveAnyPrice) {
      date = addDays(date, 1);
      continue;
    }
    upsertSnapshot(date, totalValue, totalCost, totalValue - totalCost);
    written++;
    date = addDays(date, 1);
  }
  console.log(`[backfill] snapshots regenerated: ${written} days`);
}

let scheduled = false;

/** Start the daily cron (idempotent within a process). */
export function startScheduler(): void {
  if (scheduled) return;
  const expr = process.env.SCHEDULER_CRON ?? "0 0 23 * * *";
  if (!cron.validate(expr)) {
    console.error("[scheduler] invalid SCHEDULER_CRON:", expr);
    return;
  }
  cron.schedule(
    expr,
    () => {
      void runDailyBatch().catch((e) => console.error("[scheduler] batch error", e));
    },
    { timezone: "Asia/Tokyo" },
  );
  scheduled = true;
  console.log(`[scheduler] started: "${expr}" (Asia/Tokyo)`);
}
