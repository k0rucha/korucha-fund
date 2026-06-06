-- 共有カード: 発行時点のポートフォリオスナップショット
CREATE TABLE share_cards (
  id                  TEXT PRIMARY KEY,
  created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  total_value_jpy     REAL NOT NULL,
  total_cost_jpy      REAL NOT NULL,
  unrealized_pnl_jpy  REAL NOT NULL,
  holdings_json       TEXT NOT NULL
);
CREATE INDEX idx_share_cards_created ON share_cards(created_at);
