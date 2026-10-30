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

# 5. multipleito (1.1): --pleitos só com o pleitos.json presente e um uv-servico que conhece a opção
nega "sem pleitos.json, sem --pleitos" "$(args "$BASE_ENV")" '--pleitos'
printf '{"pleitos":[]}' > "$T/pleitos.json"
A=$(args "${BASE_ENV}PLEITOS_JSON=$T/pleitos.json"$'\n')
nega "binário antigo (sem --pleitos no --help), sem --pleitos" "$A" '--pleitos'
sed -i 's/--rotas --acesso/--rotas --acesso --pleitos/' "$T/opt/bin/uv-servico"
A=$(args "${BASE_ENV}PLEITOS_JSON=$T/pleitos.json"$'\n')
confere "com o pleitos.json e o binário novo → --pleitos" "$A" "--pleitos $T/pleitos.json"

# 6. multipleito (1.1): o modo aovivo do uv-ciclo.sh troca pleito, ciclo, candidatos, pastas e UFs por cima do no.env
A=$(UV_PLEITO=3250 UV_CICLO=ele2024 UV_CANDIDATOS=/c/cand.json UV_DADOS=/c/dados UV_SAIDA=/c/saida UV_ARMAZEM=/c/fed UV_UFS=sp \
    UV_EXTRA='--continuar --aceitar-incompleto' args "${BASE_ENV}EXTRA='--x'"$'\n')
confere "aovivo: pleito e ciclo do ciclo" "$A" '--pleito 3250 --ciclo ele2024 '
confere "aovivo: candidatos, dados e saída do ciclo" "$A" '--candidatos /c/cand.json --dados /c/dados --saida /c/saida '
confere "aovivo: armazém do ciclo, federação da produção" "$A" "--federacao $T/etc/federacao.json --no no-a --armazem /c/fed "
confere "aovivo: só as UFs do pleito" "$A" '--ufs sp '
confere "aovivo: o req/s da produção" "$A" '--rps 95 '
confere "aovivo: EXTRA do no.env e o do ciclo, por último" "$A" '--x --continuar --aceitar-incompleto $'
A=$(args "$BASE_ENV")
confere "sem as variáveis do ciclo: como antes" "$A" "--pleito 3221 .*--dados /srv/urna-velox/dados --saida /srv/urna-velox/saida "

# 7. no.env sem PLEITO (multipleito, §3.6): com o ciclo, o pleito do ciclo; sem, o serviço não sobe (78) e diz por quê
SEM=$(sed '/^PLEITO=/d' <<< "$BASE_ENV")
A=$(UV_PLEITO=3237 UV_CICLO=ele2026 args "$SEM"$'\n')
confere "sem PLEITO no no.env, com o ciclo: o pleito do ciclo" "$A" '--pleito 3237 --ciclo ele2026 '
printf '%s\n' "$SEM" > "$T/etc/no.env"; E=$(bash "$T/uv-iniciar.sh" 2>&1); RC=$?
N=$((N + 1)); if [ "$RC" = 78 ] && grep -q "sem pleito" <<< "$E"; then echo "ok $N: sem PLEITO e sem ciclo: sai com 78 e diz por quê"; else echo "FALHOU $N: sem PLEITO: rc=$RC $E"; FALHAS=$((FALHAS + 1)); fi
A=$(UV_PLEITO=3237 args "$(sed '/^CICLO=/d' <<< "$SEM")"$'\n')
confere "sem CICLO no no.env: ele2026" "$A" '--pleito 3237 --ciclo ele2026 '
grep -q "^RestartPreventExitStatus=78" "$AQUI/../servidor/urna-velox.service" && echo "ok: unidade não reinicia no 78" || { echo "FALHOU: urna-velox.service sem RestartPreventExitStatus=78"; FALHAS=$((FALHAS + 1)); }

echo "$N testes, $FALHAS falhas"
[ "$FALHAS" = 0 ]
