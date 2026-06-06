import { listTransactions, createTransaction } from "@/db/queries/transactions";
import { upsertSymbol } from "@/db/queries/symbols";

export const dynamic = "force-dynamic";

interface ImportRow {
  symbol: string;
  txn_type: string;
  quantity: number;
  price: number;
  currency: string;
  fee?: number;
  txn_date: string;
  fx_rate_to_jpy?: number | null;
  notes?: string | null;
}

/** Dedup fingerprint — identical to OLD/src/handlers/admin.rs. */
function fingerprint(t: {
  symbol: string;
  txnType: string;
  txnDate: string;
  quantity: number;
  price: number;
  fee: number;
}): string {
  return `${t.symbol}|${t.txnType}|${t.txnDate}|${t.quantity.toFixed(6)}|${t.price.toFixed(6)}|${t.fee.toFixed(6)}`;
}

export async function POST(req: Request) {
  const form = await req.formData();
  const file = form.get("file");
  if (!(file instanceof File)) {
    return Response.json({ error: "no file uploaded" }, { status: 400 });
  }

  let rows: ImportRow[];
  try {
    rows = JSON.parse(await file.text());
  } catch (e) {
    return Response.json(
      { error: "invalid JSON: " + (e instanceof Error ? e.message : String(e)) },
      { status: 400 },
    );
  }
  if (!Array.isArray(rows)) {
    return Response.json({ error: "expected a JSON array" }, { status: 400 });
  }

  const existingKeys = new Set(
    listTransactions().map((t) =>
      fingerprint({
        symbol: t.symbol,
        txnType: t.txnType,
        txnDate: t.txnDate,
        quantity: t.quantity,
        price: t.price,
        fee: t.fee,
      }),
    ),
  );

  const symbolsToTrack = new Map<string, string>();
  let imported = 0;
  let skipped = 0;

  for (const r of rows) {
    const fee = r.fee ?? 0;
    const fp = fingerprint({
      symbol: r.symbol,
      txnType: r.txn_type,
      txnDate: r.txn_date,
      quantity: r.quantity,
      price: r.price,
      fee,
    });
    if (existingKeys.has(fp)) {
      skipped++;
      continue;
    }
    createTransaction({
      symbol: r.symbol,
      txnType: r.txn_type,
      quantity: r.quantity,
      price: r.price,
      currency: r.currency,
      fee,
      txnDate: r.txn_date,
      fxRateToJpy: r.fx_rate_to_jpy ?? null,
      notes: r.notes ?? null,
    });
    existingKeys.add(fp);
    symbolsToTrack.set(r.symbol, r.currency);
    imported++;
  }

  for (const [symbol, currency] of symbolsToTrack) {
    upsertSymbol(symbol, null, currency);
  }

  return Response.json({ imported, skipped });
}
