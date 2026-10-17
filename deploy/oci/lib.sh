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

if [ "$TRANSPORTE" = "oci" ]; then
  export PATH="$HOME/.local/bin:$HOME/bin:$PATH"
  OCI_PY="$HOME/lib/oracle-cli/bin/python3"
  [ -x "$OCI_PY" ] || OCI_PY="$(command -v python3)"
  : "${OCI_COMPARTIMENTO:?defina OCI_COMPARTIMENTO no nos.env}" "${OCI_BUCKET:?defina OCI_BUCKET no nos.env}"
fi

_oci_op() {
  "$OCI_PY" - "$@" <<'PY'
import base64, os, sys, time, uuid
import oci

acao = sys.argv[1]
comp = os.environ["OCI_COMPARTIMENTO"]
bucket = os.environ["OCI_BUCKET"]
ns = os.environ.get("OCI_NAMESPACE", "")

cfg = oci.config.from_file()
ia_client = oci.compute_instance_agent.ComputeInstanceAgentClient(cfg)
os_client = oci.object_storage.ObjectStorageClient(cfg)
if not ns:
    ns = os_client.get_namespace().data

def del_obj(nome):
    try:
        os_client.delete_object(ns, bucket, nome)
    except Exception:
        pass

def rodar_remoto(inst_id, script_bytes, timeout_s=1800):
    tok = f"{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    in_obj = None
    try:
        if len(script_bytes) <= 2000:
            b64_in = base64.b64encode(script_bytes).decode("ascii")
            prep = f"echo '{b64_in}' | base64 -d > /tmp/uv-in-{tok}.sh"
        else:
            in_obj = f"in-{tok}.sh"
            os_client.put_object(ns, bucket, in_obj, script_bytes)
            prep = f"sudo /opt/urna-velox/bin/uv-bucket get '{bucket}' '{in_obj}' /tmp/uv-in-{tok}.sh '{ns}'"

        wrapper = f"""#!/usr/bin/env bash
{prep}
chmod 0700 /tmp/uv-in-{tok}.sh
bash /tmp/uv-in-{tok}.sh >/tmp/uv-out-{tok} 2>/tmp/uv-err-{tok}
RC=$?
OUT_SZ=$(wc -c </tmp/uv-out-{tok} 2>/dev/null || echo 0)
ERR_SZ=$(wc -c </tmp/uv-err-{tok} 2>/dev/null || echo 0)
if [ $((OUT_SZ + ERR_SZ)) -le 450 ]; then
  B_OUT=$(base64 -w0 /tmp/uv-out-{tok})
  B_ERR=$(base64 -w0 /tmp/uv-err-{tok})
  printf 'UV_RES:%d:INLINE:%s:%s\\n' "$RC" "$B_OUT" "$B_ERR"
else
  sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' 'out-{tok}' /tmp/uv-out-{tok} '{ns}'
  sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' 'err-{tok}' /tmp/uv-err-{tok} '{ns}'
  printf 'UV_RES:%d:BUCKET:out-%s:err-%s\\n' "$RC" "{tok}" "{tok}"
fi
rm -f /tmp/uv-in-{tok}.sh /tmp/uv-out-{tok} /tmp/uv-err-{tok}
exit 0
"""
        details = oci.compute_instance_agent.models.CreateInstanceAgentCommandDetails(
            compartment_id=comp,
            execution_time_out_in_seconds=timeout_s,
            display_name=f"uv-{tok[:20]}",
            target=oci.compute_instance_agent.models.InstanceAgentCommandTarget(instance_id=inst_id),
            content=oci.compute_instance_agent.models.InstanceAgentCommandContent(
                source=oci.compute_instance_agent.models.InstanceAgentCommandSourceViaTextDetails(
                    text=wrapper
                ),
                output=oci.compute_instance_agent.models.InstanceAgentCommandOutputViaTextDetails()
            )
        )
        cmd_id = ia_client.create_instance_agent_command(details).data.id
        t0 = time.time()
        while time.time() - t0 < timeout_s + 60:
            ex = ia_client.get_instance_agent_command_execution(cmd_id, inst_id).data
            st = ex.lifecycle_state
            if st in ("SUCCEEDED", "FAILED", "TIMED_OUT", "CANCELED"):
                texto = (ex.content.text or "") if ex.content else ""
                for linha in reversed(texto.splitlines()):
                    if linha.startswith("UV_RES:"):
                        partes = linha.split(":", 4)
                        rc = int(partes[1])
                        modo = partes[2]
                        if modo == "INLINE":
                            out_b = base64.b64decode(partes[3]) if partes[3] else b""
                            err_b = base64.b64decode(partes[4]) if partes[4] else b""
                        else:
                            o_obj, e_obj = partes[3], partes[4]
                            try:
                                out_b = os_client.get_object(ns, bucket, o_obj).data.content
                            finally:
                                del_obj(o_obj)
                            try:
                                err_b = os_client.get_object(ns, bucket, e_obj).data.content
                            finally:
                                del_obj(e_obj)
                        return rc, out_b, err_b
                rc = ex.content.exit_code if (ex.content and ex.content.exit_code is not None) else 1
                return (rc if rc != 0 else 1), b"", f"Run Command {st}: {texto}\n".encode("utf-8")
            time.sleep(1.5)
        return 124, b"", b"Run Command: tempo limite esgotado\n"
    finally:
        if in_obj:
            del_obj(in_obj)

if acao == "em":
    inst_id = sys.argv[2]
    tem_stdin = sys.argv[3] == "1"
    cmd_str = sys.argv[4]
    if tem_stdin:
        with os.fdopen(3, "rb", closefd=False) as fd3:
            stdin_bytes = fd3.read()
        if cmd_str.strip() in ("", "bash -s", "bash"):
            script_bytes = stdin_bytes
        else:
            tok_eof = f"UV_EOF_{uuid.uuid4().hex}"
            script_bytes = cmd_str.encode("utf-8") + f" <<'{tok_eof}'\n".encode("utf-8") + stdin_bytes + f"\n{tok_eof}\n".encode("utf-8")
    else:
        script_bytes = cmd_str.encode("utf-8") + b"\n"
    rc, out_b, err_b = rodar_remoto(inst_id, script_bytes)
    if out_b:
        sys.stdout.buffer.write(out_b)
        sys.stdout.buffer.flush()
    if err_b:
        sys.stderr.buffer.write(err_b)
        sys.stderr.buffer.flush()
    sys.exit(rc)

elif acao == "para":
    inst_id, origem, destino = sys.argv[2], sys.argv[3], sys.argv[4]
    tok = f"para-{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    try:
        with open(origem, "rb") as f:
            os_client.put_object(ns, bucket, tok, f)
        cmd = (
            f"sudo /opt/urna-velox/bin/uv-bucket get '{bucket}' '{tok}' '/tmp/{tok}' '{ns}' && "
            f"(mkdir -p \"$(dirname '{destino}')\" 2>/dev/null || sudo mkdir -p \"$(dirname '{destino}')\") && "
            f"(cp '/tmp/{tok}' '{destino}' 2>/dev/null || sudo cp '/tmp/{tok}' '{destino}') && "
            f"sudo rm -f '/tmp/{tok}'"
        )
        rc, out_b, err_b = rodar_remoto(inst_id, cmd.encode("utf-8"))
        if err_b:
            sys.stderr.buffer.write(err_b)
        sys.exit(rc)
    finally:
        del_obj(tok)

elif acao == "de":
    inst_id, origem, destino_local = sys.argv[2], sys.argv[3], sys.argv[4]
    tok = f"de-{int(time.time()*1000)}-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    try:
        cmd = f"sudo /opt/urna-velox/bin/uv-bucket put '{bucket}' '{tok}' '{origem}' '{ns}'"
        rc, out_b, err_b = rodar_remoto(inst_id, cmd.encode("utf-8"))
        if rc != 0:
            if err_b:
                sys.stderr.buffer.write(err_b)
            sys.exit(rc)
        os.makedirs(os.path.dirname(os.path.abspath(destino_local)), exist_ok=True)
        resp = os_client.get_object(ns, bucket, tok)
        with open(destino_local, "wb") as f:
            for chunk in resp.data.raw.stream(1024 * 1024, decode_content=False):
                f.write(chunk)
        sys.exit(0)
    finally:
        del_obj(tok)
PY
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
