from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import field_validator
import json


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8")

    database_url: str = "sqlite+aiosqlite:///./korucha-fund.db"
    admin_user: str = "admin"
    admin_pass: str = "changeme"
    port: int = 8000
    scheduler_cron: str = "0 23 * * *"
    cors_origins: list[str] = ["http://localhost:5173"]
    log_level: str = "info"

    @field_validator("cors_origins", mode="before")
    @classmethod
    def parse_cors(cls, v):
        if isinstance(v, str):
            return json.loads(v)
        return v


settings = Settings()
