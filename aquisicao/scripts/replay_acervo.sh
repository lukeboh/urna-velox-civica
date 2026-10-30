#!/usr/bin/env bash
# Multipleito (1.1): prova de um acervo de replay (montar_acervo.sh ou montado à mão) — repete a noite no uv-simulador
# com um uv-servico, como o uv-ciclo.sh no modo rápido (sem federação), e confere o resultado com os -u.json oficiais.
# Nenhuma requisição ao TSE. Sai com 0 só se a conferência final for idêntica ao oficial.
#
#   replay_acervo.sh <acervo> [saída]        (saída padrão: um temporário apagado no fim, se passar)
#
# Variáveis: UV_BIN (pasta do uv-simulador e do uv-servico; padrão: target/release ou target/debug), ACEL (500).
set -euo pipefail
A="${1:?uso: replay_acervo.sh <acervo> [saída]}"
R="${2:-$(mktemp -d "${TMPDIR:-/tmp}/uv-replay-acervo-XXXXXX")}"
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
B="${UV_BIN:-}"; [ -n "$B" ] || { [ -x "$RAIZ/target/release/uv-servico" ] && B="$RAIZ/target/release" || B="$RAIZ/target/debug"; }
ACEL="${ACEL:-500}"
# B-076: diz qual binário fez o replay (o padrão, target/release do repositório, pode ser de uma versão antiga)
echo "replay com $("$B/uv-servico" --version 2>&1 | head -n 1) ($B)"
[ -f "$A/.completo" ] && [ -f "$A/acervo.json" ] || { echo "acervo incompleto em $A (falta .completo ou acervo.json)" >&2; exit 2; }
read -r P CICLO CORTE < <(python3 -c "import json,sys; a=json.load(open(sys.argv[1])); print(a['pleito'], a['ciclo'], a['corte'])" "$A/acervo.json")
mkdir -p "$R/sim"
"$B/uv-simulador" --dados "$A/dados" --cs "$A/cs" --metricas "$A/linha-do-tempo.csv" --oficial "$A/oficial" \
  --pleito "$P" --ciclo "$CICLO" --acelerar "$ACEL" --horario recebido --corte "$CORTE" --porta 0 \
  --porta-arquivo "$R/sim/porta" --log "$R/sim/simulador.log" --acompanhar-servico 1 > "$R/sim/simulador.out" 2> "$R/sim/simulador.err" &
SIM=$!
trap 'kill $SIM 2>/dev/null || true' EXIT
for _ in $(seq 1 120); do [ -s "$R/sim/porta" ] && break; kill -0 $SIM 2>/dev/null || { cat "$R/sim/simulador.err" >&2; exit 3; }; sleep 0.5; done
PORTA=$(cat "$R/sim/porta")
RC=0
timeout "${LIMITE_S:-1800}" "$B/uv-servico" --pleito "$P" --ciclo "$CICLO" --candidatos "$A/candidatos.json" \
  --dados "$R/dados" --saida "$R/saida" --base "http://127.0.0.1:$PORTA" --aceleracao-local "$ACEL" --rps 2000 \
  --ordem "$A/linha-do-tempo.csv" > "$R/servico.out" 2> "$R/servico.err" || RC=$?
python3 - "$R/saida/conferencia.json" "$RC" <<'PY'
import json, sys
try: c = json.load(open(sys.argv[1]))
except OSError: sys.exit(f'sem conferência (uv-servico saiu com {sys.argv[2]})')
r = {'arquivos': c.get('arquivos'), 'identicos': c.get('identicos'), 'divergentes': len(c.get('divergentes', [])),
     'situacoes': c.get('situacoes'), 'faltando': len(c.get('faltando', []))}
print(json.dumps(r, ensure_ascii=False))
ok = sys.argv[2] == '0' and r['arquivos'] and r['identicos'] == r['arquivos'] and not r['faltando'] \
    and (r['situacoes'] or {}).get('divergentes', 0) == 0
sys.exit(0 if ok else 1)
PY
echo "conferência idêntica ao oficial ($R/saida/conferencia.md)"
[ -n "${2:-}" ] || rm -rf "$R"
