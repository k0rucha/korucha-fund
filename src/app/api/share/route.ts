import { computeCurrentPortfolio, generateId, normalizeSpan } from "@/lib/share";
import { insertShareCard } from "@/db/queries/shareCards";

export const dynamic = "force-dynamic";

export async function POST(req: Request) {
  const span = normalizeSpan(new URL(req.url).searchParams.get("span"));
  const { totalValue, totalCost, pnl, holdings } = computeCurrentPortfolio();
  const holdingsJson = JSON.stringify(holdings);

  let id = "";
  let lastErr: unknown;
  for (let attempt = 0; attempt < 3; attempt++) {
    const candidate = generateId();
    try {
      insertShareCard({
        id: candidate,
        totalValueJpy: totalValue,
        totalCostJpy: totalCost,
        unrealizedPnlJpy: pnl,
        holdingsJson,
        defaultSpan: span,
      });
      id = candidate;
      lastErr = undefined;
      break;
    } catch (e) {
      lastErr = e; // PK collision (astronomically unlikely) → retry
    }
  }
  if (lastErr) {
    return Response.json({ error: "failed to create share card" }, { status: 500 });
  }

  return Response.json({ id, url: `/share/${id}` });
}
