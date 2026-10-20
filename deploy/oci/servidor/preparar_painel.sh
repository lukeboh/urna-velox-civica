#!/usr/bin/env bash
# Roda NO SERVIDOR (como root): instala o uv-painel num nó já preparado (preparar.sh). Idempotente.
# Uso: sudo bash preparar_painel.sh <pasta com uv-painel, uv-painel-acao, uv-painel-iniciar.sh, uv-painel.service e painel.env> <IP privado do par>
set -euo pipefail
D="${1:?pasta dos arquivos}"; PAR="${2:?IP privado do par}"; PORTA=8095

echo "== usuário uvpainel e pastas"
id uvpainel >/dev/null 2>&1 || useradd --system --no-create-home --home-dir /srv/urna-velox/painel --shell /usr/sbin/nologin uvpainel
install -d -o uvpainel -g uvpainel -m 0750 /srv/urna-velox/painel
install -d -o uvpainel -g uvpainel -m 0700 /srv/urna-velox/painel/pendente
chmod 0755 /srv/urna-velox /srv/urna-velox/saida 2>/dev/null || true

echo "== binário, ajudante e serviço"
install -o root -g root -m 0755 "$D/uv-painel" "$D/uv-painel-iniciar.sh" /opt/urna-velox/bin/
install -o root -g root -m 0750 "$D/uv-painel-acao" /opt/urna-velox/bin/
install -o root -g root -m 0644 "$D/uv-painel.service" /etc/systemd/system/
# o uv-iniciar.sh passa ao uv-servico o --rotas quando existe /etc/urna-velox/rotas_tse.json (endereços do site)
if [ -f "$D/uv-iniciar.sh" ]; then install -o root -g root -m 0755 "$D/uv-iniciar.sh" /opt/urna-velox/bin/; fi
if [ -f "$D/painel.env" ]; then install -o root -g uvpainel -m 0640 "$D/painel.env" /etc/urna-velox/painel.env; fi
[ -f /etc/urna-velox/painel.env ] || { echo "falta /etc/urna-velox/painel.env"; exit 1; }

echo "== sudo restrito: só o uv-painel-acao (ações nomeadas, ele confere os argumentos)"
cat > /etc/sudoers.d/urna-velox-painel <<'SUDO'
uvpainel ALL=(root) NOPASSWD: /opt/urna-velox/bin/uv-painel-acao
Defaults:uvpainel !requiretty, env_reset
SUDO
chmod 0440 /etc/sudoers.d/urna-velox-painel
visudo -cf /etc/sudoers.d/urna-velox-painel >/dev/null

echo "== firewall: a porta do painel ($PORTA) só para o par ($PAR); o público entra pelo Caddy (443)"
if ! iptables -C INPUT -p tcp -s "$PAR" --dport "$PORTA" -m conntrack --ctstate NEW -j ACCEPT 2>/dev/null; then
  iptables -I INPUT 1 -p tcp -s "$PAR" --dport "$PORTA" -m conntrack --ctstate NEW -j ACCEPT
fi
netfilter-persistent save >/dev/null 2>&1 || true

systemctl daemon-reload
systemctl enable uv-painel >/dev/null 2>&1
systemctl restart uv-painel
sleep 2
systemctl is-active uv-painel
curl -fsS --max-time 5 http://127.0.0.1:$PORTA/painel/vivo
