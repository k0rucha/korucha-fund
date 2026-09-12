from dataclasses import dataclass, field
from typing import Optional
import datetime


@dataclass
class Holding:
    symbol: str
    currency: str
    quantity: float
    total_cost_jpy: float


@dataclass
class HoldingCalcResult:
    holdings: list[Holding]
    total_cost_jpy: float


def _fx(txn_currency: str, fx_rate: Optional[float]) -> float:
    if txn_currency == "USD":
        return fx_rate if fx_rate else 1.0
    return 1.0


def calculate_holdings(transactions: list) -> HoldingCalcResult:
    """加重平均コスト法でホールディングを計算する。"""
    qty: dict[str, float] = {}
    cost: dict[str, float] = {}
    currency_map: dict[str, str] = {}

    for txn in sorted(transactions, key=lambda t: (t.txn_date, t.id)):
        sym = txn.symbol
        currency_map[sym] = txn.currency
        rate = _fx(txn.currency, txn.fx_rate_to_jpy)

        if txn.txn_type == "BUY":
            cost_jpy = (txn.price * txn.quantity + txn.fee) * rate
            qty[sym] = qty.get(sym, 0.0) + txn.quantity
            cost[sym] = cost.get(sym, 0.0) + cost_jpy
        elif txn.txn_type == "SELL":
            prev_qty = qty.get(sym, 0.0)
            if prev_qty > 0:
                ratio = txn.quantity / prev_qty
                cost[sym] = cost.get(sym, 0.0) * (1 - ratio)
            qty[sym] = max(0.0, qty.get(sym, 0.0) - txn.quantity)

    holdings = [
        Holding(
            symbol=sym,
            currency=currency_map[sym],
            quantity=q,
            total_cost_jpy=cost.get(sym, 0.0),
        )
        for sym, q in qty.items()
        if q > 1e-9
    ]
    return HoldingCalcResult(
        holdings=holdings,
        total_cost_jpy=sum(cost.get(h.symbol, 0.0) for h in holdings),
    )


def calculate_holdings_as_of(transactions: list, as_of: datetime.date) -> HoldingCalcResult:
    filtered = [t for t in transactions if t.txn_date <= as_of]
    return calculate_holdings(filtered)


def calculate_realized_pnl(transactions: list) -> float:
    qty: dict[str, float] = {}
    cost: dict[str, float] = {}
    realized = 0.0

    for txn in sorted(transactions, key=lambda t: (t.txn_date, t.id)):
        sym = txn.symbol
        rate = _fx(txn.currency, txn.fx_rate_to_jpy)

        if txn.txn_type == "BUY":
            cost_jpy = (txn.price * txn.quantity + txn.fee) * rate
            qty[sym] = qty.get(sym, 0.0) + txn.quantity
            cost[sym] = cost.get(sym, 0.0) + cost_jpy
        elif txn.txn_type == "SELL":
            prev_qty = qty.get(sym, 0.0)
            if prev_qty > 0:
                ratio = txn.quantity / prev_qty
                sell_cost = cost.get(sym, 0.0) * ratio
                proceeds = (txn.price * txn.quantity - txn.fee) * rate
                realized += proceeds - sell_cost
                cost[sym] = cost.get(sym, 0.0) - sell_cost
            qty[sym] = max(0.0, qty.get(sym, 0.0) - txn.quantity)

    return realized
