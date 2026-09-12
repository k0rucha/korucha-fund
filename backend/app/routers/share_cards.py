import json
import datetime
import logging
from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import Response
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from app.database import get_db
from app.models.transaction import Transaction
from app.models.symbol import Symbol
from app.models.price_cache import PriceCache
from app.models.fx_cache import FxCache
from app.models.snapshot import Snapshot
from app.models.share_card import ShareCard
from app.schemas.share_card import (
    ShareCardCreate, ShareCardCreated, ShareCardRead, CardHolding,
)
from app.services.portfolio import calculate_holdings
from app.services.id_gen import generate_id
from app.services.ogp import generate_portfolio_ogp

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/share-cards", tags=["share-cards"])


async def _current_value(holdings, db: AsyncSession) -> float | None:
    from app.routers.portfolio import _get_latest_price, _get_latest_fx
    usdjpy = await _get_latest_fx(db)
    total = 0.0
    for h in holdings:
        price, _ = await _get_latest_price(h.symbol, db)
        if price is None:
            return None
        rate = usdjpy if h.currency == "USD" else 1.0
        total += price * h.quantity * (rate or 1.0)
    return total


@router.post("", response_model=ShareCardCreated, status_code=201)
async def create_share_card(body: ShareCardCreate, db: AsyncSession = Depends(get_db)):
    txn_result = await db.execute(select(Transaction))
    calc = calculate_holdings(list(txn_result.scalars().all()))

    sym_result = await db.execute(select(Symbol))
    sym_map = {s.symbol: s.name for s in sym_result.scalars().all()}

    from app.routers.portfolio import _get_latest_price, _get_latest_fx
    usdjpy = await _get_latest_fx(db)

    total_value = 0.0
    card_holdings = []
    for h in calc.holdings:
        price, _ = await _get_latest_price(h.symbol, db)
        rate = usdjpy if h.currency == "USD" else 1.0
        v = (price * h.quantity * (rate or 1.0)) if price else 0.0
        total_value += v
        card_holdings.append(CardHolding(
            symbol=h.symbol,
            name=sym_map.get(h.symbol),
            quantity=h.quantity,
            value_jpy=v,
        ))

    card = ShareCard(
        id=generate_id(),
        created_at=datetime.datetime.now(),
        total_value_jpy=total_value,
        total_cost_jpy=calc.total_cost_jpy,
        unrealized_pnl_jpy=total_value - calc.total_cost_jpy,
        holdings_json=json.dumps([h.model_dump() for h in card_holdings], ensure_ascii=False),
        default_span=body.span,
    )
    db.add(card)
    await db.commit()
    return ShareCardCreated(id=card.id, url=f"/share/{card.id}")


@router.get("/{card_id}", response_model=ShareCardRead)
async def get_share_card(card_id: str, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(ShareCard).where(ShareCard.id == card_id))
    card = result.scalar_one_or_none()
    if card is None:
        raise HTTPException(status_code=404, detail="Share card not found")

    holdings = [CardHolding(**h) for h in json.loads(card.holdings_json)]

    # current value
    from app.routers.portfolio import _get_latest_price, _get_latest_fx
    usdjpy = await _get_latest_fx(db)

    txn_result = await db.execute(select(Transaction))
    calc = calculate_holdings(list(txn_result.scalars().all()))

    curr_val: float | None = None
    try:
        total = 0.0
        for h in calc.holdings:
            price, _ = await _get_latest_price(h.symbol, db)
            rate = usdjpy if h.currency == "USD" else 1.0
            if price is None:
                raise ValueError
            total += price * h.quantity * (rate or 1.0)
        curr_val = total
    except (ValueError, Exception):
        pass

    delta = (curr_val - card.total_value_jpy) if curr_val is not None else None

    # chart history from snapshots
    snap_result = await db.execute(select(Snapshot).order_by(Snapshot.date))
    snaps = snap_result.scalars().all()
    issue_date = card.created_at.date()
    history = [s for s in snaps if s.date >= issue_date]

    return ShareCardRead(
        id=card.id,
        created_at=card.created_at,
        total_value_jpy=card.total_value_jpy,
        total_cost_jpy=card.total_cost_jpy,
        unrealized_pnl_jpy=card.unrealized_pnl_jpy,
        default_span=card.default_span,
        holdings=holdings,
        current_value_jpy=curr_val,
        value_delta_jpy=delta,
        history_dates=[s.date.isoformat() for s in history],
        history_values=[s.total_value_jpy for s in history],
        history_pnls=[s.unrealized_pnl_jpy for s in history],
    )


@router.get("/{card_id}/ogp.png")
async def get_share_card_ogp(card_id: str, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(ShareCard).where(ShareCard.id == card_id))
    card = result.scalar_one_or_none()
    if card is None:
        raise HTTPException(status_code=404, detail="Share card not found")

    from app.routers.portfolio import _get_latest_price, _get_latest_fx
    usdjpy = await _get_latest_fx(db)
    txn_result = await db.execute(select(Transaction))
    calc = calculate_holdings(list(txn_result.scalars().all()))

    curr_val: float | None = None
    try:
        total = 0.0
        for h in calc.holdings:
            price, _ = await _get_latest_price(h.symbol, db)
            rate = usdjpy if h.currency == "USD" else 1.0
            if price is None:
                raise ValueError
            total += price * h.quantity * (rate or 1.0)
        curr_val = total
    except Exception:
        pass

    delta = (curr_val - card.total_value_jpy) if curr_val is not None else None
    png = generate_portfolio_ogp(
        total_value_jpy=card.total_value_jpy,
        total_cost_jpy=card.total_cost_jpy,
        unrealized_pnl_jpy=card.unrealized_pnl_jpy,
        created_at=card.created_at,
        current_value_jpy=curr_val,
        value_delta_jpy=delta,
    )
    return Response(
        content=png,
        media_type="image/png",
        headers={"Cache-Control": "public, max-age=31536000, immutable"},
    )
