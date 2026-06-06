import { eq } from "drizzle-orm";
import { db, schema } from "@/db";

const { shareCards } = schema;

export type ShareCard = typeof shareCards.$inferSelect;

export interface InsertShareCard {
  id: string;
  totalValueJpy: number;
  totalCostJpy: number;
  unrealizedPnlJpy: number;
  holdingsJson: string;
  defaultSpan: string;
}

/** Throws on PK collision (SqliteError) — the caller retries with a new id. */
export function insertShareCard(card: InsertShareCard): void {
  db.insert(shareCards).values(card).run();
}

export function getShareCard(id: string): ShareCard | null {
  return db.select().from(shareCards).where(eq(shareCards.id, id)).get() ?? null;
}
