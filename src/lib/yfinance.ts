//! Yahoo Finance access (yahoo-finance2 v3). Ports OLD/src/services/yfinance.rs.
//! v3 replaced `historical()` with `chart()`, so daily history + latest close
//! both come from `chart(symbol, { interval: "1d" })`.
import YahooFinance from "yahoo-finance2";
import { bulkInsertPrices, upsertPrice } from "@/db/queries/prices";
import { bulkInsertUsdJpy, upsertUsdJpy } from "@/db/queries/fx";
import {
  upsertSymbol,
  getSymbolName,
  updateSymbolName,
} from "@/db/queries/symbols";
import { tryRecordApiRequest } from "@/db/queries/apiStats";

// yahoo-finance2 v3 exposes a class — instantiate one per process.
const yahooFinance = new YahooFinance();

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** A Date → JST `YYYY-MM-DD` (spec §11: all dates JST). */
function toJstDate(d: Date): string {
  return new Date(d.getTime() + 9 * 3600 * 1000).toISOString().slice(0, 10);
}

interface ChartQuote {
  date: Date;
  close: number | null;
}

async function dailyQuotes(symbol: string, period1: Date): Promise<ChartQuote[]> {
  const res = await yahooFinance.chart(symbol, { period1, interval: "1d" });
  return (res.quotes ?? []) as ChartQuote[];
}

function usableRows(quotes: ChartQuote[]): [string, number][] {
  const rows: [string, number][] = [];
  for (const q of quotes) {
    if (q.close != null && Number.isFinite(q.close) && q.close !== 0 && q.date) {
      rows.push([toJstDate(new Date(q.date)), q.close]);
    }
  }
  return rows;
}

export async function getLatestClosePrice(
  symbol: string,
): Promise<{ price: number; date: string }> {
  const quotes = await dailyQuotes(symbol, new Date(Date.now() - 7 * 86400 * 1000));
  const rows = usableRows(quotes);
  const last = rows.at(-1);
  if (!last) throw new Error(`no usable close price for ${symbol}`);
  return { date: last[0], price: last[1] };
}

/** Short/long company name via the search API. */
export async function fetchSymbolName(symbol: string): Promise<string | null> {
  const res = await yahooFinance.search(symbol);
  const quotes = (res.quotes ?? []) as Array<{
    symbol?: string;
    longname?: string;
    shortname?: string;
  }>;
  const pick = (q?: { longname?: string; shortname?: string }) =>
    q?.longname || q?.shortname || null;
  const exact = quotes.find((q) => q.symbol === symbol);
  return pick(exact) ?? pick(quotes[0]) ?? null;
}

export async function updatePriceCache(symbol: string): Promise<void> {
  const { price, date } = await getLatestClosePrice(symbol);
  upsertPrice(symbol, date, price);

  const currency = symbol.endsWith(".T") ? "JPY" : "USD";
  upsertSymbol(symbol, null, currency);

  if (!getSymbolName(symbol)) {
    try {
      const name = await fetchSymbolName(symbol);
      if (name) updateSymbolName(symbol, name, null);
    } catch {
      /* name is best-effort */
    }
  }
}

/** Daily closes from `start` (YYYY-MM-DD) through today; INSERT OR IGNORE. */
export async function backfillPriceHistory(
  symbol: string,
  start: string,
): Promise<number> {
  const quotes = await dailyQuotes(symbol, new Date(start + "T00:00:00Z"));
  return bulkInsertPrices(symbol, usableRows(quotes));
}

export async function backfillFxHistory(start: string): Promise<number> {
  const quotes = await dailyQuotes("USDJPY=X", new Date(start + "T00:00:00Z"));
  return bulkInsertUsdJpy(usableRows(quotes));
}

export async function updateFxCache(): Promise<void> {
  const { price, date } = await getLatestClosePrice("USDJPY=X");
  upsertUsdJpy(date, price);
}

/**
 * Claim a rate-limit slot, then refresh the given symbols + FX. Returns
 * `{ success, updatedFromApi }`. The slot is claimed atomically BEFORE any
 * network I/O so concurrent callers can't both proceed.
 */
export async function tryUpdatePricesFromApi(
  symbols: string[],
): Promise<{ success: boolean; updatedFromApi: boolean }> {
  let claimed: boolean;
  try {
    claimed = tryRecordApiRequest();
  } catch {
    return { success: false, updatedFromApi: false };
  }
  if (!claimed) return { success: true, updatedFromApi: false };

  let success = true;
  for (const symbol of symbols) {
    try {
      await updatePriceCache(symbol);
    } catch {
      success = false;
    }
    await sleep(200);
  }
  try {
    await updateFxCache();
  } catch {
    success = false;
  }
  return { success, updatedFromApi: true };
}
