#!/usr/bin/env bash
# Multipleito (1.1): monta sozinho o acervo de replay de um pleito encerrado (docs/operacao/CICLO_DIARIO.md, "Acervo
# automático"): tudo o que o uv-ciclo.sh precisa para repetir a noite no uv-simulador, sem nenhuma requisição ao TSE
# durante os replays.
#
#   montar_acervo.sh <pleito> <destino> [pleitos.json]
#
# <destino>/ (o mesmo leiaute dos acervos montados à mão, /srv/urna-velox/acervo/<pleito>):
#   dados/<uf>.uvpk          BUs (uv-coletor baixar: aux.json + bu.dat de cada seção)
#   cs/<uf>-p<pleito>-cs.json  seções de cada UF
#   linha-do-tempo.csv       métricas da coleta (recebido_tse = recebimento do BU no TSE, do aux.json)
#   oficial/                 -u.json finais do TSE (os do preparar, com a destinação final dos candidatos)
#   candidatos.json          catálogo do pleito (uv-coletor preparar, sem --ignorar-destinacao: B-005)
#   acervo.json              pleito, ciclo, nomes, data, corte e duração da cadência (lido pelo uv-ciclo.sh)
#   .completo                criado por último: o acervo está pronto para a playlist
#
# Regras do CLAUDE.md: ≤ 95 req/s ao TSE (RPS), pausa e saída no HTTP 429 (o disjuntor do uv-coletor); BU já baixado
# não é baixado de novo (retomável: rodar de novo continua de onde parou); nunca durante a apuração de uma eleição
# (o uv-pleitos.sh só chama fora da noite do pleito e do dia seguinte). Pleito com mais de MAX_SECOES seções (as
# ordinárias inteiras) só com FORCAR=1: um suplementar tem dezenas de seções; uma ordinária, meio milhão.
#
# Variáveis: UV_COLETOR (binário; padrão: o do PATH), BASE (https://resultados.tse.jus.br), RPS (95), MAX_SECOES (20000),
# FORCAR (1 = sem o limite de seções).
set -euo pipefail
P="${1:?uso: montar_acervo.sh <pleito> <destino> [pleitos.json]}"
D="${2:?uso: montar_acervo.sh <pleito> <destino> [pleitos.json]}"
PJ="${3:-}"
C="${UV_COLETOR:-uv-coletor}"
BASE="${BASE:-https://resultados.tse.jus.br}"
RPS="${RPS:-95}"
MAX_SECOES="${MAX_SECOES:-20000}"
log() { echo "$(date -Iseconds) [acervo $P] $*" >&2; }
[ -f "$D/.completo" ] && { log "já completo em $D"; exit 0; }
mkdir -p "$D/cs" "$D/dados" "$D/metricas"

# 1. o pleito no pleitos.json (ou no ele-c.json, pelo uv-coletor pleitos): ciclo, UFs, nomes, data
if [ -z "$PJ" ]; then PJ="$D/pleitos.json"; "$C" pleitos --base "$BASE" --saida "$PJ" >/dev/null; fi
INFO=$(python3 - "$PJ" "$P" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
p = next((x for x in d['pleitos'] if str(x['codigo']) == sys.argv[2]), None)
if p is None: sys.exit(f'pleito {sys.argv[2]} não está em {sys.argv[1]}')
if p['situacao'] != 'encerrado': sys.exit(f"pleito {sys.argv[2]} está {p['situacao']}: o acervo é só de pleito encerrado")
ufs = 'todas' if 'br' in p['ufs'] else ','.join(p['ufs'])
print(p['ciclo'], ufs, sep='\t')
PY
) || { log "$INFO"; exit 2; }
CICLO=$(cut -f1 <<<"$INFO"); UFS=$(cut -f2 <<<"$INFO")
ALVO=(--base "$BASE" --ciclo "$CICLO" --pleito "$P")
log "ciclo $CICLO, UFs $UFS"

# 2. seções (cs.json) e o limite de tamanho
"$C" secoes "${ALVO[@]}" --ufs "$UFS" --cache "$D/cs" | tee "$D/secoes.txt" >&2
N=$(sed -n 's/^total: \([0-9]*\) seções.*/\1/p' "$D/secoes.txt")
[ -n "$N" ] && [ "$N" -gt 0 ] || { log "nenhuma seção nos cs.json"; exit 3; }
if [ "$N" -gt "$MAX_SECOES" ] && [ "${FORCAR:-0}" != 1 ]; then
  log "$N seções (> MAX_SECOES=$MAX_SECOES): pleito grande, monte com FORCAR=1 (combinar antes: horas de coleta e dezenas de GB)"
  exit 4
fi

