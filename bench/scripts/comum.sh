# shellcheck shell=bash
# Configuração compartilhada pelos scripts do benchmark. Não execute diretamente: é carregado com `source`.
#
# Variáveis que você pode definir antes de rodar (todas opcionais):
#   UV_WORK      pasta de trabalho (BUs gerados, binários, dependências)  padrão: ~/uvbench
#   UV_OUT       pasta de resultados                                      padrão: bench/results
#   UV_THREADS   lista de nº de threads a medir                           padrão: 1 2 4 8 12 16 ... até nproc
#   UV_BUS       BUs totalizados em cada medição                          padrão: 500000 (eleição geral)
#   UV_NCORPUS   BUs distintos gerados no corpus                          padrão: 20000
#   UV_RAPIDO=1  modo rápido (~1–2 min): menos threads, menos BUs, sem repetições

set -u
BENCH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$BENCH/.." && pwd)"
UV_WORK="${UV_WORK:-$HOME/uvbench}"
UV_OUT="${UV_OUT:-$BENCH/results}"
UV_NCORPUS="${UV_NCORPUS:-20000}"
NPROC="$(nproc)"

if [ "${UV_RAPIDO:-0}" = "1" ]; then
  UV_BUS="${UV_BUS:-100000}"
  UV_THREADS="${UV_THREADS:-$(printf '%s\n' 1 4 "$NPROC" | awk -v n="$NPROC" '$1<=n' | sort -n | uniq | tr '\n' ' ')}"
  REPS_DENSO=1; BUS_BC=10000; BUS_ASN1JS=2000
else
  UV_BUS="${UV_BUS:-500000}"
  UV_THREADS="${UV_THREADS:-$( (printf '%s\n' 1 2 4 8 12 16 20 24 28 32 48 64 | awk -v n="$NPROC" '$1<n'; echo "$NPROC") | sort -n | uniq | tr '\n' ' ')}"
  REPS_DENSO=3; BUS_BC=100000; BUS_ASN1JS=20000
fi

# Rust instalado pelo setup.sh fica em ~/.cargo/bin
[ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
UB="$BENCH/rust/target/release/urnabench"
MB="$BENCH/rust/target/release/membench"
JCP="$UV_WORK/java:$UV_WORK/bcprov.jar"
PY="$UV_WORK/.venv/bin/python"

titulo() { printf '\n\033[1;34m== %s ==\033[0m\n' "$*"; }
falha()  { printf '\033[1;31mERRO:\033[0m %s\n' "$*" >&2; exit 1; }
mkdir -p "$UV_WORK" "$UV_OUT"
