#!/usr/bin/env bash
# Catálogo → totalização dos BUs → comparação com a divulgação oficial (Presidente e Governador).
# Uso: totalizar_comparar.sh <pasta dos .uvpk> <pasta de saída>
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
B="${UV_BIN:-$UV_WORK/target/release/uv-coletor}"
DADOS="${1:-$UV_WORK/dados}"
OUT="${2:-$UV_WORK/resultado}"
OFICIAL="${UV_OFICIAL:-$UV_WORK/oficial}"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
mkdir -p "$OUT"
"$B" catalogo --dados "$DADOS" --saida "$OUT/catalogo.txt" 2>&1 | tee "$OUT/catalogo.log"
"$B" totalizar --dados "$DADOS" --catalogo "$OUT/catalogo.txt" --threads "1 28" --repeticoes 3 \
  --totais "$OUT/totais_uf.csv" --resumo "$OUT/totalizacao.csv" 2> "$OUT/totalizacao.log" | tee -a "$OUT/totalizacao.log"
grep -E "BUs em memória|conferência" "$OUT/totalizacao.log"
MET=$(ls "$UV_WORK"/metricas_*.csv 2>/dev/null | grep -v -E 'inicios|taxa' | paste -sd,)
"$PY" "$AQ/scripts/comparar_oficial.py" --totais "$OUT/totais_uf.csv" --oficial "$OFICIAL" --dados "$DADOS" --metricas "$MET" --saida "$OUT/comparacao_oficial.md" > "$OUT/comparacao.log"
echo "comparação: código de saída $? (0 = sem divergências) → $OUT/comparacao_oficial.md"
grep -E "Abrangências com divergência" "$OUT/comparacao_oficial.md"
