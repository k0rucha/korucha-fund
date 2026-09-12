import time
import threading
import asyncio


def generate_id() -> str:
    for _ in range(3):
        ns = time.time_ns()
        tid = threading.get_ident()
        raw = ns ^ (tid * 0x9E3779B97F4A7C15)
        return f"{raw & 0xFFFFFFFFFFFFFFFF:016x}"
    raise RuntimeError("ID generation failed after 3 attempts")
