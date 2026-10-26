#!/usr/bin/env bash
# Funções comuns dos scripts de implantação (carregado pelos NN_*.sh). Ver docs/HANDOFF_IMPLANTACAO_OCI.md.
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$AQUI/nos.env" ] || { echo "falta $AQUI/nos.env (copie nos.env.exemplo e preencha)"; exit 1; }
# shellcheck disable=SC1091
source "$AQUI/nos.env"
TRANSPORTE="${TRANSPORTE:-ssh}"
SSH_CHAVE="${SSH_CHAVE/#\~/$HOME}"
SSH_OPCOES=(-i "$SSH_CHAVE" -o StrictHostKeyChecking=accept-new -o ServerAliveInterval=30 -o ConnectTimeout=15)

# no_var <A|B> <CAMPO> → valor de NO_<A|B>_<CAMPO>
no_var() { local v="NO_$1_$2"; echo "${!v}"; }
host_de() { no_var "$1" IP_PUBLICO; }
ocid_de() { no_var "$1" OCID; }
outro_de() { [ "$1" = A ] && echo B || echo A; }

msg() { printf '\n\033[1;36m== %s\033[0m\n' "$*"; }
aviso() { printf '\033[1;33m!! %s\033[0m\n' "$*"; }
erro() { printf '\033[1;31mXX %s\033[0m\n' "$*" >&2; exit 1; }

