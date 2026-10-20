#!/usr/bin/env bash
# 02 — Configura a borda pública (Caddy, TLS e firewall) nos nós do Urna Velox.
# Idempotente. Uso: ./02_configurar_borda.sh [dominio-ou-subdominio]
# Exemplo: ./02_configurar_borda.sh urnavelox.com.br
source "$(dirname "$0")/lib.sh"

DOMINIO="${1:-${URL_DOMINIO:-}}"
[ -n "$DOMINIO" ] || erro "informe o domínio ou subdomínio. Exemplo: ./02_configurar_borda.sh urnavelox.com.br"

msg "Configurando borda para o domínio: $DOMINIO"

# 1. Security List da sub-rede dos nós (B-019): 80/443 para a internet; 8080 (balanceamento), 8095 (painel) e a
#    federação para a sub-rede. Pelo caminho que a instância usa (VNIC → sub-rede → Security List), só acrescentando o
#    que falta. Antes procurava a lista "uv-seguranca" pelo nome e, sem achar, seguia em silêncio (a 8095 nunca abriu).
if [ "$TRANSPORTE" = "oci" ]; then
    msg "Security List: portas 80/443 (internet) e 8080, 8095, $FED_PORTA-$((FED_PORTA + 1)) (sub-rede)"
    sl() { OCI_COMPARTIMENTO="$OCI_COMPARTIMENTO" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
             _oci_op liberar-portas "$@" "$(ocid_de A)" "$(ocid_de B)"; }
    sl 0.0.0.0/0 80,443 && sl sub-rede "8080,8095,$FED_PORTA,$((FED_PORTA + 1))" \
        || erro "não consegui conferir/abrir as portas na Security List (veja a mensagem acima); sem a 8095 os painéis não se enxergam"
else
    aviso "TRANSPORTE=$TRANSPORTE: confira na Security List da sub-rede as portas 80/443 (internet) e 8080, 8095, $FED_PORTA-$((FED_PORTA + 1)) (sub-rede)"
fi

# 2. Instala e configura Caddy nos nós A e B
SCRIPT_DIR="$(dirname "$0")"
for N in A B; do
    IP=$(host_de "$N")
    OUTRO=$(outro_de "$N")
    IP_PAR=$(no_var "$OUTRO" IP_PRIVADO)
    msg "Instalando e configurando Caddy no nó $N ($IP) com balanceamento para o nó $OUTRO ($IP_PAR)..."
    
    para "$N" "$SCRIPT_DIR/servidor/Caddyfile.template" "/tmp/Caddyfile.template"
    para "$N" "$SCRIPT_DIR/servidor/instalar_caddy.sh" "/tmp/instalar_caddy.sh"
    
    # "bash -s": sem ele o Run Command recebe o comando vazio e nada é instalado (B-015)
    em "$N" "bash -s" <<CMD || erro "nó $N: a instalação do Caddy falhou"
sudo /opt/urna-velox/bin/uv-operar.sh bash /tmp/instalar_caddy.sh "$DOMINIO" "$IP_PAR"
rm -f /tmp/instalar_caddy.sh /tmp/Caddyfile.template
CMD
    msg "Nó $N configurado e ativo"
done

msg "Borda configurada com sucesso nos 2 nós para: https://$DOMINIO"
echo ""
echo "IPs dos seus nós na OCI:"
echo "  Nó A: $(host_de A)"
echo "  Nó B: $(host_de B)"
echo ""
echo "No painel do DNS (ex.: Registro.br), aponte o domínio $DOMINIO para:"
echo "  IP Nó A: $(host_de A)"
echo "  IP Nó B: $(host_de B)"
echo "(O Caddy em ambos os nós balanceia automaticamente 50/50 pela rede privada!)"
