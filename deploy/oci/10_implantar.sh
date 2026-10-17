#!/usr/bin/env bash
# 10 — Implanta os dois nós: prepara os servidores, compila (no A) e copia os binários (para o B), gera e instala o
# federacao.json e a configuração de cada nó, monta o candidatos.json (uma vez, no A) e instala o serviço systemd.
# NÃO inicia a coleta (ver 20_iniciar.sh). Idempotente: rodar de novo atualiza tudo. Uso: ./10_implantar.sh [--sem-compilar]
source "$(dirname "$0")/lib.sh"
COMPILAR=1; [ "${1:-}" = "--sem-compilar" ] && COMPILAR=0
G="$AQUI/gerado"; mkdir -p "$G"

msg "0. conferências"
for N in A B; do em "$N" true || erro "sem SSH para o nó $N ($(host_de $N)) — confira SSH_CHAVE, o IP e a regra da porta 22"; done
norm() { echo "$1" | tr '[:upper:]' '[:lower:]' | xargs; }
if [ "$(norm "$NO_A_ORGAO")" = "$(norm "$NO_B_ORGAO")" ] && [ "$NO_A_COLETA" = true ] && [ "$NO_B_COLETA" = true ]; then
  erro "os dois nós são do mesmo órgão ($NO_A_ORGAO) e os dois coletam: o limite de 100 req/s é por órgão (CLAUDE.md).
   Ponha NO_B_COLETA=false (o B só recebe e totaliza) ou declare o órgão real de cada servidor."
fi
for N in A B; do
  ARQ=$(em "$N" uname -m); [ "$ARQ" = aarch64 ] || aviso "nó $N é $ARQ (esperado aarch64; os binários do A não servem no B)"
done

msg "1. configuração da federação (gerado/)"
# os valores vão por variáveis de ambiente (aspas e acentos no nome do órgão não quebram nada)
END_A="$(endereco_de A)" END_B="$(endereco_de B)" \
NO_A_ID="$NO_A_ID" NO_A_ORGAO="$NO_A_ORGAO" NO_A_IP_PUBLICO="$NO_A_IP_PUBLICO" NO_A_COLETA="$NO_A_COLETA" \
NO_B_ID="$NO_B_ID" NO_B_ORGAO="$NO_B_ORGAO" NO_B_IP_PUBLICO="$NO_B_IP_PUBLICO" NO_B_COLETA="$NO_B_COLETA" RPS="$RPS" \
python3 - "$G/federacao.json" <<'EOF'
import json, os, sys
e = os.environ
nos = []
for n in ("A", "B"):
    d = {"id": e[f"NO_{n}_ID"], "orgao": e[f"NO_{n}_ORGAO"], "ip_saida": e[f"NO_{n}_IP_PUBLICO"], "endereco": e[f"END_{n}"]}
    if e[f"NO_{n}_COLETA"].lower() != "true":
        d["coleta"] = False
    nos.append(d)
rps = float(e["RPS"])
json.dump({"versao": 1, "nos": nos, "teto_coleta_rps": rps}, open(sys.argv[1], "w"), ensure_ascii=False, indent=2)
EOF
cat "$G/federacao.json"
for N in A B; do
  cat > "$G/no-$N.env" <<EOF
# gerado por 10_implantar.sh em $(date -Iseconds)
NO_ID=$(no_var $N ID)
PLEITO=$PLEITO
CICLO=$CICLO
AMBIENTE=$AMBIENTE
BASE=$BASE
RPS=$RPS
FED_PORTA=$FED_PORTA
ECO_IP=$ECO_IP
HTTP_LOCAL=$HTTP_LOCAL
OCI_BUCKET=${OCI_BUCKET:-}
OCI_NAMESPACE=${OCI_NAMESPACE:-}
EOF
done

chave_manifesto   # lib.sh: chave de assinatura com senha; grava gerado/signatarios_permitidos

