#!/usr/bin/env bash
# Passo 7 — confere a totalização com um decodificador independente (asn1tools + bu.asn1 oficial):
# a UF de referência e, se houver, todos os pacotes coletados.
source "$(dirname "$0")/comum.sh"
titulo "Conferência com decodificador de referência (UF $UV_UF_REF)"
"$PY" "$AQ/scripts/conferir_referencia.py" --dados "$DADOS" --ufs "$UV_UF_REF" --totais "$UV_OUT/totais_$UV_UF_REF.csv" \
   | tee "$UV_OUT/conferencia.txt"
if [ -s "$UV_WORK/totais_todas.csv" ]; then
  titulo "Conferência com decodificador de referência (todos os pacotes)"
  "$PY" "$AQ/scripts/conferir_referencia.py" --dados "$DADOS" --totais "$UV_WORK/totais_todas.csv" | tee -a "$UV_OUT/conferencia.txt"
fi
