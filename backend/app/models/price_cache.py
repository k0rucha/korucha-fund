from sqlalchemy import String, Float, Date, PrimaryKeyConstraint
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class PriceCache(Base):
    __tablename__ = "price_cache"

    symbol: Mapped[str] = mapped_column(String, nullable=False)
    date: Mapped[datetime.date] = mapped_column(Date, nullable=False)
    close_price: Mapped[float] = mapped_column(Float, nullable=False)

    __table_args__ = (
        PrimaryKeyConstraint("symbol", "date"),
    )
