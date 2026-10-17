#!/usr/bin/env bash
# Passo 3 — varredura: vazão e latência da coleta (aux.json + bu.dat) por concorrência e protocolo,
# sempre com o teto de $UV_RPS_MAX req/s (nunca acima de 100).
# Cada medição sorteia seções diferentes (semente própria) para não medir o cache do CDN.
# Não grava pacotes: mede só a rede.
source "$(dirname "$0")/comum.sh"
rodar() { # rotulo semente n args...
  local rot=$1 sem=$2 n=$3; shift 3
  "$UC" baixar "${ALVO[@]}" --ufs "$UV_UFS_VARREDURA" --limite "$n" --semente "$sem" --sem-gravar \
     --dados "$UV_WORK/varredura" --rotulo "$rot" --resumo "$RESUMO" "$@" 2>&1 | grep -E "^== |latência"
}
sem=100
titulo "Concorrência (HTTP/2, teto fixo de $UV_RPS_MAX req/s)"
for c in $UV_CONC; do
  n=$(( c * AMOSTRA_MULT * 2 )); [ $n -lt 40 ] && n=40
  sem=$((sem + 1)); rodar "conc" $sem $n -c $c --rps "$UV_RPS_MAX"
done
titulo "HTTP/1.1 × HTTP/2 com concorrência 32"
sem=$((sem + 1)); rodar "http1" $sem $(( 32 * AMOSTRA_MULT * 2 )) -c 32 --http1 --rps "$UV_RPS_MAX"
titulo "Conexões HTTP/2 com concorrência 64"
sem=$((sem + 1)); rodar "conexoes" $sem $(( 64 * AMOSTRA_MULT * 2 )) -c 64 --conexoes 4 --rps "$UV_RPS_MAX"
