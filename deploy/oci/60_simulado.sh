#!/usr/bin/env bash
# 60 — Simulado de uma eleição já encerrada contra a CDN do TSE, na ordem de recebimento dos BUs no TSE.
#
# Para os dois nós, guarda a saída anterior (não apaga: move para /srv/urna-velox/anteriores/<data>), envia o
# candidatos.json do pleito e o arquivo de ordem (recebido_tse de cada seção, do aux.json), troca o pleito no no.env
# (a cópia original fica em /etc/urna-velox/no.env.antes-simulado) e inicia os dois nós. Contra uma eleição encerrada
# todas as seções já estão publicadas: sem o arquivo de ordem a coleta seguiria os cs.json (disponibilização), e até
# a 0.6.0 ia UF por UF em ordem alfabética.
#
#   ./60_simulado.sh --confirmo-tse --combinado [--pleito 3220] [--insumos <pasta>]
#   (insumos por padrão: <pasta do pleito>/simulado, com a pasta do pleito em /mnt/d/dados/urna-velox/pleitos/
#   no formato AAAA-MM-DD-código, ex.: 2026-10-04-3220; ver pasta_pleito no lib.sh)
#   ./60_simulado.sh --restaurar      # para os nós e devolve o no.env original (pleito de produção)
#
# Depois: ./30_verificar.sh (andamento) e, no fim, ./61_relatorio.sh (conferência, 429 e ordem de totalização).
# Atenção (CLAUDE.md): contra o TSE, ≤ 95 req/s no IP do nó que coleta; baixa de novo os BUs do pleito (combinado
# com o Luciano em 15/10/2026 para o simulado de 16/10).
source "$(dirname "$0")/lib.sh"
CONF=0; COMB=0; REST=0; SPLEITO=3220; INS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --confirmo-tse) CONF=1 ;; --combinado) COMB=1 ;; --restaurar) REST=1 ;;
    --pleito) SPLEITO="$2"; shift ;; --insumos) INS="$2"; shift ;;
    *) erro "opção desconhecida: $1" ;;
  esac
  shift
done
[ "$REST" = 1 ] || [ -n "$INS" ] || INS="$(pasta_pleito "$SPLEITO")/simulado"

if [ "$REST" = 1 ]; then
  for N in A B; do
    msg "nó $N: parando e restaurando o no.env de produção"
    em "$N" "sudo systemctl stop urna-velox; if [ -f /etc/urna-velox/no.env.antes-simulado ]; then sudo mv /etc/urna-velox/no.env.antes-simulado /etc/urna-velox/no.env; echo restaurado; else echo 'sem no.env.antes-simulado: nada a restaurar'; fi; grep -E '^(PLEITO|ORDEM|CANDIDATOS)=' /etc/urna-velox/no.env || true"
    liberar_manual "$N"
  done
  aviso "a saída do simulado ficou em /srv/urna-velox/saida; antes da produção, mova-a (o serviço retomaria dela)"
  exit 0
fi

[ "$CONF" = 1 ] || erro "o simulado roda contra o site real do TSE ($BASE): confirme com --confirmo-tse"
[ "$COMB" = 1 ] || erro "antes de 26/10/2026 nada roda contra o TSE sem combinar: use --combinado só se foi combinado"
for f in candidatos.json "ordem_$SPLEITO.csv"; do [ -s "$INS/$f" ] || erro "falta $INS/$f"; done
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); sys.exit(0 if str(d.get('pleito'))==sys.argv[2] else 1)" "$INS/candidatos.json" "$SPLEITO" \
  || erro "$INS/candidatos.json não é do pleito $SPLEITO"
grep -Eq '"agregada":[[:space:]]*true' "$INS/candidatos.json" \
  || erro "$INS/candidatos.json não marca as seções agregadas (gerado antes da 0.4.9): a apuração nunca chegaria a 100 %"

DATA=$(date +%Y%m%d-%H%M%S)
# trava antes de parar: o uv-atualizador não pode religar o serviço no meio da troca de pastas nem derrubá-lo depois
for N in A B; do travar_manual "$N" "60_simulado-$SPLEITO"; done
for N in A B; do
  msg "nó $N: parando e guardando a saída anterior em /srv/urna-velox/anteriores/$DATA"
  em "$N" "sudo systemctl stop urna-velox; sudo mkdir -p /srv/urna-velox/anteriores/$DATA /srv/urna-velox/simulado &&
           for d in saida dados federacao; do [ -e /srv/urna-velox/\$d ] && sudo mv /srv/urna-velox/\$d /srv/urna-velox/anteriores/$DATA/; done;
           sudo install -d -o urnavelox -g urnavelox /srv/urna-velox/saida /srv/urna-velox/dados /srv/urna-velox/federacao;
           df -h /srv | awk 'NR==2{print \"disco livre:\", \$4}'"
  msg "nó $N: candidatos.json e ordem do pleito $SPLEITO"
  para "$N" "$INS/candidatos.json" /tmp/uv-sim-candidatos.json
  para "$N" "$INS/ordem_$SPLEITO.csv" /tmp/uv-sim-ordem.csv
  em "$N" "sudo install -o urnavelox -m 0644 /tmp/uv-sim-candidatos.json /srv/urna-velox/simulado/candidatos.json &&
           sudo install -o urnavelox -m 0644 /tmp/uv-sim-ordem.csv /srv/urna-velox/simulado/ordem.csv &&
           { [ -f /etc/urna-velox/no.env.antes-simulado ] || sudo cp /etc/urna-velox/no.env /etc/urna-velox/no.env.antes-simulado; } &&
           sudo sed -i -E '/^(PLEITO|ORDEM|CANDIDATOS)=/d' /etc/urna-velox/no.env &&
           printf 'PLEITO=%s\nCANDIDATOS=/srv/urna-velox/simulado/candidatos.json\nORDEM=/srv/urna-velox/simulado/ordem.csv\n' '$SPLEITO' | sudo tee -a /etc/urna-velox/no.env >/dev/null &&
           grep -E '^(PLEITO|ORDEM|CANDIDATOS|BASE|RPS)=' /etc/urna-velox/no.env && /opt/urna-velox/bin/uv-servico --help | grep -q -- '--ordem' && echo 'binário com --ordem: ok'" \
    || erro "nó $N: o binário não tem --ordem (implante a 0.6.1+: ./compilar_local_arm64.sh && ./10_implantar.sh --sem-compilar)"
done

msg "iniciando os dois nós juntos"
em A "sudo systemctl start urna-velox" & em B "sudo systemctl start urna-velox" & wait
sleep 20
"$AQUI/30_verificar.sh"
msg "acompanhe com ./30_verificar.sh; no fim: ./61_relatorio.sh. Para voltar ao pleito de produção: ./60_simulado.sh --restaurar"
