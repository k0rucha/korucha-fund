import { listTransactions } from "@/db/queries/transactions";
import { countHistorySince, getPriceOnOrBefore } from "@/db/queries/prices";
import { getLatestUsdJpy } from "@/db/queries/fx";
import { getSymbolNames, upsertSymbol } from "@/db/queries/symbols";
import { recordApiRequestForced } from "@/db/queries/apiStats";
import { insertTickerShareCard } from "@/db/queries/tickerShareCards";
import { calculateHoldings } from "@/lib/portfolio";
import { backfillPriceHistory, fetchSymbolName } from "@/lib/yfinance";
import { generateId, normalizeSpan } from "@/lib/share";
import { jstToday, addDays } from "@/lib/format";

export const dynamic = "force-dynamic";

const BACKFILL_ROW_THRESHOLD = 5;
const BACKFILL_DAYS = 35;

export async function POST(req: Request) {
  const url = new URL(req.url);
  const rawSymbol = (url.searchParams.get("symbol") ?? "").trim().toUpperCase();
  if (!rawSymbol) {
    return Response.json({ error: "symbol is required" }, { status: 400 });
  }
  const span = normalizeSpan(url.searchParams.get("span"));
  const today = jstToday();

  // Opt-in backfill: only fires when we have little recent history.
  const since = addDays(today, -BACKFILL_DAYS);
  if (countHistorySince(rawSymbol, since) < BACKFILL_ROW_THRESHOLD) {
    try {
      await backfillPriceHistory(rawSymbol, since);
      try {
        recordApiRequestForced();
      } catch {
        /* stats best-effort */
      }
    } catch {
      /* backfill best-effort; we still need a price below */
    }
  }

  const issuePrice = getPriceOnOrBefore(rawSymbol, today);
  if (issuePrice === null) {
    return Response.json(
      {
        error: `銘柄 ${rawSymbol} の価格データがまだありません。管理画面から取引を追加するか、しばらく待ってから再試行してください。`,
      },
      { status: 400 },
    );
  }

  const meta = getSymbolNames().find((s) => s.symbol === rawSymbol);
  const currency = meta?.currency ?? (rawSymbol.endsWith(".T") ? "JPY" : "USD");
  let displayName = meta?.name ?? null;
  if (!displayName) {
    try {
      displayName = await fetchSymbolName(rawSymbol);
    } catch {
      /* name is best-effort */
    }
  }
  upsertSymbol(rawSymbol, displayName, currency);

  const fxRateAtIssue = currency === "USD" ? getLatestUsdJpy() : null;

  // Owner's position in this symbol, if any.
  const holding = calculateHoldings(
    listTransactions().filter((t) => t.symbol === rawSymbol),
  )[0];
  let quantity: number | null = null;
  let avgCostNative: number | null = null;
  let issueValueJpy: number | null = null;
  let issuePnlJpy: number | null = null;
  if (holding && holding.quantity > 0) {
    const fx = holding.currency === "USD" ? (fxRateAtIssue ?? 150) : 1;
    const value = holding.quantity * issuePrice * fx;
    quantity = holding.quantity;
    avgCostNative = holding.averageCostNative;
    issueValueJpy = value;
    issuePnlJpy = value - holding.totalCostJpy;
  }

  let id = "";
  let lastErr: unknown;
  for (let attempt = 0; attempt < 3; attempt++) {
    const candidate = generateId();
    try {
      insertTickerShareCard({
        id: candidate,
        symbol: rawSymbol,
        displayName,
        currency,
        issuePriceNative: issuePrice,
        fxRateAtIssue,
        quantity,
        avgCostNative,
        issueValueJpy,
        issuePnlJpy,
        defaultSpan: span,
      });
      id = candidate;
      lastErr = undefined;
      break;
    } catch (e) {
      lastErr = e;
    }
  }
  if (lastErr) {
    return Response.json({ error: "failed to create ticker card" }, { status: 500 });
  }

  return Response.json({ id, url: `/ticker/${id}` });
}
