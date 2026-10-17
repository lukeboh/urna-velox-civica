#!/usr/bin/env bash
# Copia para aquisicao/results os resultados (pequenos) da coleta e da totalização nacionais.
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
R="$AQ/results"
S="$AQ/scripts"
cp "$UV_WORK/resultado/comparacao_oficial.md" "$R/comparacao_oficial_nacional.md"
cp "$UV_WORK/resultado/totais_uf.csv" "$R/totais_uf_nacional.csv"
cp "$UV_WORK/resumo_nacional.csv" "$R/coleta_nacional_resumo.csv"
python3 "$S/erros_coleta.py" "$UV_WORK/metricas_nacional.csv" > "$R/coleta_nacional_erros.txt"
python3 "$S/chegadas_resumo.py" "$UV_WORK"/metricas_nacional.csv "$UV_WORK"/metricas_multi.csv "$UV_WORK"/metricas_retomada.csv "$R/metricas_df.csv" > "$R/chegadas_por_hora.txt"
ls -la "$R"/comparacao_oficial_nacional.md "$R"/totais_uf_nacional.csv "$R"/coleta_nacional_*.* "$R"/chegadas_por_hora.txt
cut -c1-150 "$R/coleta_nacional_resumo.csv"
