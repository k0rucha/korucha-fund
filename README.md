# こるちゃファンド

個人株式ポートフォリオを可視化する Web ダッシュボード。

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/k0rucha/korucha-fund)

- **フロントページ** — 保有銘柄・損益・時系列グラフを公開表示
- **管理画面** (`/admin`) — 取引履歴の追加・削除・JSON インポート/エクスポート（Basic Auth 保護）
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

- Rust 1.94+（`cargo`、SQLx 0.9 の最低要件）
- Node.js / npm（CSSクラスを変更して `static/app.css` を再生成する場合のみ）
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

## systemdで自動起動（ユーザーサービス）

releaseバイナリをビルドし、DB・秘密設定・静的資産を`target`の外へ配置します。

```bash
cargo build --release
install -d -m 700 ~/.config/korucha-fund ~/.local/share/korucha-fund/static ~/.config/systemd/user
install -m 600 .env ~/.config/korucha-fund/korucha-fund.env
cp -a static/. ~/.local/share/korucha-fund/static/
install -m 644 deploy/korucha-fund.service ~/.config/systemd/user/korucha-fund.service

# 既存DBがリポジトリ直下にある場合は、初回起動前にコピー
if [ -f korucha-fund.db ] && [ ! -e ~/.local/share/korucha-fund/korucha-fund.db ]; then
  install -m 600 korucha-fund.db ~/.local/share/korucha-fund/korucha-fund.db
fi

systemctl --user daemon-reload
systemctl --user enable --now korucha-fund.service
loginctl enable-linger "$USER"
```

状態とログは次のコマンドで確認できます。

```bash
systemctl --user status korucha-fund.service
journalctl --user -u korucha-fund.service -f
```

更新時は`cargo build --release`と静的資産のコピー後に`systemctl --user restart korucha-fund.service`を実行します。

## 環境変数

`.env.example` を参照。

| 変数名 | 必須 | 説明 | デフォルト |
|---|---|---|---|
| `DATABASE_URL` | ○ | SQLite URL | `sqlite://./korucha-fund.db` |
| `ADMIN_USER` | ○ | Basic Auth ユーザー名 | — |
| `ADMIN_PASS` | ○ | Basic Auth パスワード | — |
| `PORT` | — | リッスンポート | `3000` |
| `PUBLIC_BASE_URL` | — | OGP URL・書き込み元検証に使う公開URL（パスなし） | `https://fund.korucha.com` |
| `SCHEDULER_CRON` | — | 終値取得バッチの6フィールド cron 式（JST） | `0 0 23 * * *` |

## コマンド

```bash
cargo run                    # 開発サーバー起動
cargo test                   # テスト実行
cargo build --release        # リリースビルド
npm ci && npm run css        # Tailwind CSS を再生成

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
    ↓ HTML / JSON
handlers/              Axum・Askama・HTTP 入出力
    ↓
services/              korucha-fund のユースケース
    ├──→ domain/       取引・保有・損益の純粋な計算
    ├──→ db/ ──→ domain/   SQLite (SQLx) と行型変換
    └──→ clients/      Yahoo Finance
```

- `domain` は Axum・SQLx・Yahoo Finance に依存しない。
- `services` は具体的な `db` / `clients` を直接使う。このアプリに実装は一つしかないため、repository trait や DI コンテナは置かない。
- `handlers` は HTTP の抽出・レスポンス・表示用フォーマットを担当し、複数のデータ取得や更新を伴う処理は `services` に委譲する。
- `main.rs` は設定、DB 接続、バックグラウンド処理、HTTP サーバーを組み立てる。

- `transactions` テーブルが唯一の真実。保有・損益は毎回そこから導出する。
- `price_cache` / `fx_cache` は外部 API キャッシュ（消えても再取得可）。
- `snapshots` は時系列グラフ用の事前計算値（消えると過去履歴が失われる）。
- Yahoo Finance API は **1日15回・96分インターバル** でレート制限を管理。
