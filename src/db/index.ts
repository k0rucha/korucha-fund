import Database from "better-sqlite3";
import { drizzle } from "drizzle-orm/better-sqlite3";
import * as schema from "./schema";

/**
 * Resolve DATABASE_URL to a plain fs path. Accepts both `sqlite://<path>` and
 * the sqlx-style `sqlite:<path>` (the existing .env uses the latter, e.g.
 * `sqlite:korucha-fund.db`).
 */
function resolveDbPath(): string {
  const url = process.env.DATABASE_URL ?? "sqlite:korucha-fund.db";
  return url.replace(/^sqlite:(\/\/)?/, "") || "korucha-fund.db";
}

// Reuse a single connection across HMR reloads in dev to avoid leaking handles.
const globalForDb = globalThis as unknown as {
  __sqlite?: Database.Database;
};

const sqlite = globalForDb.__sqlite ?? new Database(resolveDbPath());
sqlite.pragma("journal_mode = WAL");
sqlite.pragma("foreign_keys = ON");
if (process.env.NODE_ENV !== "production") {
  globalForDb.__sqlite = sqlite;
}

export const db = drizzle(sqlite, { schema });
export { schema };
// Raw better-sqlite3 handle — used for the few queries that are clearer as
// hand-written SQL (correlated subqueries, the atomic rate-limit UPDATE).
export { sqlite as sqliteClient };
