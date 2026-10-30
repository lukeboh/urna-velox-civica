#!/usr/bin/env bash
# Chamado pelo systemd (urna-velox.service): monta a linha de comando do uv-servico a partir de /etc/urna-velox/no.env.
# Retomada: depois de qualquer queda o systemd chama de novo, e o serviço relê os pacotes e o diário (nada se perde).
#
# Multipleito (1.1): o uv-ciclo.sh (modo aovivo, pleito em curso descoberto no ele-c.json) chama este mesmo script com
# UV_PLEITO, UV_CICLO, UV_CANDIDATOS, UV_DADOS, UV_SAIDA, UV_ARMAZEM, UV_UFS e UV_EXTRA, que valem por cima do no.env:
# a configuração do nó (servidor, req/s, federação, IP de saída, HTTP) continua a da produção.
set -euo pipefail
source /etc/urna-velox/no.env
# no.env sem PLEITO (multipleito, §3.6 do HANDOFF_1.1.0): quem escolhe o pleito é o uv-ciclo.sh; sem ele, o serviço
# não sobe (sai com 78, que o urna-velox.service não reinicia)
PLEITO="${UV_PLEITO:-${PLEITO:-}}"; CICLO="${UV_CICLO:-${CICLO:-ele2026}}"; CANDIDATOS="${UV_CANDIDATOS:-${CANDIDATOS:-}}"
UFS="${UV_UFS:-${UFS:-}}"; EXTRA="${EXTRA:-}${UV_EXTRA:+ $UV_EXTRA}"
if ! [[ "$PLEITO" =~ ^[0-9]+$ ]]; then
  echo "uv-iniciar: sem pleito (no.env sem PLEITO e sem UV_PLEITO): no multipleito quem escolhe o pleito é o uv-ciclo.sh" >&2
  exit 78
fi
ARGS=(--pleito "$PLEITO" --ciclo "$CICLO" --ambiente "$AMBIENTE" --base "$BASE"
  --candidatos "${CANDIDATOS:-/srv/urna-velox/preparo/candidatos.json}"
  --dados "${UV_DADOS:-/srv/urna-velox/dados}" --saida "${UV_SAIDA:-/srv/urna-velox/saida}" --rps "$RPS"
  # releitura dos cs.json com seções chegando (INTERVALO_CS no no.env; padrão 30 s, o mesmo do uv-servico). O ritmo
  # ocioso (--intervalo-cs-ocioso) e o --tse-intervalo continuam os do binário
  --intervalo-cs "${INTERVALO_CS:-30}"
  --federacao /etc/urna-velox/federacao.json --no "$NO_ID" --armazem "${UV_ARMAZEM:-/srv/urna-velox/federacao}"
  --federacao-escuta "0.0.0.0:$FED_PORTA")
[ -n "${ECO_IP:-}" ] && ARGS+=(--eco-ip "$ECO_IP")
[ -n "${HTTP_LOCAL:-}" ] && ARGS+=(--http "$HTTP_LOCAL")
[ -n "${ACELERACAO:-}" ] && ARGS+=(--aceleracao-local "$ACELERACAO")
[ -n "${IP_LOCAL:-}" ] && ARGS+=(--ip-local "$IP_LOCAL")
[ -n "${UFS:-}" ] && ARGS+=(--ufs "$UFS")
# fila de coleta na ordem de recebimento dos BUs no TSE (métricas com recebido_tse; 60_simulado.sh)
[ -n "${ORDEM:-}" ] && ARGS+=(--ordem "$ORDEM")
# caminhos do site de resultados (painel dos nós, docs/operacao/PAINEL.md): só se o arquivo existir e o binário souber ler
if [ -s /etc/urna-velox/rotas_tse.json ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--rotas'; then
  ARGS+=(--rotas /etc/urna-velox/rotas_tse.json)
fi
# modo espelho (ACESSO=interno: servidor espelho dos arquivos, fora da borda pública): teto calibrado pelo painel; 0 ou vazio = sem limite. O uv-servico
# recusa interno para resultados/cdn.tse.jus.br (lá vale sempre o limite de 100 req/s por órgão)
if [ "${ACESSO:-borda}" = interno ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--acesso'; then
  ARGS+=(--acesso interno --teto-interno "${TETO_RPS_INTERNO:-0}")
fi
# multipleito (1.1): a lista de pleitos do ele-c.json na página (/api/v1/pleitos), do uv-pleitos.sh
if [ -s "${PLEITOS_JSON:-/srv/urna-velox/pleitos/pleitos.json}" ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--pleitos'; then
  ARGS+=(--pleitos "${PLEITOS_JSON:-/srv/urna-velox/pleitos/pleitos.json}")
fi
# multipleito (1.1): as séries de consulta dos pleitos encerrados (/consulta/*, geradas pelo uv-pleitos.sh)
if [ -d "${CONSULTA_DIR:-/srv/urna-velox/consulta}" ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--consulta'; then
  ARGS+=(--consulta "${CONSULTA_DIR:-/srv/urna-velox/consulta}")
fi
[ -n "${EXTRA:-}" ] && read -r -a X <<< "$EXTRA" && ARGS+=("${X[@]}")
exec /opt/urna-velox/bin/uv-servico "${ARGS[@]}"
