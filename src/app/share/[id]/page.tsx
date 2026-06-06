import Link from "next/link";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getShareCard } from "@/db/queries/shareCards";
import { listSnapshots } from "@/db/queries/snapshots";
import { computeCurrentPortfolio, type CardHolding } from "@/lib/share";
import {
  formatWithCommas,
  signedYen,
  signedPct,
  jstFromUtcTimestamp,
  jstToday,
  daysBetween,
} from "@/lib/format";
import ShareChart from "@/components/charts/ShareChart";
import CopyUrlButton from "@/components/CopyUrlButton";

export const dynamic = "force-dynamic";

const pnlCls = (n: number) =>
  n >= 0 ? "text-[rgb(var(--da-pos))]" : "text-[rgb(var(--da-neg))]";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ id: string }>;
}): Promise<Metadata> {
  const { id } = await params;
  const card = getShareCard(id);
  if (!card) return { title: "カードが見つかりません — こるちゃファンド" };

  const { date } = jstFromUtcTimestamp(card.createdAt ?? "");
  const issuePnlPct =
    card.totalCostJpy > 0 ? (card.unrealizedPnlJpy / card.totalCostJpy) * 100 : 0;
  const title = `こるちゃファンド 戦績カード — 評価額 ¥${formatWithCommas(card.totalValueJpy)}`;
  const description = `発行日 ${date} ・ 含み損益 ${signedYen(card.unrealizedPnlJpy)} (${signedPct(issuePnlPct)}) ・ 投資元本 ¥${formatWithCommas(card.totalCostJpy)}`;

  return {
    title,
    description,
    openGraph: {
      type: "website",
      title,
      description,
      siteName: "こるちゃファンド",
      url: `/share/${id}`,
    },
    twitter: { card: "summary_large_image", title, description },
  };
}

export default async function SharePage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const card = getShareCard(id);
  if (!card) notFound();

  const issueHoldings: CardHolding[] = JSON.parse(card.holdingsJson);
  issueHoldings.sort((a, b) => b.currentValueJpy - a.currentValueJpy);

  const { totalValue: curValue } = computeCurrentPortfolio();
  const issuePnlPct =
    card.totalCostJpy > 0 ? (card.unrealizedPnlJpy / card.totalCostJpy) * 100 : 0;
  const valueDelta = curValue - card.totalValueJpy;
  const valueDeltaPct =
    card.totalValueJpy > 0 ? (valueDelta / card.totalValueJpy) * 100 : 0;

  const { date: issueDate, display: createdDisplay } = jstFromUtcTimestamp(
    card.createdAt ?? "",
  );
  const daysSince = daysBetween(issueDate, jstToday());

  const snaps = listSnapshots().filter((s) => s.date <= issueDate);

  return (
    <main className="mx-auto max-w-3xl p-6">
      <div className="mb-8 flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="mb-1 text-xs font-bold uppercase tracking-widest text-da-gray-400">
            SHARED CARD
          </div>
          <h1 className="text-2xl font-black text-da-blue-1200">
            こるちゃファンド 戦績カード
          </h1>
          <p className="mt-1 text-sm text-da-gray-600">発行日時 {createdDisplay}</p>
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
            発行時点のスナップショット ・ {createdDisplay}
          </div>
        </div>

        <div className="mb-10">
          <div className="mb-3 text-xs font-bold uppercase tracking-widest text-da-gray-600">
            発行時点の総評価額
          </div>
          <div className="mb-4 text-5xl font-black leading-none text-da-blue-1200 tabular-nums">
            ¥{formatWithCommas(card.totalValueJpy)}
          </div>
          <div className="flex flex-wrap gap-x-8 gap-y-3 text-sm">
            <div>
              <span className="text-da-gray-600">投資元本</span>
              <span className="ml-2 font-bold tabular-nums text-da-gray-800">
                ¥{formatWithCommas(card.totalCostJpy)}
              </span>
            </div>
            <div>
              <span className="text-da-gray-600">含み損益</span>
              <span className={`ml-2 font-bold tabular-nums ${pnlCls(card.unrealizedPnlJpy)}`}>
                {signedYen(card.unrealizedPnlJpy)}
              </span>
            </div>
            <div>
              <span className="text-da-gray-600">損益率</span>
              <span className={`ml-2 font-bold tabular-nums ${pnlCls(issuePnlPct)}`}>
                {signedPct(issuePnlPct)}
              </span>
            </div>
          </div>
        </div>

        <div className="mb-10">
          <ShareChart
            dates={snaps.map((s) => s.date)}
            values={snaps.map((s) => s.totalValueJpy)}
            pnls={snaps.map((s) => s.unrealizedPnlJpy)}
            defaultSpan={card.defaultSpan}
          />
        </div>

        {issueHoldings.length > 0 && (
          <div className="mb-10">
            <div className="mb-4 text-xs font-bold uppercase tracking-widest text-da-gray-600">
              発行時の保有銘柄
            </div>
            <div className="overflow-x-auto">
              <table className="min-w-full text-sm">
                <thead>
                  <tr className="border-b border-da-gray-200 text-xs text-da-gray-400">
                    <th className="px-3 py-2 text-left">銘柄</th>
                    <th className="px-3 py-2 text-right">評価額 (¥)</th>
                    <th className="px-3 py-2 text-right">含み損益</th>
                    <th className="px-3 py-2 text-right">損益率</th>
                  </tr>
                </thead>
                <tbody>
                  {issueHoldings.map((h) => (
                    <tr key={h.symbol} className="border-b border-da-gray-200/60">
                      <td className="px-3 py-3">
                        <div className="font-bold text-da-blue-1200">{h.symbol}</div>
                        {h.name && <div className="text-xs text-da-gray-600">{h.name}</div>}
                      </td>
                      <td className="px-3 py-3 text-right font-bold tabular-nums text-da-gray-800">
                        ¥{formatWithCommas(h.currentValueJpy)}
                      </td>
                      <td className={`px-3 py-3 text-right font-bold tabular-nums ${pnlCls(h.unrealizedPnlJpy)}`}>
                        {signedYen(h.unrealizedPnlJpy)}
                      </td>
                      <td className={`px-3 py-3 text-right font-bold tabular-nums ${pnlCls(h.unrealizedPnlPct)}`}>
                        {signedPct(h.unrealizedPnlPct)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        <div className="border-t border-da-gray-200 pt-5">
          <div className="mb-2 text-[10px] font-bold uppercase tracking-widest text-da-gray-400">
            この発行から
          </div>
          {daysSince === 0 ? (
            <p className="text-sm text-da-gray-600">
              本日発行 — 現在の評価額は{" "}
              <span className="font-bold tabular-nums text-da-gray-800">
                ¥{formatWithCommas(curValue)}
              </span>
            </p>
          ) : (
            <p className="text-sm text-da-gray-600">
              {daysSince}日経過 — 現在{" "}
              <span className="font-bold tabular-nums text-da-gray-800">
                ¥{formatWithCommas(curValue)}
              </span>
              <span className={`ml-2 font-bold tabular-nums ${pnlCls(valueDelta)}`}>
                ({signedYen(valueDelta)} / {signedPct(valueDeltaPct)})
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
