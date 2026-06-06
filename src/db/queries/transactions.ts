import { desc, eq } from "drizzle-orm";
import { db, schema } from "@/db";
import type { Transaction } from "@/lib/portfolio";

const { transactions } = schema;

export interface CreateTransaction {
  symbol: string;
  txnType: string;
  quantity: number;
  price: number;
  currency: string;
  fee: number;
  txnDate: string;
  fxRateToJpy: number | null;
  notes: string | null;
}

/** Newest-first (matches OLD: ORDER BY txn_date DESC, id DESC). */
export function listTransactions(): Transaction[] {
  return db
    .select()
    .from(transactions)
    .orderBy(desc(transactions.txnDate), desc(transactions.id))
    .all();
}

export function createTransaction(data: CreateTransaction): number {
  const res = db.insert(transactions).values(data).run();
  return Number(res.lastInsertRowid);
}

export function deleteTransaction(id: number): number {
  return db.delete(transactions).where(eq(transactions.id, id)).run().changes;
}
