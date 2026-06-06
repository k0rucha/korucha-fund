//! OGP card rendering for next/og (Satori). Reproduces the 1200×630 design of
//! OLD/src/handlers/ogp.rs: the chart is embedded as an SVG <img> (ported from
//! the Rust path generators); text + gradients use Satori-supported CSS.
import { readFileSync } from "fs";
import { join } from "path";
import type { ReactNode } from "react";
import { formatWithCommas, signedPct } from "@/lib/format";

const fontDir = join(process.cwd(), "assets/fonts");
export const ogpFonts = [
  { name: "Noto", data: readFileSync(join(fontDir, "NotoSansJP-Regular.ttf")), weight: 400 as const, style: "normal" as const },
  { name: "Noto", data: readFileSync(join(fontDir, "NotoSansJP-Bold.ttf")), weight: 700 as const, style: "normal" as const },
  { name: "Noto", data: readFileSync(join(fontDir, "NotoSansJP-Black.ttf")), weight: 900 as const, style: "normal" as const },
];
export const ogpSize = { width: 1200, height: 630 };

// ─── Chart SVG (ports chart_line_path / chart_area_path / chart_fragment) ────
function linePath(values: number[], w: number, h: number): string | null {
  if (values.length < 2) return null;
  const n = values.length;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = Math.max(max - min, 1);
  const pad = range * 0.08;
  const lo = min - pad;
  const span = range + 2 * pad;
  const toX = (i: number) => (i / (n - 1)) * w;
  const toY = (v: number) => h - ((v - lo) / span) * h;
  let p = "";
  values.forEach((v, i) => {
    p += `${i === 0 ? "M" : " L"} ${toX(i).toFixed(1)},${toY(v).toFixed(1)}`;
  });
  return p;
}

function chartSvg(values: number[], w = 1040, h = 408): string {
  if (values.length < 2) return "";
  const first = values[0];
  const last = values[values.length - 1];
  const color = last > first ? "#34D399" : last < first ? "#F87171" : "#7799FF";
  const line = linePath(values, w, h) ?? "";
  const area = `${line} L ${w.toFixed(1)},${h.toFixed(1)} L 0.0,${h.toFixed(1)} Z`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="${color}" stop-opacity="0.28"/><stop offset="100%" stop-color="${color}" stop-opacity="0"/></linearGradient></defs><path d="${area}" fill="url(#g)"/><path d="${line}" fill="none" stroke="${color}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/></svg>`;
}

function chartDataUri(values: number[]): string | null {
  const svg = chartSvg(values);
  return svg ? `data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}` : null;
}

// ─── Delta helpers (port delta_color / delta_str) ───────────────────────────
function deltaColor(prev: number | null, current: number): string {
  if (prev !== null && prev > 0) {
    return current > prev ? "#34D399" : current < prev ? "#F87171" : "#AABBCC";
  }
  return "#AABBCC";
}

function deltaStr(current: number, prev: number | null, unit: string, integer: boolean): string {
  if (prev === null || prev <= 0) return "—";
  const d = current - prev;
  const pct = (d / prev) * 100;
  const sign = d >= 0 ? "+" : "-";
  const abs = integer ? formatWithCommas(Math.abs(d)) : Math.abs(d).toFixed(2);
  return `${sign}${unit}${abs} (${signedPct(pct)})`;
}

const FONT = "Noto";
const rule = { height: 1, backgroundColor: "rgba(255,255,255,0.15)", display: "flex" } as const;

