from pydantic import BaseModel, field_validator
from typing import Literal
import datetime


class TransactionCreate(BaseModel):
    symbol: str
    txn_type: Literal["BUY", "SELL"]
    quantity: float
    price: float
    currency: Literal["JPY", "USD"]
    fee: float = 0.0
    txn_date: datetime.date
    fx_rate_to_jpy: float | None = None
    notes: str | None = None

    @field_validator("quantity", "price")
    @classmethod
    def must_be_positive(cls, v: float) -> float:
        if v <= 0:
            raise ValueError("must be positive")
        return v

    @field_validator("fee")
    @classmethod
    def fee_non_negative(cls, v: float) -> float:
        if v < 0:
            raise ValueError("must be non-negative")
        return v


class TransactionRead(BaseModel):
    model_config = {"from_attributes": True}

    id: int
    symbol: str
    txn_type: str
    quantity: float
    price: float
    currency: str
    fee: float
    txn_date: datetime.date
    fx_rate_to_jpy: float | None
    notes: str | None
    created_at: datetime.datetime
    symbol_name: str | None = None


class ImportResult(BaseModel):
    imported: int
    skipped: int
