import { z } from "zod";
import { createTransaction } from "@/db/queries/transactions";
import { upsertSymbol } from "@/db/queries/symbols";

export const dynamic = "force-dynamic";

const schema = z.object({
  symbol: z.string().trim().min(1),
  txnType: z.enum(["BUY", "SELL"]),
  quantity: z.number(),
  price: z.number(),
  currency: z.string().trim().min(1),
  fee: z.number().optional(),
  txnDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  fxRateToJpy: z.number().nullable().optional(),
  notes: z.string().nullable().optional(),
});

export async function POST(req: Request) {
  let raw: unknown;
  try {
    raw = await req.json();
  } catch {
    return Response.json({ error: "invalid JSON" }, { status: 400 });
  }

  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    return Response.json({ error: parsed.error.issues[0]?.message ?? "invalid" }, { status: 400 });
  }
  const d = parsed.data;

  const id = createTransaction({
    symbol: d.symbol,
    txnType: d.txnType,
    quantity: d.quantity,
    price: d.price,
    currency: d.currency,
    fee: d.fee ?? 0,
    txnDate: d.txnDate,
    fxRateToJpy: d.fxRateToJpy ?? null,
    notes: d.notes ?? null,
  });

  // Track the symbol's currency so it shows up in the symbols table. Name +
  // price-cache population happens via yfinance (P4).
  upsertSymbol(d.symbol, null, d.currency);

  return Response.json({ id, ok: true });
}
