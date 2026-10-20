#!/usr/bin/env bash
# B-005: candidatos.json das simulações do ciclo diário com a destinação final (`dvt`) dos -u.json oficiais.
#
# As simulações repetem eleições já encerradas (452, 453, 3220): a destinação de cada candidato (válido, anulado,
# anulado sub judice…) já é conhecida e está nos -u.json finais do cache do `preparar`. Montado com
# `--ignorar-destinacao` (todos "Válido", como na véspera de uma eleição), o 1º turno de 2024 dava 33 situações
# divergentes do oficial (candidatos depois cassados/indeferidos) e o alerta `destinacao_presumida` em toda simulação.
# A noite real continua com o `preparar` normal: sem `dvt` no arquivo, a destinação é presumida e o alerta aparece.
#
#   ./candidatos_simulacao.sh            gera em $UVREAL/<pleito>/preparo-simulacao (só local, sem rede: --offline)
#   ./candidatos_simulacao.sh --enviar   também troca /srv/urna-velox/acervo/<pleito>/candidatos.json nos nós A e B
#                                        (vale a partir da próxima eleição do ciclo; a que está no ar não muda)
set -euo pipefail
AQUI="$(cd "$(dirname "$0")" && pwd)"
U="${UVREAL:-/home/dev/uvreal}"
C="${UV_COLETOR:-$U/target-dev/release/uv-coletor}"
[ -x "$C" ] || { echo "falta $C (aquisicao/scripts/compilar_dev.sh)"; exit 1; }
declare -A ARGS=([452]="--ciclo ele2024 --pleito 452" [453]="--ciclo ele2024 --pleito 453" [3220]="--pleito 3220 --cargos 1,3,5")
declare -A FONTES=([452]=$U/2024-1t/preparo_fontes [453]=$U/2024-2t/preparo_fontes [3220]=$U/preparo_2026_1t)
declare -A SAIDA=([452]=$U/2024-1t/preparo-simulacao [453]=$U/2024-2t/preparo-simulacao [3220]=$U/preparo_2026_1t-simulacao)

for P in 452 453 3220; do
  mkdir -p "${SAIDA[$P]}"
  # sem --ignorar-destinacao; --offline: só o cache (os -u.json finais), nenhuma requisição ao TSE
  # shellcheck disable=SC2086
  "$C" preparar ${ARGS[$P]} --saida "${SAIDA[$P]}" --fontes "${FONTES[$P]}" --offline 2>&1 | tail -3
  python3 - "${SAIDA[$P]}/candidatos.json" "$P" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert str(d['pleito']) == sys.argv[2], 'pleito errado'
cs = [c for e in d['eleicoes'] for g in e['cargos'] for k in g['circunscricoes'] for c in k['candidatos']]
nao = sum(1 for c in cs if not c['destinacao'].startswith('Válido'))
print(f"{sys.argv[2]}: {len(cs)} candidatos, {nao} com destinação diferente de 'Válido', destinacao_presumida={d['destinacao_presumida']}")
sys.exit(1 if d['destinacao_presumida'] else 0)
PY
done

[ "${1:-}" = --enviar ] || exit 0
cd "$AQUI"
source ./lib.sh
for P in 452 453 3220; do
  for N in A B; do
    para "$N" "${SAIDA[$P]}/candidatos.json" /tmp/uv-cand-$P.json
    em "$N" "sudo install -o urnavelox -g urnavelox -m 0644 /tmp/uv-cand-$P.json /srv/urna-velox/acervo/$P/candidatos.json && sudo rm -f /tmp/uv-cand-$P.json && python3 -c \"import json;d=json.load(open('/srv/urna-velox/acervo/$P/candidatos.json'));print('$N $P destinacao_presumida', d['destinacao_presumida'])\""
  done
done
