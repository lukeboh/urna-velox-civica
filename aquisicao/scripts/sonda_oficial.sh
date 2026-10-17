#!/usr/bin/env bash
# Sonda os endereços dos arquivos oficiais de divulgação (1 req/s) e mostra o início do resultado do Brasil.
# Em 2026 o resultado por abrangência está em <ciclo>/<eleição>/dados/<abr>/<abr>-c<cargo>-e<eleição>-u.json;
# "dados-simplificados/...-r.json" (formato de 2022/2024) responde 404.
D="${UV_WORK:-$HOME/uvreal}/oficial/sonda"
mkdir -p "$D" && cd "$D"
B=https://resultados.tse.jus.br/oficial/${UV_CICLO:-ele2026}
for u in 6257/dados-simplificados/br/br-c0001-e006257-r.json 6257/dados/br/br-c0001-e006257-u.json \
         6259/dados/df/df-c0003-e006259-u.json 6257/config/mun-e006257-cm.json; do
  f=$(echo "$u" | tr / _)
  curl -s --compressed -A 'urna-velox/0.1' -o "$f" -w "%{http_code} %{size_download} $u\n" "$B/$u"
  sleep 1
done
head -c 1500 6257_dados_br_br-c0001-e006257-u.json; echo
