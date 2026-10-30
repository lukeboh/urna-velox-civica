#!/usr/bin/env bash
# 00 — Cria na OCI (pela CLI `oci`) a rede e as duas instâncias Arm (Ampere A1) dos nós do Urna Velox.
# Opcional: o mesmo pode ser feito pelo console (docs/handoffs/HANDOFF_IMPLANTACAO_OCI.md, §2). Idempotente: o que já existe
# (pelo nome) é reaproveitado. Ao fim, imprime os IPs para colar no nos.env.
#
# Pré-requisitos: `oci` instalado e configurado (`oci setup config`), e no nos.env:
#   OCI_COMPARTIMENTO=ocid1.compartment...   OCI_AD=(vazio = o primeiro AD)   ADMIN_CIDR=<seu IP>/32
#   SSH_CHAVE_PUB=~/.ssh/oci_urna_velox.pub  OCI_OCPUS=2  OCI_MEMORIA_GB=12  OCI_DISCO_GB=100
# (Always Free: até 4 OCPUs e 24 GB de A1 somados, e 200 GB de disco: 2 × (2 OCPU, 12 GB, 100 GB) cabe.)
source "$(dirname "$0")/lib.sh"
command -v oci >/dev/null || erro "instale a OCI CLI: https://docs.oracle.com/iaas/Content/API/SDKDocs/cliinstall.htm"
: "${OCI_COMPARTIMENTO:?defina OCI_COMPARTIMENTO no nos.env}" "${ADMIN_CIDR:?defina ADMIN_CIDR (seu IP)/32 no nos.env}"
C="$OCI_COMPARTIMENTO"
PUB="${SSH_CHAVE_PUB:-$SSH_CHAVE.pub}"; PUB="${PUB/#\~/$HOME}"
[ -f "$PUB" ] || erro "chave pública $PUB não encontrada (ssh-keygen -t ed25519 -f ${SSH_CHAVE})"
OCPUS="${OCI_OCPUS:-2}"; MEM="${OCI_MEMORIA_GB:-12}"; DISCO="${OCI_DISCO_GB:-100}"
VCN_CIDR=10.0.0.0/16; SUB_CIDR=10.0.1.0/24

q() { oci "$@" --all 2>/dev/null || oci "$@"; }   # algumas listagens não aceitam --all
achar() { q "$@" --query "data[?\"lifecycle-state\"!='TERMINATED'] | [0].id" --raw-output 2>/dev/null | grep -v '^null$' || true; }

AD="${OCI_AD:-$(oci iam availability-domain list --compartment-id "$C" --query 'data[0].name' --raw-output)}"
msg "compartimento $C, AD $AD"

VCN=$(achar network vcn list --compartment-id "$C" --display-name uv-vcn)
if [ -z "$VCN" ]; then
  msg "criando VCN uv-vcn ($VCN_CIDR)"
  VCN=$(oci network vcn create --compartment-id "$C" --cidr-blocks "[\"$VCN_CIDR\"]" --display-name uv-vcn --dns-label uvvcn \
    --wait-for-state AVAILABLE --query data.id --raw-output)
fi
IGW=$(achar network internet-gateway list --compartment-id "$C" --vcn-id "$VCN" --display-name uv-igw)
if [ -z "$IGW" ]; then
  msg "criando internet gateway"
  IGW=$(oci network internet-gateway create --compartment-id "$C" --vcn-id "$VCN" --is-enabled true --display-name uv-igw \
    --wait-for-state AVAILABLE --query data.id --raw-output)
fi
RT=$(oci network vcn get --vcn-id "$VCN" --query 'data."default-route-table-id"' --raw-output)
oci network route-table update --rt-id "$RT" --force \
  --route-rules "[{\"destination\":\"0.0.0.0/0\",\"destinationType\":\"CIDR_BLOCK\",\"networkEntityId\":\"$IGW\"}]" >/dev/null

