import { computeDashboard } from "@/lib/dashboard";
import { computeComposition } from "@/lib/composition";
import { computeTimeseries } from "@/lib/timeseries";
import Header from "@/components/Header";
import KpiTiles from "@/components/dashboard/KpiTiles";
import HoldingsTable from "@/components/dashboard/HoldingsTable";
import CompositionChart from "@/components/charts/CompositionChart";
import TimeseriesChart from "@/components/charts/TimeseriesChart";
import ShareControls from "@/components/ShareControls";

// Always render fresh — reads the live SQLite DB.
export const dynamic = "force-dynamic";

export default async function Home() {
  const data = computeDashboard();
  const composition = computeComposition();
  const timeseries = computeTimeseries();

  return (
    <main className="mx-auto max-w-6xl p-6">
      <Header />

      <KpiTiles data={data} />

      <div className="mt-6">
        <ShareControls />
      </div>

      <section className="mt-6 grid gap-6 lg:grid-cols-2">
        <div className="border border-da-gray-200 p-4">
          <h2 className="mb-2 text-sm font-bold text-da-gray-600">資産推移</h2>
          <div className="h-72">
            <TimeseriesChart data={timeseries} />
          </div>
        </div>
        <div className="border border-da-gray-200 p-4">
          <h2 className="mb-2 text-sm font-bold text-da-gray-600">構成</h2>
          <div className="h-72">
            <CompositionChart data={composition} />
          </div>
        </div>
      </section>

      <section className="mt-6 border border-da-gray-200 p-4">
        <h2 className="mb-3 text-sm font-bold text-da-gray-600">保有銘柄</h2>
        <HoldingsTable holdings={data.holdings} />
      </section>
    </main>
  );
}
