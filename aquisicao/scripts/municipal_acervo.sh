#!/usr/bin/env bash
# Eleição municipal a partir do acervo local de dados abertos (sem rede): importa os BUs dos zips originais
# (bu e busa) para .uvpk, monta o catálogo, totaliza (1 e 28 threads) e confere por município com os
# arquivos oficiais (Prefeito e consultas: comparar_municipal.py).
#
# Uso: municipal_acervo.sh <ano> <turno> <pleito> <pasta de trabalho> [eleição:cargo,…]
#   ex.: municipal_acervo.sh 2024 1 452 ~/uvreal/2024-1t 619:11,633:55,634:53,635:51,636:54,637:52
# A pasta de trabalho precisa de dados/config/<uf>-p<pleito>-cs.json (uv-coletor secoes --cache, ou uma
# coleta), de oficial/ (baixar_oficial_municipal.py) e, para as seções anuladas, de metricas_aux.csv
# (uv-coletor baixar --so-aux). Saída em <trabalho>/resultado.
set -uo pipefail
ANO="$1"; TURNO="$2"; PLEITO="$3"; W="$4"; ELES="${5:-619:11}"
UV_WORK="${UV_WORK:-$HOME/uvreal}"
B="${UV_BIN:-$UV_WORK/target-dev/release/uv-coletor}"
ACERVO="${UV_ACERVO:-/mnt/d/dados/tse-arquivos-urna}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
D="$W/dados"; O="$W/resultado"
mkdir -p "$D" "$O"
for z in "$ACERVO/originais/$ANO/${TURNO}t"/bu_imgbu_logjez_rdv_vscmr_${ANO}_${TURNO}t_*.zip; do
  uf=$(basename "$z" .zip); uf=${uf##*_}; uf=${uf,,}
  if [ -s "$D/$uf.uvpk" ]; then echo "$uf: já importado"; continue; fi
  "$B" zip --ciclo "ele$ANO" --pleito "$PLEITO" --uf "$uf" --local "$z" --dados "$D" \
    --resumo "$O/importacao_zip.csv" --rotulo "acervo-$ANO-${TURNO}t" 2>&1 | grep -E 'extraídos|pacotes'
done
"$B" catalogo --dados "$D" --saida "$O/catalogo.txt" --vagas "" 2>&1 | tee "$O/catalogo.log"
# seções anuladas (aux.json "Anulada", BU "Excluído"): das métricas de `baixar --so-aux`, se houver
ANUL=(); [ -s "$W/metricas_aux.csv" ] && ANUL=(--anuladas "$W/metricas_aux.csv")
"$B" totalizar --dados "$D" --catalogo "$O/catalogo.txt" --threads "1 28" --repeticoes 3 "${ANUL[@]}" \
  --totais "$O/totais_uf.csv" --totais-municipio "$O/totais_municipio.csv" --resumo "$O/totalizacao.csv" \
  2> "$O/totalizacao.log" | tee -a "$O/totalizacao.log"
grep -E "BUs em memória|conferência" "$O/totalizacao.log"
cd "$AQ/scripts" && "$PY" comparar_municipal.py --totais "$O/totais_municipio.csv" --oficial "$W/oficial" --dados "$D" \
  --eleicoes "$ELES" ${ANUL:+--metricas "$W/metricas_aux.csv"} --saida "$O/comparacao_municipal.md" > "$O/comparacao.log"
echo "comparação: código de saída $? (0 = sem divergências) → $O/comparacao_municipal.md"
sed -n '1,12p' "$O/comparacao_municipal.md"