# Dados de cada pleito no D: (fora do git): uma pasta por pleito, com o nome AAAA-MM-DD-código (data da eleição e
# código do pleito, ex.: 2026-10-25-3221), em /mnt/d/dados/urna-velox/pleitos (ou $UV_PLEITOS).
PLEITOS_D="${UV_PLEITOS:-/mnt/d/dados/urna-velox/pleitos}"
pasta_pleito() { # <código> → caminho da pasta AAAA-MM-DD-<código>; erro se não houver exatamente uma
  local c=("$PLEITOS_D"/[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]-"$1")
  [ ${#c[@]} -eq 1 ] && [ -d "${c[0]}" ] || erro "pasta do pleito $1 não encontrada em $PLEITOS_D (formato AAAA-MM-DD-$1)"
  echo "${c[0]}"
}

if [ "$TRANSPORTE" = "oci" ]; then
  export PATH="$HOME/.local/bin:$HOME/bin:$PATH"
  OCI_PY="$HOME/lib/oracle-cli/bin/python3"
  [ -x "$OCI_PY" ] || OCI_PY="$(command -v python3)"
  : "${OCI_COMPARTIMENTO:?defina OCI_COMPARTIMENTO no nos.env}" "${OCI_BUCKET:?defina OCI_BUCKET no nos.env}"
fi

_oci_op() {
  "$OCI_PY" "$AQUI/uv_oci.py" "$@"
}

# Transporte para o nó: em (executa comando), para (envia arquivo), de (baixa arquivo)
em() {
  local n="$1"; shift
  if [ "$TRANSPORTE" = "oci" ]; then
    local tem_stdin=0
    if [[ "$*" == *"bash -s"* ]] || [ -f /dev/stdin ]; then tem_stdin=1; fi
    OCI_COMPARTIMENTO="$OCI_COMPARTIMENTO" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
      _oci_op em "$(ocid_de "$n")" "$tem_stdin" "$*" 3<&0
  else
    ssh "${SSH_OPCOES[@]}" "$SSH_USUARIO@$(host_de "$n")" "$@"
  fi
}

para() {
  local n="$1" origem="$2" destino="$3"
  if [ "$TRANSPORTE" = "oci" ]; then
    OCI_COMPARTIMENTO="$OCI_COMPARTIMENTO" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
      _oci_op para "$(ocid_de "$n")" "$origem" "$destino"
  else
    scp "${SSH_OPCOES[@]}" -q "$origem" "$SSH_USUARIO@$(host_de "$n"):$destino"
  fi
}

de() {
  local n="$1" origem="$2" destino_local="$3"
  if [ "$TRANSPORTE" = "oci" ]; then
    OCI_COMPARTIMENTO="$OCI_COMPARTIMENTO" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
      _oci_op de "$(ocid_de "$n")" "$origem" "$destino_local"
  else
    mkdir -p "$(dirname "$destino_local")"
    scp "${SSH_OPCOES[@]}" -q "$SSH_USUARIO@$(host_de "$n"):$origem" "$destino_local"
  fi
}

# Trava de controle manual (0.6.6): enquanto existir /etc/urna-velox/controle-manual no nó, o uv-atualizador não mexe
# nele (nem binários, nem estado do serviço). Gravada por 20_iniciar.sh e 60_simulado.sh; apagada por 21_parar.sh e
# 60_simulado.sh --restaurar. Sem ela, o manifesto em vigor só é reaplicado quando uma série nova é publicada.
TRAVA_MANUAL=/etc/urna-velox/controle-manual
travar_manual() { # <A|B> <motivo>
  em "$1" "printf '%s %s\n' '$2' \"\$(date -Iseconds)\" | sudo tee $TRAVA_MANUAL >/dev/null && echo 'controle manual: travado ($2)'"
}
liberar_manual() { # <A|B>
  em "$1" "sudo rm -f $TRAVA_MANUAL && echo 'controle manual: liberado (o atualizador volta a seguir o manifesto na próxima série)'"
}

# Chave Ed25519 que assina os manifestos do uv-atualizador (0.6.6). Quem tem a privada publica versões nos nós: ela
# fica só na máquina do operador e COM SENHA (o ssh-keygen pede a senha ao assinar; com o ssh-agent, uma vez por
# sessão). Nunca é gerada sem senha; uma chave existente sem senha gera aviso (proteja com: ssh-keygen -p -f <chave>).
CHAVE_ASSINATURA="${UV_CHAVE_MANIFESTO:-$HOME/.ssh/uv_manifesto_ed25519}"
chave_manifesto() {
  if [ ! -f "$CHAVE_ASSINATURA" ]; then
    [ -t 0 ] || erro "falta a chave de assinatura $CHAVE_ASSINATURA. Crie COM senha, num terminal: ssh-keygen -t ed25519 -C manifesto@urna-velox -f $CHAVE_ASSINATURA"
    msg "criando a chave de assinatura dos manifestos em $CHAVE_ASSINATURA — escolha uma senha"
    mkdir -p "$(dirname "$CHAVE_ASSINATURA")"
    ssh-keygen -t ed25519 -C "manifesto@urna-velox" -f "$CHAVE_ASSINATURA" || erro "chave não criada"
  fi
  chmod 0600 "$CHAVE_ASSINATURA"
  if ssh-keygen -y -P "" -f "$CHAVE_ASSINATURA" >/dev/null 2>&1; then
    aviso "a chave $CHAVE_ASSINATURA está SEM senha: quem copiar o arquivo publica versões nos nós. Proteja: ssh-keygen -p -f $CHAVE_ASSINATURA"
  fi
  PUB_KEY="$(awk '{print $1, $2}' "${CHAVE_ASSINATURA}.pub")"
  mkdir -p "$AQUI/gerado"
  printf 'manifesto@urna-velox namespaces="urna-velox-manifesto" %s\n' "$PUB_KEY" > "$AQUI/gerado/signatarios_permitidos"
}

# endereço de sincronização do nó (o que vai no federacao.json)
endereco_de() {
  if [ "$FED_REDE" = "privado" ]; then echo "$(no_var "$1" IP_PRIVADO):$FED_PORTA"
  else echo "$(no_var "$1" IP_PUBLICO):$FED_PORTA"; fi
}
# IP de onde o outro nó chega à porta da federação (regra de firewall)
origem_par_de() {
  local o; o=$(outro_de "$1")
  if [ "$FED_REDE" = "privado" ]; then no_var "$o" IP_PRIVADO; else no_var "$o" IP_PUBLICO; fi
}

# Contra o site real do TSE, nada de coleta antes de 26/10/2026 sem combinar (CLAUDE.md, HANDOFF_0.6.0.md).
base_e_tse() { [[ "$BASE" != http://127.0.0.1* && "$BASE" != http://localhost* ]]; }
