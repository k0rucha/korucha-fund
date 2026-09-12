import datetime
import logging
from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, text
from sqlalchemy.dialects.sqlite import insert as sqlite_insert
from app.database import get_db
from app.models.transaction import Transaction
from app.models.symbol import Symbol
from app.models.price_cache import PriceCache
from app.models.fx_cache import FxCache
from app.models.snapshot import Snapshot
from app.models.api_stats import ApiRequestStats
from app.schemas.portfolio import (
    DashboardResponse, HoldingView, CompositionItem, TimeseriesResponse, RefreshResponse,
)
from app.services.portfolio import calculate_holdings, calculate_realized_pnl
from app.services.yfinance_service import (
    MAX_REQUESTS_PER_DAY, MIN_INTERVAL_MINUTES,
    get_latest_close, get_fx_history,
)

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/portfolio", tags=["portfolio"])


async def _get_latest_price(symbol: str, db: AsyncSession) -> tuple[float | None, datetime.date | None]:
    result = await db.execute(
        select(PriceCache)
        .where(PriceCache.symbol == symbol)
        .order_by(PriceCache.date.desc())
        .limit(1)
    )
    row = result.scalar_one_or_none()
    if row:
        return row.close_price, row.date
    return None, None


async def _get_latest_fx(db: AsyncSession) -> float | None:
    result = await db.execute(
        select(FxCache)
        .where(FxCache.pair == "USDJPY")
        .order_by(FxCache.date.desc())
        .limit(1)
    )
    row = result.scalar_one_or_none()
    return row.rate if row else None


async def _get_snapshot_near(db: AsyncSession, target: datetime.date, max_days: int) -> float | None:
    floor = target - datetime.timedelta(days=max_days)
    result = await db.execute(
        select(Snapshot)
        .where(Snapshot.date >= floor, Snapshot.date <= target)
        .order_by(Snapshot.date.desc())
        .limit(1)
    )
    row = result.scalar_one_or_none()
    return row.total_value_jpy if row else None


@router.get("/dashboard", response_model=DashboardResponse)
async def get_dashboard(db: AsyncSession = Depends(get_db)):
    txn_result = await db.execute(select(Transaction))
    transactions = txn_result.scalars().all()

    sym_result = await db.execute(select(Symbol))
    sym_map = {s.symbol: s.name for s in sym_result.scalars().all()}

    calc = calculate_holdings(list(transactions))
    realized_pnl = calculate_realized_pnl(list(transactions))

    usdjpy = await _get_latest_fx(db)
    today = datetime.date.today()

    holdings_view = []
    total_value = 0.0
    has_price = True

    for h in calc.holdings:
        price, price_date = await _get_latest_price(h.symbol, db)
        rate = usdjpy if h.currency == "USD" else 1.0

        if price is not None and rate is not None:
            value_jpy = price * h.quantity * rate
            pnl_jpy = value_jpy - h.cost_jpy if hasattr(h, "cost_jpy") else value_jpy - h.total_cost_jpy
            pnl_jpy = value_jpy - h.total_cost_jpy
            pnl_pct = pnl_jpy / h.total_cost_jpy * 100 if h.total_cost_jpy else None
            total_value += value_jpy

            avg_cost_native = (
                (h.total_cost_jpy / h.quantity / rate) if h.currency == "USD"
                else (h.total_cost_jpy / h.quantity)
            ) if h.quantity > 0 else 0.0
        else:
            value_jpy = None
            pnl_jpy = None
            pnl_pct = None
            avg_cost_native = 0.0
            has_price = False

        # dod/mom delta per holding — use snapshot-based delta (portfolio level, simplified)
        holdings_view.append(HoldingView(
            symbol=h.symbol,
            name=sym_map.get(h.symbol),
            quantity=h.quantity,
            avg_cost_native=avg_cost_native,
            currency=h.currency,
            current_price=price,
            current_value_jpy=value_jpy,
            cost_jpy=h.total_cost_jpy,
            unrealized_pnl_jpy=pnl_jpy,
            pnl_pct=pnl_pct,
            dod_delta_jpy=None,
            mom_delta_jpy=None,
        ))

    total_cost = calc.total_cost_jpy
    total_value_opt = total_value if has_price else None
    unrealized = (total_value - total_cost) if total_value_opt is not None else None
    pnl_pct_total = (unrealized / total_cost * 100) if (unrealized is not None and total_cost > 0) else None
    cumulative = (realized_pnl + unrealized) if unrealized is not None else None

    # dod/mom portfolio deltas from snapshots
    yesterday = today - datetime.timedelta(days=1)
    month_ago = today - datetime.timedelta(days=30)
    dod_ref = await _get_snapshot_near(db, yesterday, 7)
    mom_ref = await _get_snapshot_near(db, month_ago, 60)
    dod_delta = (total_value - dod_ref) if (total_value_opt and dod_ref) else None
    mom_delta = (total_value - mom_ref) if (total_value_opt and mom_ref) else None

    # last_updated from most recent price_cache entry
    last_upd_result = await db.execute(
        select(PriceCache).order_by(PriceCache.date.desc()).limit(1)
    )
    last_upd_row = last_upd_result.scalar_one_or_none()

    return DashboardResponse(
        holdings=holdings_view,
        total_cost_jpy=total_cost,
        total_value_jpy=total_value_opt,
        total_unrealized_pnl_jpy=unrealized,
        total_pnl_pct=pnl_pct_total,
        realized_pnl_jpy=realized_pnl,
        cumulative_pnl_jpy=cumulative,
        dod_delta_jpy=dod_delta,
        mom_delta_jpy=mom_delta,
        last_updated=datetime.datetime.combine(last_upd_row.date, datetime.time()) if last_upd_row else None,
    )


