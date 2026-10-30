#!/usr/bin/env bash
# 80 — Instala (ou atualiza) o painel dos nós (uv-painel, docs/operacao/PAINEL.md) nos nós A e B. Idempotente.
#
# Pré-requisitos:
#   * ./compilar_local_arm64.sh já rodado (o pacote traz o uv-painel);
#   * no nos.env: URL_DOMINIO (o mesmo do 02), PAINEL_GOOGLE_ID, PAINEL_GOOGLE_SEGREDO e PAINEL_EMAILS (quem entra);
#   * no Google Cloud (cliente OAuth "Aplicativo da Web"): URI de redirecionamento https://<URL_DOMINIO>/painel/oauth/retorno.
#
# O segredo comum dos dois painéis (sessão e assinatura entre os nós) é gerado uma vez em gerado/painel.segredo.
# Não mexe no urna-velox.service: só cria o usuário uvpainel, o sudo restrito, a porta 8095 para o par, o serviço
# uv-painel e a rota /painel do Caddy (o 02 é chamado no fim; ele reinicia o Caddy, um piscar na página pública).
#
# Uso: ./80_painel.sh [--sem-borda]
source "$(dirname "$0")/lib.sh"

BORDA=1; [ "${1:-}" = "--sem-borda" ] && BORDA=0
: "${URL_DOMINIO:?defina URL_DOMINIO no nos.env (o domínio do 02_configurar_borda.sh)}"
: "${PAINEL_GOOGLE_ID:?defina PAINEL_GOOGLE_ID no nos.env (cliente OAuth do Google)}"
: "${PAINEL_GOOGLE_SEGREDO:?defina PAINEL_GOOGLE_SEGREDO no nos.env}"
: "${PAINEL_EMAILS:?defina PAINEL_EMAILS no nos.env (e-mails autorizados, por vírgula)}"
G="$AQUI/gerado"; mkdir -p "$G"; chmod 0700 "$G"
PACOTE="$G/uv-binarios-aarch64.tar.gz"
[ -f "$PACOTE" ] || erro "falta $PACOTE — rode ./compilar_local_arm64.sh"
tar -tzf "$PACOTE" bin/uv-painel >/dev/null 2>&1 || erro "o pacote não tem bin/uv-painel — rode ./compilar_local_arm64.sh de novo"

if [ ! -s "$G/painel.segredo" ]; then
  msg "gerando o segredo comum dos painéis (gerado/painel.segredo)"
  ( umask 077; openssl rand -base64 48 | tr -d '\n' > "$G/painel.segredo" )
fi
SEGREDO=$(cat "$G/painel.segredo")

T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
tar -xzf "$PACOTE" -C "$T" bin/uv-painel
for f in uv-painel-acao uv-painel-iniciar.sh uv-painel.service preparar_painel.sh uv-iniciar.sh; do cp "$AQUI/servidor/$f" "$T/"; done
cp "$T/bin/uv-painel" "$T/uv-painel"

for N in A B; do
  OUTRO=$(outro_de "$N"); PAR=$(no_var "$OUTRO" IP_PRIVADO)
  msg "nó $N: painel.env (par $OUTRO em $PAR:8095)"
  ( umask 077; cat > "$T/painel.env" <<ENV
# gerado por 80_painel.sh — segredos: 0640 root:uvpainel no servidor
UV_PAINEL_URL=https://$URL_DOMINIO
UV_PAINEL_GOOGLE_ID=$PAINEL_GOOGLE_ID
UV_PAINEL_GOOGLE_SEGREDO=$PAINEL_GOOGLE_SEGREDO
UV_PAINEL_SEGREDO=$SEGREDO
UV_PAINEL_EMAILS=$PAINEL_EMAILS
UV_PAINEL_PARES=http://$PAR:8095
ENV
  )
  em "$N" "rm -rf /tmp/uv-painel && mkdir -m 0700 -p /tmp/uv-painel"
  for f in uv-painel uv-painel-acao uv-painel-iniciar.sh uv-painel.service preparar_painel.sh uv-iniciar.sh painel.env; do
    para "$N" "$T/$f" "/tmp/uv-painel/$f"
  done
  msg "nó $N: instalando"
  em "$N" "sudo /opt/urna-velox/bin/uv-operar.sh bash /tmp/uv-painel/preparar_painel.sh /tmp/uv-painel $PAR; R=\$?; rm -rf /tmp/uv-painel; exit \$R" \
    || erro "nó $N: a instalação do painel falhou"
done

if [ "$BORDA" = 1 ]; then
  msg "borda: Security List (porta 8095 na sub-rede) e a rota /painel do Caddy nos dois nós"
  "$AQUI/02_configurar_borda.sh" "$URL_DOMINIO"
fi

# os dois painéis têm de se enxergar pela rede privada (B-019): cada um busca a visão do outro na 8095
FALHOU_PAR=0
for N in A B; do
  PAR=$(no_var "$(outro_de "$N")" IP_PRIVADO)
  if em "$N" "curl -fsS -m 5 -o /dev/null http://$PAR:8095/painel/vivo"; then echo "nó $N → painel do par ($PAR:8095): ok"
  else aviso "nó $N não alcança o painel do par em $PAR:8095 (Security List da sub-rede, iptables do par ou uv-painel parado)"; FALHOU_PAR=1; fi
done
[ "$FALHOU_PAR" = 0 ] || aviso "o painel vai mostrar o outro nó como \"fora do alcance\" até isso ser resolvido"

msg "pronto: https://$URL_DOMINIO/painel/ (entra com o Google: $PAINEL_EMAILS)"
