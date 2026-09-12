from sqlalchemy import String, Float, DateTime, Index, func
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class TickerShareCard(Base):
    __tablename__ = "ticker_share_cards"

    id: Mapped[str] = mapped_column(String, primary_key=True)
    created_at: Mapped[datetime.datetime] = mapped_column(
        DateTime, nullable=False, server_default=func.now()
    )
    symbol: Mapped[str] = mapped_column(String, nullable=False)
    display_name: Mapped[str | None] = mapped_column(String, nullable=True)
    currency: Mapped[str] = mapped_column(String, nullable=False)
    issue_price_native: Mapped[float] = mapped_column(Float, nullable=False)
    fx_rate_at_issue: Mapped[float | None] = mapped_column(Float, nullable=True)
    quantity: Mapped[float | None] = mapped_column(Float, nullable=True)
    avg_cost_native: Mapped[float | None] = mapped_column(Float, nullable=True)
    issue_value_jpy: Mapped[float | None] = mapped_column(Float, nullable=True)
    issue_pnl_jpy: Mapped[float | None] = mapped_column(Float, nullable=True)
    default_span: Mapped[str] = mapped_column(String, nullable=False, default="30d")

    __table_args__ = (
        Index("idx_ticker_share_cards_created", "created_at"),
        Index("idx_ticker_share_cards_symbol", "symbol"),
    )