# 3. BUs (retomável: o uv-coletor pula o que já está nos pacotes) e a linha do tempo
# numa retomada, as seções que já deram 404 no aux.json (agregadas, sem BU próprio) ou "Não instalada" não são pedidas
# de novo: num pleito encerrado isso não muda (e milhares de 404 seguidos provocam 429, CLAUDE.md)
ANTES=$(ls "$D"/metricas/coleta-*.csv 2>/dev/null | grep -v '\.inicios\.csv$' | paste -sd, || true)
PULAR=(); [ -n "$ANTES" ] && PULAR=(--pular-sem-bu "$ANTES")
"$C" baixar "${ALVO[@]}" --ufs "$UFS" --dados "$D/dados" --metricas "$D/metricas/coleta-$(date +%Y%m%d-%H%M%S).csv" \
  --rps "$RPS" -c 16 "${PULAR[@]}" >&2
cp -n "$D"/dados/config/*-cs.json "$D/cs/" 2>/dev/null || true
# as métricas de todas as execuções (a última vence, por seção), ordenadas pelo recebimento no TSE
# (o uv-coletor grava ao lado um <nome>.inicios.csv, requisições por segundo, que não entra)
python3 - "$D/linha-do-tempo.csv" $(ls "$D"/metricas/coleta-*.csv | grep -v '\.inicios\.csv$') <<'PY'
import csv, sys
from datetime import datetime
cab, por = None, {}
for f in sorted(sys.argv[2:]):
    with open(f, newline='') as h:
        r = csv.DictReader(h)
        cab = cab or r.fieldnames
        for x in r: por[(x['uf'], x['municipio'], x['zona'], x['secao'])] = x
def chave(x):
    try: return (0, datetime.strptime(x['recebido_tse'], '%d/%m/%Y %H:%M:%S'))
    except (ValueError, KeyError): return (1, datetime.max)
with open(sys.argv[1], 'w', newline='') as h:
    w = csv.DictWriter(h, fieldnames=cab)
    w.writeheader()
    for x in sorted(por.values(), key=chave): w.writerow(x)
print(len(por), 'seções na linha do tempo', file=sys.stderr)
PY

# 4. candidatos.json com a destinação final e os -u.json oficiais (o preparar guarda em fontes/u)
"$C" preparar "${ALVO[@]}" --saida "$D" --fontes "$D/fontes" >&2
rm -rf "$D/oficial"; mkdir -p "$D/oficial"
cp "$D"/fontes/u/*-u.json "$D/oficial/" 2>/dev/null || cp "$D"/fontes/*-u.json "$D/oficial/"

# 5. acervo.json (o que o uv-ciclo.sh precisa saber de um pleito que não está na lista fixa) e .completo
python3 - "$PJ" "$P" "$D" <<'PY'
import csv, json, sys
from datetime import datetime, timedelta
d = json.load(open(sys.argv[1]))
p = next(x for x in d['pleitos'] if str(x['codigo']) == sys.argv[2])
dia = datetime.strptime(p['data'], '%d/%m/%Y')
rec = []
with open(f"{sys.argv[3]}/linha-do-tempo.csv", newline='') as h:
    for x in csv.DictReader(h):
        try: rec.append(datetime.strptime(x['recebido_tse'], '%d/%m/%Y %H:%M:%S'))
        except (ValueError, KeyError): pass
rec.sort()
# cadência a 1× até 99 % dos BUs recebidos (contando das 17h), com 15 min de folga; depois o nó A acelera
inicio = dia.replace(hour=17)
noite = [r for r in rec if dia <= r < dia + timedelta(days=1, hours=2)]
p99 = noite[min(len(noite) - 1, int(len(noite) * 0.99))] if noite else inicio + timedelta(hours=2)
cadencia = max(1800, int((p99 - inicio).total_seconds()) + 900)
a = {'pleito': p['codigo'], 'ciclo': p['ciclo'], 'nome': p['nome'], 'titulo': p['titulo'], 'data': p['data'],
     'corte': (dia + timedelta(days=1)).strftime('%d/%m/%Y') + ' 02:00:00', 'tempo_cadencia_s': cadencia,
     'secoes': len(rec), 'montado': datetime.now().isoformat(timespec='seconds')}
json.dump(a, open(f"{sys.argv[3]}/acervo.json", 'w'), ensure_ascii=False, indent=1)
print(json.dumps(a, ensure_ascii=False), file=sys.stderr)
PY
[ -s "$D/candidatos.json" ] && ls "$D"/dados/*.uvpk >/dev/null 2>&1 && ls "$D"/oficial/*-u.json >/dev/null 2>&1 \
  || { log "acervo incompleto (candidatos, pacotes ou oficiais faltando)"; exit 5; }
touch "$D/.completo"
log "pronto em $D"
