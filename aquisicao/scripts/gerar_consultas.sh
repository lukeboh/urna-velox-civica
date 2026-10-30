#!/usr/bin/env bash
# Multipleito (1.1): gera a série de consulta de pleitos encerrados (a base do slider da página; ver
# docs/propostas/PROPOSTA_SERIE_CONSULTA.md). Para cada acervo de replay pronto (montar_acervo.sh: `.completo` e
# `acervo.json`): replay rápido local (replay_acervo.sh, com o acervo de versões do uv-servico) e uv-serie, com o lado do
# TSE das parciais oficiais (quando houver) ou reconstruído pela hora de recebimento de cada BU. A série nova só toma o
# lugar da anterior se a última versão de cada tópico for idêntica à publicação final. No fim, o índice geral
# `<destino>/indice.json` (a lista que a página usa para oferecer "Consultar a noite"). Nenhuma requisição ao TSE.
#
#   gerar_consultas.sh --destino <pasta> --acervo <acervo> [--parciais <pasta tse-parciais>] [--forcar]
#   gerar_consultas.sh --destino <pasta> --acervos <pasta com um acervo por pleito> [--forcar]
#   gerar_consultas.sh --destino <pasta> --indice           (só refaz o índice geral)
#
# Variáveis: UV_BIN (pasta do uv-simulador, uv-servico e uv-serie; padrão target/release), ACEL (500), TMPDIR.
# Uma série que já existe e é mais nova que o `.completo` do acervo não é refeita (--forcar refaz).
set -uo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
B="${UV_BIN:-$RAIZ/target/release}"
ACEL="${ACEL:-500}"
DEST="" ; UM="" ; TODOS="" ; PARC="" ; FORCAR="" ; SO_INDICE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --destino) DEST="$2"; shift ;;
    --acervo) UM="$2"; shift ;;
    --acervos) TODOS="$2"; shift ;;
    --parciais) PARC="$2"; shift ;;
    --forcar) FORCAR=1 ;;
    --indice) SO_INDICE=1 ;;
    *) echo "opção desconhecida: $1" >&2; exit 2 ;;
  esac
  shift
done
[ -n "$DEST" ] || { echo "uso: gerar_consultas.sh --destino <pasta> (--acervo <acervo> | --acervos <pasta> | --indice)" >&2; exit 2; }
mkdir -p "$DEST"

gerar_um() {
  local A="$1" P D R T
  [ -f "$A/.completo" ] && [ -f "$A/acervo.json" ] || { echo "$A: acervo incompleto (falta .completo ou acervo.json)"; return 1; }
  P=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['pleito'])" "$A/acervo.json")
  D="$DEST/$P/serie"
  # a série da noite em curso (EM_CURSO, do vigia) dá lugar à final assim que o acervo fica pronto
  if [ -z "$FORCAR" ] && [ -f "$D/indice.json" ] && [ ! -f "$D/EM_CURSO" ] && [ "$D/indice.json" -nt "$A/.completo" ]; then echo "$P: série em dia"; return 0; fi
  R=$(mktemp -d "${TMPDIR:-/tmp}/uv-consulta-$P-XXXXXX")
  echo "$P: replay rápido do acervo ($(date +%T))"
  # o replay confere com o oficial; uma divergência conhecida (B-066) não impede a série, que confere com a publicação
  UV_BIN="$B" ACEL="$ACEL" nice -n 10 bash "$(dirname "$0")/replay_acervo.sh" "$A" "$R" > "$R/replay.log" 2>&1 \
    || echo "$P: o replay não fechou idêntico ao oficial: $(grep -m1 '^{' "$R/replay.log")"
  [ -d "$R/saida/serie" ] || { echo "$P: replay sem acervo de versões (uv-servico antigo?)"; tail -5 "$R/replay.log"; rm -rf "$R"; return 1; }
  local tse=()
  if [ -n "$PARC" ]; then tse=(--tse-parciais "$PARC"); else tse=(--tse-dados "$A/dados" --linha-do-tempo "$A/linha-do-tempo.csv"); fi
  T="$DEST/$P/serie.nova"
  rm -rf "$T"
  if nice -n 10 "$B/uv-serie" --saida "$R/saida" --candidatos "$A/candidatos.json" --destino "$T" --oficial "$A/oficial" "${tse[@]}" > "$R/serie.json"; then
    rm -rf "$D"; mv "$T" "$D"
    cp "$R/serie.json" "$DEST/$P/montagem.json"
    echo "$P: série pronta, $(du -sh "$D" | cut -f1) ($(date +%T))"
  else
    echo "$P: o uv-serie recusou (nada mudou):"; head -c 1500 "$R/serie.json"; echo
    rm -rf "$R" "$T"; return 1
  fi
  rm -rf "$R"
}

indice() {
  python3 - "$DEST" <<'PY'
import gzip, json, os, sys
d = sys.argv[1]
out = []
for p in sorted(os.listdir(d)):
    f = os.path.join(d, p, 'serie', 'indice.json')
    if not p.isdigit() or not os.path.exists(f):
        continue
    i = json.load(open(f))
    out.append({'pleito': int(p), 'inicio': i.get('inicio'), 'fim': i.get('fim'), 'topicos': len(i.get('topicos') or {}),
                'tse': (i.get('tse') or {}).get('fonte'), 'gerado_por': i.get('gerado_por'), 'em_curso': bool(i.get('em_curso'))})
corpo = json.dumps({'v': 1, 'pleitos': out}, ensure_ascii=False, separators=(',', ':'))
for nome, dados in (('indice.json', corpo.encode()), ('indice.json.gz', gzip.compress(corpo.encode(), 9))):
    t = os.path.join(d, nome + '.tmp')
    open(t, 'wb').write(dados)
    os.replace(t, os.path.join(d, nome))
print(f'índice: {len(out)} pleito(s) com série em {d}')
PY
}

RC=0
if [ -n "$UM" ]; then gerar_um "$UM" || RC=1; fi
if [ -n "$TODOS" ]; then
  for A in "$TODOS"/*/; do [ -f "$A/acervo.json" ] && { gerar_um "${A%/}" || RC=1; }; done
fi
indice
exit $RC
