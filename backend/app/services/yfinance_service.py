import yfinance as yf
import datetime
from typing import Optional
import logging

logger = logging.getLogger(__name__)

MAX_REQUESTS_PER_DAY = 15
MIN_INTERVAL_MINUTES = 96


def get_latest_close(symbol: str) -> Optional[float]:
    try:
        ticker = yf.Ticker(symbol)
        hist = ticker.history(period="5d")
        if hist.empty:
            return None
        return float(hist["Close"].iloc[-1])
    except Exception as e:
        logger.warning("Failed to fetch price for %s: %s", symbol, e)
        return None


def get_symbol_name(symbol: str) -> Optional[str]:
    try:
        info = yf.Ticker(symbol).info
        return info.get("longName") or info.get("shortName")
    except Exception as e:
        logger.warning("Failed to fetch name for %s: %s", symbol, e)
        return None


def get_price_history(symbol: str, days: int = 35) -> dict[datetime.date, float]:
    try:
        ticker = yf.Ticker(symbol)
        hist = ticker.history(period=f"{days}d")
        result = {}
        for dt, row in hist.iterrows():
            d = dt.date() if hasattr(dt, "date") else dt
            result[d] = float(row["Close"])
        return result
    except Exception as e:
        logger.warning("Failed to fetch history for %s: %s", symbol, e)
        return {}


def get_fx_history(pair: str = "USDJPY=X", days: int = 35) -> dict[datetime.date, float]:
    try:
        ticker = yf.Ticker(pair)
        hist = ticker.history(period=f"{days}d")
        result = {}
        for dt, row in hist.iterrows():
            d = dt.date() if hasattr(dt, "date") else dt
            result[d] = float(row["Close"])
        return result
    except Exception as e:
        logger.warning("Failed to fetch FX history: %s", e)
        return {}
