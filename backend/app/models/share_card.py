from sqlalchemy import String, Float, DateTime, Text, Index, func
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class ShareCard(Base):
    __tablename__ = "share_cards"

    id: Mapped[str] = mapped_column(String, primary_key=True)
    created_at: Mapped[datetime.datetime] = mapped_column(
        DateTime, nullable=False, server_default=func.now()
    )
    total_value_jpy: Mapped[float] = mapped_column(Float, nullable=False)
    total_cost_jpy: Mapped[float] = mapped_column(Float, nullable=False)
    unrealized_pnl_jpy: Mapped[float] = mapped_column(Float, nullable=False)
    holdings_json: Mapped[str] = mapped_column(Text, nullable=False)
    default_span: Mapped[str] = mapped_column(String, nullable=False, default="all")

    __table_args__ = (
        Index("idx_share_cards_created", "created_at"),
    )
