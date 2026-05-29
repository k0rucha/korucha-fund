from httpx import AsyncClient, ASGITransport
from app.main import app
import pytest


@pytest.mark.asyncio
async def test_status_ok():
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        response = await client.get("/api/status")
    assert response.status_code == 200
    assert response.json()["status"] == "ok"
