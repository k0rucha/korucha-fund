import { and, desc, eq, lte } from "drizzle-orm";
import { db, schema } from "@/db";

const { fxCache } = schema;
const PAIR = "USDJPY";

export function getLatestUsdJpy(): number | null {
  const row = db
    .select({ rate: fxCache.rate })
    .from(fxCache)
    .where(eq(fxCache.pair, PAIR))
    .orderBy(desc(fxCache.date))
    .limit(1)
    .get();
  return row?.rate ?? null;
}

export function bulkInsertUsdJpy(rows: [string, number][]): number {
  if (rows.length === 0) return 0;
  return db.transaction((tx) => {
    let inserted = 0;
    for (const [date, rate] of rows) {
      inserted += tx
        .insert(fxCache)
        .values({ pair: PAIR, date, rate })
        .onConflictDoNothing()
        .run().changes;
    }
    return inserted;
  });
}

export function getUsdJpyOnOrBefore(date: string): number | null {
  const row = db
    .select({ rate: fxCache.rate })
    .from(fxCache)
    .where(and(eq(fxCache.pair, PAIR), lte(fxCache.date, date)))
    .orderBy(desc(fxCache.date))
    .limit(1)
    .get();
  return row?.rate ?? null;
}

/** Upsert a single USDJPY rate (ON CONFLICT DO UPDATE) — used by daily/refresh. */
export function upsertUsdJpy(date: string, rate: number): void {
  db
    .insert(fxCache)
    .values({ pair: PAIR, date, rate })
    .onConflictDoUpdate({
      target: [fxCache.pair, fxCache.date],
      set: { rate },
    })
    .run();
}
