#!/usr/bin/env bash
# B-067: repete no simulador local a noite do 2º turno de 2026 (3221) com a configuração de produção (2 nós federados,
# 95 req/s cada, cs.json a cada 10 s, sondagem ligada), cada nó saindo por um IP local próprio, em tempo real (1×).
# Nenhuma requisição ao TSE. O simulador registra cada requisição (ms, status, IP): a regra da borda que explica os
# 429 de 25/10 é procurada depois, sobre esse registro (aquisicao/scripts/b067_regras.py), e a que reproduzir os
# horários reais é confirmada com --limite-ip-rps/--limite-ip-regra.
#
#   ensaio_b067.sh <nome> [inicio "dd/mm/aaaa hh:mm:ss"] [fim hh:mm:ss] [opções extras do simulador...]
#   ex.: ensaio_b067.sh registro "25/10/2026 18:45:00" 21:40:00 --latencia-ms 5-40
#        ensaio_b067.sh deslizante "25/10/2026 18:45:00" 21:40:00 --latencia-ms 5-40 --limite-ip-rps 100 --limite-ip-regra deslizante
set -uo pipefail
NOME="${1:?nome do ensaio}"; shift
INICIO="${1:-25/10/2026 18:45:00}"; shift || true
FIM="${1:-21:40:00}"; shift || true
EXTRA=("$@")
R="$(cd "$(dirname "$0")/../.." && pwd)"
A="${UV_ACERVO:-$HOME/uvreal/acervo-3221}"
B="${UV_BIN:-$HOME/uvreal/target-dev/release}"
W="$HOME/uvreal/ensaios/b067-$NOME"
IPS=(127.0.0.2 127.0.0.3)
rm -rf "$W"; mkdir -p "$W"
ulimit -n 65536 2>/dev/null || true
PIDS=()
parar() { trap - INT TERM EXIT; kill "${PIDS[@]}" 2>/dev/null; sleep 3; kill -9 "${PIDS[@]}" 2>/dev/null; wait 2>/dev/null; rm -rf "$W"/no-*/dados "$W"/no-*/federacao; echo "== parado $(date +%T)"; }
trap parar INT TERM EXIT
{
  echo "ensaio $NOME: início do relógio $INICIO, fim $FIM, extras: ${EXTRA[*]:-nenhum}"
  "$B/uv-servico" --version; "$B/uv-simulador" --version
  git -C "$R" log --oneline -1
  python3 - "$A/linha-do-tempo.csv" "$INICIO" <<'E'
import csv, sys, datetime as D
t0 = D.datetime.strptime(sys.argv[2], '%d/%m/%Y %H:%M:%S'); antes = depois = 0
for r in csv.DictReader(open(sys.argv[1], encoding='utf-8')):
    v = (r.get('recebido_tse') or '').strip()
    if not v: continue
    if D.datetime.strptime(v, '%d/%m/%Y %H:%M:%S') <= t0: antes += 1
    else: depois += 1
print(f'BUs recebidos até o início do relógio: {antes}; depois: {depois}')
E
} | tee "$W/ensaio.txt"
"$B/uv-simulador" --dados "$A/dados" --cs "$A/cs" --metricas "$A/linha-do-tempo.csv" --oficial "$A/oficial" --pleito 3221 \
  --ciclo ele2026 --acelerar 1 --inicio "$INICIO" --horario recebido --corte "26/10/2026 02:00:00" --log "$W/simulador.log" \
  --porta-arquivo "$W/porta" "${EXTRA[@]}" > "$W/simulador.out" 2> "$W/simulador.err" &
PIDS+=($!)
for _ in $(seq 1 1500); do [ -s "$W/porta" ] && break; sleep 0.2; done
[ -s "$W/porta" ] || { echo "o simulador não subiu:"; tail -5 "$W/simulador.err"; exit 2; }
P=$(cat "$W/porta"); echo "simulador na porta $P: $(curl -s -m 2 http://127.0.0.1:$P/controle)" | tee -a "$W/ensaio.txt"
porta() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }
FP1=$(porta); FP2=$(porta)
cat > "$W/federacao.json" <<J
{"versao": 1, "teto_coleta_rps": 95.0, "nos": [
 {"id": "no-a", "orgao": "Órgão A", "ip_saida": "${IPS[0]}", "endereco": "${IPS[0]}:$FP1"},
 {"id": "no-b", "orgao": "Órgão B", "ip_saida": "${IPS[1]}", "endereco": "${IPS[1]}:$FP2"}]}
J
i=0
for id in no-a no-b; do
  ip=${IPS[$i]}; i=$((i+1)); d="$W/$id"; mkdir -p "$d"
  # o mesmo do uv-iniciar.sh de produção (no.env de 25/10: RPS=95, INTERVALO_CS=10, --http), contra o simulador
  "$B/uv-servico" --pleito 3221 --ciclo ele2026 --ambiente oficial --base "http://127.0.0.1:$P" \
    --candidatos "$A/candidatos.json" --dados "$d/dados" --saida "$d/saida" --rps 95 --intervalo-cs 10 \
    --federacao "$W/federacao.json" --no "$id" --armazem "$d/federacao" --ip-local "$ip" \
    --http "$ip:0" --continuar 2> "$d/servico.err" &
  PIDS+=($!)
done
T0=$(date +%s)
FIM_TXT="${INICIO%% *} $FIM"
while :; do
  sleep 30
  # "1"/"0" (passou do fim?) e o relógio do simulador
  REL=$(curl -s -m 2 "http://127.0.0.1:$P/controle" | python3 -c '
import json, sys, datetime as D
j = json.load(sys.stdin); f = lambda s: D.datetime.strptime(s, "%d/%m/%Y %H:%M:%S")
print(int(f(j["relogio"]) >= f(sys.argv[1])), j["relogio"])' "$FIM_TXT" 2>/dev/null)
  L="$(( $(date +%s) - T0 )) s · relógio ${REL#* }"
  for id in no-a no-b; do
    L+=" · $id $(python3 -c "import json;s=json.load(open('$W/$id/saida/saude.json'));r=s['rede'];print(s['secoes']['aplicadas'],'fila',s.get('fila',{}).get('total',s.get('fila')) if isinstance(s.get('fila'),dict) else s.get('fila'),'429',r['http429'],'aux404',r['aux_404'])" 2>/dev/null)"
  done
  echo "$L" >> "$W/progresso.txt"
  for pid in "${PIDS[@]}"; do kill -0 "$pid" 2>/dev/null || { echo "processo $pid saiu" >> "$W/progresso.txt"; break 2; }; done
  [ "${REL%% *}" = 1 ] && break
done
echo "== fim do ensaio às $(date +%T), relógio ${REL#* }" | tee -a "$W/ensaio.txt"
kill -TERM "${PIDS[0]}" 2>/dev/null; sleep 2   # o simulador grava o resumo ao sair
