// Parity check for the portfolio math port. Mirrors the Rust unit test in
// OLD/src/services/portfolio.rs plus a realized-P&L scenario.
// Run: node scripts/parity-portfolio.ts   (Node 24 strips types automatically)
import {
  calculateHoldings,
  calculateRealizedPnl,
  type Transaction,
} from "../src/lib/portfolio.ts";

let pass = 0;
let fail = 0;
const approx = (a: number, b: number, eps = 1e-6) => Math.abs(a - b) <= eps;
function check(name: string, cond: boolean) {
  if (cond) {
    pass++;
    console.log("  ok   " + name);
  } else {
    fail++;
    console.log("  FAIL " + name);
  }
}

function tx(
  id: number,
  symbol: string,
  type: string,
  qty: number,
  price: number,
  cur: string,
  fee: number,
  fx: number | null,
): Transaction {
  return {
    id,
    symbol,
    txnType: type,
    quantity: qty,
    price,
    currency: cur,
    fee,
    txnDate: "2026-01-0" + id,
    fxRateToJpy: fx,
    notes: null,
    createdAt: null,
  };
}

// --- calculateHoldings (matches Rust test) ---
const txs = [
  tx(1, "AAPL", "BUY", 10, 150, "USD", 0, 140),
  tx(2, "AAPL", "BUY", 10, 160, "USD", 0, 150),
  tx(3, "7203.T", "BUY", 100, 2000, "JPY", 100, null),
];
const h = calculateHoldings(txs);
check("2 holdings", h.length === 2);
const aapl = h.find((x) => x.symbol === "AAPL")!;
check("AAPL qty 20", aapl.quantity === 20);
check("AAPL avg 155", approx(aapl.averageCostNative, 155));
check("AAPL costJpy 450000", approx(aapl.totalCostJpy, 10 * 150 * 140 + 10 * 160 * 150));
const toyota = h.find((x) => x.symbol === "7203.T")!;
check("Toyota qty 100", toyota.quantity === 100);
check("Toyota avg 2001", approx(toyota.averageCostNative, 2001));
check("Toyota costJpy 200100", approx(toyota.totalCostJpy, 200100));

// --- calculateRealizedPnl ---
// Buy 10@100 JPY, sell 5@120 JPY → cost allocated 500, proceeds 600, realized 100.
const r = calculateRealizedPnl([
  tx(1, "X", "BUY", 10, 100, "JPY", 0, null),
  tx(2, "X", "SELL", 5, 120, "JPY", 0, null),
]);
check("realized 100", approx(r, 100));

console.log(`\n${pass} passed, ${fail} failed`);
if (fail > 0) process.exit(1);
