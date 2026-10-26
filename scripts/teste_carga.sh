#!/usr/bin/env bash
# teste_carga.sh — Roda o teste de carga e estresse do Urna Velox
# Uso:
#   ./scripts/teste_carga.sh [--local] [--degraus "1000,3000,5000,10000"]
#   ./scripts/teste_carga.sh --producao [--url https://urnavelox.com.br]
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
ulimit -n 65536 2>/dev/null || true

python3 "$RAIZ/scripts/teste_carga.py" "$@"
