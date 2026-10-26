#!/usr/bin/env bash
# Testes do uv-iniciar.sh (linha de comando do uv-servico montada a partir do no.env), sem root: roda uma cópia com os
# caminhos /etc/urna-velox e /opt/urna-velox trocados por uma pasta temporária e um uv-servico falso que só imprime os
# argumentos. Uso: bash test_uv_iniciar.sh
set -uo pipefail
AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
mkdir -p "$T/etc" "$T/opt/bin"
sed -e "s#/etc/urna-velox#$T/etc#g" -e "s#/opt/urna-velox#$T/opt#g" "$AQUI/../servidor/uv-iniciar.sh" > "$T/uv-iniciar.sh"
cat > "$T/opt/bin/uv-servico" <<'SH'
#!/usr/bin/env bash
[ "${1:-}" = --help ] && { echo "--rotas --acesso"; exit 0; }
printf '%s\n' "$@"
SH
chmod +x "$T/opt/bin/uv-servico"
BASE_ENV=$'NO_ID=no-a\nCICLO=ele2026\nAMBIENTE=oficial\nBASE=https://resultados.tse.jus.br\nRPS=95\nFED_PORTA=7700\nPLEITO=3221\n'
FALHAS=0; N=0
args() { printf '%s' "$1" > "$T/etc/no.env"; bash "$T/uv-iniciar.sh" | tr '\n' ' '; }
confere() { # descrição, saída, padrão (grep -E) que deve aparecer
  N=$((N + 1)); if grep -Eq -- "$3" <<< "$2"; then echo "ok $N: $1"; else echo "FALHOU $N: $1 — args: $2"; FALHAS=$((FALHAS + 1)); fi; }
nega() {
  N=$((N + 1)); if grep -Eq -- "$3" <<< "$2"; then echo "FALHOU $N: $1 — args: $2"; FALHAS=$((FALHAS + 1)); else echo "ok $N: $1"; fi; }

# 1. sem INTERVALO_CS no no.env: o padrão de 30 s, explícito na linha de comando
A=$(args "$BASE_ENV")
confere "sem INTERVALO_CS → --intervalo-cs 30" "$A" '(^| )--intervalo-cs 30 '
confere "o --rps continua o do no.env" "$A" '(^| )--rps 95 '
nega "não mexe no --intervalo-cs-ocioso" "$A" '--intervalo-cs-ocioso'
nega "não mexe no --tse-intervalo" "$A" '--tse-intervalo'
# 2. INTERVALO_CS=10 (produção, 2º turno de 2026)
A=$(args "${BASE_ENV}INTERVALO_CS=10"$'\n')
confere "INTERVALO_CS=10 → --intervalo-cs 10" "$A" '(^| )--intervalo-cs 10 '
nega "um --intervalo-cs só" "$A" '--intervalo-cs [0-9.]+ .*--intervalo-cs '
# 3. vazio vale o padrão
A=$(args "${BASE_ENV}INTERVALO_CS="$'\n')
confere "INTERVALO_CS vazio → 30" "$A" '(^| )--intervalo-cs 30 '
# 4. o EXTRA continua indo por último (pode sobrescrever, como antes)
A=$(args "${BASE_ENV}INTERVALO_CS=10"$'\n'"EXTRA='--aceitar-incompleto'"$'\n')
confere "EXTRA por último" "$A" '--intervalo-cs 10 .*--aceitar-incompleto $'

echo "$N testes, $FALHAS falhas"
[ "$FALHAS" = 0 ]
