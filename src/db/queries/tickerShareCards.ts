import { eq } from "drizzle-orm";
import { db, schema } from "@/db";

const { tickerShareCards } = schema;

export type TickerShareCard = typeof tickerShareCards.$inferSelect;

export interface InsertTickerShareCard {
  id: string;
  symbol: string;
  displayName: string | null;
  currency: string;
  issuePriceNative: number;
  fxRateAtIssue: number | null;
  quantity: number | null;
  avgCostNative: number | null;
  issueValueJpy: number | null;
  issuePnlJpy: number | null;
  defaultSpan: string;
}

/** Throws on PK collision (SqliteError) — the caller retries with a new id. */
export function insertTickerShareCard(card: InsertTickerShareCard): void {
  db.insert(tickerShareCards).values(card).run();
}

export function getTickerShareCard(id: string): TickerShareCard | null {
  return (
    db.select().from(tickerShareCards).where(eq(tickerShareCards.id, id)).get() ??
    null
  );
}
