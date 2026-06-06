import Link from "next/link";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTickerShareCard } from "@/db/queries/tickerShareCards";
import { getPriceOnOrBefore, listHistory } from "@/db/queries/prices";
import {
  formatWithCommas,
  signedYen,
  signedPct,
  jstFromUtcTimestamp,
  jstToday,
  daysBetween,
} from "@/lib/format";
import TickerChart from "@/components/charts/TickerChart";
import CopyUrlButton from "@/components/CopyUrlButton";

export const dynamic = "force-dynamic";

const pnlCls = (n: number) =>
  n >= 0 ? "text-[rgb(var(--da-pos))]" : "text-[rgb(var(--da-neg))]";

/** `+$1,234` / `-¥1,234` (native unit, integer-rounded like the old app). */
function signedNative(n: number, unit: string): string {
  return (n >= 0 ? "+" : "-") + unit + formatWithCommas(Math.abs(n));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ id: string }>;
}): Promise<Metadata> {
  const { id } = await params;
  const card = getTickerShareCard(id);
  if (!card) return { title: "カードが見つかりません — こるちゃファンド" };

  const { date } = jstFromUtcTimestamp(card.createdAt ?? "");
  const unit = card.currency === "USD" ? "$" : "¥";
  const dn = card.displayName ? `${card.displayName} (${card.symbol})` : card.symbol;
  const title = `${dn} 戦績カード — 価格 ${unit}${formatWithCommas(card.issuePriceNative)}`;

  const hasPos = (card.quantity ?? 0) > 0;
  const posSuffix = hasPos
    ? ` ・ 保有数 ${formatWithCommas(card.quantity ?? 0)} ・ 含み損益 ${signedYen(card.issuePnlJpy ?? 0)}`
    : "";
  const description = `発行日 ${date} ・ ${card.symbol} の発行時価格は ${unit}${formatWithCommas(card.issuePriceNative)}${posSuffix}`;

  return {
    title,
    description,
    openGraph: {
      type: "website",
      title,
      description,
      siteName: "こるちゃファンド",
      url: `/ticker/${id}`,
    },
    twitter: { card: "summary_large_image", title, description },
  };
}

export default async function TickerPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const card = getTickerShareCard(id);
  if (!card) notFound();

  const { date: issueDate, display: createdDisplay } = jstFromUtcTimestamp(
    card.createdAt ?? "",
  );
  const today = jstToday();
  const daysSince = daysBetween(issueDate, today);
  const unit = card.currency === "USD" ? "$" : "¥";

  const currentPrice = getPriceOnOrBefore(card.symbol, today) ?? card.issuePriceNative;
  const priceDelta = currentPrice - card.issuePriceNative;
  const priceDeltaPct =
    card.issuePriceNative !== 0 ? (priceDelta / card.issuePriceNative) * 100 : 0;

  const hasPosition = (card.quantity ?? 0) > 0;
  const avg = card.avgCostNative ?? 0;
  const issuePnlPct =
    hasPosition && avg > 0 ? ((card.issuePriceNative - avg) / avg) * 100 : 0;
  const issuePriceJpy =
    card.currency === "USD"
      ? card.issuePriceNative * (card.fxRateAtIssue ?? 150)
      : card.issuePriceNative;

  const history = listHistory(card.symbol, issueDate);
  const displayName = card.displayName ?? "";

  return (
    <main className="mx-auto max-w-3xl p-6">
      <div className="mb-8 flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="mb-1 text-xs font-bold uppercase tracking-widest text-da-gray-400">
            TICKER CARD
          </div>
          <h1 className="text-2xl font-black text-da-blue-1200">
            {displayName || card.symbol}
          </h1>
          <p className="mt-1 text-sm text-da-gray-600">
            {card.symbol} ・ 発行日時 {createdDisplay}
          </p>
        </div>
        <Link href="/" className="text-sm font-bold text-da-blue-900 hover:text-da-blue-600">
          ファンドの最新状況へ →
        </Link>
      </div>

      <div className="mb-8 border border-da-gray-200 p-8">
        <div className="mb-8 flex items-center gap-4">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src="/logo.webp" alt="" className="h-10 w-10 object-contain" />
          <div className="text-xs text-da-gray-400">
            {card.symbol} の発行時スナップショット ・ {createdDisplay}
          </div>
        </div>

        <div className="mb-10">
          <div className="mb-3 text-xs font-bold uppercase tracking-widest text-da-gray-600">
            発行時の価格
          </div>
          <div className="mb-4 text-5xl font-black leading-none text-da-blue-1200 tabular-nums">
            {unit}
            {formatWithCommas(card.issuePriceNative)}
          </div>
          {card.currency === "USD" && (
            <div className="text-sm text-da-gray-600">
              JPY 換算:{" "}
              <span className="font-bold tabular-nums text-da-gray-800">
                ¥{formatWithCommas(issuePriceJpy)}
              </span>
            </div>
          )}
        </div>

        {hasPosition && (
          <div className="mb-10 border border-da-gray-200 bg-da-gray-50/40 p-5">
            <div className="mb-3 text-[10px] font-bold uppercase tracking-widest text-da-gray-400">
              発行時の自分の保有
            </div>
            <dl className="grid grid-cols-2 gap-4 text-sm md:grid-cols-4">
              <div>
                <dt className="mb-1 text-da-gray-600">保有数</dt>
                <dd className="font-bold tabular-nums text-da-gray-800">
                  {(card.quantity ?? 0).toFixed(2)}
                </dd>
              </div>
              <div>
                <dt className="mb-1 text-da-gray-600">取得単価</dt>
                <dd className="font-bold tabular-nums text-da-gray-800">
                  {unit}
                  {avg.toFixed(2)}
                </dd>
              </div>
              <div>
                <dt className="mb-1 text-da-gray-600">評価額</dt>
                <dd className="font-bold tabular-nums text-da-gray-800">
                  ¥{formatWithCommas(card.issueValueJpy ?? 0)}
                </dd>
              </div>
              <div>
                <dt className="mb-1 text-da-gray-600">含み損益</dt>
                <dd className={`font-bold tabular-nums ${pnlCls(card.issuePnlJpy ?? 0)}`}>
                  {signedYen(card.issuePnlJpy ?? 0)}{" "}
                  <span className="ml-1 text-xs">({signedPct(issuePnlPct)})</span>
                </dd>
              </div>
            </dl>
          </div>
        )}

        <div className="mb-10">
          <TickerChart
            dates={history.map((h) => h.date)}
            prices={history.map((h) => h.close)}
            defaultSpan={card.defaultSpan}
            currency={card.currency}
          />
        </div>

        <div className="border-t border-da-gray-200 pt-5">
          <div className="mb-2 text-[10px] font-bold uppercase tracking-widest text-da-gray-400">
            この発行から
          </div>
          {daysSince === 0 ? (
            <p className="text-sm text-da-gray-600">
              本日発行 — 現在価格は{" "}
              <span className="font-bold tabular-nums text-da-gray-800">
                {unit}
                {formatWithCommas(currentPrice)}
              </span>
            </p>
          ) : (
            <p className="text-sm text-da-gray-600">
              {daysSince}日経過 — 現在{" "}
              <span className="font-bold tabular-nums text-da-gray-800">
                {unit}
                {formatWithCommas(currentPrice)}
              </span>
              <span className={`ml-2 font-bold tabular-nums ${pnlCls(priceDelta)}`}>
                ({signedNative(priceDelta, unit)} / {signedPct(priceDeltaPct)})
              </span>
            </p>
          )}
        </div>
      </div>

      <div className="text-center">
        <CopyUrlButton />
      </div>
    </main>
  );
}
