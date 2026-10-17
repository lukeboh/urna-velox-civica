#!/usr/bin/env bash
# Passo 3 — teste de correção: todas as variantes (3 linguagens, parser manual e bibliotecas)
# totalizam os mesmos 3.000 BUs e precisam chegar ao MESMO checksum do vetor de totais,
# sem nenhum BU inválido. Se alguma divergir, o script para com erro: não adianta medir
# velocidade de um totalizador que conta errado.
source "$(dirname "$0")/comum.sh"
titulo "Verificando se todas as variantes totalizam igual"
N=3000; T=4; K="$UV_WORK/.verif"; : > "$K"
run() { # $1 = rótulo; resto = comando. Captura "checksum=" (stderr) e a coluna invalid (stdout).
  local lab=$1; shift
  local out; out=$(CHECK=1 "$@" 2>&1 | grep -v '^Picked up') || true
  local ck; ck=$(echo "$out" | sed -n 's/^checksum=//p' | tail -1)
  local inv; inv=$(echo "$out" | grep -E '^(rust|java|node|bun),' | tail -1 | awk -F, '{print $NF}')
  printf '  %-28s checksum=%-14s inválidos=%s\n' "$lab" "${ck:-?}" "${inv:-?}"
  echo "${ck:-SEM} ${inv:-SEM} $lab" >> "$K"
}
run "rust manual-dense"   "$UB" run --dir "$UV_WORK" --kind geral --variant manual-dense --threads $T --bus $N
run "rust rasn-dense"     "$UB" run --dir "$UV_WORK" --kind geral --variant rasn-dense   --threads $T --bus $N
run "rust manual-dense-sha512" "$UB" run --dir "$UV_WORK" --kind geral --variant manual-dense-sha512 --threads $T --bus $N
run "java manual-dense"   java -cp "$JCP" Bench "$UV_WORK" geral manual-dense $T $N
run "java bc-dense"       java -cp "$JCP" Bench "$UV_WORK" geral bc-dense     $T $N
run "node manual-dense"   node "$UV_WORK/js/bench.mjs" "$UV_WORK" geral manual-dense $T $N
run "node asn1js-dense"   node "$UV_WORK/js/bench.mjs" "$UV_WORK" geral asn1js-dense $T $N
command -v bun >/dev/null && run "bun manual-dense" bun "$UV_WORK/js/bench.mjs" "$UV_WORK" geral manual-dense $T $N
# As variantes com mapas aninhados (manual-dashmap, manual-chm) não usam o vetor denso e
# por isso não têm checksum comparável; são verificadas pela coluna "invalid" na medição.
n_ck=$(awk '{print $1}' "$K" | sort -u | wc -l)
bad=$(awk '$1=="SEM" || $2!="0"' "$K" | wc -l)
[ "$n_ck" = "1" ] && [ "$bad" = "0" ] || falha "as variantes divergiram (veja acima)."
echo "  OK: todas as variantes chegaram ao mesmo resultado."
