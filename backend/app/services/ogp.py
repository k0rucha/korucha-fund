"""OGP image generation (1200x630 PNG) using Pillow."""
import io
import datetime
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from typing import Optional

STATIC_DIR = Path(__file__).parent.parent.parent.parent / "static"
FONT_PATH = STATIC_DIR / "NotoSansJP-Regular.ttf"
WIDTH, HEIGHT = 1200, 630
BG_COLOR = (22, 23, 29)
ACCENT = (170, 59, 255)
TEXT_PRIMARY = (243, 244, 246)
TEXT_SECONDARY = (156, 163, 175)
GREEN = (52, 211, 153)
RED = (248, 113, 113)


def _load_font(size: int) -> ImageFont.FreeTypeFont:
    try:
        return ImageFont.truetype(str(FONT_PATH), size)
    except Exception:
        return ImageFont.load_default()


def _fmt_jpy(value: float) -> str:
    return f"¥{value:,.0f}"


def _fmt_pnl(value: float) -> str:
    sign = "+" if value >= 0 else ""
    return f"{sign}¥{value:,.0f}"


def generate_portfolio_ogp(
    total_value_jpy: float,
    total_cost_jpy: float,
    unrealized_pnl_jpy: float,
    created_at: datetime.datetime,
    current_value_jpy: Optional[float] = None,
    value_delta_jpy: Optional[float] = None,
) -> bytes:
    img = Image.new("RGB", (WIDTH, HEIGHT), BG_COLOR)
    draw = ImageDraw.Draw(img)

    # accent bar at top
    draw.rectangle([(0, 0), (WIDTH, 6)], fill=ACCENT)

    font_lg = _load_font(64)
    font_md = _load_font(32)
    font_sm = _load_font(24)
    font_xs = _load_font(20)

    # Title
    draw.text((60, 40), "korucha-fund", font=font_md, fill=TEXT_SECONDARY)

    # Main value
    val = current_value_jpy if current_value_jpy is not None else total_value_jpy
    draw.text((60, 100), _fmt_jpy(val), font=font_lg, fill=TEXT_PRIMARY)

    # P&L
    pnl = unrealized_pnl_jpy
    pnl_color = GREEN if pnl >= 0 else RED
    draw.text((60, 190), f"含み損益  {_fmt_pnl(pnl)}", font=font_md, fill=pnl_color)

    # Delta since issue
    if value_delta_jpy is not None:
        delta_color = GREEN if value_delta_jpy >= 0 else RED
        draw.text((60, 245), f"発行時比  {_fmt_pnl(value_delta_jpy)}", font=font_md, fill=delta_color)

    # Cost basis
    draw.text((60, 320), f"取得原価  {_fmt_jpy(total_cost_jpy)}", font=font_sm, fill=TEXT_SECONDARY)

    # Issue date
    draw.text((60, 570), created_at.strftime("%Y-%m-%d %H:%M JST"), font=font_xs, fill=TEXT_SECONDARY)

    # Bottom accent bar
    draw.rectangle([(0, HEIGHT - 6), (WIDTH, HEIGHT)], fill=ACCENT)

    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def generate_ticker_ogp(
    symbol: str,
    display_name: Optional[str],
    currency: str,
    issue_price_native: float,
    created_at: datetime.datetime,
    current_price_native: Optional[float] = None,
    price_delta_native: Optional[float] = None,
    issue_pnl_jpy: Optional[float] = None,
) -> bytes:
    img = Image.new("RGB", (WIDTH, HEIGHT), BG_COLOR)
    draw = ImageDraw.Draw(img)

    draw.rectangle([(0, 0), (WIDTH, 6)], fill=ACCENT)

    font_lg = _load_font(64)
    font_md = _load_font(32)
    font_sm = _load_font(24)
    font_xs = _load_font(20)

    draw.text((60, 40), display_name or symbol, font=font_md, fill=TEXT_SECONDARY)
    draw.text((60, 80), symbol, font=font_sm, fill=TEXT_SECONDARY)

    cur_sym = "¥" if currency == "JPY" else "$"
    price = current_price_native if current_price_native is not None else issue_price_native
    draw.text((60, 130), f"{cur_sym}{price:,.2f}", font=font_lg, fill=TEXT_PRIMARY)

    if price_delta_native is not None:
        delta_color = GREEN if price_delta_native >= 0 else RED
        sign = "+" if price_delta_native >= 0 else ""
        draw.text((60, 220), f"発行時比  {sign}{cur_sym}{price_delta_native:,.2f}", font=font_md, fill=delta_color)

    if issue_pnl_jpy is not None:
        pnl_color = GREEN if issue_pnl_jpy >= 0 else RED
        draw.text((60, 280), f"含み損益  {_fmt_pnl(issue_pnl_jpy)}", font=font_md, fill=pnl_color)

    draw.text((60, 570), created_at.strftime("%Y-%m-%d %H:%M JST"), font=font_xs, fill=TEXT_SECONDARY)
    draw.rectangle([(0, HEIGHT - 6), (WIDTH, HEIGHT)], fill=ACCENT)

    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()
