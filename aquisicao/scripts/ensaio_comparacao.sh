#!/usr/bin/env bash
# Ensaio da comparação Urna Velox × TSE (etapa 7, docs/handoffs/HANDOFF_0.7.0.md, critérios 1, 2 e 4), sem nenhuma requisição ao
# TSE: uv-simulador com o "TSE simulado" (-u.json progressivos, atraso de divulgação sorteado) + uv-servico
# --http + N abas da página em modos variados (site/testes/comparacao.mjs, Playwright).
#
#   bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/ensaio_comparacao.sh <nome> <2024-2t|2026-1t> <aceleração> [abas]
#
# Saída em ~/uvreal/ensaios/etapa7-<nome> (CLAUDE.md, "Disco"; arquivar depois com scripts/disco.sh arquivar):
# pagina/ (abas.json, resumo.json, fotos), saida/ do serviço, simulador.log, RESUMO.json (rede, conferência, marcos).
set -euo pipefail
NOME="$1"; CONJ="$2"; ACEL="$3"; ABAS="${4:-10}"
U="${UV_WORK:-$HOME/uvreal}"
R=/mnt/d/git/lukeboh/urna-velox
B="${UV_BIN:-$U/target-dev/release}"
W="$U/ensaios/etapa7-$NOME"
EXTRA=()
case "$CONJ" in
  2024-2t) DADOS=$U/2024-2t/dados; CS=$U/2024-2t/dados/config; MET=$U/2024-2t/metricas_2t.csv; OF=$U/2024-2t/oficial
           PLEITO=453; CICLO=ele2024; CAND=$U/2024-2t/preparo/candidatos.json
           HASHES='#/am02550/c0011,#/sp71072/c0011,#/mg41238/c0011,#/pa04278/c0011,#/rj58653/c0011' ;;
  2026-1t) DADOS=$U/dados; CS=$U/config; MET=$U/metricas_multi.csv,$U/metricas_nacional.csv,$U/metricas_retomada.csv; OF=$U/oficial
           PLEITO=3220; CICLO=ele2026; CAND=$U/etapa5/preparo-2026-1t/candidatos.json
           EXTRA=(--corte "05/10/2026 02:00:00" --horario recebido)
           HASHES='#/br/c0001,#/sp/c0003,#/mg/c0005,#/zz/c0001,#/rj/c0003' ;;
  *) echo "conjunto: 2024-2t ou 2026-1t"; exit 1 ;;
esac
for f in "$B/uv-simulador" "$B/uv-servico"; do [ -x "$f" ] || { echo "falta $f (compilar_dev.sh)"; exit 1; }; done
bash "$R/scripts/disco.sh" --curto || true
rm -rf "$W"; mkdir -p "$W"
ulimit -n 65536 2>/dev/null || true
PIDS=()
parar() { trap - INT TERM EXIT; kill "${PIDS[@]}" 2>/dev/null || true; wait 2>/dev/null || true; rm -rf "$W/dados"; }
trap parar INT TERM EXIT

"$B/uv-simulador" --dados "$DADOS" --cs "$CS" --metricas "$MET" --oficial "$OF" --pleito "$PLEITO" --ciclo "$CICLO" --acelerar "$ACEL" \
  "${EXTRA[@]}" --tse-simulado "$CAND" --log "$W/simulador.log" --porta-arquivo "$W/porta" > "$W/simulador.out" 2> "$W/simulador.err" &
PIDS+=($!)
for _ in $(seq 1 600); do [ -s "$W/porta" ] && break; sleep 0.2; done
[ -s "$W/porta" ] || { echo "o simulador não subiu; veja $W/simulador.err"; exit 1; }
P=$(cat "$W/porta")
# relógio parado até o serviço e as abas estarem prontos
INICIO=$(curl -s -m 2 "http://127.0.0.1:$P/controle" | python3 -c 'import json,sys; print(json.load(sys.stdin)["inicio"])')
curl -s -m 2 -X POST "http://127.0.0.1:$P/controle/posicao?t=$INICIO&x=0" > /dev/null
HP=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
"$B/uv-servico" --pleito "$PLEITO" --ciclo "$CICLO" --candidatos "$CAND" --dados "$W/dados" --saida "$W/saida" \
  --base "http://127.0.0.1:$P" --aceleracao-local "$ACEL" --http "127.0.0.1:$HP" --continuar 2> "$W/servico.err" &
SPID=$!; PIDS+=($SPID)
for _ in $(seq 1 600); do (exec 3<>"/dev/tcp/127.0.0.1/$HP") 2>/dev/null && break; sleep 0.2; done
echo "simulador :$P · serviço :$HP · $CONJ a ${ACEL}× · $ABAS abas"
( cd "$R/site" && node testes/comparacao.mjs --url "http://127.0.0.1:$HP/" --saida "$W/pagina" --oficial "$OF" --hashes "$HASHES" --abas "$ABAS" --limite 7200 > "$W/pagina.out" 2>&1 ) &
NPID=$!
sleep 8
curl -s -m 2 -X POST "http://127.0.0.1:$P/controle/velocidade?x=$ACEL" > /dev/null
wait $NPID || echo "!! a página saiu com erro (veja $W/pagina.out)"
# a página fechou: mais um intervalo e nenhuma busca nova do TSE pode aparecer (nada em segundo plano)
date +%s.%N > "$W/pagina_fechada"
sleep 3
kill -TERM $SPID 2>/dev/null || true
wait $SPID 2>/dev/null || true
python3 "$R/aquisicao/scripts/analisar_comparacao.py" "$W" | tee "$W/RESUMO.txt"
