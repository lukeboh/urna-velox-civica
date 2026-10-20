#!/usr/bin/env bash
# Chamado pelo systemd (urna-velox.service): monta a linha de comando do uv-servico a partir de /etc/urna-velox/no.env.
# Retomada: depois de qualquer queda o systemd chama de novo, e o serviço relê os pacotes e o diário (nada se perde).
set -euo pipefail
source /etc/urna-velox/no.env
ARGS=(--pleito "$PLEITO" --ciclo "$CICLO" --ambiente "$AMBIENTE" --base "$BASE"
  --candidatos "${CANDIDATOS:-/srv/urna-velox/preparo/candidatos.json}"
  --dados /srv/urna-velox/dados --saida /srv/urna-velox/saida --rps "$RPS"
  --federacao /etc/urna-velox/federacao.json --no "$NO_ID" --armazem /srv/urna-velox/federacao
  --federacao-escuta "0.0.0.0:$FED_PORTA")
[ -n "${ECO_IP:-}" ] && ARGS+=(--eco-ip "$ECO_IP")
[ -n "${HTTP_LOCAL:-}" ] && ARGS+=(--http "$HTTP_LOCAL")
[ -n "${ACELERACAO:-}" ] && ARGS+=(--aceleracao-local "$ACELERACAO")
[ -n "${IP_LOCAL:-}" ] && ARGS+=(--ip-local "$IP_LOCAL")
[ -n "${UFS:-}" ] && ARGS+=(--ufs "$UFS")
# fila de coleta na ordem de recebimento dos BUs no TSE (métricas com recebido_tse; 60_simulado.sh)
[ -n "${ORDEM:-}" ] && ARGS+=(--ordem "$ORDEM")
# caminhos do site de resultados (painel dos nós, docs/PAINEL.md): só se o arquivo existir e o binário souber ler
if [ -s /etc/urna-velox/rotas_tse.json ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--rotas'; then
  ARGS+=(--rotas /etc/urna-velox/rotas_tse.json)
fi
# modo espelho (ACESSO=interno: servidor espelho dos arquivos, fora da borda pública): teto calibrado pelo painel; 0 ou vazio = sem limite. O uv-servico
# recusa interno para resultados/cdn.tse.jus.br (lá vale sempre o limite de 100 req/s por órgão)
if [ "${ACESSO:-borda}" = interno ] && /opt/urna-velox/bin/uv-servico --help 2>/dev/null | grep -q -- '--acesso'; then
  ARGS+=(--acesso interno --teto-interno "${TETO_RPS_INTERNO:-0}")
fi
[ -n "${EXTRA:-}" ] && read -r -a X <<< "$EXTRA" && ARGS+=("${X[@]}")
exec /opt/urna-velox/bin/uv-servico "${ARGS[@]}"