function Frame({ chartValues, children }: { chartValues: number[]; children: ReactNode }) {
  const chart = chartDataUri(chartValues);
  return (
    <div
      style={{
        width: 1200,
        height: 630,
        display: "flex",
        position: "relative",
        backgroundColor: "#000060",
        fontFamily: FONT,
        color: "#ffffff",
      }}
    >
      <div style={{ position: "absolute", top: 0, left: 0, width: 1200, height: 6, backgroundColor: "#FB5801", display: "flex" }} />
      {chart && (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={chart} width={1040} height={408} alt="" style={{ position: "absolute", top: 98, left: 80 }} />
      )}
      <div style={{ position: "absolute", top: 0, left: 0, width: 1200, height: 385, display: "flex", backgroundImage: "linear-gradient(#000060, rgba(0,0,96,0))" }} />
      <div style={{ position: "absolute", top: 480, left: 0, width: 1200, height: 150, display: "flex", backgroundImage: "linear-gradient(rgba(0,0,96,0), #000060)" }} />
      <div style={{ position: "absolute", top: 0, left: 0, width: 1200, height: 630, display: "flex", flexDirection: "column", padding: "56px 80px" }}>
        {children}
      </div>
    </div>
  );
}

function Header({ title, date }: { title: string; date: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end" }}>
      <div style={{ fontSize: 32, fontWeight: 700, opacity: 0.9, display: "flex" }}>{title}</div>
      <div style={{ fontSize: 24, color: "#8899CC", display: "flex" }}>{date}</div>
    </div>
  );
}

export interface PortfolioOgpProps {
  totalValueJpy: number;
  totalCostJpy: number;
  unrealizedPnlJpy: number;
  createdAt: string;
  symbols: string[];
  chartValues: number[];
  prevDayValue: number | null;
  prevMonthValue: number | null;
}

export function PortfolioCard(p: PortfolioOgpProps) {
  const pnlPct = p.totalCostJpy > 0 ? (p.unrealizedPnlJpy / p.totalCostJpy) * 100 : 0;
  const pnlColor = p.unrealizedPnlJpy >= 0 ? "#34D399" : "#F87171";
  const sign = p.unrealizedPnlJpy >= 0 ? "+" : "-";
  const pnlText = `${sign}¥${formatWithCommas(Math.abs(p.unrealizedPnlJpy))}  (${signedPct(pnlPct)})`;
  return (
    <Frame chartValues={p.chartValues}>
      <Header title="こるちゃファンド" date={p.createdAt} />
      <div style={{ ...rule, marginTop: 25, marginBottom: 28 }} />
      <div style={{ fontSize: 21, color: "#8899CC", display: "flex" }}>発行時点の総評価額</div>
      <div style={{ fontSize: 88, fontWeight: 900, marginTop: 6, display: "flex" }}>{`¥${formatWithCommas(p.totalValueJpy)}`}</div>
      <div style={{ fontSize: 44, fontWeight: 700, color: pnlColor, marginTop: 6, display: "flex" }}>{pnlText}</div>
      <div style={{ fontSize: 25, color: "#7788BB", marginTop: 8, display: "flex" }}>{`投資元本 ¥${formatWithCommas(p.totalCostJpy)}`}</div>
      <div style={{ flex: 1, display: "flex" }} />
      <div style={{ ...rule, marginBottom: 18 }} />
      <div style={{ display: "flex", fontSize: 22, marginBottom: 14, alignItems: "center" }}>
        <div style={{ color: "#6677AA", display: "flex" }}>前日比</div>
        <div style={{ color: deltaColor(p.prevDayValue, p.totalValueJpy), fontWeight: 600, marginLeft: 14, display: "flex" }}>{deltaStr(p.totalValueJpy, p.prevDayValue, "¥", true)}</div>
        <div style={{ color: "#6677AA", marginLeft: 56, display: "flex" }}>前月比</div>
        <div style={{ color: deltaColor(p.prevMonthValue, p.totalValueJpy), fontWeight: 600, marginLeft: 14, display: "flex" }}>{deltaStr(p.totalValueJpy, p.prevMonthValue, "¥", true)}</div>
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 20, alignItems: "center" }}>
        <div style={{ opacity: 0.45, display: "flex" }}>{p.symbols.slice(0, 6).join("  ·  ")}</div>
        <div style={{ opacity: 0.3, fontSize: 18, display: "flex" }}>fund.korucha.com</div>
      </div>
    </Frame>
  );
}

