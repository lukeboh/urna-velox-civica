#!/usr/bin/env bash
# Gera os fixtures dos testes do motor a partir dos dados locais (~/uvreal no WSL), sem rede.
#   bu/       BUs avulsos (-bu.dat / -busa.dat) e o texto esperado do leitor (.txt)
#   2024-1t/  municípios inteiros do 1º turno de 2024: dados/<uf>.uvpk, oficial/*-u.json, metricas.csv
#   2024-2t/  idem, 2º turno de 2024
# Uso: motor/tests/fixtures/gerar.sh   (UV_BIN = uv-coletor; UV_MOTOR = uv-motor; padrão: ~/uvreal/target-dev)
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
M="${UV_MOTOR:-$UV_WORK/target-dev/release/uv-motor}"
F="$(cd "$(dirname "$0")" && pwd)"
rm -rf "$F/bu" "$F/2024-1t" "$F/2024-2t"
mkdir -p "$F/bu"

# 1. BUs avulsos: 2026 (bu e busa), 2024 com consulta popular, 2024 2º turno, seção anulada pela Junta (MA 07315/0023/0354)
"$B" filtrar --dados "$UV_WORK/dados" --secoes "df:97012/1/3,mg:50075/47/41" --saida "$F/bu" --soltos
mv "$F/bu/o03220mg5007500470041-bu.dat" "$F/bu/o03220mg5007500470041-busa.dat"
"$B" filtrar --dados "$UV_WORK/2024-1t/dados" --secoes "rr:3158/4/127,ma:7315/23/354" --saida "$F/bu" --soltos
"$B" filtrar --dados "$UV_WORK/2024-2t/dados" --secoes "ba:34134/170/8" --saida "$F/bu" --soltos
for b in "$F"/bu/*.dat; do "$M" inspecionar "$b" > "${b%.dat}.txt"; done

# 2. Municípios inteiros com os arquivos oficiais e as métricas da coleta
#   ac01031 pequeno; rr03158 e rs84824 com consulta popular (cargos 55 e 52); ma07315 com a seção anulada;
#   mg46191 empate no 1º lugar decidido pela idade; sp62081 sem eleito (art. 224); se31313 totalização não
#   finalizada (tf = n); rs85766 candidato anulado com votos e outro eleito
municipios() { # conjunto lista-uf:mun métricas
  local c="$1" l="$2" met="$3" W="$UV_WORK/$1"
  mkdir -p "$F/$c/oficial"
  "$B" filtrar --dados "$W/dados" --secoes "$l" --saida "$F/$c/dados"
  local re="" x
  for x in ${l//,/ }; do
    local uf="${x%%:*}" mu; mu=$(printf '%05d' "${x#*:}")
    cp "$W/oficial/$uf$mu"-c*-u.json "$F/$c/oficial/"
    re="$re|^[^,]*,$uf,${x#*:},"
  done
  { head -1 "$W/$met"; grep -E "${re#|}" "$W/$met"; } > "$F/$c/metricas.csv"
}
municipios 2024-1t "ac:1031,rr:3158,rs:84824,ma:7315,mg:46191,sp:62081,se:31313,rs:85766" metricas_aux.csv
municipios 2024-2t "ba:34134" metricas_2t.csv
du -sh "$F"
ls -la "$F/bu" "$F/2024-1t/dados" "$F/2024-2t/dados"
