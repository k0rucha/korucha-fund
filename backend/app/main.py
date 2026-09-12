import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles
from pathlib import Path

from app.config import settings
from app.database import init_db
from app.routers import transactions, portfolio, share_cards, ticker_share_cards, status
from app.services.scheduler import start_scheduler, stop_scheduler

logging.basicConfig(level=settings.log_level.upper())
logger = logging.getLogger(__name__)

STATIC_DIR = Path(__file__).parent.parent.parent / "static"
FRONTEND_DIST = Path(__file__).parent.parent.parent / "frontend" / "dist"


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    start_scheduler(settings.scheduler_cron, settings.database_url)
    logger.info("korucha-fund API started on port %s", settings.port)
    yield
    stop_scheduler()
    logger.info("korucha-fund API stopped")


app = FastAPI(title="korucha-fund API", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(transactions.router)
app.include_router(portfolio.router)
app.include_router(share_cards.router)
app.include_router(ticker_share_cards.router)
app.include_router(status.router)

if STATIC_DIR.exists():
    app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

# SPA fallback — serve React's index.html for all non-API routes
if FRONTEND_DIST.exists():
    app.mount("/assets", StaticFiles(directory=str(FRONTEND_DIST / "assets")), name="assets")

    @app.get("/{full_path:path}", include_in_schema=False)
    async def spa_fallback(request: Request, full_path: str):
        from fastapi.responses import FileResponse
        index = FRONTEND_DIST / "index.html"
        if index.exists():
            return FileResponse(str(index))
        return JSONResponse({"detail": "Frontend not built"}, status_code=404)
