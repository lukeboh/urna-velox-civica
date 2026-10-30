#!/usr/bin/env bash
# Backtest da previsão do "matematicamente" e do fim da apuração (etapa 8, docs/handoffs/HANDOFF_0.8.0.md) nas três noites
# que temos: 2026 1º turno (Presidente, Governador, Senador), 2024 1º e 2º turnos (Prefeito).
#
#   scripts/backtest_previsao.sh gabaritos   # 1ª passada: grava os gabaritos de cada noite (curvas de avanço)
#   scripts/backtest_previsao.sh backtest    # 2ª passada: cada noite prevista só com os gabaritos das outras duas
#   scripts/backtest_previsao.sh embutir     # junta os três gabaritos em motor/src/gabaritos.json (vai no binário)
#
# Variáveis: UV_DADOS (padrão ~/uvreal), UV_BIN (padrão ~/uvreal/target-dev/release/uv-motor), SAIDA (padrão
# ~/uvreal/ensaios/etapa8-backtest), THREADS (padrão 16), EXTRA (opções a mais do `uv-motor previsao`).
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
U="${UV_DADOS:-$HOME/uvreal}"
B="${UV_BIN:-$U/target-dev/release/uv-motor}"
S="${SAIDA:-$U/ensaios/etapa8-backtest}"
T="${THREADS:-16}"
EXTRA="${EXTRA:-}"
mkdir -p "$S/gabaritos"
M2026="$(ls "$U"/metricas_*.csv | grep -v -e inicios -e taxa | paste -sd,)"

noite() { # nome → argumentos de entrada
  case "$1" in
    2026-1t) echo "--dados $U/dados --candidatos $U/preparo_2026_1t/candidatos.json --chegadas $M2026 --anuladas $M2026 --vagas 5=2 --cargos 1,3,5 --classes br,uf,mun" ;;
    2024-1t) echo "--dados $U/2024-1t/dados --oficial $U/2024-1t/oficial --chegadas $U/2024-1t/metricas_aux.csv --anuladas $U/2024-1t/metricas_aux.csv --cargos 11 --classes br,uf,mun" ;;
    2024-2t) echo "--dados $U/2024-2t/dados --oficial $U/2024-2t/oficial --chegadas $U/2024-2t/metricas_2t.csv --anuladas $U/2024-2t/metricas_2t.csv --cargos 11 --classes mun" ;;
  esac
}
NOITES="${NOITES:-2026-1t 2024-1t 2024-2t}"
case "${1:-backtest}" in
  gabaritos)
    for n in $NOITES; do
      # shellcheck disable=SC2046
      "$B" previsao $(noite "$n") --nome "$n" --gerar-gabaritos "$S/gabaritos/$n.json" --sem-previsao --saida "$S/$n-gab"
    done ;;
  backtest)
    G="$(ls "$S"/gabaritos/*.json | paste -sd,)"
    R="$S/${ROTULO:-padrao}"
    mkdir -p "$R"
    for n in $NOITES; do
      # shellcheck disable=SC2046,SC2086
      "$B" previsao $(noite "$n") --nome "$n" --gabaritos "$G" --threads "$T" $EXTRA --saida "$R/$n" | tee "$R/$n.log"
    done ;;
  embutir)
    python3 - "$S"/gabaritos/*.json > "$RAIZ/motor/src/gabaritos.json" <<'PY'
import json, sys
g = {"grade": None, "curvas": []}
for p in sys.argv[1:]:
    x = json.load(open(p))
    g["grade"] = g["grade"] or x["grade"]
    g["curvas"] += x["curvas"]
json.dump(g, sys.stdout, separators=(",", ":"))
PY
    echo "gabaritos embutidos em motor/src/gabaritos.json" ;;
esac
