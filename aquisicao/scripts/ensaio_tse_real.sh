#!/usr/bin/env bash
# Ensaio da comparação Urna Velox × TSE contra o site REAL do TSE (etapa 7, critério 3 do docs/HANDOFF_0.7.0.md):
# uv-servico --sem-coleta --http contra https://resultados.tse.jus.br e 3 abas da página (2 no PC, 1 no celular) por N
# minutos. A única coisa que vai ao TSE são as buscas da comparação: o -u.json de cada tela, no máximo uma vez a cada
# 30 s por tópico (3 telas ≈ 6 req/min). Nenhum cs.json, aux.json ou BU (--sem-coleta).
#
#   bash ensaio_tse_real.sh --confirmo-tse --combinado <nome> <3220|3221> [minutos=30]
#
# 3220 (1º turno de 2026, encerrado): o serviço parte dos pacotes do acervo (~/uvreal/dados, por links) e a coluna do
# Urna Velox fica a final; a do TSE tem de ser igual ao arquivo. 3221 (2º turno, antes da apuração): as duas zeradas.
# Saída em ~/uvreal/ensaios/etapa7-<nome> (CLAUDE.md, "Disco"): pagina/, saida/, RESUMO.json.
set -euo pipefail
CONF=0; COMB=0; POS=()
for a in "$@"; do
  case "$a" in --confirmo-tse) CONF=1 ;; --combinado) COMB=1 ;; *) POS+=("$a") ;; esac
done
[ "$CONF" = 1 ] || { echo "vai ao site real do TSE: confirme com --confirmo-tse"; exit 1; }
if [ "$(date +%Y%m%d)" -lt 20261026 ] && [ "$COMB" = 0 ]; then
  echo "antes de 26/10/2026 nada roda contra o TSE sem combinar com o Luciano: use --combinado só se foi combinado"; exit 1
fi
NOME="${POS[0]:?nome}"; PLEITO="${POS[1]:?3220 ou 3221}"; MIN="${POS[2]:-30}"
U="${UV_WORK:-$HOME/uvreal}"
R=/mnt/d/git/lukeboh/urna-velox
B="${UV_BIN:-$U/target-dev/release}"
W="$U/ensaios/etapa7-$NOME"
BASE=https://resultados.tse.jus.br
case "$PLEITO" in
  3220) CAND=$U/etapa5/preparo-2026-1t/candidatos.json; ACERVO=$U/dados; OFIC=(--oficial "$U/oficial") ;;
  3221) CAND=$U/2026-2t/preparo/candidatos.json; ACERVO=; OFIC=() ;;
  *) echo "pleito: 3220 ou 3221"; exit 1 ;;
esac
[ -x "$B/uv-servico" ] || { echo "falta $B/uv-servico"; exit 1; }
bash "$R/scripts/disco.sh" --curto || true
rm -rf "$W"; mkdir -p "$W/dados" "$W/oficial"
# pacotes do acervo por links simbólicos (o serviço só lê: --sem-coleta não grava pacote novo)
[ -n "$ACERVO" ] && for f in "$ACERVO"/*.uvpk; do ln -s "$f" "$W/dados/"; done
PIDS=()
parar() { trap - INT TERM EXIT; kill "${PIDS[@]}" 2>/dev/null || true; wait 2>/dev/null || true; rm -rf "$W/dados"; }
trap parar INT TERM EXIT
HP=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
"$B/uv-servico" --pleito "$PLEITO" --ciclo ele2026 --candidatos "$CAND" --dados "$W/dados" --saida "$W/saida" \
  --base "$BASE" --sem-coleta --http "127.0.0.1:$HP" --continuar 2> "$W/servico.err" &
SPID=$!; PIDS+=($SPID)
for _ in $(seq 1 1500); do (exec 3<>"/dev/tcp/127.0.0.1/$HP") 2>/dev/null && break; sleep 0.2; done
# telas: Presidente no Brasil e os dois primeiros governadores (c0003) da lista do serviço
HASHES=$(curl -s -m 5 "http://127.0.0.1:$HP/api/v1/topicos" | python3 -c '
import json, sys
t = json.load(sys.stdin)
t = [x if isinstance(x, str) else (x.get("topico") or x.get("id")) for x in t]
esc = [x for x in t if x == "br-c0001"] + [x for x in t if x.endswith("-c0003") and len(x) == 8][:2]
esc = (esc + [x for x in t if x not in esc])[:3]
print(",".join("#/" + x.replace("-", "/") for x in esc))')
echo "serviço :$HP · pleito $PLEITO contra $BASE · $MIN min · telas $HASHES"
( cd "$R/site" && node testes/comparacao.mjs --url "http://127.0.0.1:$HP/" --saida "$W/pagina" --hashes "$HASHES" --abas 3 \
    "${OFIC[@]}" --minimo $((MIN * 60)) --limite $((MIN * 60 + 600)) > "$W/pagina.out" 2>&1 ) || echo "!! a página saiu com erro (veja $W/pagina.out)"
date +%s.%N > "$W/pagina_fechada"
sleep 35
curl -s -m 5 "http://127.0.0.1:$HP/api/v1/estado" > "$W/estado_fim.json" || true
kill -TERM $SPID 2>/dev/null || true
wait $SPID 2>/dev/null || true
python3 - "$W" <<'PY' | tee "$W/RESUMO.txt"
import json, os, sys
W = sys.argv[1]
s = json.load(open(os.path.join(W, "saida/saude.json")))
t = s.get("tse_resultados", {})
p = json.load(open(os.path.join(W, "pagina/resumo.json")))
abas = json.load(open(os.path.join(W, "pagina/abas.json")))
r = {"tse_resultados": t, "http429": s.get("rede", {}).get("http429"), "pagina": {k: p.get(k) for k in ("abas", "duracao_s", "cls_max", "rolagem_horizontal", "erros", "externas")},
     "telas": [{"aba": a["aba"], "modo": a["modo"], "movel": a["movel"], "topico": a.get("topico"), "arquivo": a.get("arquivo"), "fontes": a.get("fontes"), "divergencias": a.get("divergencias")} for a in abas]}
json.dump(r, open(os.path.join(W, "RESUMO.json"), "w"), ensure_ascii=False, indent=1)
print(f"buscas {t.get('buscas')} · 200 {t.get('r200')} · 304 {t.get('r304')} · 404 {t.get('r404')} · erros {t.get('erros')} · fora_do_intervalo {t.get('fora_do_intervalo')} · 429 {r['http429']}")
print(f"página: {r['pagina']['abas']} abas, {r['pagina']['duracao_s']} s, CLS máx {r['pagina']['cls_max']}, rolagem {r['pagina']['rolagem_horizontal']}, erros {len(r['pagina']['erros'] or [])}")
for x in r["telas"]: print(f"  aba {x['aba']} {x['modo']}{' celular' if x['movel'] else ''} {x['topico']}: {x['fontes']} · divergências {x['divergencias']}")
PY
