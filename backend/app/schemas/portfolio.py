from pydantic import BaseModel
import datetime


class HoldingView(BaseModel):
    symbol: str
    name: str | None
    quantity: float
    avg_cost_native: float
    currency: str
    current_price: float | None
    current_value_jpy: float | None
    cost_jpy: float
    unrealized_pnl_jpy: float | None
    pnl_pct: float | None
    dod_delta_jpy: float | None
    mom_delta_jpy: float | None


class DashboardResponse(BaseModel):
    holdings: list[HoldingView]
    total_cost_jpy: float
    total_value_jpy: float | None
    total_unrealized_pnl_jpy: float | None
    total_pnl_pct: float | None
    realized_pnl_jpy: float
    cumulative_pnl_jpy: float | None
    dod_delta_jpy: float | None
    mom_delta_jpy: float | None
    last_updated: datetime.datetime | None


class CompositionItem(BaseModel):
    symbol: str
    label: str
    value_jpy: float


class TimeseriesResponse(BaseModel):
    dates: list[str]
    values: list[float]
    costs: list[float]
    pnls: list[float]


class RefreshResponse(BaseModel):
    ok: bool
    updated_from_api: bool
    remaining_api_requests: int
    message: str | None = None
