#!/usr/bin/env bash
# uv-pleitos — multipleito (1.1): mantém o pleitos.json e os acervos de replay em dia, sozinho (docs/operacao/CICLO_DIARIO.md,
# "Multipleito"). Chamado pelo uv-pleitos.timer a cada 15 min, nos dois nós (cada um com o seu acervo, como hoje).
#
# 1. uv-coletor pleitos: relê o ele-c.json do TSE (uma requisição) e grava /srv/urna-velox/pleitos/pleitos.json, com a
#    situação de cada pleito e "replay" nos que têm o acervo pronto. A página (uv-servico --pleitos), a playlist e o
#    vigia leem esse arquivo; um pleito novo no ele-c.json aparece sem ninguém configurar.
# 2. Para cada pleito encerrado sem acervo (do mais novo para o mais antigo), montar_acervo.sh (≤ 95 req/s, retomável).
#    Nunca na noite de uma eleição nem no dia seguinte (regra do CLAUDE.md: nada de coleta durante a apuração sem
#    combinar): com algum pleito do dia, de ontem ou em curso, só o passo 1. No máximo MAX_POR_RODADA acervos por
#    rodada; pleito que falhou espera ESPERA_FALHA_H horas (o motivo fica em <acervo>/<pleito>/falhou); acima de
#    MAX_SECOES seções, nunca sozinho (uma ordinária inteira: combinar e montar com FORCAR=1).
# 2b. Catálogo pré-eleição dos agendados (§3.6 do HANDOFF_1.1.0): uv-coletor preparar (≤ 5 req/s, para no 1º HTTP 429)
#    para cada pleito agendado nos próximos PREPARO_DIAS dias sem <acervo>/<pleito>/preparo/candidatos.json completo,
#    assim que os cs.json saírem; as fontes ficam em <acervo>/<pleito>/fontes-aovivo, as mesmas que o modo aovivo do
#    uv-ciclo.sh usa na noite (e o candidatos.json daqui é o de reserva se o preparo da noite falhar). Tentativa sem
#    sucesso (cs.json ainda não publicados) ou com fontes incompletas (código 2) espera PREPARO_H horas.
# 3. Séries de consulta (o slider da página): gerar_consultas.sh para os acervos prontos sem série (replay local e
#    uv-serie, com nice; nenhuma requisição ao TSE), servidas pelo uv-servico --consulta. Também nunca na noite de uma
#    eleição nem no dia seguinte (o nó está totalizando ao vivo).
#
# Variáveis (opcionais, em /etc/urna-velox/ciclo.env ou no ambiente): PLEITOS_DIR, ACERVO_DIR, BASE, MAX_POR_RODADA (2),
# ESPERA_FALHA_H (6), MAX_SECOES (20000), MONTAR_ACERVOS (1; 0 = só o pleitos.json), CONSULTA_DIR
# (/srv/urna-velox/consulta), GERAR_CONSULTAS (1; 0 = não gera as séries), PREPARAR_AGENDADOS (1), PREPARO_DIAS (45),
# PREPARO_H (6), AMBIENTE_TSE (oficial).
set -uo pipefail
export TZ=America/Sao_Paulo
[ -f /etc/urna-velox/ciclo.env ] && source /etc/urna-velox/ciclo.env
BIN="${UV_BIN:-/opt/urna-velox/bin}"
PD="${PLEITOS_DIR:-/srv/urna-velox/pleitos}"
AC="${ACERVO_DIR:-/srv/urna-velox/acervo}"
BASE="${BASE_TSE:-https://resultados.tse.jus.br}"
MAX_POR_RODADA="${MAX_POR_RODADA:-2}"
ESPERA_FALHA_H="${ESPERA_FALHA_H:-6}"
export MAX_SECOES="${MAX_SECOES:-20000}"
mkdir -p "$PD" "$AC"
log() { echo "$(date -Iseconds) [uv-pleitos] $*" | tee -a "$PD/pleitos.log" >&2; }

# 1. pleitos.json (o ele-c.json fica guardado: as regravações depois de um acervo não pedem nada ao TSE)
if ! "$BIN/uv-coletor" pleitos --base "$BASE" --saida "$PD/pleitos.json" --guardar "$PD/ele-c.json" --acervo "$AC" > "$PD/pleitos.txt" 2> "$PD/pleitos.err"; then
  log "ele-c.json indisponível: $(tail -1 "$PD/pleitos.err")"
  [ -s "$PD/pleitos.json" ] || exit 1
fi
[ "${MONTAR_ACERVOS:-1}" = 1 ] || exit 0

# 2. acervos que faltam (a decisão de quais e se pode, em Python: datas e situação vêm do pleitos.json)
FALTAM=$(python3 - "$PD/pleitos.json" "$AC" "$ESPERA_FALHA_H" "$MAX_POR_RODADA" <<'PY'
import json, os, sys, time
from datetime import date, datetime, timedelta, timezone
d = json.load(open(sys.argv[1]))
ac, espera_h, maximo = sys.argv[2], float(sys.argv[3]), int(sys.argv[4])
hoje = datetime.now(timezone(timedelta(hours=-3))).date()
quietos = []
for p in d['pleitos']:
    try: dia = date.fromisoformat(p['data_iso'])
    except (KeyError, ValueError): continue
    if p['situacao'] == 'em_curso' or dia <= hoje <= dia + timedelta(days=1):
        quietos.append(f"{p['codigo']} ({p['data']})")
if quietos:
    print('QUIETO ' + ', '.join(quietos)); sys.exit()
r = []
for p in d['pleitos']:            # do mais novo para o mais antigo (a ordem do pleitos.json)
    if p['situacao'] != 'encerrado' or p.get('replay'): continue
    f = os.path.join(ac, str(p['codigo']), 'falhou')
    if os.path.exists(f) and time.time() - os.path.getmtime(f) < espera_h * 3600: continue
    r.append(str(p['codigo']))
print(' '.join(r[:maximo]))
PY
)
case "$FALTAM" in
  QUIETO*) log "noite ou dia seguinte de eleição (${FALTAM#QUIETO }): nenhum acervo montado nesta rodada"; exit 0 ;;
  "") ;;
