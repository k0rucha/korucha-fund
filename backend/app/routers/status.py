import datetime
from fastapi import APIRouter, Depends
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from app.database import get_db
from app.models.api_stats import ApiRequestStats
from app.schemas.share_card import StatusResponse
from app.services.yfinance_service import MAX_REQUESTS_PER_DAY

router = APIRouter(prefix="/api", tags=["status"])


@router.get("/status", response_model=StatusResponse)
async def get_status(db: AsyncSession = Depends(get_db)):
    today = datetime.date.today()
    result = await db.execute(
        select(ApiRequestStats).where(ApiRequestStats.reset_date == today)
    )
    stats = result.scalar_one_or_none()
    count = stats.request_count if stats else 0
    last_req = stats.last_request_time if stats else None

    # try to get next scheduler run from app state
    next_run: datetime.datetime | None = None
    try:
        from app.services.scheduler import get_next_run
        next_run = get_next_run()
    except Exception:
        pass

    return StatusResponse(
        request_count=count,
        max_requests_per_day=MAX_REQUESTS_PER_DAY,
        remaining_requests=max(0, MAX_REQUESTS_PER_DAY - count),
        last_request_time=last_req,
        next_scheduler_run=next_run,
    )
