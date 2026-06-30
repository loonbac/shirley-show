# syntax=docker/dockerfile:1

# ---------- 1) Compila el frontend React con Vite ----------
FROM node:22-alpine AS web
RUN corepack enable
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY index.html vite.config.js ./
COPY public ./public
COPY src ./src
RUN pnpm build   # genera /app/dist (incluye los assets de public/, p.ej. logo.png)

# ---------- 2) Compila el backend Go (binario estático, sin CGO) ----------
FROM golang:1.25-alpine AS api
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY cmd ./cmd
COPY internal ./internal
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /out/server ./cmd/server \
 && CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /out/seed ./cmd/seed

# ---------- 3) Imagen final mínima ----------
FROM alpine:3.20
RUN apk add --no-cache ca-certificates tzdata && adduser -D -u 10001 app
WORKDIR /app
COPY --from=api /out/server /out/seed /usr/local/bin/
COPY --from=web /app/dist ./dist
RUN mkdir -p /app/data && chown -R app /app
USER app

ENV PORT=3001 \
    WEB_DIR=dist \
    DB_PATH=/app/data/eventos.db \
    WA_STORE_PATH=/app/data/whatsapp.db \
    MEDIA_DIR=/app/data/media

EXPOSE 3001
# Monta /app/data como volumen para conservar la base de datos entre despliegues.
VOLUME ["/app/data"]

# Health check: Coolify/Docker esperan a que /api/health responda antes de enviar tráfico.
# Usa ${PORT} (no un puerto fijo) porque Coolify puede inyectar otro puerto en runtime.
# wget viene incluido en Alpine (busybox).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -q -O /dev/null "http://127.0.0.1:${PORT}/api/health" || exit 1

CMD ["server"]
