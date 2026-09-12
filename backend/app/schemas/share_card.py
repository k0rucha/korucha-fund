from pydantic import BaseModel
from typing import Literal
import datetime


class CardHolding(BaseModel):
    symbol: str
    name: str | None
    quantity: float
    value_jpy: float


class ShareCardCreate(BaseModel):
    span: Literal["all", "7d", "30d"] = "all"


class ShareCardCreated(BaseModel):
    id: str
    url: str


class ShareCardRead(BaseModel):
    id: str
    created_at: datetime.datetime
    total_value_jpy: float
    total_cost_jpy: float
    unrealized_pnl_jpy: float
    default_span: str
    holdings: list[CardHolding]
    current_value_jpy: float | None
    value_delta_jpy: float | None
    history_dates: list[str]
    history_values: list[float]
    history_pnls: list[float]


class TickerShareCardCreate(BaseModel):
    symbol: str
    span: Literal["all", "7d", "30d"] = "30d"


class TickerShareCardCreated(BaseModel):
    id: str
    url: str


class TickerShareCardRead(BaseModel):
    id: str
    created_at: datetime.datetime
    symbol: str
    display_name: str | None
    currency: str
    issue_price_native: float
    fx_rate_at_issue: float | None
    quantity: float | None
    avg_cost_native: float | None
    issue_value_jpy: float | None
    issue_pnl_jpy: float | None
    default_span: str
    current_price_native: float | None
    price_delta_native: float | None
    history_dates: list[str]
    history_prices: list[float]


class StatusResponse(BaseModel):
    request_count: int
    max_requests_per_day: int
    remaining_requests: int
    last_request_time: datetime.datetime | None
    next_scheduler_run: datetime.datetime | None
