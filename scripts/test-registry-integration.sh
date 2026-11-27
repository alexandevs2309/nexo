#!/usr/bin/env bash
set -euo pipefail
# Harness reproducible para registry_integration (Fase 09 D).
# Requiere Docker. Levanta PostgreSQL efímero, ejecuta tests ignorados y limpia.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTAINER="bsk-registry-test-$$"
PORT="${BSK_REGISTRY_PG_PORT:-55432}"
DB="basilisco_test"
USER="basilisco"
PASS="basilisco_test_pw"
export DATABASE_URL="postgres://${USER}:${PASS}@127.0.0.1:${PORT}/${DB}"
export BASILISCO_REGISTRY_ID="test-reg"

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "[registry-harness] starting ephemeral postgres on :$PORT"
docker run -d --rm --name "$CONTAINER" \
  -e POSTGRES_USER="$USER" -e POSTGRES_PASSWORD="$PASS" -e POSTGRES_DB="$DB" \
  -p "127.0.0.1:${PORT}:5432" postgres:16 >/dev/null

echo "[registry-harness] waiting for postgres..."
for i in $(seq 1 60); do
  if docker exec "$CONTAINER" pg_isready -U "$USER" -d "$DB" >/dev/null 2>&1; then break; fi
  sleep 1
  if [ "$i" -eq 60 ]; then echo "postgres no listo" >&2; exit 1; fi
done

echo "[registry-harness] running ignored registry integration tests"
cd "$ROOT/prototipo"
cargo test --test registry_integration -- --ignored --nocapture
echo "[registry-harness] OK"
