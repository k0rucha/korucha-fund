import datetime
import logging
from fastapi import APIRouter, Depends, HTTPException, BackgroundTasks
from fastapi.responses import Response
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from app.database import get_db
from app.models.transaction import Transaction
from app.models.price_cache import PriceCache
from app.models.fx_cache import FxCache
from app.models.ticker_share_card import TickerShareCard
from app.schemas.share_card import (
    TickerShareCardCreate, TickerShareCardCreated, TickerShareCardRead,
)
from app.services.portfolio import calculate_holdings
from app.services.id_gen import generate_id
from app.services.ogp import generate_ticker_ogp

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/ticker-share-cards", tags=["ticker-share-cards"])


async def _backfill_ticker_bg(symbol: str, db_url: str):
    from app.services.yfinance_service import get_price_history
    from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
    from sqlalchemy.dialects.sqlite import insert as sqlite_insert
    from app.models.price_cache import PriceCache

    engine = create_async_engine(db_url)
    Session = async_sessionmaker(engine, expire_on_commit=False)
    history = get_price_history(symbol, days=35)
    async with Session() as session:
        for d, price in history.items():
            stmt = sqlite_insert(PriceCache).values(
                symbol=symbol, date=d, close_price=price
            ).on_conflict_do_update(
                index_elements=["symbol", "date"],
                set_={"close_price": price},
            )
            await session.execute(stmt)
        await session.commit()
    await engine.dispose()


@router.post("", response_model=TickerShareCardCreated, status_code=201)
async def create_ticker_share_card(
    body: TickerShareCardCreate,
    background_tasks: BackgroundTasks,
    db: AsyncSession = Depends(get_db),
):
    symbol = body.symbol.upper()

    # get current price from cache
    price_result = await db.execute(
        select(PriceCache)
        .where(PriceCache.symbol == symbol)
        .order_by(PriceCache.date.desc())
        .limit(1)
    )
    price_row = price_result.scalar_one_or_none()
    if price_row is None:
        raise HTTPException(status_code=404, detail=f"No price data for {symbol}. Try refreshing first.")

    issue_price = price_row.close_price
    currency = "JPY" if symbol.endswith(".T") else "USD"

    # FX rate if USD
    fx_rate: float | None = None
    if currency == "USD":
        fx_result = await db.execute(
            select(FxCache).where(FxCache.pair == "USDJPY").order_by(FxCache.date.desc()).limit(1)
        )
        fx_row = fx_result.scalar_one_or_none()
        fx_rate = fx_row.rate if fx_row else None

    # get holding info for this symbol
    txn_result = await db.execute(
        select(Transaction).where(Transaction.symbol == symbol)
    )
    txns = txn_result.scalars().all()
    calc = calculate_holdings(list(txns))
    holding = next((h for h in calc.holdings if h.symbol == symbol), None)

    quantity: float | None = None
    avg_cost_native: float | None = None
    issue_value_jpy: float | None = None
    issue_pnl_jpy: float | None = None

    if holding:
        quantity = holding.quantity
        rate = fx_rate if currency == "USD" else 1.0
        avg_cost_native = (holding.total_cost_jpy / holding.quantity / (rate or 1.0)) if holding.quantity > 0 else None
        issue_value_jpy = issue_price * holding.quantity * (rate or 1.0)
        issue_pnl_jpy = issue_value_jpy - holding.total_cost_jpy

    # backfill if fewer than 5 price entries
    price_count_result = await db.execute(
        select(PriceCache).where(PriceCache.symbol == symbol)
    )
    price_count = len(price_count_result.scalars().all())
    if price_count < 5:
        from app.config import settings
        background_tasks.add_task(_backfill_ticker_bg, symbol, settings.database_url)

    # fetch display name
    from app.models.symbol import Symbol as SymbolModel
    sym_result = await db.execute(select(SymbolModel).where(SymbolModel.symbol == symbol))
    sym_row = sym_result.scalar_one_or_none()

    card = TickerShareCard(
        id=generate_id(),
        created_at=datetime.datetime.now(),
        symbol=symbol,
        display_name=sym_row.name if sym_row else None,
        currency=currency,
        issue_price_native=issue_price,
        fx_rate_at_issue=fx_rate,
        quantity=quantity,
        avg_cost_native=avg_cost_native,
        issue_value_jpy=issue_value_jpy,
        issue_pnl_jpy=issue_pnl_jpy,
        default_span=body.span,
    )
    db.add(card)
    await db.commit()
    return TickerShareCardCreated(id=card.id, url=f"/ticker/{card.id}")


@router.get("/{card_id}", response_model=TickerShareCardRead)
async def get_ticker_share_card(card_id: str, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(TickerShareCard).where(TickerShareCard.id == card_id))
    card = result.scalar_one_or_none()
    if card is None:
        raise HTTPException(status_code=404, detail="Ticker share card not found")

    # current price
    price_result = await db.execute(
        select(PriceCache)
        .where(PriceCache.symbol == card.symbol)
        .order_by(PriceCache.date.desc())
        .limit(1)
    )
    price_row = price_result.scalar_one_or_none()
    current_price = price_row.close_price if price_row else None
    delta = (current_price - card.issue_price_native) if current_price else None

    # price history from cache
    hist_result = await db.execute(
        select(PriceCache)
        .where(PriceCache.symbol == card.symbol, PriceCache.date >= card.created_at.date())
        .order_by(PriceCache.date)
    )
    hist_rows = hist_result.scalars().all()

    return TickerShareCardRead(
        id=card.id,
        created_at=card.created_at,
        symbol=card.symbol,
        display_name=card.display_name,
        currency=card.currency,
        issue_price_native=card.issue_price_native,
        fx_rate_at_issue=card.fx_rate_at_issue,
        quantity=card.quantity,
        avg_cost_native=card.avg_cost_native,
        issue_value_jpy=card.issue_value_jpy,
        issue_pnl_jpy=card.issue_pnl_jpy,
        default_span=card.default_span,
        current_price_native=current_price,
        price_delta_native=delta,
        history_dates=[r.date.isoformat() for r in hist_rows],
        history_prices=[r.close_price for r in hist_rows],
    )


@router.get("/{card_id}/ogp.png")
async def get_ticker_share_card_ogp(card_id: str, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(TickerShareCard).where(TickerShareCard.id == card_id))
    card = result.scalar_one_or_none()
    if card is None:
        raise HTTPException(status_code=404, detail="Ticker share card not found")

    price_result = await db.execute(
        select(PriceCache)
        .where(PriceCache.symbol == card.symbol)
        .order_by(PriceCache.date.desc())
        .limit(1)
    )
    price_row = price_result.scalar_one_or_none()
    current_price = price_row.close_price if price_row else None
    delta = (current_price - card.issue_price_native) if current_price else None

    png = generate_ticker_ogp(
        symbol=card.symbol,
        display_name=card.display_name,
        currency=card.currency,
        issue_price_native=card.issue_price_native,
        created_at=card.created_at,
        current_price_native=current_price,
        price_delta_native=delta,
        issue_pnl_jpy=card.issue_pnl_jpy,
    )
    return Response(
        content=png,
        media_type="image/png",
        headers={"Cache-Control": "public, max-age=31536000, immutable"},
    )
