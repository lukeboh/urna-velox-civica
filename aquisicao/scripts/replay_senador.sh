#!/usr/bin/env bash
# Replay da noite do Brasil publicando só Senador (x0 e x60) e, para comparação, Presidente + Governador + Senador (x60).
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
S="$(dirname "$0")/replay_noite.sh"
export UV_CATALOGO="$UV_WORK/resultado/catalogo.txt"
UV_CARGOS_PUB=5 bash "$S" "$UV_WORK/dados" 0 "$UV_WORK/replay-senador/x0"
UV_CARGOS_PUB=5 bash "$S" "$UV_WORK/dados" 60 "$UV_WORK/replay-senador/x60"
UV_CARGOS_PUB=1,3,5 bash "$S" "$UV_WORK/dados" 60 "$UV_WORK/replay-maj/x60"
echo FIM
