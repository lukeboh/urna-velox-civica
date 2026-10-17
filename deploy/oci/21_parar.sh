#!/usr/bin/env bash
# 21 — Para os dois nós (parada limpa: SIGTERM; o nó grava o último lote e serve os pares por uma carência).
# Os dados ficam: iniciar de novo retoma de onde parou. Libera a trava de controle manual (o uv-atualizador volta a
# seguir o manifesto, mas só reaplica estado quando sair uma série nova). Uso: ./21_parar.sh [A|B]
source "$(dirname "$0")/lib.sh"
for N in ${1:-A B}; do
  msg "parando o nó $N"
  em "$N" "sudo systemctl stop urna-velox; systemctl is-active urna-velox || true"
  liberar_manual "$N"
done
