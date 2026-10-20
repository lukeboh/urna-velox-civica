#!/usr/bin/env bash
# Teste do B-008: com o Caddy dos nós na frente (servidor/Caddyfile.template) e o uv-servico fora do ar — o que acontece
# na troca de eleição do ciclo (o uv-ciclo.sh para o serviço, espera ~1 min e sobe o novo) e na partida da noite real —,
# o visitante tem de ver uma página HTML ("atualizando"), nunca um arquivo para salvar. Depois, com o serviço de volta,
# a página dele passa normalmente.
#
#   bash deploy/oci/testes/pagina_indisponivel.sh
#
# Precisa do Caddy da mesma versão dos nós (v2.11.7): $UV_CADDY ou ~/uvreal/ferramentas/caddy (baixado do GitHub se
# faltar) e do Playwright do site (site/node_modules). Nada sai da máquina além do download do Caddy.
set -uo pipefail
RAIZ="$(cd "$(dirname "$0")/../../.." && pwd)"
CADDY="${UV_CADDY:-$HOME/uvreal/ferramentas/caddy}"
if [ ! -x "$CADDY" ]; then
  mkdir -p "$(dirname "$CADDY")"
  curl -sSLf https://github.com/caddyserver/caddy/releases/download/v2.11.7/caddy_2.11.7_linux_amd64.tar.gz \
    | tar xz -C "$(dirname "$CADDY")" caddy || { echo "sem o Caddy ($CADDY)"; exit 2; }
fi
T=$(mktemp -d /tmp/uv-b008-XXXX)
PC=${UV_PORTA_CADDY:-38443}; PUP=${UV_PORTA_SERVICO:-38080}
PIDS=()
fim() { for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null; done; wait 2>/dev/null; rm -rf "$T"; }
trap fim EXIT

# o Caddyfile dos nós, só com a porta do serviço e o log trocados para o teste (sem TLS: endereço http://)
sed -e "s#/var/log/caddy/urna-velox-acesso.log#$T/acesso.log#" -e "s#:8080#:$PUP#g" \
  "$RAIZ/deploy/oci/servidor/Caddyfile.template" > "$T/Caddyfile"
DOMINIO="http://127.0.0.1:$PC" PAR_IP=127.0.0.1 XDG_DATA_HOME="$T/dados" XDG_CONFIG_HOME="$T/cfg" \
  "$CADDY" run --config "$T/Caddyfile" --adapter caddyfile >"$T/caddy.log" 2>&1 &
PIDS+=($!)
for _ in $(seq 1 50); do curl -s -o /dev/null "http://127.0.0.1:$PC/" && break; sleep 0.2; done

falhas=0
caso() { # <nome> <comando...>
  local n="$1"; shift
  if "$@"; then echo "  ok: $n"; else echo "  FALHOU: $n"; falhas=$((falhas + 1)); fi
}
abrir() { (cd "$RAIZ/site" && node testes/indisponivel.mjs "$@"); }
json_api() { # a API, fora do ar, responde JSON (os clientes do protocolo esperam JSON)
  local ct; ct=$(curl -s -o /dev/null -w '%{content_type}' "http://127.0.0.1:$PC/api/v1/estado")
  [[ "$ct" == application/json* ]] || { echo "    /api/v1/estado com Content-Type '$ct'"; return 1; }
}

echo "1) serviço fora do ar (troca de eleição)"
caso "página HTML, não download" abrir "http://127.0.0.1:$PC/"
sleep 6   # a verificação de saúde (health_interval 5s) tira os nós da rotação: a resposta muda de 502 para 503
caso "página HTML também com os nós fora da rotação" abrir "http://127.0.0.1:$PC/"
caso "API responde JSON" json_api

echo "2) serviço de volta"
mkdir -p "$T/up/api/v1"
printf '<!doctype html><title>Urna Velox</title><p>pagina-do-servico</p>\n' > "$T/up/index.html"
echo '{}' > "$T/up/api/v1/estado"
(cd "$T/up" && exec python3 -m http.server "$PUP" --bind 127.0.0.1 >/dev/null 2>&1) &
PIDS+=($!)
sleep 6   # a verificação de saúde devolve o nó à rotação
caso "a página do serviço passa" abrir "http://127.0.0.1:$PC/" pagina-do-servico

if [ "$falhas" -gt 0 ]; then echo "pagina_indisponivel: $falhas caso(s) falharam"; exit 1; fi
echo "pagina_indisponivel: ok"