export interface TickerOgpProps {
  symbol: string;
  displayName: string;
  currency: string;
  issuePriceNative: number;
  fxRateAtIssue: number | null;
  quantity: number | null;
  avgCostNative: number | null;
  issuePnlJpy: number | null;
  createdAt: string;
  chartValues: number[];
  prevDayPrice: number | null;
  prevMonthPrice: number | null;
}

export function TickerCard(p: TickerOgpProps) {
  const unit = p.currency === "USD" ? "$" : "¥";
  const integer = p.currency === "JPY";
  const name = p.displayName ? `${p.displayName} (${p.symbol})` : p.symbol;
  const hasPosition = (p.quantity ?? 0) > 0 && p.avgCostNative != null && p.issuePnlJpy != null;

  let posPnl: ReactNode = null;
  let posQty: ReactNode = null;
  if (hasPosition) {
    const avg = p.avgCostNative as number;
    const pnl = p.issuePnlJpy as number;
    const pnlPct = avg > 0 ? ((p.issuePriceNative - avg) / avg) * 100 : 0;
    const pnlColor = pnl >= 0 ? "#34D399" : "#F87171";
    const sign = pnl >= 0 ? "+" : "-";
    posPnl = (
      <div style={{ fontSize: 44, fontWeight: 700, color: pnlColor, marginTop: 6, display: "flex" }}>
        {`${sign}¥${formatWithCommas(Math.abs(pnl))}  (${signedPct(pnlPct)})`}
      </div>
    );
    posQty = (
      <div style={{ fontSize: 25, color: "#7788BB", marginTop: 8, display: "flex" }}>
        {`保有数 ${(p.quantity as number).toFixed(2)} 株`}
      </div>
    );
  }

  const jpyLine =
    p.currency === "USD" && p.fxRateAtIssue != null ? (
      <div style={{ fontSize: 30, color: "#8899CC", marginTop: 8, display: "flex" }}>
        {`≈ ¥${formatWithCommas(p.issuePriceNative * p.fxRateAtIssue)}`}
      </div>
    ) : null;

  return (
    <Frame chartValues={p.chartValues}>
      <Header title="こるちゃファンド" date={p.createdAt} />
      <div style={{ ...rule, marginTop: 25, marginBottom: 28 }} />
      <div style={{ fontSize: 28, color: "#8899CC", display: "flex" }}>{name}</div>
      <div style={{ fontSize: 88, fontWeight: 900, marginTop: 6, display: "flex" }}>{`${unit}${formatWithCommas(p.issuePriceNative)}`}</div>
      {jpyLine}
      {posPnl}
      {posQty}
      <div style={{ flex: 1, display: "flex" }} />
      <div style={{ ...rule, marginBottom: 18 }} />
      <div style={{ display: "flex", fontSize: 22, marginBottom: 14, alignItems: "center" }}>
        <div style={{ color: "#6677AA", display: "flex" }}>前日比</div>
        <div style={{ color: deltaColor(p.prevDayPrice, p.issuePriceNative), fontWeight: 600, marginLeft: 14, display: "flex" }}>{deltaStr(p.issuePriceNative, p.prevDayPrice, unit, integer)}</div>
        <div style={{ color: "#6677AA", marginLeft: 56, display: "flex" }}>前月比</div>
        <div style={{ color: deltaColor(p.prevMonthPrice, p.issuePriceNative), fontWeight: 600, marginLeft: 14, display: "flex" }}>{deltaStr(p.issuePriceNative, p.prevMonthPrice, unit, integer)}</div>
      </div>
      <div style={{ display: "flex", justifyContent: "flex-end", fontSize: 18 }}>
        <div style={{ opacity: 0.3, display: "flex" }}>fund.korucha.com</div>
      </div>
    </Frame>
  );
}
