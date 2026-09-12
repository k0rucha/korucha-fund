# korucha-fund: Python + React 移行設計ドキュメント

現行の Rust / Askama SSR 構成を **FastAPI + React (SPA)** へ移行するための設計仕様書。  
コードは含まない。エンドポイント、CRUD、スキーマ、ビジネスロジックの要件を定義する。

---

## 目次

1. [アーキテクチャ概要](#1-アーキテクチャ概要)
2. [ディレクトリ構成 (推奨)](#2-ディレクトリ構成-推奨)
3. [環境変数 / 設定](#3-環境変数--設定)
4. [データベース設計 (SQLAlchemy モデル)](#4-データベース設計-sqlalchemy-モデル)
5. [Pydantic スキーマ](#5-pydantic-スキーマ)
6. [FastAPI エンドポイント一覧](#6-fastapi-エンドポイント一覧)
7. [ビジネスロジック仕様](#7-ビジネスロジック仕様)
8. [認証](#8-認証)
9. [バックグラウンドタスク / スケジューラ](#9-バックグラウンドタスク--スケジューラ)
10. [React フロントエンド構成](#10-react-フロントエンド構成)
11. [マイグレーション戦略](#11-マイグレーション戦略)

---

## 1. アーキテクチャ概要

```
[React SPA]  <──fetch/JSON──>  [FastAPI]  <──SQLAlchemy──>  [SQLite]
                                   │
                                   └── [Yahoo Finance API]
                                   └── [APScheduler (日次バッチ)]
```

| 項目 | 現行 (Rust) | 移行後 (Python) |
|------|-------------|-----------------|
| Web フレームワーク | Axum 0.7 | FastAPI |
| テンプレート | Askama (SSR) | React (SPA) |
| DB ドライバ | SQLx (SQLite) | SQLAlchemy 2.x + aiosqlite |
| バリデーション | Rust 型システム | Pydantic v2 |
| スケジューラ | tokio-cron-scheduler | APScheduler 3.x |
| 外部 API | yahoo_finance_api crate | yfinance または requests |
| OGP 画像 | resvg (SVG→PNG) | Pillow または cairosvg |
| 認証 | HTTP Basic Auth | HTTP Basic Auth (FastAPI) |

---

## 2. ディレクトリ構成 (推奨)

```
korucha-fund/
├── backend/
│   ├── app/
│   │   ├── main.py               # FastAPI アプリ初期化、ルーター登録
│   │   ├── config.py             # Settings (pydantic-settings)
│   │   ├── database.py           # SQLAlchemy engine / session
│   │   ├── auth.py               # Basic Auth 依存関数
│   │   ├── models/               # SQLAlchemy ORM モデル
│   │   │   ├── transaction.py
│   │   │   ├── symbol.py
│   │   │   ├── price_cache.py
│   │   │   ├── fx_cache.py
│   │   │   ├── snapshot.py
│   │   │   ├── share_card.py
│   │   │   ├── ticker_share_card.py
│   │   │   └── api_stats.py
│   │   ├── schemas/              # Pydantic スキーマ
│   │   │   ├── transaction.py
│   │   │   ├── portfolio.py
│   │   │   ├── share_card.py
│   │   │   └── ticker_share_card.py
│   │   ├── routers/              # FastAPI ルーター (1ファイル = 1リソース)
│   │   │   ├── transactions.py
│   │   │   ├── portfolio.py
│   │   │   ├── share_cards.py
│   │   │   ├── ticker_share_cards.py
│   │   │   └── status.py
│   │   └── services/             # ビジネスロジック
│   │       ├── portfolio.py      # ホールディング・P&L 計算
│   │       ├── yfinance.py       # Yahoo Finance API ラッパー
│   │       ├── scheduler.py      # APScheduler 設定
│   │       ├── ogp.py            # OGP 画像生成
│   │       └── id_gen.py         # カード ID 生成
│   ├── alembic/                  # DB マイグレーション
│   ├── static/                   # 静的ファイル (フォント・ロゴ)
│   ├── requirements.txt
│   └── .env.example
└── frontend/
    ├── src/
    │   ├── pages/
    │   ├── components/
    │   ├── hooks/
    │   └── api/                  # fetch ラッパー
    ├── package.json
    └── vite.config.ts
```

---

## 3. 環境変数 / 設定

`pydantic-settings` の `BaseSettings` を使い `.env` から読み込む。

| 変数名 | 型 | デフォルト | 説明 |
|--------|----|-----------|------|
| `DATABASE_URL` | str | `sqlite+aiosqlite:///./korucha-fund.db` | SQLAlchemy 接続文字列 |
| `ADMIN_USER` | str | — | Basic Auth ユーザー名 (必須) |
| `ADMIN_PASS` | str | — | Basic Auth パスワード (必須) |
| `PORT` | int | `8000` | uvicorn ポート番号 |
| `SCHEDULER_CRON` | str | `"0 23 * * *"` | 日次バッチ cron (5フィールド形式) |
| `CORS_ORIGINS` | list[str] | `["http://localhost:5173"]` | 許可する React 開発サーバー origin |
| `RUST_LOG` → `LOG_LEVEL` | str | `"info"` | ログレベル |

---

## 4. データベース設計 (SQLAlchemy モデル)

SQLite ファイルは変更なし。Alembic でマイグレーション管理。

### 4.1 `transactions` テーブル

| カラム | SQLAlchemy 型 | 制約 | 説明 |
|--------|--------------|------|------|
| `id` | `Integer` | PK, autoincrement | |
| `symbol` | `String` | NOT NULL | 例: `"7203.T"`, `"AAPL"` |
| `txn_type` | `Enum("BUY","SELL")` | NOT NULL | 取引種別 |
| `quantity` | `Float` | NOT NULL | 数量 |
| `price` | `Float` | NOT NULL | 1株あたり価格 (現地通貨) |
| `currency` | `Enum("JPY","USD")` | NOT NULL | |
| `fee` | `Float` | NOT NULL, default=0 | 手数料 |
| `txn_date` | `Date` | NOT NULL | 約定日 (JST) |
| `fx_rate_to_jpy` | `Float` | nullable | 取引時 USD→JPY レート |
| `notes` | `String` | nullable | |
| `created_at` | `DateTime` | default=now (JST) | |

インデックス: `(symbol, txn_date)`

### 4.2 `symbols` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `symbol` | `String` | PK | |
| `name` | `String` | nullable | 会社名 |
| `currency` | `Enum("JPY","USD")` | NOT NULL | |
| `exchange` | `String` | nullable | |
| `updated_at` | `DateTime` | nullable | |

### 4.3 `price_cache` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `symbol` | `String` | PK (複合) | |
| `date` | `Date` | PK (複合) | JST 日付 |
| `close_price` | `Float` | NOT NULL | 終値 |

### 4.4 `fx_cache` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `pair` | `String` | PK (複合) | 例: `"USDJPY"` |
| `date` | `Date` | PK (複合) | JST 日付 |
| `rate` | `Float` | NOT NULL | USD→JPY レート |

### 4.5 `snapshots` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `date` | `Date` | PK | JST 日付 |
| `total_value_jpy` | `Float` | NOT NULL | 評価額合計 (JPY) |
| `total_cost_jpy` | `Float` | NOT NULL | 取得原価合計 (JPY) |
| `unrealized_pnl_jpy` | `Float` | NOT NULL | 含み損益 (JPY) |

### 4.6 `share_cards` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `id` | `String` | PK | hex ID (ナノ秒+スレッドID) |
| `created_at` | `DateTime` | NOT NULL | 発行日時 |
| `total_value_jpy` | `Float` | NOT NULL | 発行時評価額 |
| `total_cost_jpy` | `Float` | NOT NULL | 発行時原価 |
| `unrealized_pnl_jpy` | `Float` | NOT NULL | 発行時含み損益 |
| `holdings_json` | `Text` | NOT NULL | JSON (CardHolding 配列) |
| `default_span` | `String` | NOT NULL, default=`"all"` | `"all"` / `"7d"` / `"30d"` |

インデックス: `created_at`

### 4.7 `ticker_share_cards` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `id` | `String` | PK | |
| `created_at` | `DateTime` | NOT NULL | |
| `symbol` | `String` | NOT NULL | |
| `display_name` | `String` | nullable | |
| `currency` | `Enum("JPY","USD")` | NOT NULL | |
| `issue_price_native` | `Float` | NOT NULL | 発行時価格 (現地通貨) |
| `fx_rate_at_issue` | `Float` | nullable | 発行時 USDJPY (USD のみ) |
| `quantity` | `Float` | nullable | 保有数量 |
| `avg_cost_native` | `Float` | nullable | 平均取得単価 (現地通貨) |
| `issue_value_jpy` | `Float` | nullable | 発行時ポジション価値 (JPY) |
| `issue_pnl_jpy` | `Float` | nullable | 発行時含み損益 (JPY) |
| `default_span` | `String` | NOT NULL, default=`"30d"` | |

インデックス: `created_at`, `symbol`

### 4.8 `api_request_stats` テーブル

| カラム | 型 | 制約 | 説明 |
|--------|----|------|------|
| `id` | `Integer` | PK | |
| `last_request_time` | `DateTime` | nullable | 最終リクエスト時刻 (JST) |
| `request_count` | `Integer` | default=0 | 当日リクエスト数 |
| `reset_date` | `Date` | UNIQUE, default=today | リセット基準日 |

定数:
- `MAX_REQUESTS_PER_DAY = 15`
- `MIN_INTERVAL_MINUTES = 96` (1440 / 15)

---

## 5. Pydantic スキーマ

### 5.1 Transaction スキーマ

#### `TransactionCreate` (POST リクエスト)
```
symbol          : str         (例: "7203.T")
txn_type        : Literal["BUY", "SELL"]
quantity        : float       (> 0)
price           : float       (> 0)
currency        : Literal["JPY", "USD"]
fee             : float       (>= 0, default=0)
txn_date        : date        (JST)
fx_rate_to_jpy  : float|None
notes           : str|None
```

#### `TransactionRead` (レスポンス)
```
id              : int
symbol          : str
txn_type        : str
quantity        : float
price           : float
currency        : str
fee             : float
txn_date        : date
fx_rate_to_jpy  : float|None
notes           : str|None
created_at      : datetime
symbol_name     : str|None    (JOIN で付与)
```

#### `TransactionImport` (インポート用)
`TransactionCreate` と同じフィールド。バッチ受け付けは `list[TransactionImport]`。

### 5.2 Portfolio スキーマ

#### `HoldingView` (ダッシュボード用保有銘柄行)
```
symbol          : str
name            : str|None
quantity        : float
avg_cost_native : float       (現地通貨)
currency        : str
current_price   : float|None
current_value_jpy : float|None
cost_jpy        : float
unrealized_pnl_jpy : float|None
pnl_pct         : float|None
dod_delta_jpy   : float|None  (前日比)
mom_delta_jpy   : float|None  (前月比)
```

#### `DashboardResponse`
```
holdings            : list[HoldingView]
total_cost_jpy      : float
total_value_jpy     : float|None
total_unrealized_pnl_jpy : float|None
total_pnl_pct       : float|None
realized_pnl_jpy    : float
cumulative_pnl_jpy  : float|None
dod_delta_jpy       : float|None
mom_delta_jpy       : float|None
last_updated        : datetime|None   (最終価格更新時刻)
```

#### `CompositionItem` (円グラフ用)
```
symbol    : str
label     : str           (表示名、長すぎる場合は短縮)
value_jpy : float
```

#### `TimeseriesResponse` (折れ線グラフ用)
```
dates   : list[str]   (ISO 形式 "YYYY-MM-DD")
values  : list[float]
costs   : list[float]
pnls    : list[float]
```

### 5.3 Share Card スキーマ

#### `ShareCardCreate` (POST リクエスト)
```
span : Literal["all", "7d", "30d"]  (default="all")
```

#### `ShareCardRead` (レスポンス - 詳細表示用)
```
id                  : str
created_at          : datetime
total_value_jpy     : float
total_cost_jpy      : float
unrealized_pnl_jpy  : float
default_span        : str
holdings            : list[CardHolding]
current_value_jpy   : float|None      (再計算)
value_delta_jpy     : float|None      (発行時との差)
history_dates       : list[str]
history_values      : list[float]
history_pnls        : list[float]
```

#### `CardHolding` (holdings_json に格納・返却)
```
symbol    : str
name      : str|None
quantity  : float
value_jpy : float     (発行時価値)
```

#### `ShareCardCreated` (POST レスポンス)
```
id  : str
url : str   (例: "/share/{id}")
```

### 5.4 Ticker Share Card スキーマ

#### `TickerShareCardCreate` (POST リクエスト)
```
symbol : str
span   : Literal["all", "7d", "30d"]  (default="30d")
```

#### `TickerShareCardRead` (レスポンス - 詳細表示用)
```
id                   : str
created_at           : datetime
symbol               : str
display_name         : str|None
currency             : str
issue_price_native   : float
fx_rate_at_issue     : float|None
quantity             : float|None
avg_cost_native      : float|None
issue_value_jpy      : float|None
issue_pnl_jpy        : float|None
default_span         : str
current_price_native : float|None    (再計算)
price_delta_native   : float|None    (発行時との差)
history_dates        : list[str]
history_prices       : list[float]
```

#### `TickerShareCardCreated` (POST レスポンス)
```
id  : str
url : str   (例: "/ticker/{id}")
```

### 5.5 Status スキーマ

#### `StatusResponse`
```
request_count          : int
max_requests_per_day   : int
remaining_requests     : int
last_request_time      : datetime|None
next_scheduler_run     : datetime|None
```

### 5.6 Refresh スキーマ

#### `RefreshResponse`
```
ok                      : bool
updated_from_api        : bool
remaining_api_requests  : int
message                 : str|None
```

---

## 6. FastAPI エンドポイント一覧

### 6.1 ポートフォリオ / ダッシュボード

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| `GET` | `/api/portfolio/dashboard` | 不要 | `DashboardResponse` を返す。現在価格・P&L 計算済み。 |
| `GET` | `/api/portfolio/composition` | 不要 | `list[CompositionItem]` を返す。円グラフ用。 |
| `GET` | `/api/portfolio/timeseries` | 不要 | `TimeseriesResponse` を返す。スナップショット履歴全件。 |
| `POST` | `/api/portfolio/refresh` | 不要 | Yahoo Finance から最新価格を取得、スナップショット生成。`RefreshResponse` を返す。 |

### 6.2 取引 (Admin 保護)

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| `GET` | `/api/admin/transactions` | Basic Auth | `list[TransactionRead]` を返す (symbol_name 付き)。クエリパラメータ: `symbol`, `limit`, `offset`。 |
| `POST` | `/api/admin/transactions` | Basic Auth | 取引を 1 件追加。バックグラウンドで価格キャッシュ取得を起動。`TransactionRead` を返す。 |
| `DELETE` | `/api/admin/transactions/{id}` | Basic Auth | 取引を削除。`204 No Content` を返す。 |
| `GET` | `/api/admin/transactions/export` | Basic Auth | 全取引を JSON ファイルとしてダウンロード (`application/json`, `Content-Disposition: attachment`)。 |
| `POST` | `/api/admin/transactions/import` | Basic Auth | JSON ファイル (multipart/form-data, field=`file`) をインポート。重複はフィンガープリントで除外。`{"imported": N, "skipped": M}` を返す。 |

### 6.3 シェアカード (ポートフォリオスナップショット)

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| `POST` | `/api/share-cards` | 不要 | スナップショットカードを発行。`ShareCardCreated` を返す。 |
| `GET` | `/api/share-cards/{id}` | 不要 | カード詳細 + 現在価値差分 + チャート履歴。`ShareCardRead` を返す。 |
| `GET` | `/api/share-cards/{id}/ogp.png` | 不要 | OGP 画像 (1200×630 PNG)。`Cache-Control: public, max-age=31536000, immutable`。 |

### 6.4 ティッカーシェアカード

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| `POST` | `/api/ticker-share-cards` | 不要 | ティッカースナップショットカードを発行。価格履歴が少ない場合はバックグラウンドで 35 日分を取得。`TickerShareCardCreated` を返す。 |
| `GET` | `/api/ticker-share-cards/{id}` | 不要 | カード詳細 + 価格差分 + チャート履歴。`TickerShareCardRead` を返す。 |
| `GET` | `/api/ticker-share-cards/{id}/ogp.png` | 不要 | OGP 画像 (1200×630 PNG)。 |

### 6.5 ステータス

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| `GET` | `/api/status` | 不要 | `StatusResponse` を返す。 |

### 6.6 静的ファイル

| パス | 説明 |
|------|------|
| `GET /static/{path}` | FastAPI `StaticFiles` マウント (フォント・ロゴ等) |
| `GET /` および SPA ルート | React の `index.html` を返す (全フォールバック) |

### エラーレスポンス形式

全エラーは以下の統一形式:
```json
{
  "detail": "エラーメッセージ"
}
```

HTTP ステータスコード:
- `400` Bad Request (バリデーション失敗)
- `401` Unauthorized (Basic Auth 失敗)
- `404` Not Found
- `429` Too Many Requests (API レート制限超過)
- `500` Internal Server Error

---

## 7. ビジネスロジック仕様

### 7.1 ホールディング計算 (`services/portfolio.py`)

**入力**: `list[Transaction]` (txn_date 昇順、同一日は id 昇順でソート)

**アルゴリズム**:
```
symbol ごとに以下を追跡:
  quantity   : 保有数量
  total_cost : 取得原価合計 (JPY)

BUY の場合:
  cost_jpy = price * quantity * fx_rate  (USD の場合は fx_rate を使用、JPY は 1.0)
  cost_jpy += fee * fx_rate             (手数料も原価に加算)
  total_cost += cost_jpy
  quantity += transaction.quantity

SELL の場合:
  sell_ratio = transaction.quantity / quantity (売却前数量)
  total_cost -= total_cost * sell_ratio        (比例配分で原価を減算)
  quantity -= transaction.quantity

返却: quantity > 0 の symbol のみ
```

**平均取得単価** (表示用):
```
avg_cost_native = total_cost_jpy / quantity / current_fx_rate  (USD の場合)
avg_cost_native = total_cost_jpy / quantity                    (JPY の場合)
```

### 7.2 実現損益計算

**入力**: `list[Transaction]` (昇順ソート済み)

```
symbol ごとに cost_basis_jpy を追跡 (BUY/SELL で更新)

SELL の場合:
  sell_cost_jpy = cost_basis_jpy * (transaction.quantity / 売却前数量)
  proceeds_jpy  = price * quantity * fx_rate - fee * fx_rate
  realized_pnl += proceeds_jpy - sell_cost_jpy
  cost_basis_jpy -= sell_cost_jpy
```

### 7.3 スナップショット生成

**処理フロー**:
1. 全ホールディングを `calculate_holdings_as_of(date)` で算出
2. 各保有銘柄について、`date` 以前で最新の `close_price` を取得
3. USD 保有がある場合、`date` 以前で最新の `USDJPY` レートを取得
   - レートが存在しない場合はそのスナップショット日をスキップ (データ汚染防止)
4. `total_value_jpy = Σ (quantity × close_price × fx_rate)`
5. `total_cost_jpy` は `calculate_holdings_as_of` から取得
6. UPSERT で保存 (冪等)

### 7.4 日次・月次デルタ

```
dod_delta_jpy: 今日の total_value_jpy - 1日前のスナップショット
mom_delta_jpy: 今日の total_value_jpy - 30日前のスナップショット

表示条件:
  - dod: 参照スナップショットが 7 日以内に存在すること
  - mom: 参照スナップショットが 60 日以内に存在すること
```

### 7.5 API レート制限管理

```
定数:
  MAX_REQUESTS_PER_DAY = 15
  MIN_INTERVAL_MINUTES = 96

リクエスト前チェック (アトミックに DB で実行):
  1. reset_date が今日でなければ request_count = 0 にリセット
  2. request_count >= MAX_REQUESTS_PER_DAY → 429 エラー
  3. last_request_time から MIN_INTERVAL_MINUTES 以内 → 429 エラー
  4. 問題なければ request_count += 1、last_request_time = now() を更新

スケジューラやバックフィルはこのチェックをバイパスしてよい
(外部から呼ばれる /api/portfolio/refresh エンドポイントのみ適用)
```

### 7.6 取引インポート重複排除

フィンガープリント = `f"{symbol}|{txn_type}|{txn_date}|{quantity}|{price}|{fee}"`

同一フィンガープリントが既に存在する行はスキップ。

### 7.7 カード ID 生成

```
id = hex(nanoseconds_since_epoch XOR thread_id_hash)
衝突した場合は 1ms スリープして最大 3 回リトライ
```

Python 実装のヒント: `time.time_ns()` + `threading.get_ident()` の XOR でほぼ同等の効果。

### 7.8 OGP 画像生成 (`services/ogp.py`)

- サイズ: 1200 × 630 px
- 形式: PNG
- 内容: ポートフォリオ全体または単一ティッカーの主要指標 + 簡易チャート
- フォント: `NotoSansJP-Regular.ttf` (日本語テキスト対応必須)
- 推奨ライブラリ: `Pillow` (PIL) または SVG テンプレート + `cairosvg`
- キャッシュ: `Cache-Control: public, max-age=31536000, immutable`

---

## 8. 認証

### Basic Auth (HTTP)

- 対象ルート: `/api/admin/*` のみ
- FastAPI の `HTTPBasic` と `HTTPBasicCredentials` を使用
- タイミングアタック対策: `secrets.compare_digest()` で比較
- 失敗時: `401 Unauthorized` + `WWW-Authenticate: Basic realm="korucha-fund admin"` ヘッダー

```
依存関数: verify_admin(credentials: HTTPBasicCredentials = Depends(security)) -> bool
→ 全 admin ルーターに Depends() で注入
```

セッション・JWT・Cookie は使用しない (現行踏襲)。

---

## 9. バックグラウンドタスク / スケジューラ

### 9.1 FastAPI BackgroundTasks (リクエスト駆動)

以下の処理をレスポンス返却後に非同期実行:

| トリガー | タスク内容 |
|---------|------------|
| `POST /api/admin/transactions` | 追加された symbol の最新価格・シンボル名をキャッシュ |
| `POST /api/admin/transactions/import` | インポートで追加された全新 symbol の価格・シンボル名キャッシュ |
| `POST /api/ticker-share-cards` | 価格履歴が少ない場合、35 日分のバックフィル |

### 9.2 APScheduler (日次バッチ)

- ライブラリ: `APScheduler` (`AsyncIOScheduler`)
- トリガー: cron (`SCHEDULER_CRON` 環境変数、デフォルト: 毎日 23:00 JST)
- タスク:
  1. 保有中全銘柄の最新終値を取得 → `price_cache` に UPSERT
  2. USD 保有がある場合 `USDJPY` レートを取得 → `fx_cache` に UPSERT
  3. 本日分のスナップショットを生成 → `snapshots` に UPSERT
- レート制限: スケジューラタスクは `/api/portfolio/refresh` とは別扱い。デイリーカウントを共有しつつも、スケジューラ自身は制限チェックをバイパスする

### 9.3 バックフィルコマンド (CLI / 管理用)

`python -m app.services.scheduler backfill` のように起動:
1. 最古の取引日から今日まで全日程で価格履歴を取得
2. 全スナップショットを再計算 (日次でループ、冪等)

---

## 10. React フロントエンド構成

### 10.1 ページ一覧

| パス | コンポーネント | 説明 |
|------|--------------|------|
| `/` | `DashboardPage` | KPI タイル、円グラフ、折れ線グラフ、保有一覧テーブル |
| `/admin` | `AdminPage` | 取引一覧、追加フォーム、インポート/エクスポート (Basic Auth は `Authorization` ヘッダーで) |
| `/share/:id` | `ShareCardPage` | ポートフォリオスナップショットカード (OGP メタタグ付き) |
| `/ticker/:id` | `TickerShareCardPage` | ティッカースナップショットカード (OGP メタタグ付き) |
| `/status` | `StatusPage` | API 使用状況 |
| `*` | `NotFoundPage` | 404 |

### 10.2 主要コンポーネント

```
components/
├── KpiTiles.tsx          ← 評価額・損益・dod/mom デルタ
├── CompositionChart.tsx  ← Chart.js ドーナツグラフ
├── TimeseriesChart.tsx   ← Chart.js 折れ線グラフ
├── HoldingsTable.tsx     ← 保有銘柄テーブル (ソート対応)
├── TransactionForm.tsx   ← 取引追加フォーム
├── TransactionTable.tsx  ← 取引一覧テーブル
├── ShareCardPreview.tsx  ← スナップショットカード表示
└── RefreshButton.tsx     ← 手動更新ボタン
```

### 10.3 API クライアント

- `src/api/portfolio.ts` — `/api/portfolio/*` の fetch ラッパー
- `src/api/transactions.ts` — `/api/admin/transactions/*` の fetch ラッパー (Basic Auth ヘッダー付与)
- `src/api/shareCards.ts` — `/api/share-cards/*` および `/api/ticker-share-cards/*`

### 10.4 チャートスパンフィルタリング

スパン (`all` / `7d` / `30d`) はフロントエンドでフィルタリング。
バックエンドは全データを返す (現行踏襲)。

### 10.5 OGP メタタグ

`/share/:id` および `/ticker/:id` ではサーバーサイドレンダリングが必要なため、
以下のいずれかを選択:
- **SSR (推奨)**: Next.js の `getServerSideProps` で OGP タグを動的生成
- **代替案**: FastAPI で `/share/{id}` のみ HTML を返す (SPA に混在)

---

## 11. マイグレーション戦略

### 11.1 DB 移行

既存 SQLite ファイルはそのまま使用可能 (スキーマ変更なし)。  
Alembic で初期状態として既存スキーマを `revision --autogenerate` で取り込む。

### 11.2 移行推奨順序

```
Phase 1: バックエンド基盤
  ├── SQLAlchemy モデル定義 + Alembic マイグレーション
  ├── Pydantic スキーマ定義
  ├── 取引 CRUD (/api/admin/transactions)
  └── ポートフォリオ計算サービス (portfolio.py)

Phase 2: コアデータ API
  ├── /api/portfolio/dashboard
  ├── /api/portfolio/composition
  ├── /api/portfolio/timeseries
  └── /api/portfolio/refresh

Phase 3: スナップショットカード
  ├── /api/share-cards
  ├── /api/ticker-share-cards
  └── OGP 画像生成

Phase 4: スケジューラ + バックフィル
  ├── APScheduler 設定
  └── backfill コマンド

Phase 5: React フロントエンド
  ├── DashboardPage (グラフ・テーブル)
  ├── AdminPage
  └── Share / TickerShare ページ
```

### 11.3 主要な依存パッケージ (参考)

**バックエンド**:
```
fastapi
uvicorn[standard]
sqlalchemy[asyncio]
aiosqlite
alembic
pydantic
pydantic-settings
python-multipart    ← ファイルアップロード
yfinance            ← Yahoo Finance (または requests + yahoofinancials)
apscheduler
Pillow              ← OGP 画像生成
python-jose         ← 将来の JWT 対応 (任意)
```

**フロントエンド**:
```
react
react-router-dom
chart.js + react-chartjs-2
axios または標準 fetch
tailwindcss
vite
typescript
```
