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
[ -n "${EXTRA:-}" ] && read -r -a X <<< "$EXTRA" && ARGS+=("${X[@]}")
exec /opt/urna-velox/bin/uv-servico "${ARGS[@]}"
