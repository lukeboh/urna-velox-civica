#!/usr/bin/env bash
# As três conferências da etapa 0 com um binário: catálogo → totalização → comparação com os arquivos oficiais.
#   2026, 1º turno (Presidente, Governador, Senador; 83 abrangências)
#   2024, 1º turno (Prefeito e consultas; 5.569 municípios)
#   2024, 2º turno (Prefeito; 51 municípios)
# Uso: conferir_tres.sh <pasta de saída>   (UV_BIN = binário; padrão ~/uvreal/target-dev/release/uv-coletor)
# Saída: uma linha por conjunto com o checksum da totalização (1 thread), a conferência do estado e o resultado
# da comparação; relatórios completos em <saída>/{2026,2024-1t,2024-2t}.
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
OUT="${1:?pasta de saída}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
mkdir -p "$OUT"
cs() { grep -E '^rust,real-dense,real,1,' "$1" | head -1 | cut -d, -f12; }

# 2026
O="$OUT/2026"; D="$UV_WORK/dados"; mkdir -p "$O"
"$B" catalogo --dados "$D" --saida "$O/catalogo.txt" > "$O/catalogo.log" 2>&1
"$B" totalizar --dados "$D" --catalogo "$O/catalogo.txt" --threads "1 28" --totais "$O/totais_uf.csv" > "$O/totalizacao.csv" 2> "$O/totalizacao.log"
MET=$(ls "$UV_WORK"/metricas_*.csv 2>/dev/null | grep -v -E 'inicios|taxa' | paste -sd,)
"$PY" "$AQ/scripts/comparar_oficial.py" --totais "$O/totais_uf.csv" --oficial "$UV_WORK/oficial" --dados "$D" --metricas "$MET" \
  --saida "$O/comparacao_oficial.md" > /dev/null; r=$?
echo "2026    checksum $(cs "$O/totalizacao.csv") | $(grep -h '^conferência' "$O/totalizacao.log" | head -1)"
echo "2026    código $r | $(grep -oE 'Abrangências com divergência: [0-9]+' "$O/comparacao_oficial.md") | $(grep -E 'candidatos com a mesma situação' "$O/comparacao_oficial.md" | tr '\n' ' ')"

# 2024, 1º e 2º turnos
for x in "2024-1t:619:11,633:55,634:53,635:51,636:54,637:52:metricas_aux.csv" "2024-2t:620:11:metricas_2t.csv"; do
  nome="${x%%:*}"; resto="${x#*:}"; met="${resto##*:}"; eles="${resto%:*}"
  W="$UV_WORK/$nome"; O="$OUT/$nome"; mkdir -p "$O"
  "$B" catalogo --dados "$W/dados" --saida "$O/catalogo.txt" --vagas "" > "$O/catalogo.log" 2>&1
  "$B" totalizar --dados "$W/dados" --catalogo "$O/catalogo.txt" --threads "1 28" --anuladas "$W/$met" \
    --totais "$O/totais_uf.csv" --totais-municipio "$O/totais_municipio.csv" > "$O/totalizacao.csv" 2> "$O/totalizacao.log"
  ( cd "$AQ/scripts" && "$PY" comparar_municipal.py --totais "$O/totais_municipio.csv" --oficial "$W/oficial" --dados "$W/dados" \
      --eleicoes "$eles" --metricas "$W/$met" --saida "$O/comparacao_municipal.md" > /dev/null ); r=$?
  echo "$nome checksum $(cs "$O/totalizacao.csv") | $(grep -h '^conferência' "$O/totalizacao.log" | head -1)"
  echo "$nome código $r | $(grep -oE 'divergência ou sem arquivo oficial: [0-9]+' "$O/comparacao_municipal.md")"
  sed -n '5,10p' "$O/comparacao_municipal.md" | grep '^|'
done
