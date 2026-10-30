#!/usr/bin/env bash
set -euo pipefail

###############################################################################
# Shirley Show - Verification Harness Script for Real Browser Responsive Testing
#
# 1. Spawns Shirley Show server with an ephemeral port and temporary data directory.
# 2. Seeds realistic mock data: presentation (lyrics + note + audio track),
#    event with ticket tiers, 2 paid tickets, 1 pending ticket, 1 lead, 1 expense,
#    and 1 event poster image.
# 3. Executes the Node 24 + CDP responsive verification suite (verify-responsive.mjs).
# 4. Cleans up background processes and temporary directories on exit (trap EXIT).
# 5. Propagates the exit status of the verification script.
###############################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Temporary directories
DATA_DIR=$(mktemp -d -t shirley-responsive-data-XXXXXX)
TMP_MEDIA_DIR=$(mktemp -d -t shirley-responsive-media-XXXXXX)
SCREENSHOTS_DIR="$REPO_ROOT/target/responsive-screenshots"
mkdir -p "$SCREENSHOTS_DIR"

SERVER_PID=""

cleanup() {
  local exit_code=$?
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Deteniendo servidor Shirley Show (PID: $SERVER_PID)..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$DATA_DIR" "$TMP_MEDIA_DIR" 2>/dev/null || true
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

echo "========================================================================"
echo "🚀 Iniciando arnés de verificación responsive con Chromium real"
echo "   Directorio de datos temporal: $DATA_DIR"
echo "   Directorio de capturas:       $SCREENSHOTS_DIR"
echo "========================================================================"

# Find an available ephemeral port
PORT=$(node -e '
  const s = require("net").createServer();
  s.listen(0, "127.0.0.1", () => {
    console.log(s.address().port);
    s.close();
  });
')

# El directorio de artefactos puede estar fuera del repo (cargo lo resuelve por
# workspace), así que se pregunta en lugar de asumir <repo>/target.
TARGET_DIR="$(cd "$REPO_ROOT" && cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' || true)"
[ -n "$TARGET_DIR" ] || TARGET_DIR="$REPO_ROOT/target"

SERVER_BIN="$TARGET_DIR/debug/shirley-show"
if [ ! -f "$SERVER_BIN" ]; then
  echo "Binario $SERVER_BIN no encontrado. Compilando..."
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
fi

# Launch server
SHIRLEY_BIND="127.0.0.1:$PORT" \
SHIRLEY_DATA_DIR="$DATA_DIR" \
SHIRLEY_FRONTEND_DIR="$REPO_ROOT/frontend" \
"$SERVER_BIN" > "$DATA_DIR/server.log" 2>&1 &
SERVER_PID=$!

echo "Servidor Shirley Show iniciado con PID $SERVER_PID en http://127.0.0.1:$PORT"

# Wait for health check
READY=0
for i in $(seq 1 40); do
  if curl -s "http://127.0.0.1:$PORT/api/presentations" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 0.1
done

if [ "$READY" -ne 1 ]; then
  echo "❌ Error: El servidor no respondió a tiempo."
  cat "$DATA_DIR/server.log"
  exit 1
fi

echo "Sembrando datos iniciales vía API..."

# 1. Presentation
PRES_JSON=$(curl -s -f -X POST "http://127.0.0.1:$PORT/api/presentations" \
  -H "Content-Type: application/json" \
  -d '{"title": "Gira Acústica 2026"}')
PRES_ID=$(echo "$PRES_JSON" | node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf-8")).presentation.id)')

# 2. Lyrics
curl -s -f -X POST "http://127.0.0.1:$PORT/api/presentations/$PRES_ID/items" \
  -H "Content-Type: application/json" \
  -d '{"type": "lyrics", "body": "Estrofa 1:\nBajo la noche en Barranco cantamos sin parar\n\nCoro:\nShirley Show en vivo"}' >/dev/null

# 3. Note
curl -s -f -X POST "http://127.0.0.1:$PORT/api/presentations/$PRES_ID/items" \
  -H "Content-Type: application/json" \
  -d '{"type": "note", "body": "Probar micrófono 2 y balance de monitores antes de iniciar."}' >/dev/null

# 4. Audio track (valid MP3 with ID3 magic bytes)
MP3_FILE="$TMP_MEDIA_DIR/sample.mp3"
printf '\x49\x44\x33\x03\x00\x00\x00\x00\x00\x00\xAA\xBB' > "$MP3_FILE"
curl -s -f -X POST "http://127.0.0.1:$PORT/api/presentations/$PRES_ID/tracks" \
  -F "title=Pista Principal - Intro" \
  -F "audio=@$MP3_FILE;type=audio/mpeg" >/dev/null

# 5. Event with ticket tiers
EVT_JSON=$(curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos" \
  -H "Content-Type: application/json" \
  -d '{
    "nombre": "Gran Noche de Gala - Barranco",
    "fecha_hora": "2026-11-20T20:00:00Z",
    "lugar": "Centro Cultural La Noche",
    "tipos_de_entrada": [
      { "nombre": "General", "precio": 35.0, "capacidad": 100 },
      { "nombre": "VIP", "precio": 60.0, "capacidad": 30 }
    ]
  }')
EVT_ID=$(echo "$EVT_JSON" | node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf-8")).evento.id)')

# 6. Two paid tickets
PAID_TIX_JSON=$(curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos/$EVT_ID/entradas" \
  -H "Content-Type: application/json" \
  -d '{
    "nombre_comprador": "Carlos Rodriguez",
    "contacto": "999111222",
    "tipo_entrada": "General",
    "cantidad": 2,
    "medio_pago": "Yape",
    "estado_pago": "Pagado"
  }')
TICKET_TOKEN=$(echo "$PAID_TIX_JSON" | node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf-8")).entradas[0].scan_token)')

# 7. One pending ticket
curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos/$EVT_ID/entradas" \
  -H "Content-Type: application/json" \
  -d '{
    "nombre_comprador": "Lucia Morales",
    "contacto": "988777666",
    "tipo_entrada": "VIP",
    "cantidad": 1,
    "medio_pago": "Efectivo",
    "estado_pago": "Pendiente"
  }' >/dev/null

# 8. One lead (interesado)
curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos/$EVT_ID/interesados" \
  -H "Content-Type: application/json" \
  -d '{
    "nombre": "Andres Mendoza",
    "contacto": "+51944333222",
    "canal": "Instagram",
    "promotora": "Shirley",
    "personas": 3
  }' >/dev/null

# 9. One expense (gasto)
curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos/$EVT_ID/gastos" \
  -H "Content-Type: application/json" \
  -d '{
    "categoria": "Sonido/Luces",
    "monto": 250.0,
    "descripcion": "Alquiler de consola y luces LED",
    "fecha": "2026-11-19"
  }' >/dev/null

# 10. One poster (afiche con magic bytes JPEG)
POSTER_FILE="$TMP_MEDIA_DIR/poster.jpg"
printf '\xFF\xD8\xFF\xE0\x00\x10\x4A\x46\x49\x46' > "$POSTER_FILE"
curl -s -f -X POST "http://127.0.0.1:$PORT/api/eventos/$EVT_ID/imagen" \
  -F "imagen=@$POSTER_FILE;type=image/jpeg" >/dev/null

echo "✅ Datos sembrados correctamente:"
echo "   Presentación ID: $PRES_ID"
echo "   Evento ID:       $EVT_ID"
echo "   Token Entrada:   $TICKET_TOKEN"

# Execute verify-responsive.mjs
node "$SCRIPT_DIR/verify-responsive.mjs" \
  --base-url="http://127.0.0.1:$PORT" \
  --presentation-id="$PRES_ID" \
  --event-id="$EVT_ID" \
  --ticket-token="$TICKET_TOKEN" \
  --screenshots-dir="$SCREENSHOTS_DIR" \
  "$@"
