#!/usr/bin/env bash
# Simulação da noite do 1º turno de 2026 (04/10) para ver na página de resultados (etapa 5).
#
#   bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/simular_site.sh [aceleração] [porta]
#
# Sobe o uv-simulador (que serve, só na máquina, os BUs já baixados no horário em que o TSE os recebeu) e o
# uv-servico com a página em http://localhost:<porta>/ (padrão 10× e 18080). Abra o endereço no Chrome do Windows.
# Nenhuma requisição ao TSE. Ctrl+C encerra os dois. Cada execução começa do zero em $UV_WORK/simulacao-site.
#
# A barra "Replay" no pé da página controla a simulação, como a de um vídeo: tocar/pausar, velocidade (1× a 300×) e a
# linha do tempo da noite — arrastar para a frente avança rápido até o ponto; para trás, o serviço recomeça do zero e
# avança até lá (alguns segundos). Vale para todas as abas. Também por linha de comando:
#   curl -X POST -H 'Content-Type: application/json' -d '{"acelerar":60}' http://localhost:18080/api/v1/replay/velocidade
# Duração a velocidade constante (a noite vai das 17h às 2h; quase tudo chega até as 23h): 300× ≈ 2 min,
# 60× ≈ 6 min, 10× ≈ 36 min, 1× = a noite real.
#
# Variáveis opcionais: UV_WORK (padrão ~/uvreal), UV_BIN (padrão $UV_WORK/target-dev/release),
#   UV_CANDIDATOS (padrão $UV_WORK/etapa5/preparo-2026-1t/candidatos.json).
set -euo pipefail
ACEL="${1:-10}"
case "$ACEL" in ''|*[!0-9.]*) echo "aceleração inválida: $ACEL (de 1 a 300)"; exit 1 ;; esac
awk -v a="$ACEL" 'BEGIN { exit !(a >= 1 && a <= 300) }' || { echo "aceleração fora de 1 a 300: $ACEL"; exit 1; }
PORTA="${2:-18080}"
U="${UV_WORK:-$HOME/uvreal}"
B="${UV_BIN:-$U/target-dev/release}"
CAND="${UV_CANDIDATOS:-$U/etapa5/preparo-2026-1t/candidatos.json}"
W="$U/simulacao-site"

for f in "$B/uv-simulador" "$B/uv-servico"; do
  [ -x "$f" ] || { echo "falta $f — compile com: bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh"; exit 1; }
done
[ -f "$CAND" ] || { echo "falta $CAND (uv-coletor preparar --pleito 3220 ... --offline)"; exit 1; }
for d in "$U/dados" "$U/config" "$U/oficial"; do [ -d "$d" ] || { echo "falta a pasta $d"; exit 1; }; done
if (exec 3<>"/dev/tcp/127.0.0.1/$PORTA") 2>/dev/null; then
  echo "a porta $PORTA já está em uso (outra simulação?). Use outra: $0 $ACEL 18081"; exit 1
fi

ulimit -n 65536 2>/dev/null || true
rm -rf "$W"; mkdir -p "$W"
PIDS=()
# ao sair, apaga a cópia dos pacotes que o serviço gravou ($W/dados, ~6 GB: regenerável; CLAUDE.md, "Disco");
# ficam saida/ e os logs. UV_MANTER_DADOS=1 mantém.
parar() { trap - INT TERM EXIT; echo; echo "encerrando…"; kill "${PIDS[@]}" 2>/dev/null || true; wait 2>/dev/null || true
  [ "${UV_MANTER_DADOS:-0}" = 1 ] || rm -rf "$W/dados"; echo "pronto (saída e logs em $W)"; }
trap parar INT TERM EXIT

"$B/uv-simulador" --dados "$U/dados" --cs "$U/config" \
  --metricas "$U/metricas_multi.csv,$U/metricas_nacional.csv,$U/metricas_retomada.csv" \
  --oficial "$U/oficial" --pleito 3220 --ciclo ele2026 --acelerar "$ACEL" \
  --corte "05/10/2026 02:00:00" --horario recebido \
  --log "$W/simulador.log" --porta-arquivo "$W/porta" > "$W/simulador.out" 2> "$W/simulador.err" &
PIDS+=($!)
for _ in $(seq 1 300); do [ -s "$W/porta" ] && break; sleep 0.2; done
[ -s "$W/porta" ] || { echo "o simulador não subiu; veja $W/simulador.err"; exit 1; }
P=$(cat "$W/porta")
# o simulador fica parado no começo da noite até o serviço subir (ele toca na velocidade escolhida quando está pronto)
INICIO=$(curl -s -m 2 "http://127.0.0.1:$P/controle" | python3 -c 'import json,sys; print(json.load(sys.stdin)["inicio"])')
curl -s -m 2 -X POST "http://127.0.0.1:$P/controle/posicao?t=$INICIO&x=0" > /dev/null

# --intervalo-cs e as demais esperas são em hora do site: o serviço as divide pela velocidade atual
"$B/uv-servico" --pleito 3220 --ciclo ele2026 --candidatos "$CAND" --dados "$W/dados" --saida "$W/saida" \
  --base "http://127.0.0.1:$P" --aceleracao-local "$ACEL" --controle-replay \
  --http "127.0.0.1:$PORTA" --continuar 2> "$W/servico.err" &
PIDS+=($!)
for _ in $(seq 1 100); do (exec 3<>"/dev/tcp/127.0.0.1/$PORTA") 2>/dev/null && break; sleep 0.2; done

cat <<EOF

  Simulação do 1º turno de 2026 a ${ACEL}× — página em:

      http://localhost:$PORTA/

  Presidente: http://localhost:$PORTA/#/br/c0001   Governador de SP: http://localhost:$PORTA/#/sp/c0003
  Barra "Replay" no pé da página: tocar/pausar, velocidade (1× a 300×) e a linha do tempo da noite.
  Ctrl+C encerra. Logs e saída em $W

EOF
# andamento a cada 30 s: relógio da simulação e seções apuradas do Brasil (Presidente)
while kill -0 "${PIDS[1]}" 2>/dev/null; do
  sleep 30
  python3 - "$PORTA" <<'PY' 2>/dev/null || true
import json, sys, urllib.request
u = f"http://127.0.0.1:{sys.argv[1]}/api/v1"
try:
    rp = json.load(urllib.request.urlopen(u + "/replay", timeout=2))
    r = json.load(urllib.request.urlopen(u + "/resultado/br-c0001", timeout=2))
    s = r.get("secoes", {}); a, c = s.get("apuradas", 0), s.get("total") or 1
    mil = lambda n: format(n, ",").replace(",", ".")
    vel = "pausado" if rp.get("pausado") else "%g×" % rp.get("acelerar", 0)
    if rp.get("busca"): vel += ", buscando"
    print("%s  seções %s de %s (%.1f%%)  [%s]%s" % (rp.get("relogio") or "—", mil(a), mil(c), 100 * a / c, vel, "  — apuração encerrada" if r.get("encerrada") else ""))
except Exception:
    print("…")
PY
done
echo "o uv-servico terminou; veja $W/servico.err"