if [ "$COMPILAR" = 1 ]; then
  msg "2. código ($REF de $REPO, por git archive)"
  git -C "$REPO" archive --format=tar.gz -o "$G/uv-src.tar.gz" "$REF"
  ls -lh "$G/uv-src.tar.gz"
else
  msg "2. binários pré-compilados ARM64 (sem código-fonte)"
  [ -f "$G/uv-binarios-aarch64.tar.gz" ] || erro "falta $G/uv-binarios-aarch64.tar.gz — gere antes com ./compilar_local_arm64.sh"
  ls -lh "$G/uv-binarios-aarch64.tar.gz"
fi

for N in A B; do
  msg "3.$N preparar o servidor $N ($(host_de $N))"
  em "$N" "mkdir -p /tmp/uv-implantar"
  ARQS_PREP=(preparar.sh uv-iniciar.sh urna-velox.service uv-atualizador urna-velox-atualizador.service urna-velox-atualizador.timer)
  [ "$COMPILAR" = 1 ] && ARQS_PREP+=(compilar.sh)
  for f in "${ARQS_PREP[@]}"; do para "$N" "$AQUI/servidor/$f" "/tmp/uv-implantar/$f"; done
  SEM_RUST_ARG=""
  [ "$COMPILAR" = 0 ] && SEM_RUST_ARG="sem-rust"
  em "$N" "sudo bash /tmp/uv-implantar/preparar.sh $(origem_par_de $N) $FED_PORTA $SEM_RUST_ARG"
done

if [ "$COMPILAR" = 1 ]; then
  msg "4. compilar no A e copiar os binários para o B (mesma arquitetura)"
  para A "$G/uv-src.tar.gz" /tmp/uv-src.tar.gz
  em A "sudo bash /tmp/uv-implantar/compilar.sh"
  em A "sudo tar -C /opt/urna-velox -czf /tmp/uv-bin-compilado.tar.gz bin VERSION && sudo chmod 0644 /tmp/uv-bin-compilado.tar.gz"
  de A /tmp/uv-bin-compilado.tar.gz "$G/uv-bin-compilado.tar.gz"
  em A "sudo rm -f /tmp/uv-bin-compilado.tar.gz"
  para B "$G/uv-bin-compilado.tar.gz" /tmp/uv-bin-compilado.tar.gz
  em B "sudo mkdir -p /opt/urna-velox && sudo tar -C /opt/urna-velox -xzf /tmp/uv-bin-compilado.tar.gz && sudo rm -f /tmp/uv-bin-compilado.tar.gz"
  em B "/opt/urna-velox/bin/uv-servico --version"
else
  msg "4. instalar binários pré-compilados no nó A e no nó B"
  for N in A B; do
    para "$N" "$G/uv-binarios-aarch64.tar.gz" /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
    em "$N" "rm -rf /tmp/uv-ext && mkdir -p /tmp/uv-ext && tar -xzf /tmp/uv-implantar/uv-binarios-aarch64.tar.gz -C /tmp/uv-ext && sudo mkdir -p /opt/urna-velox/bin && sudo install -o urnavelox -g urnavelox -m 0755 /tmp/uv-ext/bin/* /opt/urna-velox/bin/ && sudo install -o urnavelox -g urnavelox -m 0644 /tmp/uv-ext/VERSION /opt/urna-velox/VERSION && rm -rf /tmp/uv-ext && /opt/urna-velox/bin/uv-servico --version"
  done
fi

