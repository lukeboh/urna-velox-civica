#!/usr/bin/env bash
# Senador: arquivos oficiais → comparação com os totais do Urna Velox (Presidente, Governador e Senador).
# Uso: conferir_senador.sh [pasta de resultado com totais_uf.csv]
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
AQ="$(cd "$(dirname "$0")/.." && pwd)"
RES="${1:-$UV_WORK/resultado}"
PY="$UV_WORK/.venv/bin/python3"; [ -x "$PY" ] || PY=python3
echo "arquivos oficiais de Senador: $(ls "$UV_WORK"/oficial/*-c0005-e006259-u.json 2>/dev/null | wc -l) de 27"
"$PY" - "$UV_WORK/oficial/df-c0005-e006259-u.json" <<'EOF'
import json, sys
o = json.load(open(sys.argv[1], encoding="utf-8")); c = o["carg"][0]
print("exemplo DF:", c.get("nmn"), "| vagas (nv):", c.get("nv"), "| comparecimento:", o["e"]["c"], "| brancos:", o["v"]["vb"], "| nulos:", o["v"]["vn"], "| nulos técnicos:", o["v"]["vnt"])
EOF
MET=$(ls "$UV_WORK"/metricas_*.csv 2>/dev/null | grep -v -E 'inicios|taxa' | paste -sd,)
"$PY" "$AQ/scripts/comparar_oficial.py" --totais "$RES/totais_uf.csv" --oficial "$UV_WORK/oficial" --dados "$UV_WORK/dados" \
  --metricas "$MET" --cargos "${UV_CARGOS:-1,3,5}" --saida "$RES/comparacao_oficial.md" > /dev/null
echo "comparação: código $? (0 = sem divergências) → $RES/comparacao_oficial.md"
grep -E "Abrangências com divergência|candidatos com a mesma situação" "$RES/comparacao_oficial.md"
grep -E "^\| Senador" "$RES/comparacao_oficial.md" | head -30
