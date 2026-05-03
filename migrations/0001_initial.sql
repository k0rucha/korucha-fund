-- 取引履歴: 唯一の真実
CREATE TABLE transactions (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol          TEXT NOT NULL,
  txn_type        TEXT NOT NULL,
  quantity        REAL NOT NULL,
  price           REAL NOT NULL,
  currency        TEXT NOT NULL,
  fee             REAL NOT NULL DEFAULT 0,
  txn_date        DATE NOT NULL,
  fx_rate_to_jpy  REAL,
  notes           TEXT,
  created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_txn_symbol_date ON transactions(symbol, txn_date);

-- 銘柄メタ情報（yfinanceから取得・キャッシュ）
CREATE TABLE symbols (
  symbol          TEXT PRIMARY KEY,
  name            TEXT,
  currency        TEXT NOT NULL,
  exchange        TEXT,
  updated_at      DATETIME
);

-- 価格キャッシュ（日次終値）
CREATE TABLE price_cache (
  symbol          TEXT NOT NULL,
  date            DATE NOT NULL,
  close_price     REAL NOT NULL,
  PRIMARY KEY (symbol, date)
);

-- 為替レートキャッシュ
CREATE TABLE fx_cache (
  pair            TEXT NOT NULL,
  date            DATE NOT NULL,
  rate            REAL NOT NULL,
  PRIMARY KEY (pair, date)
);

-- 日次ポートフォリオ・スナップショット（時系列グラフ用）
CREATE TABLE snapshots (
  date                DATE PRIMARY KEY,
  total_value_jpy     REAL NOT NULL,
  total_cost_jpy      REAL NOT NULL,
  unrealized_pnl_jpy  REAL NOT NULL
);
