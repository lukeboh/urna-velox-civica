#!/usr/bin/env bash
# ===========================================================================
#  Urna Velox — teste REAL da fase de aquisição (Linux ou WSL2)
#
#    ./aquisicao/teste-real.sh             completo (~35 min, quase todo esperando a rede)
#    ./aquisicao/teste-real.sh --rapido    curto, só para conferir que funciona (~3 min)
#    UV_NACIONAL=1 ./aquisicao/teste-real.sh   coleta TODAS as seções (~517 mil BUs, ~9 GB, horas)
#
#  Baixa BUs de verdade de https://resultados.tse.jus.br com controle adaptativo de taxa
#  (a borda do CDN pune o IP acima de ~200 req/s) e totaliza com o modelo de estado do núcleo.
#
#  Etapas:
#    1. scripts/setup.sh            compila o uv-coletor; Python com asn1tools e matplotlib
#    2. scripts/descobrir.sh        pleitos, eleições e seções de todas as UFs
#    3. scripts/varredura.sh        vazão × concorrência, HTTP/1.1 × HTTP/2, sonda adaptativa de taxa
#    4. scripts/coleta_uf.sh        UF de referência: API, zip (faixas e inteiro), acompanhamento, fluxo
#    5. scripts/coleta_multi.sh     coleta sustentada de várias UFs (ou todas, com UV_NACIONAL=1)
#    6. scripts/totalizar.sh        catálogo + totalização dos BUs reais por nº de threads
#    7. scripts/conferir.sh         confere os totais com decodificador independente
#    8. scripts/relatorio.py        RELATORIO.md + gráficos
#  Resultados: aquisicao/results/   Detalhes e variáveis: aquisicao/README.md
# ===========================================================================
set -u
cd "$(dirname "$0")" || exit 1
[ "${1:-}" = "--rapido" ] && export UV_RAPIDO=1
[ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ] && { sed -n '2,23p' "$0"; exit 0; }
T0=$(date +%s)
source ./scripts/comum.sh
rm -f "$RESUMO"
for s in setup descobrir varredura coleta_uf coleta_multi totalizar conferir; do
  bash "./scripts/$s.sh" || falha "etapa $s"
done
"$PY" ./scripts/relatorio.py --resultados "$UV_OUT" || exit 1
printf '\nConcluído em %d s. Relatório: %s/RELATORIO.md\n' $(( $(date +%s) - T0 )) "$UV_OUT"
