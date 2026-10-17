#!/usr/bin/env bash
# Passo 2 — gera os BUs sintéticos (corpus) e os layouts de contadores em $UV_WORK.
#   corpus_geral.bin        eleição geral 2026, BUs com assinatura por votável (~17 KB/BU)
#   corpus_geral_nosig.bin  idem, sem o campo de assinatura (~6,5 KB/BU)
#   corpus_municipal.bin    eleição municipal (~6 KB/BU)
#   layout_<tipo>.csv       posição de cada (município, cargo) no vetor de contadores
# A geração é determinística (sementes fixas): os mesmos parâmetros produzem os mesmos bytes.
# Use UV_REGERAR=1 para apagar e gerar de novo.
source "$(dirname "$0")/comum.sh"
[ -x "$UB" ] || falha "binário Rust não encontrado; rode scripts/setup.sh"
titulo "Gerando BUs sintéticos ($UV_NCORPUS distintos por corpus) em $UV_WORK"
LOG="$UV_OUT/geracao.txt"; : > "$LOG"
for spec in "geral corpus_geral.bin" "geral --nosig corpus_geral_nosig.bin" "municipal corpus_municipal.bin"; do
  set -- $spec; kind=$1; file=${!#}; flag=""; [ "$#" = 3 ] && flag=$2
  if [ "${UV_REGERAR:-0}" = "1" ] || [ ! -s "$UV_WORK/$file" ] || [ "$(cat "$UV_WORK/.n_$file" 2>/dev/null)" != "$UV_NCORPUS" ]; then
    "$UB" gen --kind "$kind" --n "$UV_NCORPUS" $flag --out "$UV_WORK" 2>&1 | tee -a "$LOG"
    echo "$UV_NCORPUS" > "$UV_WORK/.n_$file"
  else
    echo "[reaproveitado] $UV_WORK/$file" | tee -a "$LOG"
    "$UB" gen --kind "$kind" --n 1 $flag --out "$UV_WORK/.tmp_layout" 2>&1 | grep 'slots_densos' | tee -a "$LOG" >/dev/null
  fi
done
rm -rf "$UV_WORK/.tmp_layout"