for N in A B; do
  msg "5.$N instalar configuração e serviço no $N"
  para "$N" "$G/federacao.json" /tmp/uv-implantar/federacao.json
  para "$N" "$G/no-$N.env" /tmp/uv-implantar/no.env
  para "$N" "$G/signatarios_permitidos" /tmp/uv-implantar/signatarios_permitidos
  em "$N" "sudo install -m 0644 /tmp/uv-implantar/federacao.json /tmp/uv-implantar/no.env /tmp/uv-implantar/signatarios_permitidos /etc/urna-velox/ &&
           sudo install -m 0755 /tmp/uv-implantar/uv-iniciar.sh /tmp/uv-implantar/uv-atualizador /opt/urna-velox/bin/ &&
           sudo install -m 0644 /tmp/uv-implantar/urna-velox.service /tmp/uv-implantar/urna-velox-atualizador.service /tmp/uv-implantar/urna-velox-atualizador.timer /etc/systemd/system/ &&
           sudo systemctl daemon-reload && sudo systemctl enable urna-velox >/dev/null 2>&1 &&
           sudo systemctl enable --now urna-velox-atualizador.timer >/dev/null 2>&1 && echo instalado"
done

msg "6. candidatos.json do pleito $PLEITO (uma vez, no A: ≤ 5 req/s; o B recebe a cópia)"
if ! em A "test -s /srv/urna-velox/preparo/candidatos.json && grep -Eq '\"agregada\":[[:space:]]*true' /srv/urna-velox/preparo/candidatos.json"; then
  if [ "$PLEITO" = "3221" ] && [ -s "${UVREAL:-/home/dev/uvreal}/2026-2t/preparo/candidatos.json" ] && grep -Eq '"agregada":[[:space:]]*true' "${UVREAL:-/home/dev/uvreal}/2026-2t/preparo/candidatos.json"; then
    para A "${UVREAL:-/home/dev/uvreal}/2026-2t/preparo/candidatos.json" /tmp/uv-implantar/candidatos.json
    em A "sudo install -o urnavelox -g urnavelox -m 0644 /tmp/uv-implantar/candidatos.json /srv/urna-velox/preparo/candidatos.json && sudo rm -f /tmp/uv-implantar/candidatos.json"
  else
    em A "sudo -u urnavelox /opt/urna-velox/bin/uv-coletor preparar --base '$BASE' --ambiente '$AMBIENTE' --ciclo '$CICLO' --pleito '$PLEITO' --saida /srv/urna-velox/preparo" \
      || aviso "preparar saiu com erro/código 2 (fontes incompletas?): veja 'incompleto' no candidatos.json; o serviço aceita com --aceitar-incompleto (EXTRA no no.env)"
  fi
fi
em A "sudo chmod -R a+rX /srv/urna-velox/preparo && sudo tar -C /srv/urna-velox -czf /tmp/uv-preparo.tar.gz preparo && sudo chmod 0644 /tmp/uv-preparo.tar.gz"
de A /tmp/uv-preparo.tar.gz "$G/uv-preparo.tar.gz"
em A "sudo rm -f /tmp/uv-preparo.tar.gz"
para B "$G/uv-preparo.tar.gz" /tmp/uv-preparo.tar.gz
em B "sudo tar -C /srv/urna-velox -xzf /tmp/uv-preparo.tar.gz && sudo chown -R urnavelox:urnavelox /srv/urna-velox/preparo && sudo chmod -R a+rX /srv/urna-velox/preparo && sudo rm -f /tmp/uv-preparo.tar.gz"

msg "7. IP de saída de cada nó (o que a CDN do TSE vai ver)"
for N in A B; do
  MED=$(em "$N" "curl -s --max-time 10 $ECO_IP" | tr -d '[:space:]'); DECL=$(no_var $N IP_PUBLICO)
  if [ "$MED" = "$DECL" ]; then echo "nó $N: $MED (= declarado)"; else aviso "nó $N: medido $MED ≠ declarado $DECL — o nó NÃO vai coletar (IP divergente)"; fi
done
[ "$FED_REDE" = publico ] && aviso "FED_REDE=publico: libere a porta $FED_PORTA na security list da OCI só para os IPs públicos dos dois nós"

msg "pronto. Próximos: ./30_verificar.sh (estado) · ./40_ensaio.sh (ensaio com o simulador) · ./20_iniciar.sh (iniciar)"
