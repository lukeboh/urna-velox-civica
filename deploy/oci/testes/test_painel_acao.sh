#!/usr/bin/env bash
# Testes do uv-painel-acao (sem root: caminhos temporários pelas variáveis de teste). Uso: bash test_painel_acao.sh
set -uo pipefail
AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACAO="$AQUI/../servidor/uv-painel-acao"
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
mkdir -p "$T/etc" "$T/pendente"
cat > "$T/systemctl" <<'SH'
#!/usr/bin/env bash
# o ciclo de simulações "roda" quando existe o arquivo ciclo-ativo
echo "$*" >> "$(dirname "$0")/chamadas"
if [ "$1" = is-active ]; then
  if [[ "$*" == *uv-ciclo-no* ]]; then [ -f "$(dirname "$0")/ciclo-ativo" ]; exit; fi
  echo active
fi
exit 0
SH
chmod +x "$T/systemctl"
cat > "$T/validador" <<'SH'
#!/usr/bin/env bash
# imita "uv-painel validar-rotas <arquivo> <base>": recusa o que tiver "ruim"
[ "$1" = validar-rotas ] && [ $# -eq 3 ] && ! grep -q ruim "$2"
SH
chmod +x "$T/validador"
export UV_ACAO_ETC="$T/etc" UV_ACAO_PENDENTE="$T/pendente" UV_ACAO_DONO="$(id -un)" UV_ACAO_SYSTEMCTL="$T/systemctl" UV_ACAO_VALIDADOR="$T/validador"
ORIGINAL=$'# nó A\nPLEITO=3221\nBASE=https://resultados.tse.jus.br\nRPS=95\nNO_ID=no-a\nEXTRA=\'--aceitar-incompleto\'\n'
FALHAS=0; N=0
ok() { N=$((N + 1)); if "$@" >/dev/null 2>"$T/err"; then echo "ok $N"; else echo "FALHOU $N: $* — $(cat "$T/err")"; FALHAS=$((FALHAS + 1)); fi; }
recusa() { N=$((N + 1)); if "$@" >/dev/null 2>&1; then echo "FALHOU $N (devia recusar): $*"; FALHAS=$((FALHAS + 1)); else echo "ok $N (recusado)"; fi; }
novo() { printf '%s' "$ORIGINAL" > "$T/etc/no.env"; printf '%s' "$1" > "$T/pendente/no.env.$2"; echo "$T/pendente/no.env.$2"; }

# 1. mudança válida: instala e guarda o anterior
P=$(novo $'# nó A\nPLEITO=3221\nBASE=https://resultados.tse.jus.br\nRPS=50\nNO_ID=no-a\nEXTRA=\'--aceitar-incompleto\'\nUFS=sp,rj\n' 1)
ok bash "$ACAO" aplicar-no-env "$P"
ok grep -qx 'RPS=50' "$T/etc/no.env"
ok grep -qx 'UFS=sp,rj' "$T/etc/no.env"
ok bash -c "ls '$T/etc/historico/' | grep -q no.env."
ok grep -qx 'RPS=95' "$T"/etc/historico/no.env.*

# modo espelho e teto (chaves novas, editáveis)
P=$(novo "${ORIGINAL}ACESSO=interno"$'\n'"TETO_RPS_INTERNO=0"$'\n' 10)
ok bash "$ACAO" aplicar-no-env "$P"
ok grep -qx 'ACESSO=interno' "$T/etc/no.env"
# 2. injeções e chaves proibidas
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL}UFS=\$(id)"$'\n' 2)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL}RPS=95;reboot"$'\n' 3)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL}UFS=\"sp\""$'\n' 4)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL}EXTRA=--controle-replay"$'\n' 5)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL//NO_ID=no-a/NO_ID=no-z}" 6)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL//NO_ID=no-a$'\n'/}" 7)"   # chave não editável sumindo
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL}PATH=/tmp"$'\n' 8)"
recusa bash "$ACAO" aplicar-no-env "$(novo "${ORIGINAL} RPS=1"$'\n' 9)"
ok grep -qx 'RPS=95' "$T/etc/no.env"   # nada disso foi instalado

# 3. arquivo fora da pasta pendente, nome errado, link
printf '%s' "$ORIGINAL" > "$T/fora"
recusa bash "$ACAO" aplicar-no-env "$T/fora"
printf '%s' "$ORIGINAL" > "$T/pendente/outro"
recusa bash "$ACAO" aplicar-no-env "$T/pendente/outro"
ln -sf "$T/fora" "$T/pendente/no.env.99"
recusa bash "$ACAO" aplicar-no-env "$T/pendente/no.env.99"
recusa bash "$ACAO" aplicar-no-env "$T/pendente/../fora"

# 4. serviço e trava
ok bash "$ACAO" servico reiniciar
ok grep -qx 'restart urna-velox.service' "$T/chamadas"
touch "$T/ciclo-ativo"
recusa bash "$ACAO" servico iniciar      # ciclo de simulações nas mesmas portas
recusa bash "$ACAO" servico reiniciar
ok bash "$ACAO" servico parar            # parar continua permitido
rm -f "$T/ciclo-ativo"
recusa bash "$ACAO" servico apagar-tudo
recusa bash "$ACAO" servico
ok bash "$ACAO" travar
ok test -f "$T/etc/controle-manual"
ok bash "$ACAO" destravar
ok test ! -f "$T/etc/controle-manual"
# 5. caminhos do site (rotas_tse.json)
printf '{"base_arquivos":"https://espelho.example"}' > "$T/pendente/rotas_tse.json.1"
ok bash "$ACAO" aplicar-rotas "$T/pendente/rotas_tse.json.1" https://resultados.tse.jus.br
ok grep -q espelho "$T/etc/rotas_tse.json"
printf '{"cs":"ruim"}' > "$T/pendente/rotas_tse.json.2"
recusa bash "$ACAO" aplicar-rotas "$T/pendente/rotas_tse.json.2" https://resultados.tse.jus.br
ok grep -q espelho "$T/etc/rotas_tse.json"   # o recusado não entrou
recusa bash "$ACAO" aplicar-rotas "$T/pendente/rotas_tse.json.1" 'https://x/$(id)'
recusa bash "$ACAO" aplicar-rotas "$T/fora" https://resultados.tse.jus.br
recusa bash "$ACAO" aplicar-rotas "$T/pendente/rotas_tse.json.1"
ok bash "$ACAO" remover-rotas
ok test ! -f "$T/etc/rotas_tse.json"
ok bash -c "ls '$T/etc/historico/' | grep -q rotas_tse.json."
recusa bash "$ACAO" qualquer-coisa
recusa bash "$ACAO"

echo "---"; [ "$FALHAS" -eq 0 ] && echo "test_painel_acao: $N ok" || { echo "test_painel_acao: $FALHAS falha(s) de $N"; exit 1; }
