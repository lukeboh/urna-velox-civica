#!/usr/bin/env bash
# Roda NO SERVIDOR (como root via uv-operar.sh): instala e configura o Caddy como balanceador e proxy reverso para o Urna Velox.
# Idempotente. Uso: bash instalar_caddy.sh <domínio> <IP privado do par>
set -euo pipefail

DOMINIO="${1:-localhost}"
PAR_IP="${2:-127.0.0.1}"
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

# Configura Caddyfile substituindo DOMINIO e PAR_IP
sed -e "s/{\$DOMINIO:localhost}/$DOMINIO/g" -e "s/{\$PAR_IP:127.0.0.1}/$PAR_IP/g" /tmp/Caddyfile.template > /etc/caddy/Caddyfile

echo "== [Caddy] liberando portas 80 e 443 na internet e 8080 para o par ($PAR_IP)"
iptables -I INPUT 1 -p tcp --dport 80 -j ACCEPT 2>/dev/null || true
iptables -I INPUT 1 -p tcp --dport 443 -j ACCEPT 2>/dev/null || true
if [ "$PAR_IP" != "127.0.0.1" ]; then
    iptables -I INPUT 1 -p tcp -s "$PAR_IP" --dport 8080 -j ACCEPT 2>/dev/null || true
fi

echo "== [Caddy] testando e reiniciando serviço"
caddy validate --config /etc/caddy/Caddyfile
systemctl enable caddy >/dev/null 2>&1 || true
systemctl restart caddy

echo "== [Caddy] ativo e respondendo em https://$DOMINIO (balanceando local + $PAR_IP)"
