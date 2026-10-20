#!/usr/bin/env bash
# Chamado pelo systemd (uv-painel.service): monta a linha de comando do uv-painel a partir do /etc/urna-velox/painel.env
# (já no ambiente) e do no.env (id do nó). Os segredos ficam no ambiente, nunca na linha de comando.
set -euo pipefail
NO_ID=$(sed -nE 's/^NO_ID=//p' /etc/urna-velox/no.env | tail -n1)
ARGS=(--no "${UV_PAINEL_NO:-$NO_ID}" --escuta "${UV_PAINEL_ESCUTA:-0.0.0.0:8095}")
for P in ${UV_PAINEL_PARES:-}; do ARGS+=(--par "$P"); done
exec /opt/urna-velox/bin/uv-painel "${ARGS[@]}"
