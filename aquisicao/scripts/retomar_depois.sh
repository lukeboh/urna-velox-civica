#!/usr/bin/env bash
# Espera até um horário (HH:MM) e então retoma a coleta pulando as seções sem BU conhecidas.
# Uso: retomar_depois.sh HH:MM rótulo   (respeita a quarentena de ≥10 min depois de um HTTP 429)
set -euo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
alvo=$(date -d "$1" +%s); agora=$(date +%s)
[ "$alvo" -gt "$agora" ] && { echo "aguardando até $1 ($((alvo - agora)) s)"; sleep $((alvo - agora)); }
date
UV_PULAR_SEM_BU="${UV_PULAR_SEM_BU:-$UV_WORK/metricas_nacional.csv}" bash "$(dirname "$0")/coleta_nacional.sh" "$2"