# Firewall da nuvem: SSH só do seu IP; federação (FED_PORTA) e a ponte do ensaio (FED_PORTA+1, 40_ensaio.sh) só de
# dentro da sub-rede; ICMP para diagnóstico. Saída liberada. (FED_REDE=publico: libere FED_PORTA só para os IPs
# públicos dos nós, à mão.)
ING="[{\"source\":\"$ADMIN_CIDR\",\"protocol\":\"6\",\"tcpOptions\":{\"destinationPortRange\":{\"min\":22,\"max\":22}}},
 {\"source\":\"$SUB_CIDR\",\"protocol\":\"6\",\"tcpOptions\":{\"destinationPortRange\":{\"min\":$FED_PORTA,\"max\":$((FED_PORTA + 1))}}},
 {\"source\":\"0.0.0.0/0\",\"protocol\":\"1\",\"icmpOptions\":{\"type\":3,\"code\":4}},
 {\"source\":\"$VCN_CIDR\",\"protocol\":\"1\",\"icmpOptions\":{\"type\":3}}]"
EGR='[{"destination":"0.0.0.0/0","protocol":"all"}]'
SL=$(achar network security-list list --compartment-id "$C" --vcn-id "$VCN" --display-name uv-seguranca)
if [ -z "$SL" ]; then
  msg "criando security list uv-seguranca"
  SL=$(oci network security-list create --compartment-id "$C" --vcn-id "$VCN" --display-name uv-seguranca \
    --ingress-security-rules "$ING" --egress-security-rules "$EGR" --wait-for-state AVAILABLE --query data.id --raw-output)
else
  oci network security-list update --security-list-id "$SL" --ingress-security-rules "$ING" --egress-security-rules "$EGR" --force >/dev/null
fi
SUB=$(achar network subnet list --compartment-id "$C" --vcn-id "$VCN" --display-name uv-sub)
if [ -z "$SUB" ]; then
  msg "criando sub-rede pública uv-sub ($SUB_CIDR)"
  SUB=$(oci network subnet create --compartment-id "$C" --vcn-id "$VCN" --cidr-block "$SUB_CIDR" --display-name uv-sub \
    --dns-label uvsub --route-table-id "$RT" --security-list-ids "[\"$SL\"]" --wait-for-state AVAILABLE --query data.id --raw-output)
fi

IMG=$(oci compute image list --compartment-id "$C" --operating-system "Canonical Ubuntu" --operating-system-version "24.04" \
  --shape VM.Standard.A1.Flex --sort-by TIMECREATED --sort-order DESC --limit 1 --query 'data[0].id' --raw-output)
[ -n "$IMG" ] && [ "$IMG" != null ] || erro "imagem Ubuntu 24.04 aarch64 não encontrada"
msg "imagem $IMG"

for N in A B; do
  NOME="uv-$(no_var $N ID)"; IPPRIV=$(no_var $N IP_PRIVADO)
  INST=$(achar compute instance list --compartment-id "$C" --display-name "$NOME")
  if [ -z "$INST" ]; then
    msg "criando instância $NOME ($OCPUS OCPU, $MEM GB, disco $DISCO GB, IP privado $IPPRIV)"
    # "Out of host capacity" é comum no Always Free: rode de novo mais tarde ou troque de AD (OCI_AD)
    INST=$(oci compute instance launch --availability-domain "$AD" --compartment-id "$C" --shape VM.Standard.A1.Flex \
      --shape-config "{\"ocpus\":$OCPUS,\"memoryInGBs\":$MEM}" --image-id "$IMG" --subnet-id "$SUB" --private-ip "$IPPRIV" \
      --assign-public-ip false --display-name "$NOME" --hostname-label "$(no_var $N ID)" --boot-volume-size-in-gbs "$DISCO" \
      --ssh-authorized-keys-file "$PUB" --wait-for-state RUNNING --query data.id --raw-output)
  fi
  # IP público RESERVADO (não muda ao reiniciar: é o IP de saída que vai no federacao.json)
  PIP=$(achar network public-ip list --compartment-id "$C" --scope REGION --lifetime RESERVED --display-name "$NOME-ip")
  if [ -z "$PIP" ]; then
    PIP=$(oci network public-ip create --compartment-id "$C" --lifetime RESERVED --display-name "$NOME-ip" --query data.id --raw-output)
  fi
  VNIC=$(oci compute instance list-vnics --instance-id "$INST" --query 'data[0].id' --raw-output)
  PRIV=$(oci network private-ip list --vnic-id "$VNIC" --query 'data[0].id' --raw-output)
  ATUAL=$(oci network public-ip get --public-ip-id "$PIP" --query 'data."assigned-entity-id"' --raw-output)
  [ "$ATUAL" = "$PRIV" ] || oci network public-ip update --public-ip-id "$PIP" --private-ip-id "$PRIV" --wait-for-state ASSIGNED >/dev/null
  IP=$(oci network public-ip get --public-ip-id "$PIP" --query 'data."ip-address"' --raw-output)
  echo "NO_${N}_IP_PUBLICO=$IP      # $NOME"
  echo "NO_${N}_IP_PRIVADO=$IPPRIV"
done
msg "pronto: confira os IPs acima no nos.env e rode ./10_implantar.sh"
