#!/usr/bin/env bash
# Passo 5 — coleta sustentada de várias UFs pela API, com controle adaptativo de taxa (AIMD).
#   padrão: UFs de UV_UFS_EXTRA (ac,al,ap,pb,rn,rr,se,to ≈ 42,9 mil seções, ~16 min)
#   UV_NACIONAL=1: todas as UFs (~517 mil seções, ~9 GB; várias horas no teto de ~100 req/s)
# Retoma de onde parou: seções já presentes nos pacotes são puladas.
source "$(dirname "$0")/comum.sh"
if [ "$UV_NACIONAL" = "1" ]; then UFS=todas; ROT=nacional; else UFS="$UV_UFS_EXTRA"; ROT=multi; fi
titulo "Coleta sustentada ($ROT): $UFS"
"$UC" baixar "${ALVO[@]}" --ufs "$UFS" -c 128 "${ADAPT[@]}" --dados "$DADOS" \
   --rotulo "$ROT" --resumo "$RESUMO" --metricas "$UV_WORK/metricas_$ROT.csv" 2>&1 | grep -vE "^\s+\[ *[0-9.]+ s\] [0-9]+/[0-9]+ seções" || true
cp "$UV_WORK/metricas_$ROT.taxa.csv" "$UV_OUT/taxa_$ROT.csv" 2>/dev/null
cp "$UV_WORK/metricas_$ROT.inicios.csv" "$UV_OUT/inicios_$ROT.csv" 2>/dev/null
# guarda nos resultados as curvas agregadas (o CSV por seção fica em UV_WORK):
#   coleta por segundo e chegada dos BUs ao TSE na noite da eleição (campo de recebimento do aux.json), por minuto
"$PY" - "$UV_WORK/metricas_$ROT.csv" "$UV_OUT" "$ROT" <<'PY'
import csv, sys, collections, datetime as dt
c = collections.Counter(); b = collections.Counter(); e = collections.Counter(); m = collections.Counter()
for r in csv.DictReader(open(sys.argv[1])):
    s = int(float(r["t_s"])); c[s] += 1; b[s] += int(r["bytes_bu"] or 0); e[s] += bool(r["erro"])
    if r["recebido_tse"].strip() and not r["erro"]:
        m[dt.datetime.strptime(r["recebido_tse"].strip(), "%d/%m/%Y %H:%M:%S").strftime("%Y-%m-%d %H:%M")] += 1
rot = sys.argv[3]
with open(f"{sys.argv[2]}/coleta_{rot}_por_segundo.csv", "w") as f:
    f.write("segundo,secoes,bytes,erros\n")
    for s in range(max(c) + 1): f.write(f"{s},{c[s]},{b[s]},{e[s]}\n")
with open(f"{sys.argv[2]}/chegada_{rot}_por_minuto.csv", "w") as f:
    f.write("minuto,bus\n")
    for k in sorted(m): f.write(f"{k},{m[k]}\n")
PY
