import { computeStatus } from "@/lib/status";
import Header from "@/components/Header";

export const dynamic = "force-dynamic";

function Row({ label, value }: { label: string; value: string | number | boolean }) {
  return (
    <div className="flex justify-between border-b border-da-gray-200/60 py-2 text-sm">
      <span className="text-da-gray-600">{label}</span>
      <span className="font-bold tabular-nums text-da-gray-800">{String(value)}</span>
    </div>
  );
}

export default async function StatusPage() {
  const s = computeStatus();
  return (
    <main className="mx-auto max-w-3xl p-6">
      <Header />
      <h2 className="mb-4 text-2xl font-black text-da-blue-1200">ステータス</h2>

      <section className="mb-6 border border-da-gray-200 p-4">
        <h3 className="mb-2 text-xs font-bold uppercase tracking-widest text-da-gray-600">
          外部API（{s.today}）
        </h3>
        <Row label="ステータス" value={s.api_stats.status_message} />
        <Row label="本日のリクエスト" value={`${s.api_stats.request_count} / ${s.api_stats.max_requests}`} />
        <Row label="残り回数" value={s.api_stats.remaining_requests} />
        <Row label="リクエスト可能" value={s.api_stats.can_request ? "はい" : "いいえ"} />
        <Row label="次回まで（分）" value={s.api_stats.minutes_until_next} />
        <Row label="最終リクエスト" value={s.api_stats.last_request_time} />
      </section>

      <section className="border border-da-gray-200 p-4">
        <h3 className="mb-2 text-xs font-bold uppercase tracking-widest text-da-gray-600">
          スケジューラ
        </h3>
        <Row label="cron 式" value={s.scheduler.cron_expression} />
        <Row label="次回実行" value={s.scheduler.next_execution_time} />
        <Row label="次回まで（分）" value={s.scheduler.minutes_until_next_execution} />
      </section>
    </main>
  );
}
