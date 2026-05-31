from sqlalchemy import Integer, DateTime, Date, UniqueConstraint, func
from sqlalchemy.orm import Mapped, mapped_column
from app.database import Base
import datetime


class ApiRequestStats(Base):
    __tablename__ = "api_request_stats"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    last_request_time: Mapped[datetime.datetime | None] = mapped_column(DateTime, nullable=True)
    request_count: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    reset_date: Mapped[datetime.date] = mapped_column(Date, nullable=False, server_default=func.current_date())

    __table_args__ = (
        UniqueConstraint("reset_date"),
    )
