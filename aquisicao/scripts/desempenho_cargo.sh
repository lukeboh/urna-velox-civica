#!/usr/bin/env bash
# Desempenho da totalização por cargo, com os BUs do Brasil já em memória (mesma métrica do bench/).
# Para cada conjunto de cargos: 1 e 28 threads, 5 repetições (mediana), e conferência dos totais com o oficial.
# Uso: desempenho_cargo.sh [cargos ...]   (padrão: "5" "1" "3" "1,3,5" "todos")
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
D="$UV_WORK/dados"; CAT="$UV_WORK/resultado/catalogo.txt"
O="$UV_WORK/desempenho-cargo"; mkdir -p "$O"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
MET=$(ls "$UV_WORK"/metricas_*.csv 2>/dev/null | grep -v -E 'inicios|taxa' | paste -sd,)
CSV="$O/desempenho_por_cargo.csv"
echo "cargos,threads,bus,mediana_s,bus_por_s,us_por_bu_thread,estado_mb,conferencia" > "$CSV"
conjuntos=("$@"); [ ${#conjuntos[@]} -eq 0 ] && conjuntos=("5" "1" "3" "1,3,5" "todos")
for cj in "${conjuntos[@]}"; do
  rot="${cj//,/-}"
  arg=(); [ "$cj" != "todos" ] && arg=(--cargos "$cj")
  "$B" totalizar --dados "$D" --catalogo "$CAT" --threads "1 28" --repeticoes 5 "${arg[@]}" \
    --totais "$O/totais_$rot.csv" > "$O/medidas_$rot.csv" 2> "$O/log_$rot.txt"
  estado=$(grep -oE 'estado [0-9.]+ MB' "$O/log_$rot.txt" | head -1 | grep -oE '[0-9.]+')
  conf=$(grep -E '^conferência' "$O/log_$rot.txt" | sed 's/conferência: //')
  # conferência com a divulgação oficial dos cargos majoritários presentes
  cc="$cj"; [ "$cj" = "todos" ] && cc="1,3,5"
  cc=$(echo "$cc" | tr ',' '\n' | grep -E '^(1|3|5)$' | paste -sd,)
  ofi="—"
  if [ -n "$cc" ]; then
    "$PY" "$AQ/scripts/comparar_oficial.py" --totais "$O/totais_$rot.csv" --oficial "$UV_WORK/oficial" --dados "$D" \
      --metricas "$MET" --cargos "$cc" --saida "$O/comparacao_$rot.md" > /dev/null
    ofi="$(grep -oE 'Abrangências com divergência: [0-9]+' "$O/comparacao_$rot.md") ($(grep -cE '✅' "$O/comparacao_$rot.md") idênticas)"
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
