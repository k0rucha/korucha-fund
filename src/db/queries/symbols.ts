import { eq, sql } from "drizzle-orm";
import { db, schema } from "@/db";

const { symbols } = schema;

export interface SymbolMeta {
  symbol: string;
  name: string | null;
  currency: string;
  exchange: string | null;
}

export function getSymbolNames(): SymbolMeta[] {
  return db
    .select({
      symbol: symbols.symbol,
      name: symbols.name,
      currency: symbols.currency,
      exchange: symbols.exchange,
    })
    .from(symbols)
    .all();
}

export function getSymbolName(symbol: string): string | null {
  const row = db
    .select({ name: symbols.name })
    .from(symbols)
    .where(eq(symbols.symbol, symbol))
    .get();
  return row?.name ?? null;
}

export function updateSymbolName(
  symbol: string,
  name: string,
  exchange: string | null,
): void {
  db
    .update(symbols)
    .set({ name, exchange, updatedAt: sql`CURRENT_TIMESTAMP` })
    .where(eq(symbols.symbol, symbol))
    .run();
}

export function upsertSymbol(
  symbol: string,
  name: string | null,
  currency: string,
): void {
  db
    .insert(symbols)
    .values({ symbol, name, currency, updatedAt: sql`CURRENT_TIMESTAMP` })
    .onConflictDoUpdate({
      target: symbols.symbol,
      set: {
        // Keep the existing name if the new one is NULL.
        name: sql`COALESCE(excluded.name, ${symbols.name})`,
        currency: sql`excluded.currency`,
        updatedAt: sql`CURRENT_TIMESTAMP`,
      },
    })
    .run();
}
