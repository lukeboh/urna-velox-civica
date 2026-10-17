#!/usr/bin/env bash
# Benchmark "noite da eleição": replay dos BUs no horário real de recebimento no TSE, totalização em fluxo
# e publicação contínua de Presidente/Governador; compara com a divulgação oficial.
# Uso: replay_noite.sh <pasta com .uvpk e catalogo> <aceleração> [pasta de saída]
#   aceleração: 60 = 1 h da noite em 1 min; 0 = sem espera (todos os BUs de uma vez, pior caso)
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
DADOS="${1:?pasta dos .uvpk}"
S="${2:-60}"
OUT="${3:-$UV_WORK/replay/x$S}"
CAT="${UV_CATALOGO:-$DADOS/../res/catalogo.txt}"
# horários de recebimento no TSE (dr/hr do aux.json) gravados pelas coletas
CH=$(ls "$UV_WORK"/metricas_*.csv "$AQ/results/metricas_df.csv" 2>/dev/null | grep -v -E 'inicios|taxa' | paste -sd,)
mkdir -p "$OUT"
# BUs reenviados nos dias seguintes (aux.json com dr/hr posterior) entram no fim da noite
CORTE="${UV_CORTE:-05/10/2026 03:00:00}"
# UV_CARGOS_PUB: cargos publicados a cada ciclo (padrão 1,3,5 = Presidente, Governador e Senador)
"$B" replay --dados "$DADOS" --catalogo "$CAT" --chegadas "$CH" --acelerar "$S" --saida "$OUT" --corte "$CORTE" \
  --cargos "${UV_CARGOS_PUB:-1,3,5}" | tee "$OUT/replay_resumo.json"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
"$PY" "$AQ/scripts/comparar_oficial.py" --totais "$OUT/totais_uf.csv" --oficial "$UV_WORK/oficial" --dados "$DADOS" --metricas "$CH" --cargos "${UV_CARGOS_PUB:-1,3,5}" --saida "$OUT/comparacao_oficial.md" > /dev/null || true
grep -E "Abrangências com divergência" "$OUT/comparacao_oficial.md"
