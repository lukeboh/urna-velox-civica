#!/usr/bin/env bash
# 50 — Atualiza o código nos dois nós (nova versão da branch): usa binários pré-compilados (--sem-compilar) ou compila
# no A e troca os binários um nó de cada vez (o outro continua, e quem volta retoma de onde parou e recebe do par o que
# perdeu). Uso: ./50_atualizar.sh [--sem-compilar]
source "$(dirname "$0")/lib.sh"
COMPILAR=1; [ "${1:-}" = "--sem-compilar" ] && COMPILAR=0
G="$AQUI/gerado"; mkdir -p "$G"
for N in A B; do
  em "$N" "mkdir -p /tmp/uv-implantar"
  for f in uv-iniciar.sh urna-velox.service; do para "$N" "$AQUI/servidor/$f" "/tmp/uv-implantar/$f"; done
done
if [ "$COMPILAR" = 1 ]; then
  git -C "$REPO" archive --format=tar.gz -o "$G/uv-src.tar.gz" "$REF"
  msg "compilando $(git -C "$REPO" rev-parse --short "$REF") no A (o serviço continua rodando enquanto compila)"
  para A "$AQUI/servidor/compilar.sh" "/tmp/uv-implantar/compilar.sh"
  para A "$G/uv-src.tar.gz" /tmp/uv-src.tar.gz
  # compila numa cópia, sem tocar em /opt até a troca
  em A "sudo bash -c 'sed s#/opt/urna-velox/bin/#/opt/urna-velox/novo/#g /tmp/uv-implantar/compilar.sh > /tmp/uv-implantar/compilar-novo.sh && mkdir -p /opt/urna-velox/novo && bash /tmp/uv-implantar/compilar-novo.sh'"
  em A "sudo tar -C /opt/urna-velox -czf /tmp/uv-novo.tar.gz novo && sudo chmod 0644 /tmp/uv-novo.tar.gz"
  de A /tmp/uv-novo.tar.gz "$G/uv-novo.tar.gz"
  em A "sudo rm -f /tmp/uv-novo.tar.gz"
  para B "$G/uv-novo.tar.gz" /tmp/uv-novo.tar.gz
  em B "sudo tar -C /opt/urna-velox -xzf /tmp/uv-novo.tar.gz && sudo rm -f /tmp/uv-novo.tar.gz"
else
  [ -f "$G/uv-binarios-aarch64.tar.gz" ] || erro "falta $G/uv-binarios-aarch64.tar.gz — gere antes com ./compilar_local_arm64.sh"
  for N in A B; do
    para "$N" "$G/uv-binarios-aarch64.tar.gz" /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
    em "$N" "sudo rm -rf /opt/urna-velox/novo /tmp/uv-extraido && mkdir -p /tmp/uv-extraido && tar -xzf /tmp/uv-implantar/uv-binarios-aarch64.tar.gz -C /tmp/uv-extraido && sudo mkdir -p /opt/urna-velox/novo && sudo cp /tmp/uv-extraido/bin/* /opt/urna-velox/novo/ && sudo cp /tmp/uv-extraido/VERSION /opt/urna-velox/VERSION.novo && rm -rf /tmp/uv-extraido"
  done
fi
for N in B A; do
  msg "trocando os binários no $N"
  # só volta a iniciar se estava rodando (uma atualização nunca começa uma coleta sozinha)
  em "$N" "ATIVO=\$(systemctl is-active urna-velox || true); sudo systemctl stop urna-velox; sudo cp /opt/urna-velox/novo/* /opt/urna-velox/bin/ &&
           { [ ! -f /opt/urna-velox/VERSION.novo ] || sudo mv /opt/urna-velox/VERSION.novo /opt/urna-velox/VERSION; } &&
           sudo chmod 0755 /opt/urna-velox/bin/* &&
           sudo install -m 0755 /tmp/uv-implantar/uv-iniciar.sh /opt/urna-velox/bin/ &&
           sudo install -m 0644 /tmp/uv-implantar/urna-velox.service /etc/systemd/system/ && sudo systemctl daemon-reload &&
           if [ \"\$ATIVO\" = active ]; then sudo systemctl start urna-velox; echo reiniciado; else echo 'estava parado: continua parado'; fi;
           /opt/urna-velox/bin/uv-servico --version"
  sleep 10
done
"$AQUI/30_verificar.sh"
