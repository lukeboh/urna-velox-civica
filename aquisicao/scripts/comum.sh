# shellcheck shell=bash
# Configuração compartilhada pelos scripts do teste real de aquisição. Carregado com `source`.
#
# Variáveis (todas opcionais):
#   UV_WORK            pasta de trabalho (binário, pacotes de BUs, zip)        padrão: ~/uvreal
#   UV_OUT             pasta de resultados                                      padrão: aquisicao/results
#   UV_CICLO/UV_PLEITO pleito a coletar                                         padrão: ele2026 / 3220 (1º turno 2026)
#   UV_UF_REF          UF da coleta completa de referência e da conferência     padrão: df
#   UV_UFS_VARREDURA   UFs sorteadas na varredura de concorrência               padrão: sp,mg,ba,rj (234 mil seções)
#   UV_CONC            concorrências da varredura                               padrão: 1 4 16 32 64
#   UV_RPS_MAX         teto de requisições/s (o coletor nunca passa de 100)     padrão: 100
#   UV_UFS_EXTRA       UFs da coleta sustentada (corpus multi-UF)               padrão: ac,al,ap,pb,rn,rr,se,to
#   UV_NACIONAL=1      coleta sustentada de TODAS as UFs (~517 mil seções, ~9 GB; horas pelo mesmo IP)
#
# Limite de uso: no máximo 100 requisições/s aos servidores do TSE (o uv-coletor impõe esse teto
# em código, TETO_RPS). Acima de ~200 req/s a borda do CDN (Akamai) devolve HTTP 429 e põe o IP de
# saída numa "caixa de penalidade" de vários minutos — em rede corporativa, um IP compartilhado (NAT).
#   UV_RAPIDO=1        execução curta (amostras menores, sem zip completo)
set -u
AQ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$AQ/.." && pwd)"
UV_WORK="${UV_WORK:-$HOME/uvreal}"
UV_OUT="${UV_OUT:-$AQ/results}"
UV_CICLO="${UV_CICLO:-ele2026}"
UV_PLEITO="${UV_PLEITO:-3220}"
UV_UF_REF="${UV_UF_REF:-df}"
UV_UFS_VARREDURA="${UV_UFS_VARREDURA:-sp,mg,ba,rj}"
UV_RPS_MAX="${UV_RPS_MAX:-100}"; [ "$UV_RPS_MAX" -gt 100 ] 2>/dev/null && UV_RPS_MAX=100
UV_UFS_EXTRA="${UV_UFS_EXTRA:-ac,al,ap,pb,rn,rr,se,to}"
UV_NACIONAL="${UV_NACIONAL:-0}"
if [ "${UV_RAPIDO:-0}" = "1" ]; then
  UV_CONC="${UV_CONC:-4 32}"; AMOSTRA_MULT=4; UV_UFS_EXTRA="${UV_UFS_EXTRA_RAPIDO:-rr}"
else
  UV_CONC="${UV_CONC:-1 4 16 32 64}"; AMOSTRA_MULT=8
fi
# controle adaptativo de taxa usado nas coletas longas
ADAPT=(--adaptativo --rps-inicial 40 --rps "$UV_RPS_MAX")
[ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
UC="$UV_WORK/target/release/uv-coletor"
DADOS="$UV_WORK/dados"
PY="$UV_WORK/.venv/bin/python"
ALVO=(--ciclo "$UV_CICLO" --pleito "$UV_PLEITO")
RESUMO="$UV_OUT/coleta_resumo.csv"
titulo() { printf '\n\033[1;34m== %s ==\033[0m\n' "$*"; }
falha()  { printf '\033[1;31mERRO:\033[0m %s\n' "$*" >&2; exit 1; }
mkdir -p "$UV_WORK" "$UV_OUT"
