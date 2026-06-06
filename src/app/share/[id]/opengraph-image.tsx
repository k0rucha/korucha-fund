import { ImageResponse } from "next/og";
import { getShareCard } from "@/db/queries/shareCards";
import { listSnapshots, getSnapshotOnOrBefore } from "@/db/queries/snapshots";
import { jstFromUtcTimestamp, addDays } from "@/lib/format";
import { PortfolioCard, ogpFonts, ogpSize } from "@/lib/ogp";
import type { CardHolding } from "@/lib/share";

export const size = ogpSize;
export const contentType = "image/png";
export const runtime = "nodejs"; // needs better-sqlite3 + fs (font files)

export default async function Image({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const card = getShareCard(id);
  if (!card) {
    return new ImageResponse(
      (
        <div
          style={{
            width: 1200,
            height: 630,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "#000060",
            color: "#ffffff",
            fontSize: 48,
            fontFamily: "Noto",
          }}
        >
          カードが見つかりません
        </div>
      ),
      { ...ogpSize, fonts: ogpFonts },
    );
  }

  const { date: issueDate } = jstFromUtcTimestamp(card.createdAt ?? "");
  const since = addDays(issueDate, -30);
  const chartValues = listSnapshots()
    .filter((s) => s.date >= since && s.date <= issueDate)
    .map((s) => s.totalValueJpy);
  const prevDayValue = getSnapshotOnOrBefore(addDays(issueDate, -1))?.totalValueJpy ?? null;
  const prevMonthValue = getSnapshotOnOrBefore(addDays(issueDate, -30))?.totalValueJpy ?? null;
  const holdings: CardHolding[] = JSON.parse(card.holdingsJson);
  const symbols = [...holdings]
    .sort((a, b) => b.currentValueJpy - a.currentValueJpy)
    .map((h) => h.symbol);

  return new ImageResponse(
    (
      <PortfolioCard
        totalValueJpy={card.totalValueJpy}
        totalCostJpy={card.totalCostJpy}
        unrealizedPnlJpy={card.unrealizedPnlJpy}
        createdAt={issueDate}
        symbols={symbols}
        chartValues={chartValues}
        prevDayValue={prevDayValue}
        prevMonthValue={prevMonthValue}
      />
    ),
    { ...ogpSize, fonts: ogpFonts },
  );
}
