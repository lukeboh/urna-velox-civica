#!/usr/bin/env bash
# Passo 4 — mede tempo e memória de todas as variantes, para cada número de threads.
# Saídas em $UV_OUT (substitui a execução anterior):
#   time.csv      uma linha por medição (formato em bench/README.md)
#   mem_rust.txt  memória do estado por desenho, em Rust
#   mem_java.txt  idem em Java;  mem_node.txt  idem em Node + custo de cada worker
#   machine.txt   CPU, memória e versões usadas
source "$(dirname "$0")/comum.sh"
[ -s "$UV_WORK/corpus_geral.bin" ] || falha "corpus não encontrado; rode scripts/gerar_bus.sh"

{ echo "# $(date -Is)"; lscpu | grep -E "Model name|^CPU\(s\)|Thread|Core|Socket|L3"; free -m | head -2; uname -r
  rustc --version; java -version 2>&1 | head -1; echo "node $(node -v)"; command -v bun >/dev/null && echo "bun $(bun -v)"
  echo "UV_BUS=$UV_BUS UV_NCORPUS=$UV_NCORPUS UV_THREADS=\"$UV_THREADS\" UV_RAPIDO=${UV_RAPIDO:-0}"; } > "$UV_OUT/machine.txt"

T="$UV_OUT/time.csv"; LOG="$UV_OUT/run.log"; : > "$LOG"
echo "lang,variant,kind,threads,bus,seconds,bus_per_s,consolidate_ms,peak_rss_mb,corpus_mb,invalid" > "$T"
r() { timeout 1800 "$@" 2>>"$LOG" | grep -v '^Picked up' | tee -a "$T"; }
W="$UV_WORK"; JS="$W/js/bench.mjs"
nthr=$(echo $UV_THREADS | wc -w); i=0
for t in $UV_THREADS; do
  i=$((i+1)); titulo "Medindo com $t thread(s)  [$i/$nthr]"
  for _ in $(seq $REPS_DENSO); do r "$UB" run --dir "$W" --kind geral --variant manual-dense --threads $t --bus $UV_BUS; done
  r "$UB" run --dir "$W" --kind geral --variant manual-dense --threads $t --bus $UV_BUS --corpus "$W/corpus_geral_nosig.bin"
  r "$UB" run --dir "$W" --kind municipal --variant manual-dense --threads $t --bus $UV_BUS
  r "$UB" run --dir "$W" --kind geral --variant manual-dense-sha512 --threads $t --bus $UV_BUS
  r "$UB" run --dir "$W" --kind geral --variant rasn-dense --threads $t --bus $UV_BUS
  r "$UB" run --dir "$W" --kind geral --variant manual-dashmap --threads $t --bus $UV_BUS
  for _ in $(seq $(( REPS_DENSO > 1 ? 2 : 1 ))); do r java -Xmx3g -cp "$JCP" Bench "$W" geral manual-dense $t $UV_BUS; done
  r java -Xmx3g -cp "$JCP" Bench "$W" municipal manual-dense $t $UV_BUS
  r java -Xmx3g -cp "$JCP" Bench "$W" geral bc-dense $t $BUS_BC
  r java -Xmx6g -cp "$JCP" Bench "$W" geral manual-chm $t $UV_BUS
  for _ in $(seq $(( REPS_DENSO > 1 ? 2 : 1 ))); do r node --max-old-space-size=4096 "$JS" "$W" geral manual-dense $t $UV_BUS; done
  r node --max-old-space-size=4096 "$JS" "$W" municipal manual-dense $t $UV_BUS
  # asn1js é ~200× mais lento: só em alguns pontos e com amostra menor (extrapolada no relatório)
  case " 1 4 8 16 $NPROC " in *" $t "*) r node --max-old-space-size=4096 "$JS" "$W" geral asn1js-dense $t $BUS_ASN1JS;; esac
  command -v bun >/dev/null && r bun "$JS" "$W" geral manual-dense $t $UV_BUS
done

titulo "Medindo a memória do estado por desenho (mesma carga nas 3 linguagens)"
cd "$W" || exit 1
"$MB" 4500000 > "$UV_OUT/mem_rust.txt" 2>&1
DUMP="$W/wl.csv" "$MB" 4500000 >/dev/null
for m in naive adder dense; do java -Xmx4g -cp "$JCP" Mem $m 2>&1 | grep -v '^Picked up'; done > "$UV_OUT/mem_java.txt"
{ for m in naive dense; do node --expose-gc --max-old-space-size=4096 "$W/js/mem.mjs" $m; done
  for n in 8 16 "$NPROC"; do node --expose-gc "$W/js/mem.mjs" workers $n; done; } > "$UV_OUT/mem_node.txt" 2>&1
cat "$UV_OUT/mem_rust.txt" "$UV_OUT/mem_java.txt" "$UV_OUT/mem_node.txt"
