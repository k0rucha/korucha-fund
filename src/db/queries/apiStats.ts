//! External-API (yfinance) rate-limit accounting. Ports OLD/src/db/api_stats.rs.
//! Uses the raw better-sqlite3 handle so the atomic check-and-increment UPDATE
//! (julianday interval guard) is a verbatim translation of the original SQL.
import { sqliteClient as sdb } from "@/db";
import { jstNow, jstToday } from "@/lib/format";

const MIN_INTERVAL_MINUTES = 96; // 1440 / 15 requests per day
const MAX_REQUESTS_PER_DAY = 15;

/** `YYYY-MM-DD HH:MM:SS` in JST, matching the stored format. */
function fmtJstTs(d: Date): string {
  return d.toISOString().slice(0, 19).replace("T", " ");
}

function ensureStatsForToday(): void {
  sdb
    .prepare(
      `INSERT OR IGNORE INTO api_request_stats (reset_date, last_request_time, request_count) VALUES (?, NULL, 0)`,
    )
    .run(jstToday());
}

export interface ApiRequestStats {
  lastRequestTime: string | null;
  requestCount: number;
  resetDate: string;
}

export function getStats(): ApiRequestStats {
  ensureStatsForToday();
  const today = jstToday();
  const row = sdb
    .prepare(
      `SELECT last_request_time AS lastRequestTime, request_count AS requestCount, reset_date AS resetDate
       FROM api_request_stats WHERE reset_date = ? LIMIT 1`,
    )
    .get(today) as ApiRequestStats | undefined;
  return row ?? { lastRequestTime: null, requestCount: 0, resetDate: today };
}

/** Informational check (use tryRecordApiRequest to actually claim a slot). */
export function canRequestApi(): {
  canRequest: boolean;
  minutesUntilNext: number;
} {
  const stats = getStats();
  if (stats.requestCount >= MAX_REQUESTS_PER_DAY) {
    return { canRequest: false, minutesUntilNext: 0 };
  }
  if (!stats.lastRequestTime) {
    return { canRequest: true, minutesUntilNext: 0 };
  }
  // Stored timestamp is JST wall-clock; parse as UTC so the delta vs jstNow()
  // (also UTC-encoded JST) is the real elapsed time.
  const last = new Date(stats.lastRequestTime.replace(" ", "T") + "Z").getTime();
  const elapsedMin = Math.floor((jstNow().getTime() - last) / 60000);
  const remaining = MIN_INTERVAL_MINUTES - elapsedMin;
  return remaining <= 0
    ? { canRequest: true, minutesUntilNext: 0 }
    : { canRequest: false, minutesUntilNext: remaining };
}

/**
 * Atomically claim a request slot. The WHERE clause enforces both the daily
 * cap and the minimum interval, so concurrent callers can't both succeed.
 */
export function tryRecordApiRequest(): boolean {
  ensureStatsForToday();
  const today = jstToday();
  const nowStr = fmtJstTs(jstNow());
  const res = sdb
    .prepare(
      `UPDATE api_request_stats
         SET last_request_time = ?, request_count = request_count + 1
       WHERE reset_date = ?
         AND request_count < ?
         AND (last_request_time IS NULL
              OR (julianday(?) - julianday(last_request_time)) * 1440 >= ?)`,
    )
    .run(nowStr, today, MAX_REQUESTS_PER_DAY, nowStr, MIN_INTERVAL_MINUTES);
  return res.changes > 0;
}

/** Force-record (daily scheduler / owner-initiated) — bypasses the limiter. */
export function recordApiRequestForced(): void {
  ensureStatsForToday();
  const today = jstToday();
  const nowStr = fmtJstTs(jstNow());
  sdb
    .prepare(
      `UPDATE api_request_stats
         SET last_request_time = ?, request_count = request_count + 1
       WHERE reset_date = ?`,
    )
    .run(nowStr, today);
}

export function requestsRemaining(): number {
  return Math.max(MAX_REQUESTS_PER_DAY - getStats().requestCount, 0);
}

export { MAX_REQUESTS_PER_DAY, MIN_INTERVAL_MINUTES };
