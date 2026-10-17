#!/usr/bin/env bash
# Mostra o aux.json de uma seção (1 requisição). Uso: ver_aux.sh <uf> <municipio> <zona> <secao>
uf=$1; mu=$(printf %05d "$2"); zn=$(printf %04d "$3"); se=$(printf %04d "$4")
B="https://resultados.tse.jus.br/oficial/${UV_CICLO:-ele2026}/arquivo-urna/${UV_PLEITO:-3220}/dados"
curl -s --compressed -A 'urna-velox/0.1 (+https://github.com/lukeboh/urna-velox-civica)' \
  "$B/$uf/$mu/$zn/$se/p00${UV_PLEITO:-3220}-$uf-m$mu-z$zn-s$se-aux.json"; echo
