# こるちゃファンド

個人株式ポートフォリオを可視化する Web ダッシュボード。

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/k0rucha/korucha-fund)

- **フロントページ** — 保有銘柄・損益・時系列グラフを公開表示
- **管理画面** (`/admin`) — 取引履歴の追加・削除・CSV インポート/エクスポート（Basic Auth 保護）
- **シェアカード** — ポートフォリオや個別銘柄のスナップショットを OGP 付きで共有
- **テーマ切り替え** — ヘッダーから「モダン」と「Windows 95 風」レトロUIを切り替え（選択は `localStorage` に保存）
- **自動バッチ** — JST 23:00 に Yahoo Finance から終値を取得してスナップショットを更新

## スタック

| 層 | 技術 |
|---|---|
| サーバー | Rust / [Axum](https://github.com/tokio-rs/axum) |
| テンプレート | [Askama](https://github.com/djc/askama) |
| フロントエンド | [HTMX](https://htmx.org) + [Chart.js](https://www.chartjs.org) + [Tailwind CSS](https://tailwindcss.com)（`data-theme` + CSS 変数でテーマ切替） |
| DB | SQLite ([sqlx](https://github.com/launchbadge/sqlx)) |
| 株価データ | [yahoo_finance_api](https://crates.io/crates/yahoo_finance_api) |
| スケジューラ | [tokio-cron-scheduler](https://crates.io/crates/tokio-cron-scheduler) |

## セットアップ

### 必要なもの

- Rust 1.85+（`cargo`）
- SQLite（ランタイム依存なし。sqlx がビルド時に `.sqlx/` を参照）

### 手順

```bash
git clone https://github.com/<your-username>/korucha-fund
cd korucha-fund

# 環境変数を設定
cp .env.example .env
$EDITOR .env

# 起動（DB は自動生成・マイグレーション適用）
cargo run
```

ブラウザで `http://localhost:3000` を開く。

## 環境変数

`.env.example` を参照。

| 変数名 | 必須 | 説明 | デフォルト |
|---|---|---|---|
| `DATABASE_URL` | ○ | SQLite URL | `sqlite://./korucha-fund.db` |
| `ADMIN_USER` | ○ | Basic Auth ユーザー名 | — |
| `ADMIN_PASS` | ○ | Basic Auth パスワード | — |
| `PORT` | — | リッスンポート | `3000` |
| `SCHEDULER_CRON` | — | 終値取得バッチの cron 式（UTC） | `0 0 23 * * *` |

## コマンド

```bash
cargo run                    # 開発サーバー起動
cargo test                   # テスト実行
cargo build --release        # リリースビルド

# Linux aarch64 向けクロスコンパイル（cross が必要）
SQLX_OFFLINE=true cross build --release --target aarch64-unknown-linux-gnu

# スキーマ変更後に .sqlx/ を再生成（DB 起動中に実行）
cargo sqlx prepare
```

### サーバーコンソールコマンド

起動中のサーバーの stdin から実行できます。

| コマンド | 説明 |
|---|---|
| `backfill` | 最古の取引日から今日まで全スナップショットを再生成 |
| `help` | コマンド一覧を表示 |
| `quit` | サーバーを終了 |

## アーキテクチャ

```
Browser (HTMX + Chart.js + Tailwind CSS)
    ↓ HTML / JSON fragments
Axum (src/main.rs)
    ├── handlers/    HTTP ハンドラ
    ├── services/    ビジネスロジック・スケジューラ
    └── db/          SQLite クエリ (sqlx)
SQLite  ←→  Yahoo Finance API
```

- `transactions` テーブルが唯一の真実。保有・損益は毎回そこから導出する。
- `price_cache` / `fx_cache` は外部 API キャッシュ（消えても再取得可）。
- `snapshots` は時系列グラフ用の事前計算値（消えると過去履歴が失われる）。
- Yahoo Finance API は **1日15回・96分インターバル** でレート制限を管理。
