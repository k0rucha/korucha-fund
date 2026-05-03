-- 銘柄共有カード: 発行時点の特定銘柄スナップショット
CREATE TABLE ticker_share_cards (
  id                  TEXT PRIMARY KEY,
  created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  symbol              TEXT NOT NULL,
  display_name        TEXT,
  currency            TEXT NOT NULL,                  -- 'JPY' | 'USD'
  -- At-issue ticker state
  issue_price_native  REAL NOT NULL,
  fx_rate_at_issue    REAL,                           -- USDJPY at issue (NULL for JPY symbols)
  -- Owner's position at issue (NULL when not held)
  quantity            REAL,
  avg_cost_native     REAL,
  issue_value_jpy     REAL,
  issue_pnl_jpy       REAL,
  -- View prefs
  default_span        TEXT NOT NULL DEFAULT '30d'
);
CREATE INDEX idx_ticker_share_cards_created ON ticker_share_cards(created_at);
CREATE INDEX idx_ticker_share_cards_symbol  ON ticker_share_cards(symbol);
