#!/usr/bin/env bash
# Passo 6 — monta o catálogo e totaliza os BUs reais já coletados (em memória), variando threads.
# Mesma métrica do benchmark sintético (bench/): tempo de parede com os BUs já em memória.
source "$(dirname "$0")/comum.sh"
NP=$(nproc)
TH="$( (printf '%s\n' 1 2 4 8 16 | awk -v n="$NP" '$1<n'; echo "$NP") | sort -n | uniq | tr '\n' ' ')"
T="$UV_OUT/totalizacao.csv"; rm -f "$T"
titulo "Catálogo (municípios e seções dos cs.json; candidatos dos próprios BUs)"
"$UC" catalogo --dados "$DADOS" 2>&1 | tee "$UV_OUT/catalogo.txt"
titulo "Totalização da UF $UV_UF_REF (threads: $TH)"
"$UC" totalizar --dados "$DADOS" --ufs "$UV_UF_REF" --threads "$TH" --repeticoes 5 --resumo "$T" --rotulo "$UV_UF_REF" \
   --totais "$UV_OUT/totais_$UV_UF_REF.csv" 2>&1 | grep -vE "^rust,"
if ls "$DADOS"/*.uvpk 2>/dev/null | grep -qv "/$UV_UF_REF.uvpk"; then
  titulo "Totalização de todos os pacotes (threads: $TH)"
  "$UC" totalizar --dados "$DADOS" --threads "$TH" --repeticoes 3 --resumo "$T" --rotulo "todas" \
     --totais "$UV_WORK/totais_todas.csv" 2>&1 | grep -vE "^rust," | tee "$UV_OUT/totalizacao_todas.txt"
fi
