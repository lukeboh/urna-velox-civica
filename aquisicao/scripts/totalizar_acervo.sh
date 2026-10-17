#!/usr/bin/env bash
# Totaliza a partir do acervo local de dados abertos (bus/<ano>/<turno>t/<UF>.zip), sem rede, e compara com a divulgação oficial.
# Uso: totalizar_acervo.sh [raiz do acervo] [ano] [turno]
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
RAIZ="${1:-/mnt/d/dados/tse-arquivos-urna}"
ANO="${2:-2026}"; TURNO="${3:-1}"
B="$UV_WORK/target/release/uv-coletor"
T="$UV_WORK/acervo-$ANO-${TURNO}t"
rm -rf "$T"; mkdir -p "$T/dados/config"
cp "$UV_WORK/dados/config/"*-cs.json "$T/dados/config/"
for z in "$RAIZ/bus/$ANO/${TURNO}t/"*.zip; do
  uf=$(basename "$z" .zip | tr 'A-Z' 'a-z')
  "$B" zip --uf "$uf" --local "$z" --dados "$T/dados" 2>&1 | grep -E "extraídos|pacotes"
done
bash "$(dirname "$0")/totalizar_comparar.sh" "$T/dados" "$T/res"
