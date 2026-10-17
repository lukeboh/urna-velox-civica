#!/usr/bin/env bash
# ===========================================================================
#  Urna Velox — benchmark completo em um comando (Linux ou WSL2)
#
#    ./bench/benchmark.sh            execução completa  (~7–10 min num desktop de 8+ núcleos)
#    ./bench/benchmark.sh --rapido   execução curta para conferir que tudo funciona (~1–2 min)
#
#  Etapas:
#    1. scripts/setup.sh      confere/instala dependências e compila Rust, Java e JS
#    2. scripts/gerar_bus.sh  gera os BUs sintéticos de teste
#    3. scripts/verificar.sh  confere que todas as variantes totalizam igual
#    4. scripts/medir.sh      mede tempo e memória de cada variante e nº de threads
#    5. scripts/relatorio.py  imprime as tabelas e gera RELATORIO.md + gráficos
#
#  Resultados: bench/results/  (RELATORIO.md, graficos/*.png, time.csv, ...)
#  Detalhes e variáveis de configuração: bench/README.md
# ===========================================================================
set -u
cd "$(dirname "$0")" || exit 1
[ "${1:-}" = "--rapido" ] && export UV_RAPIDO=1
[ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ] && { sed -n '2,19p' "$0"; exit 0; }
T0=$(date +%s)
bash ./scripts/setup.sh     || exit 1
bash ./scripts/gerar_bus.sh || exit 1
bash ./scripts/verificar.sh || exit 1
bash ./scripts/medir.sh     || exit 1
source ./scripts/comum.sh
"$PY" ./scripts/relatorio.py --resultados "$UV_OUT" || exit 1
printf '\nConcluído em %d s. Relatório: %s/RELATORIO.md\n' $(( $(date +%s) - T0 )) "$UV_OUT"
