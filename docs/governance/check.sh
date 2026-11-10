#!/usr/bin/env bash
# Consulta de gobernanza BASILISCO (SOLO LECTURA). No valida compilador, no aprueba fases.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE="$ROOT/DEVELOPMENT_STATE.md"
ROAD="$ROOT/ROADMAP_MASTER.md"
cmd="${1:-status}"
case "$cmd" in
  status)
    echo "== status (fuente: $STATE) =="; grep -n "^## Estado global\|Fase autorizada\|Estado de la fase\|Fases aprobadas\|Autorización humana" "$STATE" | head -12 || true
    echo; echo "== resumen =="; grep "^| 0\|^| 1" "$STATE" | head -16 || true
    ;;
  current)
    grep "Fase autorizada (current)" "$STATE" | head -1 || true
    ;;
  validate|report)
    echo "Acción '$cmd': seguir docs/governance/VALIDATION.md contra $ROAD para la fase 'current'. (Este script no ejecuta pruebas ni aprueba.)"
    ;;
  *)
    echo "uso: $0 {status|current|validate|report}  (solo lectura; no aprueba)"; exit 2;;
esac
