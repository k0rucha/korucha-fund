from sqlalchemy import Float, Date
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class Snapshot(Base):
    __tablename__ = "snapshots"

    date: Mapped[datetime.date] = mapped_column(Date, primary_key=True)
    total_value_jpy: Mapped[float] = mapped_column(Float, nullable=False)
    total_cost_jpy: Mapped[float] = mapped_column(Float, nullable=False)
    unrealized_pnl_jpy: Mapped[float] = mapped_column(Float, nullable=False)
