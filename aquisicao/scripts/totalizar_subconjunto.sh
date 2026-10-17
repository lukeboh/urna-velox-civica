#!/usr/bin/env bash
# Totaliza e compara com a divulgação oficial só algumas UFs, a partir de uma CÓPIA dos .uvpk
# (útil com a coleta ainda em andamento: não lê pacotes que estão sendo gravados).
# Uso: totalizar_subconjunto.sh "ac,al,df" [pasta de trabalho]
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
UFS="${1:?informe as UFs, ex.: ac,al,df}"
T="${2:-$UV_WORK/subconjunto}"
rm -rf "$T"; mkdir -p "$T/dados/config"
for u in ${UFS//,/ }; do cp "$UV_WORK/dados/$u.uvpk" "$T/dados/"; done
cp "$UV_WORK/dados/config/"*-cs.json "$T/dados/config/"
bash "$(dirname "$0")/totalizar_comparar.sh" "$T/dados" "$T/res"
# mostra só as linhas das UFs pedidas
padrao=$(echo "$UFS" | tr 'a-z' 'A-Z' | sed 's/,/|/g')
grep -E "\| ($padrao) \|" "$T/res/comparacao_oficial.md" || true
