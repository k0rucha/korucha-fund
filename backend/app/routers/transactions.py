import json
from fastapi import APIRouter, Depends, HTTPException, UploadFile, File, BackgroundTasks
from fastapi.responses import JSONResponse, Response
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, delete
from app.database import get_db
from app.auth import verify_admin
from app.models.transaction import Transaction
from app.models.symbol import Symbol
from app.schemas.transaction import TransactionCreate, TransactionRead, ImportResult
import datetime
import logging

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/admin/transactions", tags=["admin"])


async def _cache_symbol_bg(symbol: str, db_url: str):
    """バックグラウンドでシンボル名・価格をキャッシュする。"""
    from app.services.yfinance_service import get_symbol_name, get_latest_close
    from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
    from sqlalchemy.dialects.sqlite import insert as sqlite_insert
    from app.models.symbol import Symbol
    from app.models.price_cache import PriceCache

    engine = create_async_engine(db_url)
    Session = async_sessionmaker(engine, expire_on_commit=False)

    name = get_symbol_name(symbol)
    close = get_latest_close(symbol)

    async with Session() as session:
        stmt = sqlite_insert(Symbol).values(
            symbol=symbol,
            name=name,
            currency="JPY" if symbol.endswith(".T") else "USD",
            updated_at=datetime.datetime.now(),
        ).on_conflict_do_update(
            index_elements=["symbol"],
            set_={"name": name, "updated_at": datetime.datetime.now()},
        )
        await session.execute(stmt)

        if close is not None:
            today = datetime.date.today()
            stmt2 = sqlite_insert(PriceCache).values(
                symbol=symbol, date=today, close_price=close
            ).on_conflict_do_update(
                index_elements=["symbol", "date"],
                set_={"close_price": close},
            )
            await session.execute(stmt2)
        await session.commit()
    await engine.dispose()


@router.get("", response_model=list[TransactionRead])
async def list_transactions(db: AsyncSession = Depends(get_db), _=Depends(verify_admin)):
    result = await db.execute(select(Transaction).order_by(Transaction.txn_date.desc(), Transaction.id.desc()))
    txns = result.scalars().all()

    syms_result = await db.execute(select(Symbol))
    sym_map = {s.symbol: s.name for s in syms_result.scalars().all()}

    items = []
    for t in txns:
        r = TransactionRead.model_validate(t)
        r.symbol_name = sym_map.get(t.symbol)
        items.append(r)
    return items


@router.post("", response_model=TransactionRead, status_code=201)
async def add_transaction(
    body: TransactionCreate,
    background_tasks: BackgroundTasks,
    db: AsyncSession = Depends(get_db),
    _=Depends(verify_admin),
):
    txn = Transaction(**body.model_dump())
    db.add(txn)
    await db.commit()
    await db.refresh(txn)

    from app.config import settings
    background_tasks.add_task(_cache_symbol_bg, txn.symbol, settings.database_url)

    return TransactionRead.model_validate(txn)


@router.delete("/{txn_id}", status_code=204)
async def delete_transaction(
    txn_id: int,
    db: AsyncSession = Depends(get_db),
    _=Depends(verify_admin),
):
    result = await db.execute(select(Transaction).where(Transaction.id == txn_id))
    txn = result.scalar_one_or_none()
    if txn is None:
        raise HTTPException(status_code=404, detail="Transaction not found")
    await db.delete(txn)
    await db.commit()
    return Response(status_code=204)


@router.get("/export")
async def export_transactions(db: AsyncSession = Depends(get_db), _=Depends(verify_admin)):
    result = await db.execute(select(Transaction).order_by(Transaction.txn_date, Transaction.id))
    txns = result.scalars().all()
    data = [TransactionRead.model_validate(t).model_dump(mode="json") for t in txns]
    content = json.dumps(data, ensure_ascii=False, indent=2)
    return Response(
        content=content,
        media_type="application/json",
        headers={"Content-Disposition": 'attachment; filename="transactions.json"'},
    )


@router.post("/import", response_model=ImportResult)
async def import_transactions(
    background_tasks: BackgroundTasks,
    file: UploadFile = File(...),
    db: AsyncSession = Depends(get_db),
    _=Depends(verify_admin),
):
    content = await file.read()
    try:
        raw = json.loads(content)
        items = [TransactionCreate(**r) for r in raw]
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Invalid JSON: {e}")

    # fetch existing fingerprints
    result = await db.execute(select(Transaction))
    existing = result.scalars().all()

    def fingerprint(t) -> str:
        return f"{t.symbol}|{t.txn_type}|{t.txn_date}|{t.quantity}|{t.price}|{t.fee}"

    existing_fps = {fingerprint(t) for t in existing}

    imported, skipped = 0, 0
    new_symbols: set[str] = set()
    for item in items:
        fp = fingerprint(item)
        if fp in existing_fps:
            skipped += 1
            continue
        txn = Transaction(**item.model_dump())
        db.add(txn)
        existing_fps.add(fp)
        new_symbols.add(item.symbol)
        imported += 1

    await db.commit()

    from app.config import settings
    for sym in new_symbols:
        background_tasks.add_task(_cache_symbol_bg, sym, settings.database_url)

    return ImportResult(imported=imported, skipped=skipped)
