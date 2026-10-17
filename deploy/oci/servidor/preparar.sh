#!/usr/bin/env bash
# Roda NO SERVIDOR (como root, via sudo): prepara uma instância Ubuntu 24.04 aarch64 da OCI para um nó do Urna Velox.
# Idempotente. Uso: sudo bash preparar.sh <IP do par> <porta da federação>
set -euo pipefail
PAR="${1:?IP de onde o outro nó chega}"; PORTA="${2:-7700}"
export DEBIAN_FRONTEND=noninteractive

echo "== pacotes"
apt-get update -q
apt-get install -y -q build-essential pkg-config git curl jq python3 python3-venv chrony socat iptables-persistent netfilter-persistent >/dev/null

echo "== usuário de serviço e pastas"
id urnavelox >/dev/null 2>&1 || useradd --system --create-home --home-dir /srv/urna-velox --shell /bin/bash urnavelox
install -d -o urnavelox -g urnavelox /srv/urna-velox/{dados,saida,federacao,preparo,src,ensaio}
chmod 0755 /srv/urna-velox
chmod -R a+rX /srv/urna-velox/preparo /srv/urna-velox/saida 2>/dev/null || true
install -d /etc/urna-velox /opt/urna-velox/bin

echo "== suporte a OCI Run Command (ocarun sudoers, uv-operar.sh e uv-bucket com Instance Principal)"
if [ ! -x /opt/urna-velox/oci-venv/bin/python3 ]; then
  python3 -m venv /opt/urna-velox/oci-venv
  /opt/urna-velox/oci-venv/bin/pip install -q --no-cache-dir oci
fi
cat > /opt/urna-velox/bin/uv-bucket <<'PY'
#!/opt/urna-velox/oci-venv/bin/python3
import sys, oci

def main():
    if len(sys.argv) < 5:
        sys.exit("uso: uv-bucket <get|put> <bucket> <objeto> <arquivo> [namespace]")
    acao, bucket, obj, arq = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
    signer = oci.auth.signers.InstancePrincipalsSecurityTokenSigner()
    client = oci.object_storage.ObjectStorageClient(config={}, signer=signer)
    ns = sys.argv[5] if len(sys.argv) > 5 and sys.argv[5] else client.get_namespace().data
    if acao == "get":
        resp = client.get_object(ns, bucket, obj)
        with open(arq, "wb") as f:
            for chunk in resp.data.raw.stream(1024 * 1024, decode_content=False):
                f.write(chunk)
    elif acao == "put":
        with open(arq, "rb") as f:
            client.put_object(ns, bucket, obj, f)
    else:
        sys.exit(f"acao desconhecida: {acao}")

if __name__ == "__main__":
    main()
PY
chmod 0755 /opt/urna-velox/bin/uv-bucket

# ATENÇÃO (revisão de 16/10/2026): os privilégios abaixo EQUIVALEM A ROOT para o usuário ocarun. O uv-operar.sh executa
# qualquer comando ("exec $@"), e a lista do sudoers inclui tee, cp, sed, chmod, systemd-run e "bash /tmp/uv-implantar/*",
# cada um suficiente para virar root. Não é uma "lista fechada". A barreira real é o IAM da OCI: só o grupo
# Administrators cria comandos do Run Command (política uv-nos-policy). Os roteiros 10 a 61 rodam comandos de
# administração arbitrários pelo `em`, por isso o desenho é este; fechar de verdade exige trocar cada `sudo <cmd>` dos
# roteiros por ações nomeadas do uv-operar.sh (pendência registrada em docs/RELATORIO_DEPLOY_SEM_SSH.md, §7).
cat > /opt/urna-velox/bin/uv-operar.sh <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exec "$@"
SH
chmod 0755 /opt/urna-velox/bin/uv-operar.sh

if id ocarun >/dev/null 2>&1; then
  cat > /etc/sudoers.d/urna-velox-ocarun <<'SUDO'
ocarun ALL=(root) NOPASSWD: /opt/urna-velox/bin/uv-operar.sh, /opt/urna-velox/bin/uv-bucket, /usr/bin/systemctl, /usr/bin/systemd-run, /usr/bin/install, /usr/bin/mkdir, /usr/bin/tar, /usr/bin/chown, /usr/bin/chmod, /usr/bin/cp, /usr/bin/mv, /usr/bin/rm, /usr/bin/touch, /usr/bin/sed, /usr/bin/tee, /usr/bin/cat, /usr/bin/gzip, /usr/bin/test, /usr/sbin/iptables, /usr/bin/bash /tmp/uv-implantar/*
ocarun ALL=(urnavelox) NOPASSWD: ALL
SUDO
  chmod 0440 /etc/sudoers.d/urna-velox-ocarun
  visudo -cf /etc/sudoers.d/urna-velox-ocarun >/dev/null
fi

if [ -f /etc/oracle-cloud-agent/plugins/runcommand/config.yml ] && ! grep -q "pollCommandInterval: '5s'" /etc/oracle-cloud-agent/plugins/runcommand/config.yml; then
  sed -i "s/pollCommandInterval:.*/pollCommandInterval: '5s'/" /etc/oracle-cloud-agent/plugins/runcommand/config.yml
fi

echo "== Rust (opcional)"
if [ "${3:-}" != "sem-rust" ]; then
  if [ ! -x /srv/urna-velox/.cargo/bin/cargo ]; then
    sudo -u urnavelox -H bash -c 'curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null'
  fi
  sudo -u urnavelox -H /srv/urna-velox/.cargo/bin/rustc --version
else
  echo "instalando apenas binarios pré-compilados (sem Rust no servidor)"
fi

echo "== relógio (chrony com o servidor de tempo da OCI)"
grep -q '169.254.169.254' /etc/chrony/chrony.conf || echo 'server 169.254.169.254 iburst prefer' >> /etc/chrony/chrony.conf
systemctl enable --now chrony >/dev/null 2>&1 || true
systemctl restart chrony || true

echo "== firewall do sistema: a imagem da OCI rejeita tudo menos SSH; abre a federação só para o par ($PAR:$PORTA)"
if ! iptables -C INPUT -p tcp -s "$PAR" --dport "$PORTA" -m conntrack --ctstate NEW -j ACCEPT 2>/dev/null; then
  iptables -I INPUT 1 -p tcp -s "$PAR" --dport "$PORTA" -m conntrack --ctstate NEW -j ACCEPT
fi
netfilter-persistent save >/dev/null

echo "== limites do sistema"
cat > /etc/sysctl.d/90-urna-velox.conf <<'EOF'
# muitas conexões HTTP/2 e WebSocket; portas efêmeras para a coleta
net.core.somaxconn = 4096
net.ipv4.ip_local_port_range = 10240 65535
net.ipv4.tcp_fin_timeout = 15
EOF
sysctl --system >/dev/null

echo "== pronto: $(hostname) $(uname -m) $(nproc) CPUs $(free -g | awk '/Mem/{print $2}') GB"
