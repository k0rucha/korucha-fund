from sqlalchemy import Integer, String, Float, Date, DateTime, Index, func
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class Transaction(Base):
    __tablename__ = "transactions"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    symbol: Mapped[str] = mapped_column(String, nullable=False)
    txn_type: Mapped[str] = mapped_column(String, nullable=False)  # BUY | SELL
    quantity: Mapped[float] = mapped_column(Float, nullable=False)
    price: Mapped[float] = mapped_column(Float, nullable=False)
    currency: Mapped[str] = mapped_column(String, nullable=False)  # JPY | USD
    fee: Mapped[float] = mapped_column(Float, nullable=False, default=0.0)
    txn_date: Mapped[datetime.date] = mapped_column(Date, nullable=False)
    fx_rate_to_jpy: Mapped[float | None] = mapped_column(Float, nullable=True)
    notes: Mapped[str | None] = mapped_column(String, nullable=True)
    created_at: Mapped[datetime.datetime] = mapped_column(
        DateTime, nullable=False, server_default=func.now()
    )

    __table_args__ = (
        Index("idx_txn_symbol_date", "symbol", "txn_date"),
    )
