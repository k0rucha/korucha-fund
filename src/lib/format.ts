//! Shared formatting + time helpers. Ports OLD/src/util.rs.

/** Format an integer-like number with thousands separators (keeps leading `-`). */
export function formatWithCommas(n: number): string {
  const s = Math.round(n).toString();
  const neg = s.startsWith("-");
  const digits = (neg ? s.slice(1) : s).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return neg ? `-${digits}` : digits;
}

/** `+12,345` / `-12,345` style. */
export function signedWithCommas(n: number): string {
  return (n >= 0 ? "+" : "-") + formatWithCommas(Math.abs(n));
}

/** `+12.34%` / `-12.34%` style. */
export function signedPct(n: number): string {
  return (n >= 0 ? "+" : "-") + Math.abs(n).toFixed(2) + "%";
}

/** `¥12,345` (sign-less; pass an absolute or naturally-positive value). */
export function yen(n: number): string {
  return "¥" + formatWithCommas(n);
}

/** `+¥12,345` / `-¥12,345` — sign rendered before the ¥ symbol. */
export function signedYen(n: number): string {
  return (n >= 0 ? "+" : "-") + "¥" + formatWithCommas(Math.abs(n));
}

/** Now, shifted so UTC getters read JST wall-clock (spec §11: all dates JST). */
export function jstNow(): Date {
  return new Date(Date.now() + 9 * 3600 * 1000);
}

/** Today's date in JST as `YYYY-MM-DD`. */
export function jstToday(): string {
  return jstNow().toISOString().slice(0, 10);
}

/** Add (or subtract) days to a `YYYY-MM-DD` string, returning `YYYY-MM-DD`. */
export function addDays(date: string, days: number): string {
  const d = new Date(date + "T00:00:00Z");
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

/** Whole days between two `YYYY-MM-DD` strings (b − a), clamped at 0. */
export function daysBetween(a: string, b: string): number {
  const ms = new Date(b + "T00:00:00Z").getTime() - new Date(a + "T00:00:00Z").getTime();
  return Math.max(0, Math.round(ms / 86400000));
}

/** SQLite CURRENT_TIMESTAMP (UTC `YYYY-MM-DD HH:MM:SS`) → JST date + display. */
export function jstFromUtcTimestamp(ts: string): { date: string; display: string } {
  const utc = new Date((ts || "").replace(" ", "T") + "Z");
  const jst = new Date(utc.getTime() + 9 * 3600 * 1000);
  if (Number.isNaN(jst.getTime())) return { date: "", display: ts };
  return {
    date: jst.toISOString().slice(0, 10),
    display: jst.toISOString().slice(0, 16).replace("T", " "),
  };
}
