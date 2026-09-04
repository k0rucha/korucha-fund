# 開発環境構築手順書

Python (FastAPI) + React の開発環境を手元に再現する手順。  
動作確認済み環境: Python 3.11.15 / Node 22.x / uv 0.8.x

---

## 目次

1. [前提条件](#1-前提条件)
2. [リポジトリのクローン](#2-リポジトリのクローン)
3. [バックエンド環境構築 (FastAPI)](#3-バックエンド環境構築-fastapi)
4. [フロントエンド環境構築 (React)](#4-フロントエンド環境構築-react)
5. [開発サーバーの起動](#5-開発サーバーの起動)
6. [データベースマイグレーション](#6-データベースマイグレーション)
7. [テストの実行](#7-テストの実行)
8. [ディレクトリ構成の確認](#8-ディレクトリ構成の確認)
9. [よくあるエラーと対処法](#9-よくあるエラーと対処法)

---

## 1. 前提条件

| ツール | 最低バージョン | 確認コマンド |
|--------|-------------|-------------|
| Python | 3.11 以上 | `python3 --version` |
| uv | 0.4 以上 | `uv --version` |
| Node.js | 20 以上 | `node --version` |
| npm | 9 以上 | `npm --version` |
| Git | - | `git --version` |

### uv のインストール (未導入の場合)

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

インストール後、シェルを再起動するか以下を実行:

```bash
source ~/.local/share/uv/env
```

---

## 2. リポジトリのクローン

```bash
git clone <REPO_URL>
cd korucha-fund
```

---

## 3. バックエンド環境構築 (FastAPI)

### 3.1 仮想環境の作成と依存インストール

```bash
cd backend
uv sync
```

`uv sync` は `pyproject.toml` の dependencies を読み込み、`.venv/` に仮想環境を自動作成する。  
初回のみ数十秒かかる。

### 3.2 開発用依存 (pytest / httpx) も含める場合

```bash
uv sync --dev
```

### 3.3 環境変数の設定

```bash
cp .env.example .env
```

`.env` を開いて最低限以下を設定:

```dotenv
DATABASE_URL=sqlite+aiosqlite:///./korucha-fund.db
ADMIN_USER=admin
ADMIN_PASS=changeme      # 任意の強いパスワードに変更
PORT=8000
SCHEDULER_CRON=0 23 * * *
CORS_ORIGINS=["http://localhost:5173"]
LOG_LEVEL=info
```

> **注意**: `.env` は `.gitignore` に含めること。コミットしない。

### 3.4 Alembic (DB マイグレーションツール) の初期設定

`alembic.ini` の接続文字列は `sqlite:///./korucha-fund.db` (同期ドライバ) を使用する。  
`DATABASE_URL` の `aiosqlite` は実行時 (FastAPI) 用で、マイグレーション時は同期ドライバが必要。

初回のみ `alembic.ini` を確認:

```bash
grep "sqlalchemy.url" alembic.ini
# → sqlite:///./korucha-fund.db  となっていれば OK
```

### 3.5 依存パッケージ一覧

| パッケージ | バージョン | 用途 |
|-----------|-----------|------|
| `fastapi` | ≥ 0.136 | Web フレームワーク |
| `uvicorn[standard]` | ≥ 0.48 | ASGI サーバー (uvloop + watchfiles 含む) |
| `sqlalchemy` | ≥ 2.0 | ORM |
| `aiosqlite` | ≥ 0.22 | SQLite 非同期ドライバ |
| `alembic` | ≥ 1.18 | DB マイグレーション |
| `pydantic` | ≥ 2.13 | バリデーション |
| `pydantic-settings` | ≥ 2.14 | 環境変数管理 |
| `python-multipart` | ≥ 0.0.29 | ファイルアップロード (multipart/form-data) |

開発用:

| パッケージ | 用途 |
|-----------|------|
| `httpx` | テスト用 HTTP クライアント |
| `pytest` | テストフレームワーク |
| `pytest-asyncio` | 非同期テスト対応 |

---

## 4. フロントエンド環境構築 (React)

### 4.1 依存インストール

```bash
cd frontend
npm install
```

### 4.2 主要パッケージ

| パッケージ | 用途 |
|-----------|------|
| `react` + `react-dom` | UI ライブラリ |
| `react-router-dom` | SPA ルーティング |
| `chart.js` + `react-chartjs-2` | グラフ描画 |
| `tailwindcss` + `@tailwindcss/vite` | CSS フレームワーク |
| `vite` | ビルドツール / 開発サーバー |
| `typescript` | 型安全 |

### 4.3 vite.config.ts の構成ポイント

```ts
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8000',   // バックエンドへのプロキシ
    },
  },
})
```

`proxy` 設定により、フロントエンドから `/api/*` へのリクエストは自動的に  
`http://localhost:8000` に転送される。CORS の設定なしに開発できる。

### 4.4 Tailwind の有効化

`src/index.css` の先頭に必ず以下が必要:

```css
@import "tailwindcss";
```

> **注意**: v4 系の Tailwind は `@tailwind base/components/utilities` の代わりに  
> `@import "tailwindcss"` を使う。

---

## 5. 開発サーバーの起動

バックエンドとフロントエンドを別のターミナルで同時に起動する。

### ターミナル 1: バックエンド

```bash
cd backend
uv run uvicorn app.main:app --reload --port 8000
```

`--reload` フラグでファイル変更を検知して自動再起動する。

起動確認:

```bash
curl http://localhost:8000/api/status
# → {"status":"ok","message":"korucha-fund API is running"}
```

FastAPI の自動生成ドキュメント (開発中に便利):

- Swagger UI: http://localhost:8000/docs
- ReDoc: http://localhost:8000/redoc

### ターミナル 2: フロントエンド

```bash
cd frontend
npm run dev
```

ブラウザで http://localhost:5173 を開く。

---

## 6. データベースマイグレーション

### 初回: マイグレーション実行

```bash
cd backend
uv run alembic upgrade head
```

SQLite ファイル (`korucha-fund.db`) が `backend/` 直下に生成される。

### 新しいモデルを追加したとき

1. `app/models/` にモデルを追加
2. `alembic/env.py` の `target_metadata` にモデルを登録
3. 差分マイグレーションを生成:

```bash
uv run alembic revision --autogenerate -m "add_xxx_table"
```

4. 内容を確認して適用:

```bash
uv run alembic upgrade head
```

### ロールバック (1つ前に戻す)

```bash
uv run alembic downgrade -1
```

### マイグレーション履歴の確認

```bash
uv run alembic history --verbose
```

---

## 7. テストの実行

```bash
cd backend
uv run pytest tests/ -v
```

非同期テストは `pytest-asyncio` が必要。`pyproject.toml` に以下が設定済みであること:

```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
```

特定ファイルのみ実行:

```bash
uv run pytest tests/test_status.py -v
```

---

## 8. ディレクトリ構成の確認

セットアップ後の構成:

```
korucha-fund/
├── backend/
│   ├── .venv/                  ← uv が自動生成 (git 管理外)
│   ├── alembic/
│   │   ├── env.py
│   │   ├── script.py.mako
│   │   └── versions/           ← マイグレーションファイル
│   ├── app/
│   │   ├── __init__.py
│   │   ├── main.py             ← FastAPI アプリ本体
│   │   ├── config.py
│   │   ├── database.py
│   │   ├── auth.py
│   │   ├── models/
│   │   ├── schemas/
│   │   ├── routers/
│   │   └── services/
│   ├── tests/
│   │   └── test_status.py
│   ├── .env                    ← git 管理外
│   ├── .env.example
│   ├── alembic.ini
│   ├── pyproject.toml
│   └── uv.lock
├── frontend/
│   ├── node_modules/           ← npm が自動生成 (git 管理外)
│   ├── src/
│   │   ├── index.css           ← @import "tailwindcss" が必要
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── pages/
│   │   ├── components/
│   │   ├── hooks/
│   │   └── api/
│   ├── dist/                   ← ビルド成果物 (git 管理外)
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
├── docs/
│   ├── migration-python-react.md
│   └── dev-setup.md            ← このファイル
├── static/                     ← 既存の静的アセット (OGP フォント等)
└── migrations/                 ← 既存の Rust SQLx マイグレーション (参照用)
```

---

## 9. よくあるエラーと対処法

### `uv: command not found`

uv がインストールされていない、または PATH が通っていない。

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
source ~/.local/share/uv/env
```

---

### `ModuleNotFoundError: No module named 'app'`

`uvicorn` を `backend/` ディレクトリの外から実行している。  
必ず `cd backend` してから実行すること。

```bash
cd backend
uv run uvicorn app.main:app --reload
```

---

### Alembic で `Can't proceed with --autogenerate`

`alembic/env.py` に `target_metadata` が設定されていない。  
`env.py` の以下の部分にモデルの `Base.metadata` を渡す:

```python
# alembic/env.py
from app.models import Base   # 全モデルを import した Base
target_metadata = Base.metadata
```

---

### `aiosqlite` と Alembic の接続文字列の違い

| 用途 | 接続文字列 |
|------|-----------|
| FastAPI (実行時) | `sqlite+aiosqlite:///./korucha-fund.db` |
| Alembic (マイグレーション) | `sqlite:///./korucha-fund.db` |

`aiosqlite` は非同期専用のため Alembic の同期接続では使えない。  
`alembic.ini` には `sqlite:///` (同期ドライバ) を指定する。

---

### `CORS error` がブラウザコンソールに出る

`vite.config.ts` の `proxy` 設定を確認。開発時は `/api` プレフィックスを使って  
直接 `fetch('/api/status')` と書けばプロキシが転送する。

本番デプロイ時は FastAPI の `CORSMiddleware` で `allow_origins` を正しく設定する。

---

### `npm run dev` でポート競合エラー

5173 番ポートが既に使われている場合:

```bash
npm run dev -- --port 5174
```

バックエンドの `CORS_ORIGINS` も合わせて変更すること。

---

### `@import "tailwindcss"` が認識されない

`@tailwindcss/vite` プラグインが未インストール、または `vite.config.ts` に追加されていない。

```bash
npm install -D tailwindcss @tailwindcss/vite
```

`vite.config.ts` に `tailwindcss()` プラグインを追加:

```ts
import tailwindcss from '@tailwindcss/vite'
plugins: [react(), tailwindcss()]
```
