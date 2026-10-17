#!/usr/bin/env bash
# Simula a aquisição pelo site de resultados com uma curva de recebimento por minuto,
# para vários tetos de requisições/s. Saída: ~/uvreal/sim/sim_r<rps>.json e uma tabela no terminal.
set -euo pipefail
R="$(cd "$(dirname "$0")/../.." && pwd)"
M="${UV_MINUTOS:?CSV minuto;recebimento}"
S="$R/aquisicao/scripts/simular_aquisicao.py"
D="${UV_WORK:-$HOME/uvreal}/sim"
CS="${UV_INTERVALO_CS:-30}"
mkdir -p "$D"
echo "rps;cap_bus_min;backlog_max;p50_s;p90_s;p99_s;max_s"
for rps in ${UV_TETOS:-100 120 140 160 180 200 220 250 300}; do
  python3 "$S" --minutos "$M" --rps "$rps" --intervalo-cs "$CS" --saida "$D/sim_r$rps.json" > /dev/null
  python3 -c "import json,sys;r=json.load(open(sys.argv[1]));a=r['atraso_s'];print(';'.join(map(str,[$rps,r['parametros']['capacidade_bus_min'],r['backlog_max_bus'],a['p50'],a['p90'],a['p99'],a['max']])))" "$D/sim_r$rps.json"
done
