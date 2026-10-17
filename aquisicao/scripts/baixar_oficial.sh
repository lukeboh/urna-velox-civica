#!/usr/bin/env bash
# Baixa os arquivos oficiais de divulgação (resultado por abrangência) de Presidente e Governador,
# para conferir a totalização feita a partir dos BUs. Uma requisição por segundo (bem abaixo do teto de 100 req/s).
#   Presidente (eleição 6257, cargo 0001): br + 27 UFs + zz
#   Governador (eleição 6259, cargo 0003): 27 UFs
#   Senador    (eleição 6259, cargo 0005): 27 UFs
# Uso: baixar_oficial.sh [pasta]   (padrão: ~/uvreal/oficial)
set -euo pipefail
DEST="${1:-${UV_WORK:-$HOME/uvreal}/oficial}"
CICLO="${UV_CICLO:-ele2026}"
B="https://resultados.tse.jus.br/oficial/$CICLO"
UFS="ac al am ap ba ce df es go ma mg ms mt pa pb pe pi pr rj rn ro rr rs sc se sp to"
mkdir -p "$DEST"
pegar() { # eleicao cargo abr
  local f="$3-c$2-e00$1-u.json"
  local cod
  cod=$(curl -s --compressed -A 'urna-velox/0.1 (+https://github.com/lukeboh/urna-velox-civica)' -o "$DEST/$f" -w '%{http_code}' "$B/$1/dados/$3/$f")
  echo "$cod $f"
  [ "$cod" = 200 ] || rm -f "$DEST/$f"
  sleep 1
}
# UV_CARGOS: cargos a baixar (1 = Presidente, 3 = Governador, 5 = Senador). Arquivos já baixados não são pedidos de novo.
for c in ${UV_CARGOS:-1 3 5}; do
  case "$c" in
    1) for a in br $UFS zz; do [ -s "$DEST/$a-c0001-e006257-u.json" ] || pegar 6257 0001 "$a"; done ;;
    3|5) cc=$(printf %04d "$c"); for a in $UFS; do [ -s "$DEST/$a-c$cc-e006259-u.json" ] || pegar 6259 "$cc" "$a"; done ;;
    *) echo "cargo $c não suportado aqui" >&2 ;;
  esac
done
