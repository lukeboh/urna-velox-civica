#!/usr/bin/env bash
# Replay com a página aberta num teste do Playwright, como o modo rápido do ciclo diário (nenhuma requisição ao TSE:
# uv-simulador com o acervo local). Por padrão é o teste do B-014 (celular trava e esquenta): 1º turno de 2024 (452)
# inteiro a 500× com a página como celular em Prefeito de São Paulo (site/testes/carga_pagina.mjs: mensagens e bytes
# por minuto, tópicos de resultado recebidos, tempo da thread principal em tarefas longas).
#
#   bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/ensaio_carga_pagina.sh [aceleração] [hash] [args do teste…]
#
# Variáveis: UV_PLEITO=452 (padrão) ou 3220 (1º turno de 2026); UV_TESTE_PAGINA=testes/carga_pagina.mjs (padrão) ou
# outro teste com --url/--hash/--saida (ex.: testes/previsao_pagina.mjs, B-013/B-016).
# A página vem embutida no uv-servico: depois de mudar site/src, `npm run build` e compilar_dev.sh antes de rodar.
# Saída em ~/uvreal/ensaios/b014 (ou $UV_ENSAIO); a cópia dos pacotes é apagada ao sair.
set -uo pipefail
ACEL="${1:-500}"; HASH="${2:-#/sp71072/c0011}"; shift 2 2>/dev/null || shift $#
U="${UV_WORK:-$HOME/uvreal}"
R=/mnt/d/git/lukeboh/urna-velox
B="${UV_BIN:-$U/target-dev/release}"
TESTE="${UV_TESTE_PAGINA:-testes/carga_pagina.mjs}"
LT=/mnt/d/dados/urna-velox/linha-do-tempo
case "${UV_PLEITO:-452}" in
  452) PLEITO=452; CICLO=ele2024; DADOS=$U/2024-1t/dados; CS=$U/2024-1t/dados/config; OF=$U/2024-1t/oficial
       CAND=$U/2024-1t/preparo/candidatos.json; CORTE="07/10/2024 02:00:00"
       MET=$LT/452.csv; [ -f "$MET" ] || MET=$U/2024-1t/metricas_aux.csv ;;
  3220) PLEITO=3220; CICLO=ele2026; DADOS=$U/dados; CS=$U/config; OF=$U/oficial
        CAND=$U/etapa5/preparo-2026-1t/candidatos.json; CORTE="05/10/2026 02:00:00"
        MET=$LT/3220.csv; [ -f "$MET" ] || MET=$U/metricas_multi.csv,$U/metricas_nacional.csv,$U/metricas_retomada.csv ;;
  *) echo "UV_PLEITO: 452 ou 3220"; exit 2 ;;
esac
ORDEM=(); [[ "$MET" == $LT/* ]] && ORDEM=(--ordem "$MET")
W="${UV_ENSAIO:-$U/ensaios/b014}"
for f in "$B/uv-simulador" "$B/uv-servico"; do [ -x "$f" ] || { echo "falta $f (compilar_dev.sh)"; exit 2; }; done
rm -rf "$W"; mkdir -p "$W"
ulimit -n 65536 2>/dev/null || true
PIDS=()
parar() { trap - INT TERM EXIT; kill "${PIDS[@]}" 2>/dev/null || true; wait 2>/dev/null || true; rm -rf "$W/dados"; }
trap parar INT TERM EXIT
"$B/uv-simulador" --dados "$DADOS" --cs "$CS" --metricas "$MET" --oficial "$OF" --pleito $PLEITO --ciclo $CICLO \
  --acelerar "$ACEL" --horario recebido --corte "$CORTE" --log "$W/simulador.log" --porta-arquivo "$W/porta" \
  > "$W/simulador.out" 2> "$W/simulador.err" &
PIDS+=($!)
for _ in $(seq 1 600); do [ -s "$W/porta" ] && break; sleep 0.2; done
[ -s "$W/porta" ] || { echo "o simulador não subiu; veja $W/simulador.err"; exit 2; }
P=$(cat "$W/porta")
# relógio parado até o serviço e a página estarem prontos
INICIO=$(curl -s -m 2 "http://127.0.0.1:$P/controle" | python3 -c 'import json,sys; print(json.load(sys.stdin)["inicio"])')
curl -s -m 2 -X POST "http://127.0.0.1:$P/controle/posicao?t=$INICIO&x=0" > /dev/null
HP=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
"$B/uv-servico" --pleito $PLEITO --ciclo $CICLO --candidatos "$CAND" --dados "$W/dados" --saida "$W/saida" \
  --base "http://127.0.0.1:$P" --aceleracao-local "$ACEL" --rps 2000 "${ORDEM[@]}" --http "127.0.0.1:$HP" --continuar \
  --simulacao "ensaio" 2> "$W/servico.err" &
PIDS+=($!)
for _ in $(seq 1 600); do curl -s -o /dev/null -m 1 "http://127.0.0.1:$HP/api/v1/estado" && break; sleep 0.2; done
( sleep 8; curl -s -m 2 -X POST "http://127.0.0.1:$P/controle/velocidade?x=$ACEL" > /dev/null ) &
cd "$R/site" && node "$TESTE" --url "http://127.0.0.1:$HP/" --hash "$HASH" --saida "$W" "$@"
