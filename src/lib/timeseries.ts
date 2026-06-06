//! Time-series snapshot data (for the history chart). Ports OLD/src/handlers/fragments.rs.
import { listSnapshots } from "@/db/queries/snapshots";

export interface TimeseriesData {
  dates: string[];
  values: number[];
  costs: number[];
  pnls: number[];
}

export function computeTimeseries(): TimeseriesData {
  const snaps = listSnapshots();
  return {
    dates: snaps.map((s) => s.date),
    values: snaps.map((s) => s.totalValueJpy),
    costs: snaps.map((s) => s.totalCostJpy),
    pnls: snaps.map((s) => s.unrealizedPnlJpy),
  };
}
