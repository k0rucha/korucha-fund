import { listTransactions } from "@/db/queries/transactions";

export const dynamic = "force-dynamic";

export async function GET() {
  // Export in the same snake_case shape as the old Rust app so existing
  // exports round-trip through import unchanged.
  const out = listTransactions().map((t) => ({
    id: t.id,
    symbol: t.symbol,
    txn_type: t.txnType,
    quantity: t.quantity,
    price: t.price,
    currency: t.currency,
    fee: t.fee,
    txn_date: t.txnDate,
    fx_rate_to_jpy: t.fxRateToJpy,
    notes: t.notes,
    created_at: t.createdAt,
  }));

  return new Response(JSON.stringify(out, null, 2), {
    headers: {
      "Content-Type": "application/json",
      "Content-Disposition": 'attachment; filename="transactions.json"',
    },
  });
}
