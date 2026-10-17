#!/usr/bin/env bash
# Passo 4 — coleta completa da UF de referência pelos três caminhos e acompanhamento:
#   a) API de resultados (aux.json + bu.dat), gravando os pacotes .uvpk   ← corpus real
#   b) zip de dados abertos, só os BUs por HTTP Range                       (sem gravar)
#   c) zip de dados abertos inteiro + extração local                         (sem gravar; não roda no modo rápido)
#   d) acompanhamento (cs.json com If-None-Match) depois da coleta: custo de "nada mudou"
#   e) coleta em fluxo com totalização simultânea (catálogo da própria UF)
source "$(dirname "$0")/comum.sh"
U=$UV_UF_REF
titulo "a) API de resultados: UF $U completa (pacote refeito do zero)"
rm -f "$DADOS/$U.uvpk"
"$UC" baixar "${ALVO[@]}" --ufs "$U" -c 128 "${ADAPT[@]}" --dados "$DADOS" \
   --rotulo "uf-completa" --resumo "$RESUMO" --metricas "$UV_OUT/metricas_$U.csv" 2>&1 | grep -vE "^\s+\["
titulo "b) Dados abertos: só os BUs, por HTTP Range"
"$UC" zip "${ALVO[@]}" --uf "$U" --modo faixas -c 128 "${ADAPT[@]}" --sem-gravar \
   --dados "$UV_WORK/zip" --rotulo "zip-faixas" --resumo "$RESUMO" 2>&1 | grep -vE "^\s+\["
if [ "${UV_RAPIDO:-0}" != "1" ]; then
  titulo "c) Dados abertos: zip inteiro e extração local"
  "$UC" zip "${ALVO[@]}" --uf "$U" --modo completo --sem-gravar --dados "$UV_WORK/zip" \
     --rotulo "zip-completo" --resumo "$RESUMO" 2>&1 | grep -vE "^\s+\["
  rm -f "$UV_WORK/zip/dados-abertos-$U.zip"
fi
titulo "d) Acompanhamento: 3 rodadas, 10 s de intervalo"
"$UC" acompanhar "${ALVO[@]}" --ufs "$U" --rodadas 3 --intervalo 10 --dados "$DADOS" \
   --rotulo "acompanhar" --resumo "$RESUMO" 2>&1 | grep -E "rodada|^=="
titulo "e) Coleta em fluxo com totalização simultânea"
"$UC" catalogo --dados "$DADOS" --saida "$UV_WORK/catalogo_$U.txt"
"$UC" baixar "${ALVO[@]}" --ufs "$U" -c 128 "${ADAPT[@]}" --sem-gravar --totalizar --catalogo "$UV_WORK/catalogo_$U.txt" \
   --threads-totalizacao 4 --dados "$UV_WORK/fluxo" --rotulo "fluxo" --resumo "$RESUMO" 2>&1 | grep -vE "^\s+\["
