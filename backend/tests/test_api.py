"""Integration tests for the FastAPI endpoints."""
import base64
import pytest
from httpx import AsyncClient, ASGITransport
from app.main import app

BASE = "http://test"
AUTH = base64.b64encode(b"admin:changeme").decode()
HEADERS_ADMIN = {"Authorization": f"Basic {AUTH}"}


@pytest.fixture(autouse=True)
async def init_tables():
    """Ensure tables exist before each test."""
    from app.database import init_db
    await init_db()


@pytest.mark.asyncio
async def test_status():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/status")
    assert r.status_code == 200
    data = r.json()
    assert data["max_requests_per_day"] == 15


@pytest.mark.asyncio
async def test_dashboard_empty():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/portfolio/dashboard")
    assert r.status_code == 200
    assert r.json()["holdings"] == []


@pytest.mark.asyncio
async def test_composition_empty():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/portfolio/composition")
    assert r.status_code == 200
    assert r.json() == []


@pytest.mark.asyncio
async def test_timeseries_empty():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/portfolio/timeseries")
    assert r.status_code == 200
    assert r.json()["dates"] == []


@pytest.mark.asyncio
async def test_admin_auth_required():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/admin/transactions")
    assert r.status_code == 401


@pytest.mark.asyncio
async def test_transaction_crud():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        # create
        body = {
            "symbol": "7203.T",
            "txn_type": "BUY",
            "quantity": 100,
            "price": 2500,
            "currency": "JPY",
            "fee": 500,
            "txn_date": "2024-01-15",
        }
        r = await c.post("/api/admin/transactions", json=body, headers=HEADERS_ADMIN)
        assert r.status_code == 201
        txn = r.json()
        assert txn["symbol"] == "7203.T"
        txn_id = txn["id"]

        # list
        r = await c.get("/api/admin/transactions", headers=HEADERS_ADMIN)
        assert r.status_code == 200
        symbols = [t["symbol"] for t in r.json()]
        assert "7203.T" in symbols

        # delete
        r = await c.delete(f"/api/admin/transactions/{txn_id}", headers=HEADERS_ADMIN)
        assert r.status_code == 204


@pytest.mark.asyncio
async def test_share_card_create_and_get():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        # create a transaction first
        body = {
            "symbol": "7203.T",
            "txn_type": "BUY",
            "quantity": 10,
            "price": 2500,
            "currency": "JPY",
            "fee": 0,
            "txn_date": "2024-01-15",
        }
        await c.post("/api/admin/transactions", json=body, headers=HEADERS_ADMIN)

        # create share card
        r = await c.post("/api/share-cards", json={"span": "all"})
        assert r.status_code == 201
        card_id = r.json()["id"]
        assert card_id

        # get share card
        r = await c.get(f"/api/share-cards/{card_id}")
        assert r.status_code == 200
        assert r.json()["id"] == card_id


@pytest.mark.asyncio
async def test_share_card_ogp():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.post("/api/share-cards", json={"span": "all"})
        card_id = r.json()["id"]
        r = await c.get(f"/api/share-cards/{card_id}/ogp.png")
    assert r.status_code == 200
    assert r.headers["content-type"] == "image/png"
    assert len(r.content) > 1000


@pytest.mark.asyncio
async def test_404_share_card():
    async with AsyncClient(transport=ASGITransport(app=app), base_url=BASE) as c:
        r = await c.get("/api/share-cards/nonexistent")
    assert r.status_code == 404
