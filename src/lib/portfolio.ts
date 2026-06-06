//! Portfolio cost-basis + P&L math. Faithful port of OLD/src/services/portfolio.rs
//! (weighted-average cost method). Keep numerically identical to the Rust version.

export interface Transaction {
  id: number;
  symbol: string;
  txnType: string; // "BUY" | "SELL"
  quantity: number;
  price: number;
  currency: string;
  fee: number;
  txnDate: string; // YYYY-MM-DD
  fxRateToJpy: number | null;
  notes: string | null;
  createdAt: string | null;
}

export interface Holding {
  symbol: string;
  name: string | null;
  currency: string;
  quantity: number;
  averageCostNative: number;
  totalCostJpy: number;
}

/** Transactions are stored newest-first; process oldest-first for cost basis. */
function sortAsc(transactions: Transaction[]): Transaction[] {
  return [...transactions].sort((a, b) =>
    a.txnDate < b.txnDate ? -1 : a.txnDate > b.txnDate ? 1 : a.id - b.id,
  );
}

export function calculateHoldings(transactions: Transaction[]): Holding[] {
  const map = new Map<string, Holding>();

  for (const tx of sortAsc(transactions)) {
    let entry = map.get(tx.symbol);
    if (!entry) {
      entry = {
        symbol: tx.symbol,
        name: null,
        currency: tx.currency,
        quantity: 0,
        averageCostNative: 0,
        totalCostJpy: 0,
      };
      map.set(tx.symbol, entry);
    }

    const fxRate = tx.fxRateToJpy ?? 1.0;
    // Fee increases cost basis on BUY (handled below); decreases proceeds on SELL.
    const nativeCost = tx.price * tx.quantity + tx.fee;
    const jpyCost = nativeCost * fxRate;

    if (tx.txnType === "BUY") {
      const newQty = entry.quantity + tx.quantity;
      if (newQty > 0) {
        const currentNativeValue = entry.averageCostNative * entry.quantity;
        entry.averageCostNative = (currentNativeValue + nativeCost) / newQty;
        entry.totalCostJpy += jpyCost;
      }
      entry.quantity = newQty;
    } else if (tx.txnType === "SELL") {
      const newQty = entry.quantity - tx.quantity;
      if (newQty <= 0) {
        entry.quantity = 0;
        entry.averageCostNative = 0;
        entry.totalCostJpy = 0;
      } else {
        const proportion = newQty / entry.quantity;
        entry.totalCostJpy *= proportion;
        entry.quantity = newQty;
      }
    }
  }

  return [...map.values()]
    .filter((h) => h.quantity > 0)
    // Stable, deterministic baseline order; callers may re-sort for display.
    .sort((a, b) => (a.symbol < b.symbol ? -1 : a.symbol > b.symbol ? 1 : 0));
}

/**
 * Realized P&L (JPY) across all SELL transactions. Tracks cost_jpy with the
 * same weighted-average logic as calculateHoldings and, on each sell,
 * accumulates (allocated proceeds − allocated cost).
 */
export function calculateRealizedPnl(transactions: Transaction[]): number {
  const qtyMap = new Map<string, number>();
  const costMap = new Map<string, number>();
  let realized = 0;

  for (const tx of sortAsc(transactions)) {
    const qty = qtyMap.get(tx.symbol) ?? 0;
    const costJpy = costMap.get(tx.symbol) ?? 0;
    const fxRate = tx.fxRateToJpy ?? 1.0;

    if (tx.txnType === "BUY") {
      const nativeCost = tx.price * tx.quantity + tx.fee;
      costMap.set(tx.symbol, costJpy + nativeCost * fxRate);
      qtyMap.set(tx.symbol, qty + tx.quantity);
    } else if (tx.txnType === "SELL" && qty > 0) {
      const sellQty = Math.min(tx.quantity, qty);
      const costAllocated = costJpy * (sellQty / qty);
      const proratedFee = tx.quantity > 0 ? tx.fee * (sellQty / tx.quantity) : 0;
      const proceedsJpy = (tx.price * sellQty - proratedFee) * fxRate;
      realized += proceedsJpy - costAllocated;

      const newQty = Math.max(qty - sellQty, 0);
      if (newQty <= 0) {
        qtyMap.set(tx.symbol, 0);
        costMap.set(tx.symbol, 0);
      } else {
        costMap.set(tx.symbol, costJpy - costAllocated);
        qtyMap.set(tx.symbol, newQty);
      }
    }
  }

  return realized;
}

export function calculateHoldingsAsOf(
  transactions: Transaction[],
  asOf: string,
): Holding[] {
  return calculateHoldings(transactions.filter((t) => t.txnDate <= asOf));
}
