#!/usr/bin/env bash
# Gera o Caddyfile de um nó a partir do Caddyfile.template (saída padrão). Usado pelo instalar_caddy.sh (02) e pelo
# instalador da playlist (63_ciclo.sh --playlist), para os dois darem o mesmo arquivo.
#   gerar_caddyfile.sh <template> <domínio> [IP privado do par] [domínio antigo] [playlist] [dados]
# Domínio antigo (M-005: urnavelox.duckdns.org, só no nó para onde ele aponta): o bloco do 301 entra; sem ele, sai
# inteiro (outro nó nunca obteria o certificado do endereço antigo).
# Playlist (B-024; o uv-playlist só roda no A):
#   local:<IPs do B, por vírgula>  o nó tem a playlist e confia nesses IPs para o IP do visitante (X-Forwarded-For)
#   remoto:<IP público do A>       o nó leva /playlist e /api/v1/playlist ao HTTPS do A
#   (vazio)                        a playlist local, sem proxies confiáveis (como antes)
# Dados (passo 3 do plano de borda, B-026): o nome do subdomínio de leitura atrás da Cloudflare
# (dados.urnavelox.com.br); vazio, o bloco sai inteiro (o Caddy não tenta o certificado antes de o nome estar no DNS).
set -euo pipefail
T="${1:?template}"; DOM="${2:?domínio}"; PAR="${3:-127.0.0.1}"; ANT="${4:-}"; PL="${5:-}"; DAD="${6:-}"
nome_ok() { [[ "$1" =~ ^[A-Za-z0-9.-]+$ ]] || { echo "nome inválido: $1" >&2; exit 2; }; }
nome_ok "$DOM"; nome_ok "$PAR"
[ -z "$ANT" ] || nome_ok "$ANT"
[ -z "$DAD" ] || nome_ok "$DAD"
[ "$ANT" != "$DOM" ] || { echo "o domínio antigo não pode ser o atual ($DOM)" >&2; exit 2; }
[ "$DAD" != "$DOM" ] || { echo "o domínio de dados não pode ser o principal ($DOM)" >&2; exit 2; }
SED=(-e "s/{\$DOMINIO:localhost}/$DOM/g" -e "s/{\$PAR_IP:127.0.0.1}/$PAR/g")
if [ -n "$ANT" ]; then SED+=(-e "s/__DOMINIO_ANTIGO__/$ANT/g"); else SED+=(-e '/# DOMINIO_ANTIGO:inicio/,/# DOMINIO_ANTIGO:fim/d'); fi
if [ -n "$DAD" ]; then SED+=(-e "s/__DOMINIO_DADOS__/$DAD/g"); else SED+=(-e '/# DADOS:inicio/,/# DADOS:fim/d'); fi
case "$PL" in
  local:*)
    IPS="${PL#local:}"; LISTA=""
    for ip in ${IPS//,/ }; do nome_ok "$ip"; LISTA="${LISTA:+$LISTA }$ip"; done
    [ -n "$LISTA" ] || { echo "playlist local sem IPs" >&2; exit 2; }
    SED+=(-e "s/__PROXIES_CONFIAVEIS__/$LISTA/" -e '/# PLAYLIST_REMOTO:inicio/,/# PLAYLIST_REMOTO:fim/d') ;;
  remoto:*)
    A="${PL#remoto:}"; nome_ok "$A"
    SED+=(-e "s/__PLAYLIST_NO_A__/$A/" -e '/# PLAYLIST_LOCAL:inicio/,/# PLAYLIST_LOCAL:fim/d'
          -e '/# PROXIES_CONFIAVEIS:inicio/,/# PROXIES_CONFIAVEIS:fim/d') ;;
  '')
    SED+=(-e '/# PLAYLIST_REMOTO:inicio/,/# PLAYLIST_REMOTO:fim/d' -e '/# PROXIES_CONFIAVEIS:inicio/,/# PROXIES_CONFIAVEIS:fim/d') ;;
  *) echo "playlist: use local:<ips> ou remoto:<ip>" >&2; exit 2 ;;
esac
sed "${SED[@]}" "$T"
