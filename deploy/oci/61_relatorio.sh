#!/usr/bin/env bash
# 61 — Relatório do simulado: conferência com os oficiais, HTTP 429 e a ordem de totalização × recebimento no TSE.
# Traz de cada nó a saude.json, a conferencia.json e o diário de seções (secoes.log, compactado) para a pasta local e
# mede a ordem aqui (nada de script na nuvem). Uso: ./61_relatorio.sh [pasta-local] [--ordem ordem.csv]
source "$(dirname "$0")/lib.sh"
P3220=""   # /mnt/d/dados/urna-velox/pleitos/2026-10-04-3220 (AAAA-MM-DD-código), só se faltar argumento
if [ -z "${1:-}" ] || [ -z "${2:-}" ]; then P3220="$(pasta_pleito 3220)"; fi
DEST="${1:-$P3220/simulado/resultado-$(date +%Y%m%d-%H%M)}"
ORDEM="${2:-$P3220/simulado/ordem_3220.csv}"
SCRIPT="$(cd "$AQUI/../.." && pwd)/aquisicao/scripts/ordem_totalizacao.py"
mkdir -p "$DEST"
for N in A B; do
  msg "nó $N: trazendo saude.json, conferencia.json e secoes.log"
  mkdir -p "$DEST/no-$N"
  for f in saude.json conferencia.json alertas.csv; do
    if em "$N" "test -s /srv/urna-velox/saida/$f"; then
      de "$N" "/srv/urna-velox/saida/$f" "$DEST/no-$N/$f"
    else
      rm -f "$DEST/no-$N/$f"
    fi
  done
  em "$N" "gzip -c /srv/urna-velox/saida/secoes.log > /tmp/uv-secoes.log.gz && chmod 0644 /tmp/uv-secoes.log.gz"
  de "$N" /tmp/uv-secoes.log.gz "$DEST/no-$N/secoes.log.gz"
  em "$N" "rm -f /tmp/uv-secoes.log.gz"
  gunzip -f "$DEST/no-$N/secoes.log.gz"
  python3 - "$DEST/no-$N" <<'PY'
import json, os, sys
d = sys.argv[1]
try:
    c = json.load(open(os.path.join(d, 'conferencia.json')))
    print(f"conferência: {c['identicos']}/{c['arquivos']} idênticos ao oficial · situações divergentes {c['situacoes']['divergentes']} · revogações {c['matematicamente']['revogacoes']}")
except Exception as e:
    print(f"conferência: ainda não ({e.__class__.__name__}) — o serviço confere ao fim da apuração")
h = json.load(open(os.path.join(d, 'saude.json')))
r, s = h['rede'], h['secoes']
print(f"aplicadas {s['aplicadas']}/{s['catalogo']} · aux {r['aux']} · BUs baixados {r['bu_baixados']} · HTTP 429 {r['http429']} · duplicados {h['motor']['duplicados']}")
PY
done
msg "ordem de totalização × recebimento no TSE"
python3 -I "$SCRIPT" "$DEST/no-A/secoes.log" "$DEST/no-B/secoes.log" --ordem "$ORDEM" --md "$DEST/ORDEM.md"
msg "arquivos em $DEST (ORDEM.md, no-A/, no-B/)"