esac
# 2b. catálogo pré-eleição dos agendados (fora da noite de eleição, como os acervos)
if [ "${PREPARAR_AGENDADOS:-1}" = 1 ]; then
  PREP=$(python3 - "$PD/pleitos.json" "$AC" "${PREPARO_DIAS:-45}" "${PREPARO_H:-6}" <<'PY'
import json, os, sys, time
from datetime import date, datetime, timedelta, timezone
d = json.load(open(sys.argv[1]))
ac, dias, espera_h = sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
hoje = datetime.now(timezone(timedelta(hours=-3))).date()
r = []
for p in sorted(d['pleitos'], key=lambda p: p.get('data_iso', '')):    # o mais próximo primeiro
    if p.get('situacao') != 'agendado': continue
    try: dia = date.fromisoformat(p['data_iso'])
    except (KeyError, ValueError): continue
    if not (hoje < dia <= hoje + timedelta(days=dias)): continue
    pr = os.path.join(ac, str(p['codigo']), 'preparo')
    pronto = os.path.getsize(os.path.join(pr, 'candidatos.json')) > 0 if os.path.exists(os.path.join(pr, 'candidatos.json')) else False
    marcas = [os.path.join(pr, m) for m in ('falhou', 'incompleto') if os.path.exists(os.path.join(pr, m))]
    if marcas and time.time() - max(os.path.getmtime(m) for m in marcas) < espera_h * 3600: continue
    if pronto and not marcas: continue
    r.append(f"{p['codigo']}:{p.get('ciclo') or 'ele' + p['data_iso'][:4]}")
print(' '.join(r))
PY
)
  for PC in $PREP; do
    P=${PC%%:*}; CIC=${PC#*:}; PR="$AC/$P/preparo"; mkdir -p "$PR" "$AC/$P/fontes-aovivo"
    log "catálogo pré-eleição do $P ($CIC)"
    "$BIN/uv-coletor" preparar --base "$BASE" --ambiente "${AMBIENTE_TSE:-oficial}" --ciclo "$CIC" --pleito "$P" \
      --saida "$PR" --fontes "$AC/$P/fontes-aovivo" > "$PD/preparo-$P.log" 2>&1; RC=$?
    if [ "$RC" = 0 ] && [ -s "$PR/candidatos.json" ]; then
      rm -f "$PR/falhou" "$PR/incompleto"; log "catálogo pré-eleição do $P pronto"
    elif [ "$RC" = 2 ] && [ -s "$PR/candidatos.json" ]; then
      rm -f "$PR/falhou"; date -Iseconds > "$PR/incompleto"
      log "catálogo pré-eleição do $P com fontes incompletas (tenta completar em ${PREPARO_H:-6} h)"
    else
      { date -Iseconds; echo "código $RC"; tail -3 "$PD/preparo-$P.log"; } > "$PR/falhou"
      log "catálogo pré-eleição do $P ainda não saiu (código $RC; tenta de novo em ${PREPARO_H:-6} h): $(tail -1 "$PD/preparo-$P.log")"
      grep -q '429' "$PD/preparo-$P.log" && { log "HTTP 429: rodada encerrada"; exit 0; }
    fi
  done
fi
NOVOS=0
for P in $FALTAM; do
  log "montando o acervo do $P"
  if UV_COLETOR="$BIN/uv-coletor" BASE="$BASE" "$BIN/montar_acervo.sh" "$P" "$AC/$P" "$PD/pleitos.json" > "$PD/acervo-$P.log" 2>&1; then
    rm -f "$AC/$P/falhou"; NOVOS=$((NOVOS + 1)); log "acervo do $P pronto"
  else
    RC=$?; mkdir -p "$AC/$P"; { date -Iseconds; echo "código $RC"; tail -5 "$PD/acervo-$P.log"; } > "$AC/$P/falhou"
    log "acervo do $P falhou (código $RC; tenta de novo em ${ESPERA_FALHA_H} h): $(tail -1 "$PD/acervo-$P.log")"
    # HTTP 429: para tudo (a pausa de 10 min do CLAUDE.md; a próxima rodada é daqui a 15 min)
    grep -q '429' "$PD/acervo-$P.log" && { log "HTTP 429: rodada encerrada"; break; }
  fi
done
# os acervos novos entram no pleitos.json (replay) sem pedir o ele-c.json de novo
[ "$NOVOS" -gt 0 ] && "$BIN/uv-coletor" pleitos --ele-c "$PD/ele-c.json" --saida "$PD/pleitos.json" --acervo "$AC" > "$PD/pleitos.txt" 2>> "$PD/pleitos.err"
# 3. séries de consulta dos encerrados com acervo (as que já existem e estão em dia são puladas)
CONS="${CONSULTA_DIR:-/srv/urna-velox/consulta}"
if [ "${GERAR_CONSULTAS:-1}" = 1 ] && [ -x "$BIN/uv-serie" ] && [ -x "$BIN/gerar_consultas.sh" ]; then
  mkdir -p "$CONS/.tmp"
  UV_BIN="$BIN" TMPDIR="$CONS/.tmp" nice -n 15 "$BIN/gerar_consultas.sh" --destino "$CONS" --acervos "$AC" >> "$PD/consultas.log" 2>&1 \
    || log "séries de consulta: alguma falhou (ver $PD/consultas.log)"
fi
exit 0
