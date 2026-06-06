-- Track external API (yfinance) request rate limiting
-- 1 day = 1440 minutes. Max 15 requests/day = 96 minute minimum interval.
CREATE TABLE IF NOT EXISTS api_request_stats (
    id INTEGER PRIMARY KEY,
    last_request_time DATETIME,
    request_count INTEGER DEFAULT 0,
    reset_date DATE DEFAULT CURRENT_DATE,
    UNIQUE(reset_date)
);

-- Initialize stats for today
INSERT OR IGNORE INTO api_request_stats (reset_date, last_request_time, request_count)
VALUES (CURRENT_DATE, NULL, 0);
