#!/usr/bin/env bash
# trava.sh — muda a trava "no máximo X p.p. à frente do TSE" (M-020) nos dois nós, sem reiniciar o serviço
# (o uv-servico relê o TRAVA_PP do no.env a cada 5 s).
#   ./trava.sh 10        no máximo 10 pontos percentuais à frente do TSE
#   ./trava.sh 0         nunca à frente do TSE
#   ./trava.sh 100       sem limite
#   ./trava.sh desligar  tira a linha (trava desligada)
#   ./trava.sh           só mostra o valor atual
source "$(dirname "$0")/lib.sh"
V="${1:-}"
case "$V" in
  "") CMD="grep '^TRAVA_PP=' /etc/urna-velox/no.env || echo 'TRAVA_PP ausente (desligada)'" ;;
  desligar) CMD="sudo sed -i '/^TRAVA_PP=/d' /etc/urna-velox/no.env && echo 'TRAVA_PP removido (desligada)'" ;;
  *)
    [[ "$V" =~ ^[0-9]+([.][0-9]+)?$ ]] || erro "valor de 0 a 100 (ou 'desligar')"
    awk -v v="$V" 'BEGIN { exit !(v >= 0 && v <= 100) }' || erro "valor de 0 a 100"
    CMD="if grep -q '^TRAVA_PP=' /etc/urna-velox/no.env; then sudo sed -i 's/^TRAVA_PP=.*/TRAVA_PP=$V/' /etc/urna-velox/no.env; else echo 'TRAVA_PP=$V' | sudo tee -a /etc/urna-velox/no.env >/dev/null; fi; grep '^TRAVA_PP=' /etc/urna-velox/no.env" ;;
esac
for N in A B; do
  msg "nó $N"
  em "$N" "$CMD"
done
