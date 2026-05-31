import datetime
import logging
from typing import Optional
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.cron import CronTrigger

logger = logging.getLogger(__name__)
_scheduler: Optional[AsyncIOScheduler] = None


async def _daily_task(db_url: str):
    from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
    from sqlalchemy.dialects.sqlite import insert as sqlite_insert
    from sqlalchemy import select
    from app.models.transaction import Transaction
    from app.models.price_cache import PriceCache
    from app.models.fx_cache import FxCache
    from app.models.snapshot import Snapshot
    from app.services.portfolio import calculate_holdings
    from app.services.yfinance_service import get_latest_close, get_fx_history

    engine = create_async_engine(db_url)
    Session = async_sessionmaker(engine, expire_on_commit=False)

    async with Session() as session:
        txn_result = await session.execute(select(Transaction))
        calc = calculate_holdings(list(txn_result.scalars().all()))
        symbols = list({h.symbol for h in calc.holdings})
        has_usd = any(h.currency == "USD" for h in calc.holdings)
        today = datetime.date.today()

        for sym in symbols:
            price = get_latest_close(sym)
            if price is not None:
                stmt = sqlite_insert(PriceCache).values(
                    symbol=sym, date=today, close_price=price
                ).on_conflict_do_update(
                    index_elements=["symbol", "date"],
                    set_={"close_price": price},
                )
                await session.execute(stmt)
                logger.info("Cached price %s = %s", sym, price)

        if has_usd:
            fx_hist = get_fx_history("USDJPY=X", days=2)
            for d, rate in fx_hist.items():
                stmt = sqlite_insert(FxCache).values(
                    pair="USDJPY", date=d, rate=rate
                ).on_conflict_do_update(
                    index_elements=["pair", "date"],
                    set_={"rate": rate},
                )
                await session.execute(stmt)

        # generate snapshot
        fx_result = await session.execute(
            select(FxCache).where(FxCache.pair == "USDJPY").order_by(FxCache.date.desc()).limit(1)
        )
        fx_row = fx_result.scalar_one_or_none()
        usdjpy = fx_row.rate if fx_row else None

        total_value = 0.0
        all_priced = True
        for h in calc.holdings:
            price_result = await session.execute(
                select(PriceCache)
                .where(PriceCache.symbol == h.symbol)
                .order_by(PriceCache.date.desc())
                .limit(1)
            )
            pr = price_result.scalar_one_or_none()
            rate = usdjpy if h.currency == "USD" else 1.0
            if pr is not None and rate is not None:
                total_value += pr.close_price * h.quantity * rate
            else:
                all_priced = False

        if all_priced and calc.holdings:
            stmt = sqlite_insert(Snapshot).values(
                date=today,
                total_value_jpy=total_value,
                total_cost_jpy=calc.total_cost_jpy,
                unrealized_pnl_jpy=total_value - calc.total_cost_jpy,
            ).on_conflict_do_update(
                index_elements=["date"],
                set_={
                    "total_value_jpy": total_value,
                    "total_cost_jpy": calc.total_cost_jpy,
                    "unrealized_pnl_jpy": total_value - calc.total_cost_jpy,
                },
            )
            await session.execute(stmt)
            logger.info("Snapshot generated for %s: %s JPY", today, total_value)

        await session.commit()
    await engine.dispose()


def start_scheduler(cron: str, db_url: str):
    global _scheduler
    parts = cron.strip().split()
    if len(parts) == 5:
        minute, hour, day, month, dow = parts
        second = "0"
    elif len(parts) == 6:
        second, minute, hour, day, month, dow = parts
    else:
        logger.warning("Invalid cron expression: %s, skipping scheduler", cron)
        return

    _scheduler = AsyncIOScheduler()
    trigger = CronTrigger(
        second=second, minute=minute, hour=hour,
        day=day, month=month, day_of_week=dow,
        timezone="Asia/Tokyo",
    )
    _scheduler.add_job(_daily_task, trigger, args=[db_url], id="daily_refresh")
    _scheduler.start()
    logger.info("Scheduler started with cron: %s", cron)


def stop_scheduler():
    global _scheduler
    if _scheduler:
        _scheduler.shutdown(wait=False)
        _scheduler = None


def get_next_run() -> Optional[datetime.datetime]:
    if _scheduler is None:
        return None
    job = _scheduler.get_job("daily_refresh")
    return job.next_run_time if job else None
