import { sql } from "drizzle-orm";
import {
  sqliteTable,
  text,
  real,
  integer,
  primaryKey,
} from "drizzle-orm/sqlite-core";

// Mirrors OLD/migrations/*.sql exactly so we can open the existing DB as-is.
// DATE / DATETIME columns are stored as TEXT (ISO strings), matching how the
// previous sqlx/SQLite app persisted them.

// 取引履歴: 唯一の真実
export const transactions = sqliteTable("transactions", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  symbol: text("symbol").notNull(),
  txnType: text("txn_type").notNull(),
  quantity: real("quantity").notNull(),
  price: real("price").notNull(),
  currency: text("currency").notNull(),
  fee: real("fee").notNull().default(0),
  txnDate: text("txn_date").notNull(),
  fxRateToJpy: real("fx_rate_to_jpy"),
  notes: text("notes"),
  createdAt: text("created_at").default(sql`CURRENT_TIMESTAMP`),
});

// 銘柄メタ情報（yfinance から取得・キャッシュ）
export const symbols = sqliteTable("symbols", {
  symbol: text("symbol").primaryKey(),
  name: text("name"),
  currency: text("currency").notNull(),
  exchange: text("exchange"),
  updatedAt: text("updated_at"),
});

// 価格キャッシュ（日次終値）
export const priceCache = sqliteTable(
  "price_cache",
  {
    symbol: text("symbol").notNull(),
    date: text("date").notNull(),
    closePrice: real("close_price").notNull(),
  },
  (t) => [primaryKey({ columns: [t.symbol, t.date] })],
);

// 為替レートキャッシュ
export const fxCache = sqliteTable(
  "fx_cache",
  {
    pair: text("pair").notNull(),
    date: text("date").notNull(),
    rate: real("rate").notNull(),
  },
  (t) => [primaryKey({ columns: [t.pair, t.date] })],
);

// 日次ポートフォリオ・スナップショット（時系列グラフ用）
export const snapshots = sqliteTable("snapshots", {
  date: text("date").primaryKey(),
  totalValueJpy: real("total_value_jpy").notNull(),
  totalCostJpy: real("total_cost_jpy").notNull(),
  unrealizedPnlJpy: real("unrealized_pnl_jpy").notNull(),
});

// 共有カード: 発行時点のポートフォリオスナップショット
export const shareCards = sqliteTable("share_cards", {
  id: text("id").primaryKey(),
  createdAt: text("created_at").notNull().default(sql`CURRENT_TIMESTAMP`),
  totalValueJpy: real("total_value_jpy").notNull(),
  totalCostJpy: real("total_cost_jpy").notNull(),
  unrealizedPnlJpy: real("unrealized_pnl_jpy").notNull(),
  holdingsJson: text("holdings_json").notNull(),
  defaultSpan: text("default_span").notNull().default("all"),
});

// 外部 API（yfinance）レート制限の統計
export const apiRequestStats = sqliteTable("api_request_stats", {
  id: integer("id").primaryKey(),
  lastRequestTime: text("last_request_time"),
  requestCount: integer("request_count").default(0),
  resetDate: text("reset_date").default(sql`CURRENT_DATE`),
});

// 銘柄共有カード: 発行時点の特定銘柄スナップショット
export const tickerShareCards = sqliteTable("ticker_share_cards", {
  id: text("id").primaryKey(),
  createdAt: text("created_at").notNull().default(sql`CURRENT_TIMESTAMP`),
  symbol: text("symbol").notNull(),
  displayName: text("display_name"),
  currency: text("currency").notNull(),
  issuePriceNative: real("issue_price_native").notNull(),
  fxRateAtIssue: real("fx_rate_at_issue"),
  quantity: real("quantity"),
  avgCostNative: real("avg_cost_native"),
  issueValueJpy: real("issue_value_jpy"),
  issuePnlJpy: real("issue_pnl_jpy"),
  defaultSpan: text("default_span").notNull().default("30d"),
});