@router.get("/composition", response_model=list[CompositionItem])
async def get_composition(db: AsyncSession = Depends(get_db)):
    txn_result = await db.execute(select(Transaction))
    calc = calculate_holdings(list(txn_result.scalars().all()))
    usdjpy = await _get_latest_fx(db)

    sym_result = await db.execute(select(Symbol))
    sym_map = {s.symbol: s.name for s in sym_result.scalars().all()}

    items = []
    for h in calc.holdings:
        price, _ = await _get_latest_price(h.symbol, db)
        if price is None:
            continue
        rate = usdjpy if h.currency == "USD" else 1.0
        value = price * h.quantity * (rate or 1.0)
        name = sym_map.get(h.symbol) or h.symbol
        label = name[:10] + "…" if len(name) > 10 else name
        items.append(CompositionItem(symbol=h.symbol, label=label, value_jpy=value))

    return sorted(items, key=lambda x: x.value_jpy, reverse=True)


@router.get("/timeseries", response_model=TimeseriesResponse)
async def get_timeseries(db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Snapshot).order_by(Snapshot.date))
    rows = result.scalars().all()
    return TimeseriesResponse(
        dates=[r.date.isoformat() for r in rows],
        values=[r.total_value_jpy for r in rows],
        costs=[r.total_cost_jpy for r in rows],
        pnls=[r.unrealized_pnl_jpy for r in rows],
    )


async def _check_rate_limit(db: AsyncSession) -> tuple[bool, int]:
    """レート制限チェック。(allowed, remaining) を返す。"""
    today = datetime.date.today()
    result = await db.execute(
        select(ApiRequestStats).where(ApiRequestStats.reset_date == today)
    )
    stats = result.scalar_one_or_none()

    if stats is None:
        stats = ApiRequestStats(reset_date=today, request_count=0)
        db.add(stats)
        await db.flush()

    if stats.request_count >= MAX_REQUESTS_PER_DAY:
        remaining = 0
        return False, remaining

    if stats.last_request_time:
        elapsed = (datetime.datetime.now() - stats.last_request_time).total_seconds() / 60
        if elapsed < MIN_INTERVAL_MINUTES:
            return False, MAX_REQUESTS_PER_DAY - stats.request_count

    stats.request_count += 1
    stats.last_request_time = datetime.datetime.now()
    await db.commit()
    return True, MAX_REQUESTS_PER_DAY - stats.request_count


@router.post("/refresh", response_model=RefreshResponse)
async def refresh_prices(db: AsyncSession = Depends(get_db)):
    allowed, remaining = await _check_rate_limit(db)
    if not allowed:
        raise HTTPException(
            status_code=429,
            detail=f"API rate limit reached. Remaining: {remaining}/{MAX_REQUESTS_PER_DAY}",
        )

    txn_result = await db.execute(select(Transaction))
    calc = calculate_holdings(list(txn_result.scalars().all()))
    symbols = list({h.symbol for h in calc.holdings})
    has_usd = any(h.currency == "USD" for h in calc.holdings)

    today = datetime.date.today()
    updated = False

    for sym in symbols:
        price = get_latest_close(sym)
        if price is not None:
            stmt = sqlite_insert(PriceCache).values(
                symbol=sym, date=today, close_price=price
            ).on_conflict_do_update(
                index_elements=["symbol", "date"],
                set_={"close_price": price},
            )
            await db.execute(stmt)
            updated = True

    if has_usd:
        fx_hist = get_fx_history("USDJPY=X", days=2)
        for d, rate in fx_hist.items():
            stmt = sqlite_insert(FxCache).values(
                pair="USDJPY", date=d, rate=rate
            ).on_conflict_do_update(
                index_elements=["pair", "date"],
                set_={"rate": rate},
            )
            await db.execute(stmt)

    # generate today's snapshot
    usdjpy = await _get_latest_fx(db)
    total_value = 0.0
    total_cost = calc.total_cost_jpy
    all_priced = True

    for h in calc.holdings:
        price, _ = await _get_latest_price(h.symbol, db)
        rate = usdjpy if h.currency == "USD" else 1.0
        if price is not None and rate is not None:
            total_value += price * h.quantity * rate
        else:
            all_priced = False

    if all_priced and calc.holdings:
        stmt = sqlite_insert(Snapshot).values(
            date=today,
            total_value_jpy=total_value,
            total_cost_jpy=total_cost,
            unrealized_pnl_jpy=total_value - total_cost,
        ).on_conflict_do_update(
            index_elements=["date"],
            set_={
                "total_value_jpy": total_value,
                "total_cost_jpy": total_cost,
                "unrealized_pnl_jpy": total_value - total_cost,
            },
        )
        await db.execute(stmt)

    await db.commit()

    result2 = await db.execute(
        select(ApiRequestStats).where(ApiRequestStats.reset_date == today)
    )
    stats = result2.scalar_one_or_none()
    remaining2 = MAX_REQUESTS_PER_DAY - (stats.request_count if stats else 0)

    return RefreshResponse(ok=True, updated_from_api=updated, remaining_api_requests=remaining2)
