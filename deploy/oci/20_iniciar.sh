#!/usr/bin/env bash
# 20 — Inicia os dois nós (systemd). Contra o site real do TSE exige --confirmo-tse, e antes de 26/10/2026 também
# --combinado (CLAUDE.md: não coletar durante a apuração sem combinar; HANDOFF_0.6.0.md: nada da 2.0 contra o TSE antes
# do fim do 2º turno sem combinar com o Luciano). Uso: ./20_iniciar.sh [--confirmo-tse] [--combinado]
source "$(dirname "$0")/lib.sh"
CONF=0; COMB=0
for a in "$@"; do case "$a" in --confirmo-tse) CONF=1 ;; --combinado) COMB=1 ;; *) erro "opção desconhecida: $a" ;; esac; done
if base_e_tse; then
  [ "$CONF" = 1 ] || erro "BASE=$BASE é o site real do TSE. Confirme com --confirmo-tse (cada nó: ≤ $RPS req/s no seu IP; 429 → pausa de 10 min)."
  if [ "$(date +%Y%m%d)" -lt 20261026 ] && [ "$COMB" = 1 ]; then aviso "antes de 26/10/2026, com --combinado: combinado com o Luciano."
  elif [ "$(date +%Y%m%d)" -lt 20261026 ]; then erro "antes de 26/10/2026 a 2.0 não roda contra o TSE sem combinar: use --combinado só se foi combinado."; fi
fi
for N in A B; do em "$N" "test -s /srv/urna-velox/preparo/candidatos.json" || erro "nó $N sem candidatos.json: rode ./10_implantar.sh"; done
# seções agregadas (nsp) ficam fora do total desde a 0.4.9, mas só se o candidatos.json as marcar (preparar da 0.4.9+);
# um candidatos.json antigo deixa o total acima do s.ts do TSE e a apuração nunca chega a 100 %
for N in A B; do
  em "$N" "grep -Eq '\"agregada\":[[:space:]]*true' /srv/urna-velox/preparo/candidatos.json" \
    || erro "nó $N: o candidatos.json não marca as seções agregadas (gerado antes da 0.4.9). Gere de novo com o uv-coletor preparar atual e envie aos dois nós"
done
# o uv-atualizador não pode derrubar o que foi ligado à mão (até o 21_parar.sh)
for N in A B; do travar_manual "$N" 20_iniciar; done
msg "iniciando os dois nós juntos (eles se enxergam nos primeiros heartbeats antes de coletar)"
em A "sudo systemctl start urna-velox" & em B "sudo systemctl start urna-velox" & wait
sleep 5
"$AQUI/30_verificar.sh"
