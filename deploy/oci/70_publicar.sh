#!/usr/bin/env bash
# 70 — Publica no bucket (pull, Parte B de docs/HANDOFF_DEPLOY_SEM_SSH.md) o pacote de binários aarch64 e o
# manifesto.json assinado com Ed25519 (ssh-keygen -Y sign). Os nós consultam o manifesto a cada 1 min via
# uv-atualizador (systemd timer), conferem a assinatura, o número de série, o SHA-256 e a trava combinado_ate,
# e se atualizam sozinhos sem SSH e sem Run Command.
#
# Uso:
#   ./70_publicar.sh [--estado parado|producao|simulado] [--combinado-ate 2026-10-16T12:00:00-03:00] [--confirmo-tse]
#
# O estado do manifesto só é aplicado quando a série é nova (cada publicação é uma série nova) e nunca num nó com a
# trava de controle manual (20_iniciar.sh / 60_simulado.sh; liberada por 21_parar.sh / 60_simulado.sh --restaurar).
# "simulado" e "producao" só ligam o serviço com o no.env que está no nó: o pleito, o candidatos.json e o arquivo de
# ordem de um simulado são postos pelo 60_simulado.sh (o manifesto ainda não carrega insumos).
source "$(dirname "$0")/lib.sh"
G="$AQUI/gerado"; mkdir -p "$G"

ESTADO="parado"
COMB_ATE=""
CONF_TSE=0
PACOTE="$G/uv-binarios-aarch64.tar.gz"

while [ $# -gt 0 ]; do
  case "$1" in
    --estado) ESTADO="$2"; shift ;;
    --combinado-ate) COMB_ATE="$2"; shift ;;
    --confirmo-tse) CONF_TSE=1 ;;
    --pacote) PACOTE="$2"; shift ;;
    *) erro "opção desconhecida: $1" ;;
  esac
  shift
done

case "$ESTADO" in
  parado|producao|produção|simulado) [ "$ESTADO" = "produção" ] && ESTADO="producao" ;;
  *) erro "--estado deve ser parado, producao ou simulado (recebido: $ESTADO)" ;;
esac

if [ "$ESTADO" != "parado" ] && base_e_tse; then
  [ "$CONF_TSE" = 1 ] || erro "estado=$ESTADO contra $BASE exige --confirmo-tse"
  [ -n "$COMB_ATE" ] || erro "estado=$ESTADO contra o TSE exige --combinado-ate <ISO-8601> (trava do uv-atualizador)"
fi

[ -f "$PACOTE" ] || erro "falta $PACOTE — gere antes com ./compilar_local_arm64.sh"

# 1. Chave de assinatura Ed25519 (lib.sh: privada só na máquina do operador, com senha; a pública vai em
#    gerado/signatarios_permitidos e, pelo 10_implantar.sh, para os nós)
chave_manifesto

# 2. Versão e SHA-256 do pacote
VERSAO="$(tar -xOzf "$PACOTE" VERSION | tr -d '[:space:]')"
SHA256="$(sha256sum "$PACOTE" | awk '{print $1}')"
SERIE="$(date -u +%Y%m%d%H%M%S)"
OBJ_PACOTE="manifesto/uv-binarios-${VERSAO}-aarch64.tar.gz"

msg "montando manifesto.json (versão $VERSAO, série $SERIE, estado $ESTADO)"
SERIE="$SERIE" VERSAO="$VERSAO" OBJ_PACOTE="$OBJ_PACOTE" SHA256="$SHA256" \
ESTADO="$ESTADO" COMB_ATE="$COMB_ATE" PLEITO="$PLEITO" CICLO="$CICLO" AMBIENTE="$AMBIENTE" BASE="$BASE" \
NO_A_ID="$NO_A_ID" NO_B_ID="$NO_B_ID" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
python3 - "$G/manifesto.json" <<'PY'
import datetime, json, os, sys
e = os.environ
agora = datetime.datetime.now(datetime.timezone.utc).astimezone().isoformat(timespec="seconds")
no_cfg = {
    "estado": e["ESTADO"],
    "pleito": e["PLEITO"],
    "ciclo": e["CICLO"],
    "ambiente": e["AMBIENTE"],
    "base": e["BASE"],
}
doc = {
    "esquema": 1,
    "serie": int(e["SERIE"]),
    "emitido_em": agora,
    "versao": e["VERSAO"],
    "pacote": {
        "bucket": e["OCI_BUCKET"],
        "namespace": e["OCI_NAMESPACE"],
        "objeto": e["OBJ_PACOTE"],
        "sha256": e["SHA256"],
    },
    "combinado_ate": e["COMB_ATE"] or None,
    "nos": {
        e["NO_A_ID"]: no_cfg,
        e["NO_B_ID"]: no_cfg,
    },
}
with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump(doc, f, ensure_ascii=False, indent=2)
    f.write("\n")
PY

# 3. Assinar manifesto.json com Ed25519 (ssh-keygen -Y sign)
rm -f "$G/manifesto.json.sig"
ssh-keygen -Y sign -q -f "$CHAVE_ASSINATURA" -n urna-velox-manifesto "$G/manifesto.json"
ssh-keygen -Y verify -f "$G/signatarios_permitidos" -I "manifesto@urna-velox" -n urna-velox-manifesto -s "$G/manifesto.json.sig" < "$G/manifesto.json"

# 4. Enviar pacote, manifesto.json e manifesto.json.sig ao bucket da OCI
export PATH="$HOME/.local/bin:$HOME/bin:$PATH"
OCI_PY="$HOME/lib/oracle-cli/bin/python3"
[ -x "$OCI_PY" ] || OCI_PY="$(command -v python3)"

msg "publicando no bucket $OCI_BUCKET ($OBJ_PACOTE, manifesto/manifesto.json, manifesto/manifesto.json.sig)"
OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" OBJ_PACOTE="$OBJ_PACOTE" \
PACOTE="$PACOTE" MANIFESTO="$G/manifesto.json" SIG="$G/manifesto.json.sig" \
"$OCI_PY" - <<'PY'
import os, oci
cfg = oci.config.from_file()
client = oci.object_storage.ObjectStorageClient(cfg)
bucket = os.environ["OCI_BUCKET"]
ns = os.environ.get("OCI_NAMESPACE") or client.get_namespace().data
for obj, arq in [
    (os.environ["OBJ_PACOTE"], os.environ["PACOTE"]),
    ("manifesto/manifesto.json", os.environ["MANIFESTO"]),
    ("manifesto/manifesto.json.sig", os.environ["SIG"]),
]:
    with open(arq, "rb") as f:
        client.put_object(ns, bucket, obj, f)
    print(f"  enviado: {obj} ({os.path.getsize(arq)} bytes)")
PY

msg "publicado: versão $VERSAO (série $SERIE, SHA-256 ${SHA256:0:16}...). Acompanhe os nós com ./30_verificar.sh"
