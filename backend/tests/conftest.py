import pytest
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from app.database import Base
import app.models  # noqa: F401

TEST_DB_URL = "sqlite+aiosqlite:///:memory:"


@pytest.fixture(autouse=True)
async def isolated_db(monkeypatch):
    """各テストに独立したインメモリ DB を使わせる。"""
    import app.database as db_module

    engine = create_async_engine(TEST_DB_URL, echo=False)
    session_factory = async_sessionmaker(engine, expire_on_commit=False)

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    monkeypatch.setattr(db_module, "engine", engine)
    monkeypatch.setattr(db_module, "AsyncSessionLocal", session_factory)

    yield

    await engine.dispose()
