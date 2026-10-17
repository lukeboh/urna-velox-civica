#!/usr/bin/env bash
# Coleta nacional pela API de resultados: todas as UFs (inclui zz), retomando o que já está nos .uvpk.
# Teto de 95 req/s (abaixo do limite combinado de 100 req/s), controle adaptativo a partir de 40 req/s,
# disjuntor: para tudo ao somar UV_MAX_429 respostas HTTP 429 (padrão 20).
# Uso: coleta_nacional.sh [rótulo]
#   Na retomada, use um rótulo novo e passe as métricas anteriores em UV_PULAR_SEM_BU para não pedir de
#   novo as seções sabidamente sem BU (milhares de 404 seguidos fazem a borda do CDN responder 429):
#   UV_PULAR_SEM_BU=~/uvreal/metricas_nacional.csv coleta_nacional.sh retomada
# Outro pleito (ex.: 1º turno de 2024, sem DF e sem exterior; AC já no acervo local):
#   UV_CICLO=ele2024 UV_PLEITO=452 UV_DADOS=~/uvreal/2024-1t/dados UV_SAIDA=~/uvreal/2024-1t \
#   UV_UFS=al,am,ap,ba,ce,es,go,ma,mg,ms,mt,pa,pb,pe,pi,pr,rj,rn,ro,rr,rs,sc,se,sp,to coleta_nacional.sh nacional
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
B="${UV_BIN:-$UV_WORK/target/release/uv-coletor}"
RPS="${UV_RPS:-95}"
ROT="${1:-nacional}"
DADOS="${UV_DADOS:-$UV_WORK/dados}"
SAIDA="${UV_SAIDA:-$UV_WORK}"
EXTRA=()
[ -n "${UV_PULAR_SEM_BU:-}" ] && EXTRA+=(--pular-sem-bu "$UV_PULAR_SEM_BU")
"$B" baixar --ciclo "${UV_CICLO:-ele2026}" --pleito "${UV_PLEITO:-3220}" --ufs "${UV_UFS:-todas}" --dados "$DADOS" \
  -c 128 --adaptativo --rps-inicial 40 --rps "$RPS" \
  --max-429 "${UV_MAX_429:-20}" "${EXTRA[@]}" \
  --metricas "$SAIDA/metricas_$ROT.csv" --resumo "$SAIDA/resumo_nacional.csv" --rotulo "$ROT"
