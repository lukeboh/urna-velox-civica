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
    # B-025: as portas internas nunca abertas para a internet (a 8080 chegou a ficar, por uma regra criada à mão: o
    # uv-servico direto, sem o Caddy, sem o login dos comandos e sem a playlist). Só depois da regra da sub-rede, acima.
    OCI_COMPARTIMENTO="$OCI_COMPARTIMENTO" OCI_BUCKET="$OCI_BUCKET" OCI_NAMESPACE="${OCI_NAMESPACE:-}" \
        _oci_op fechar-portas 0.0.0.0/0 "8080,8090,8095,$FED_PORTA,$((FED_PORTA + 1))" "$(ocid_de A)" "$(ocid_de B)" \
        || erro "não consegui fechar para a internet as portas internas (veja a mensagem acima)"
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
    
    # M-005: o endereço antigo (DuckDNS) aponta para o nó A: só ele responde o 301 para o domínio oficial.
    # DOMINIO_ANTIGO no nos.env troca o endereço; DOMINIO_ANTIGO= (vazio) desliga o redirecionamento.
    ANTIGO=""; [ "$N" = A ] && ANTIGO="${DOMINIO_ANTIGO-urnavelox.duckdns.org}"
    # Passo 3 do plano de borda: DOMINIO_DADOS no nos.env (ex.: dados.urnavelox.com.br, já no DNS) liga o site de
    # leitura atrás da Cloudflare nos dois nós; vazio, sem o site.
    # B-024: a playlist só roda no A; o B a leva ao HTTPS do A, e o A confia no B para o IP do visitante
    if [ "$N" = A ]; then PLAYLIST="local:$(no_var B IP_PUBLICO),$(no_var B IP_PRIVADO)"; else PLAYLIST="remoto:$(no_var A IP_PUBLICO)"; fi
    para "$N" "$SCRIPT_DIR/servidor/Caddyfile.template" "/tmp/Caddyfile.template"
    para "$N" "$SCRIPT_DIR/servidor/gerar_caddyfile.sh" "/tmp/gerar_caddyfile.sh"
    para "$N" "$SCRIPT_DIR/servidor/instalar_caddy.sh" "/tmp/instalar_caddy.sh"
    
    # "bash -s": sem ele o Run Command recebe o comando vazio e nada é instalado (B-015)
    em "$N" "bash -s" <<CMD || erro "nó $N: a instalação do Caddy falhou"
sudo /opt/urna-velox/bin/uv-operar.sh bash /tmp/instalar_caddy.sh "$DOMINIO" "$IP_PAR" "$ANTIGO" "$PLAYLIST" "${DOMINIO_DADOS:-}"
rm -f /tmp/instalar_caddy.sh /tmp/Caddyfile.template /tmp/gerar_caddyfile.sh
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
