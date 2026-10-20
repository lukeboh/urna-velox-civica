#!/usr/bin/env bash
# 03 — Cria na OCI (pela CLI `oci`) um Load Balancer Flexível com os 2 nós como backend.
# Idempotente.
source "$(dirname "$0")/lib.sh"
command -v oci >/dev/null || erro "instale a OCI CLI: https://docs.oracle.com/iaas/Content/API/SDKDocs/cliinstall.htm"
: "${OCI_COMPARTIMENTO:?defina OCI_COMPARTIMENTO no nos.env}"
C="$OCI_COMPARTIMENTO"

q() { oci "$@" --all 2>/dev/null || oci "$@"; }
achar() { q "$@" --query "data[?\"lifecycle-state\"!='TERMINATED' && \"lifecycle-state\"!='DELETED'] | [0].id" --raw-output 2>/dev/null | grep -v '^null$' || true; }

VCN=$(achar network vcn list --compartment-id "$C" --display-name uv-vcn)
[ -n "$VCN" ] || erro "VCN uv-vcn não encontrada"
SUB=$(achar network subnet list --compartment-id "$C" --vcn-id "$VCN" --display-name uv-sub)
[ -n "$SUB" ] || erro "Sub-rede uv-sub não encontrada"

msg "Verificando Load Balancer na OCI..."
LB_ID=$(achar lb load-balancer list --compartment-id "$C" --display-name uv-load-balancer)

if [ -z "$LB_ID" ]; then
    msg "Criando Load Balancer Flexível uv-load-balancer (10 Mbps Always Free)..."
    LB_ID=$(oci lb load-balancer create \
        --compartment-id "$C" \
        --display-name uv-load-balancer \
        --shape-name "flexible" \
        --shape-details '{"minimumBandwidthInMbps":10,"maximumBandwidthInMbps":10}' \
        --subnet-ids "[\"$SUB\"]" \
        --is-private false \
        --wait-for-state SUCCEEDED \
        --query 'data.id' --raw-output)
fi

msg "Configurando Backend Set com Health Check na rota /api/v1/estado..."
BS_EXISTE=$(oci lb backend-set get --load-balancer-id "$LB_ID" --backend-set-name uv-backend-set 2>/dev/null || true)
if [ -z "$BS_EXISTE" ]; then
    oci lb backend-set create \
        --load-balancer-id "$LB_ID" \
        --name uv-backend-set \
        --policy "ROUND_ROBIN" \
        --health-checker '{"protocol":"HTTP","port":8080,"urlPath":"/api/v1/estado","returnCode":200,"intervalInMillis":10000,"timeoutInMillis":3000,"retries":3}' \
        --wait-for-state SUCCEEDED >/dev/null
fi

msg "Adicionando Nós A e B ao Backend Set..."
for N in A B; do
    IP=$(no_var "$N" IP_PRIVADO)
    B_EXISTE=$(oci lb backend get --load-balancer-id "$LB_ID" --backend-set-name uv-backend-set --backend-name "${IP}:8080" 2>/dev/null || true)
    if [ -z "$B_EXISTE" ]; then
        oci lb backend create \
            --load-balancer-id "$LB_ID" \
            --backend-set-name uv-backend-set \
            --ip-address "$IP" \
            --port 8080 \
            --weight 1 \
            --wait-for-state SUCCEEDED >/dev/null
        msg "Nó $N ($IP:8080) adicionado"
    fi
done

msg "Configurando Listener HTTP (porta 80 com suporte a WebSocket)..."
L_EXISTE=$(oci lb listener get --load-balancer-id "$LB_ID" --listener-name uv-http-listener 2>/dev/null || true)
if [ -z "$L_EXISTE" ]; then
    oci lb listener create \
        --load-balancer-id "$LB_ID" \
        --name uv-http-listener \
        --default-backend-set-name uv-backend-set \
        --protocol HTTP \
        --port 80 \
        --connection-configuration '{"idleTimeout":120}' \
        --wait-for-state SUCCEEDED >/dev/null
fi

LB_IP=$(oci lb load-balancer get --load-balancer-id "$LB_ID" --query 'data."ip-addresses"[0]."ip-address"' --raw-output)
msg "Load Balancer OCI pronto e ativo!"
echo "IP Público do Load Balancer: $LB_IP"
echo "Você pode apontar o seu DNS para este IP ou colocá-lo na Cloudflare."
