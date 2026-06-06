import { and, asc, desc, eq, gte, lte, sql } from "drizzle-orm";
import { db, schema } from "@/db";

const { priceCache } = schema;

export type PriceRow = typeof priceCache.$inferSelect;

/** Most recent cached close per symbol (correlated MAX(date) subquery). */
export function getLatestPrices(): PriceRow[] {
  return db
    .select()
    .from(priceCache)
    .where(
      sql`${priceCache.date} = (SELECT MAX(date) FROM price_cache p2 WHERE p2.symbol = ${priceCache.symbol})`,
    )
    .all();
}

/** Insert OR IGNORE a batch of (date, close) rows in one transaction. */
export function bulkInsertPrices(
  symbol: string,
  rows: [string, number][],
): number {
  if (rows.length === 0) return 0;
  return db.transaction((tx) => {
    let inserted = 0;
    for (const [date, close] of rows) {
      inserted += tx
        .insert(priceCache)
        .values({ symbol, date, closePrice: close })
        .onConflictDoNothing()
        .run().changes;
    }
    return inserted;
  });
}

export function getPriceOnOrBefore(symbol: string, date: string): number | null {
  const row = db
    .select({ c: priceCache.closePrice })
    .from(priceCache)
    .where(and(eq(priceCache.symbol, symbol), lte(priceCache.date, date)))
    .orderBy(desc(priceCache.date))
    .limit(1)
    .get();
  return row?.c ?? null;
}

export function countHistorySince(symbol: string, since: string): number {
  const row = db
    .select({ c: sql<number>`COUNT(*)` })
    .from(priceCache)
    .where(and(eq(priceCache.symbol, symbol), gte(priceCache.date, since)))
    .get();
  return row?.c ?? 0;
}

/** All (date, close) on or before `until`, ascending — for ticker charts. */
export function listHistory(
  symbol: string,
  until: string,
): { date: string; close: number }[] {
  return db
    .select({ date: priceCache.date, close: priceCache.closePrice })
    .from(priceCache)
    .where(and(eq(priceCache.symbol, symbol), lte(priceCache.date, until)))
    .orderBy(asc(priceCache.date))
    .all();
}

/** Upsert a single close (ON CONFLICT DO UPDATE) — used by daily/refresh. */
export function upsertPrice(symbol: string, date: string, close: number): void {
  db
    .insert(priceCache)
    .values({ symbol, date, closePrice: close })
    .onConflictDoUpdate({
      target: [priceCache.symbol, priceCache.date],
      set: { closePrice: close },
    })
    .run();
}
