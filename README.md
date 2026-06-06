# こるちゃファンド

個人投資ポートフォリオ・トラッカー。保有銘柄の評価額・損益、資産推移、構成比を表示し、
ポートフォリオ／個別銘柄の「戦績カード」を OGP 付きで共有できる。

Next.js (App Router) フルスタックアプリ。以前の Rust/Askama 実装は参照用に [`OLD/`](OLD/) に退避してある。

## スタック

| 領域 | 採用 |
|---|---|
| フレームワーク | Next.js 16 (App Router) / React 19 / TypeScript |
| DB | SQLite（[better-sqlite3](https://github.com/WiseLibs/better-sqlite3)）+ [Drizzle ORM](https://orm.drizzle.team/) |
| 株価 | [yahoo-finance2](https://github.com/gadicc/yahoo-finance2) v3 |
| 定期実行 | [node-cron](https://github.com/node-cron/node-cron)（`instrumentation.ts` で起動） |
| OGP 画像 | `next/og`（Satori）で 1200×630 PNG を生成 |
| グラフ | Chart.js / react-chartjs-2 |
| スタイル | Tailwind CSS v3 + `data-theme`（default / win95） |
| 認証 | middleware の Basic 認証（`/admin`） |

## セットアップ

```bash
npm install
cp .env.example .env      # 値を編集（特に ADMIN_PASS）
npm run dev               # http://localhost:3000
```

### 環境変数（`.env`）

| 変数 | 説明 |
|---|---|
| `DATABASE_URL` | SQLite パス（`sqlite:` / `sqlite://` 接頭辞対応）。dev は相対可。**standalone/本番は絶対パス**（例 `sqlite:/data/korucha-fund.db`）にすること — standalone はサーバ cwd が `.next/standalone` になり相対パスがずれるため |
| `ADMIN_USER` / `ADMIN_PASS` | `/admin` の Basic 認証。`#` 等を含む値は**必ずクォート**する |
| `PORT` | 待受ポート（既定 3000） |
| `SCHEDULER_CRON` | 日次バッチの cron（6フィールド `秒 分 時 日 月 曜`、JST） |
| `SITE_URL` | OGP の絶対 URL 生成に使う（既定 `https://fund.korucha.com`） |

## ビルドと本番起動

```bash
npm run build
node .next/standalone/server.js     # output: standalone
```

`next build` は `.next/standalone/`（自己完結 Node サーバ）を生成する。配信時は
`.next/static` と `public` を standalone 配下へコピーすること（Dockerfile 参照）。
スケジューラはサーバプロセス内で動くため、**単一常駐インスタンス**前提（旧・単一バイナリと同じ運用）。

### Docker

```bash
docker build -t korucha-fund .
docker run -p 3000:3000 \
  -e DATABASE_URL=sqlite:/data/korucha-fund.db \
  -e ADMIN_USER=admin -e ADMIN_PASS="change-me" \
  -e SCHEDULER_CRON="0 0 23 * * *" \
  -v "$PWD/data:/data" \
  korucha-fund
```

SQLite DB は実行時の状態なのでボリュームでマウントする。

## データモデル

`transactions`（唯一の真実）から保有・取得単価・損益を計算する。価格/為替は
`price_cache` / `fx_cache` にキャッシュし、日次スナップショットを `snapshots` に保存する。
スキーマは [`src/db/schema.ts`](src/db/schema.ts)（旧 `OLD/migrations/*.sql` を反映）。

外部 API（Yahoo Finance）は **1日15回・最低96分間隔**でレート制限する（`api_request_stats`）。
ダッシュボードの「価格更新」ボタンと日次スケジューラの両方がこの枠を消費する。

## 主な画面 / API

- `/` ダッシュボード（KPI・保有・構成・資産推移）
- `/status` API レート制限とスケジューラの状態
- `/admin` 取引の追加/削除・JSON 入出力（Basic 認証）
- `/share/:id` ポートフォリオ戦績カード（`/share/:id/opengraph-image` が OGP PNG）
- `/ticker/:id` 銘柄戦績カード（同上）
- `/api/{dashboard,composition,timeseries,status,refresh,share,ticker-share,admin/*}`

## ライセンス / 補足

`OLD/` は移行前の Rust 実装（参照用・退役）。
