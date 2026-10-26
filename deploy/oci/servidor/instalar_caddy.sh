#!/usr/bin/env bash
# Roda NO SERVIDOR (como root via uv-operar.sh): instala e configura o Caddy como balanceador e proxy reverso para o Urna Velox.
# Idempotente. Uso: bash instalar_caddy.sh <domínio> <IP privado do par> [domínio antigo, com 301 para o atual]
set -euo pipefail

DOMINIO="${1:-localhost}"
PAR_IP="${2:-127.0.0.1}"
DOMINIO_ANTIGO="${3:-}"
PLAYLIST="${4:-}"   # B-024: local:<IPs do B> no A, remoto:<IP público do A> no B (gerar_caddyfile.sh)
DOMINIO_DADOS="${5:-}"   # passo 3 do plano de borda: subdomínio de leitura atrás da Cloudflare (vazio = sem o site)
export DEBIAN_FRONTEND=noninteractive

echo "== [Caddy] instalando repositório e pacote oficial"
if ! command -v caddy >/dev/null 2>&1; then
    apt-get update -q
    apt-get install -y -q debian-keyring debian-archive-keyring apt-transport-https curl >/dev/null
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg --yes
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
    apt-get update -q
    apt-get install -y -q caddy >/dev/null
fi

echo "== [Caddy] gerando /etc/caddy/Caddyfile (domínio: $DOMINIO, par: $PAR_IP)"
install -d /etc/caddy /var/log/caddy
chown -R caddy:caddy /var/log/caddy 2>/dev/null || true

# Configura Caddyfile substituindo DOMINIO e PAR_IP (e o 301 do domínio antigo, M-005, quando dado)
# (num arquivo novo, validado antes de entrar no lugar: um erro aqui não deixa o Caddyfile em uso pela metade)
bash /tmp/gerar_caddyfile.sh /tmp/Caddyfile.template "$DOMINIO" "$PAR_IP" "$DOMINIO_ANTIGO" "$PLAYLIST" "$DOMINIO_DADOS" > /etc/caddy/Caddyfile.novo
caddy validate --config /etc/caddy/Caddyfile.novo --adapter caddyfile
mv /etc/caddy/Caddyfile.novo /etc/caddy/Caddyfile
[ -z "$DOMINIO_ANTIGO" ] || echo "== [Caddy] $DOMINIO_ANTIGO responde 301 para https://$DOMINIO"
[ -z "$DOMINIO_DADOS" ] || echo "== [Caddy] $DOMINIO_DADOS: leitura do resultado e do estado (cache de 30 s)"

echo "== [Caddy] liberando portas 80 e 443 na internet e 8080 para o par ($PAR_IP)"
iptables -I INPUT 1 -p tcp --dport 80 -j ACCEPT 2>/dev/null || true
iptables -I INPUT 1 -p tcp --dport 443 -j ACCEPT 2>/dev/null || true
if [ "$PAR_IP" != "127.0.0.1" ]; then
    iptables -C INPUT -p tcp -s "$PAR_IP" --dport 8080 -j ACCEPT 2>/dev/null \
        || iptables -I INPUT 1 -p tcp -s "$PAR_IP" --dport 8080 -j ACCEPT 2>/dev/null || true
fi
# B-025: a regra de antes da 0.7.4, sem origem, abria a 8080 para qualquer um (o Caddy chega pelo lo e o par pela
# regra acima)
while iptables -D INPUT -p tcp -m tcp --dport 8080 -m conntrack --ctstate NEW -j ACCEPT 2>/dev/null; do :; done

echo "== [Caddy] testando e reiniciando serviço"
caddy validate --config /etc/caddy/Caddyfile
# B-027: caddy validate como root cria os arquivos de log com dono root; devolve ao usuário caddy antes do restart
chown -R caddy:caddy /var/log/caddy 2>/dev/null || true
systemctl enable caddy >/dev/null 2>&1 || true
systemctl restart caddy

echo "== [Caddy] ativo e respondendo em https://$DOMINIO (balanceando local + $PAR_IP)"
