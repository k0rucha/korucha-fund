import { asc, desc, lte } from "drizzle-orm";
import { db, schema } from "@/db";

const { snapshots } = schema;

export type Snapshot = typeof snapshots.$inferSelect;

export function listSnapshots(): Snapshot[] {
  return db.select().from(snapshots).orderBy(asc(snapshots.date)).all();
}

export function getSnapshotOnOrBefore(date: string): Snapshot | null {
  return (
    db
      .select()
      .from(snapshots)
      .where(lte(snapshots.date, date))
      .orderBy(desc(snapshots.date))
      .limit(1)
      .get() ?? null
  );
}

export function upsertSnapshot(
  date: string,
  totalValueJpy: number,
  totalCostJpy: number,
  unrealizedPnlJpy: number,
): void {
  db
    .insert(snapshots)
    .values({ date, totalValueJpy, totalCostJpy, unrealizedPnlJpy })
    .onConflictDoUpdate({
      target: snapshots.date,
      set: { totalValueJpy, totalCostJpy, unrealizedPnlJpy },
    })
    .run();
}
