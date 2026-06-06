import { ImageResponse } from "next/og";
import { getTickerShareCard } from "@/db/queries/tickerShareCards";
import { getPriceOnOrBefore, listHistory } from "@/db/queries/prices";
import { jstFromUtcTimestamp, addDays } from "@/lib/format";
import { TickerCard, ogpFonts, ogpSize } from "@/lib/ogp";

export const size = ogpSize;
export const contentType = "image/png";
export const runtime = "nodejs"; // needs better-sqlite3 + fs (font files)

export default async function Image({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const card = getTickerShareCard(id);
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
  const chartValues = listHistory(card.symbol, issueDate)
    .filter((h) => h.date >= since)
    .map((h) => h.close);
  const prevDayPrice = getPriceOnOrBefore(card.symbol, addDays(issueDate, -1));
  const prevMonthPrice = getPriceOnOrBefore(card.symbol, addDays(issueDate, -30));

  return new ImageResponse(
    (
      <TickerCard
        symbol={card.symbol}
        displayName={card.displayName ?? ""}
        currency={card.currency}
        issuePriceNative={card.issuePriceNative}
        fxRateAtIssue={card.fxRateAtIssue}
        quantity={card.quantity}
        avgCostNative={card.avgCostNative}
        issuePnlJpy={card.issuePnlJpy}
        createdAt={issueDate}
        chartValues={chartValues}
        prevDayPrice={prevDayPrice}
        prevMonthPrice={prevMonthPrice}
      />
    ),
    { ...ogpSize, fonts: ogpFonts },
  );
}
