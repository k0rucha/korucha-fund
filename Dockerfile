# syntax=docker/dockerfile:1
# Multi-stage build for the Next.js standalone server.
# Debian base so better-sqlite3's native binding builds/loads against glibc.
FROM node:22-bookworm-slim AS base

FROM base AS deps
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci

FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm run build

FROM base AS runner
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0
# Standalone output (server.js + traced node_modules + assets/fonts).
COPY --from=builder /app/public ./public
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static

# The SQLite DB is runtime state — mount it and point DATABASE_URL at it, e.g.
#   docker run -p 3000:3000 \
#     -e DATABASE_URL=sqlite:/data/korucha-fund.db \
#     -e ADMIN_USER=... -e ADMIN_PASS="..." \
#     -e SCHEDULER_CRON="0 0 23 * * *" \
#     -v $PWD/data:/data  korucha-fund
EXPOSE 3000
CMD ["node", "server.js"]
