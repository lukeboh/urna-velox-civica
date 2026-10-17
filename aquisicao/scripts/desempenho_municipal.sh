#!/usr/bin/env bash
# Desempenho da totalização por cargo numa eleição municipal (BUs já em memória, mesma métrica do bench/):
# para cada conjunto de cargos, 1 e 28 threads, 5 repetições (mediana), e conferência por município com o
# oficial (comparar_municipal.py). Uso:
#   desempenho_municipal.sh <trabalho> <eleição:cargo,…> [conjuntos …]
#   ex.: desempenho_municipal.sh ~/uvreal/2024-1t 619:11,633:55,634:53,635:51,636:54,637:52 "11" "51,52,53,54,55" "todos"
# <trabalho> precisa de dados/*.uvpk, resultado/catalogo.txt, oficial/ e (opcional) metricas_aux.csv (seções anuladas).
set -uo pipefail
W="$1"; ELES="$2"; shift 2
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
D="$W/dados"; CAT="$W/resultado/catalogo.txt"
O="$W/desempenho-cargo"; mkdir -p "$O"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
ANUL=(); [ -s "$W/metricas_aux.csv" ] && ANUL=(--anuladas "$W/metricas_aux.csv")
CSV="$O/desempenho_por_cargo.csv"
echo "cargos,threads,bus,mediana_s,bus_por_s,us_por_bu_thread,estado_mb,conferencia" > "$CSV"
conjuntos=("$@"); [ ${#conjuntos[@]} -eq 0 ] && conjuntos=("11" "todos")
for cj in "${conjuntos[@]}"; do
  rot="${cj//,/-}"
  arg=(); [ "$cj" != "todos" ] && arg=(--cargos "$cj")
  "$B" totalizar --dados "$D" --catalogo "$CAT" --threads "1 28" --repeticoes 5 "${arg[@]}" "${ANUL[@]}" \
    --totais-municipio "$O/totais_municipio_$rot.csv" > "$O/medidas_$rot.csv" 2> "$O/log_$rot.txt"
  estado=$(grep -oE 'estado [0-9.]+ MB' "$O/log_$rot.txt" | head -1 | grep -oE '[0-9.]+')
  conf=$(grep -E '^conferência' "$O/log_$rot.txt" | sed 's/conferência: //')
  # confere as eleições cujos cargos estão no conjunto
  ee=$(echo "$ELES" | tr ',' '\n' | while IFS=: read -r e c; do
         if [ "$cj" = "todos" ] || echo ",$cj," | grep -q ",$c,"; then echo "$e:$c"; fi; done | paste -sd,)
  ofi="—"
  if [ -n "$ee" ]; then
    (cd "$AQ/scripts" && "$PY" comparar_municipal.py --totais "$O/totais_municipio_$rot.csv" --oficial "$W/oficial" --dados "$D" \
      --eleicoes "$ee" --saida "$O/comparacao_$rot.md" > /dev/null)
    ofi="$(grep -oE 'divergência ou sem arquivo oficial: [0-9]+' "$O/comparacao_$rot.md")"
  fi
  for t in 1 28; do
    "$PY" - "$O/medidas_$rot.csv" "$t" "$cj" "$estado" "$ofi" >> "$CSV" <<'EOF'
import csv, statistics, sys
arq, t, cj, estado, ofi = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
rs = [r for r in csv.reader(open(arq)) if len(r) > 6 and r[0] == "rust" and int(r[3]) == t]
s = statistics.median(float(r[5]) for r in rs); n = int(rs[0][4])
print(f'"{cj}",{t},{n},{s:.4f},{n / s:.0f},{s * t / n * 1e6:.2f},{estado},"{ofi}"')
EOF
  done
  echo "== cargos $cj: $conf | oficial: $ofi"
done
column -s, -t "$CSV"
