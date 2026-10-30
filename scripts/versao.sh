#!/usr/bin/env bash
# Versionamento do Urna Velox (ver docs/processo/VERSIONAMENTO.md).
#
#   scripts/versao.sh atual                 mostra a versão (arquivo VERSION)
#   scripts/versao.sh bump [patch|minor|major|alpha] "resumo"   (alpha: X.Y.Z-alpha.N → N+1)
#                                           sobe a versão em VERSION e em todos os manifestos e abre
#                                           a entrada do CHANGELOG.md (a ser completada antes do commit)
#   scripts/versao.sh definir X.Y.Z         grava uma versão exata (uso excepcional)
#   scripts/versao.sh verificar             confere se VERSION, manifestos e CHANGELOG estão coerentes
#   scripts/versao.sh tag                   cria a tag anotada vX.Y.0 no HEAD, só se o PATCH for 0 (o hook post-commit faz isso)
#
# Só usa sh, sed, awk e grep (roda no Git Bash do Windows e no WSL).
set -eu
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
cd "$RAIZ"
MANIFESTOS_CARGO="motor/Cargo.toml aquisicao/Cargo.toml bench/rust/Cargo.toml"
# workspace (motor + aquisicao): um Cargo.lock só, na raiz
PACOTES_CARGO="urna-velox:Cargo.lock uv-aquisicao:Cargo.lock urnabench:bench/rust/Cargo.lock"
MANIFESTOS_NPM="bench/js/package.json bench/js/package-lock.json"

atual() { tr -d ' \r\n' < VERSION; }

versao_cargo() { sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$1" | head -1; }
versao_lock() { # pacote arquivo
  awk -v p="$1" '{sub(/\r$/, "")} $0=="name = \""p"\"" {getline; sub(/\r$/, ""); gsub(/version = |"/, ""); print; exit}' "$2"; }
versao_npm() { sed -n 's/^  "version": *"\([^"]*\)".*/\1/p' "$1" | head -1 | tr -d '\r'; }

aplicar() { # nova versão em todos os manifestos (usa "nv" para não sobrescrever o "v" de quem chama)
  nv="$1"
  printf '%s\n' "$nv" > VERSION
  for m in $MANIFESTOS_CARGO; do
    # só a primeira linha version = (a do [package])
    awk -v v="$nv" 'BEGIN{f=0} /^version *= *"/ && !f {print "version = \"" v "\""; f=1; next} {print}' "$m" > "$m.tmp" && mv "$m.tmp" "$m"
  done
  for pl in $PACOTES_CARGO; do
    p="${pl%%:*}"; l="${pl#*:}"
    [ -f "$l" ] || continue
    awk -v p="$p" -v v="$nv" '{print} {x=$0; sub(/\r$/, "", x)} x=="name = \""p"\"" {getline; print "version = \"" v "\""}' "$l" > "$l.tmp" && mv "$l.tmp" "$l"
  done
  for m in $MANIFESTOS_NPM; do
    [ -f "$m" ] || continue
    # package.json: "version" do topo; package-lock: "version" do topo e de packages[""]
    awk -v v="$nv" '
      /^  "version": *"/ {sub(/"version": *"[^"]*"/, "\"version\": \"" v "\"")}
      /^      "version": *"/ && raiz {sub(/"version": *"[^"]*"/, "\"version\": \"" v "\""); raiz=0}
      /^    "": *\{/ {raiz=1}
      {print}' "$m" > "$m.tmp" && mv "$m.tmp" "$m"
  done
}

verificar() {
  v="$(atual)"; ok=1
  echo "$v" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-alpha\.[0-9]+)?$' || { echo "VERSION inválida: '$v'"; ok=0; }
  for m in $MANIFESTOS_CARGO; do [ "$(versao_cargo "$m" | tr -d '\r')" = "$v" ] || { echo "$m: $(versao_cargo "$m") ≠ $v"; ok=0; }; done
  for pl in $PACOTES_CARGO; do
    p="${pl%%:*}"; l="${pl#*:}"; [ -f "$l" ] || continue
    [ "$(versao_lock "$p" "$l")" = "$v" ] || { echo "$l ($p): $(versao_lock "$p" "$l") ≠ $v"; ok=0; }
  done
  [ "$(versao_npm bench/js/package.json)" = "$v" ] || { echo "bench/js/package.json: $(versao_npm bench/js/package.json) ≠ $v"; ok=0; }
  grep -q "^## \[$v\]" CHANGELOG.md || { echo "CHANGELOG.md sem a entrada ## [$v]"; ok=0; }
  if grep -A3 "^## \[$v\]" CHANGELOG.md | grep -q "PREENCHER"; then echo "CHANGELOG.md: entrada [$v] ainda com PREENCHER"; ok=0; fi
  if [ "$ok" = 1 ]; then echo "versão $v coerente"; else return 1; fi
}

abrir_changelog() { # versão resumo
  data="$(date +%Y-%m-%d)"
  awk -v v="$1" -v d="$data" -v r="$2" 'BEGIN{feito=0} /^## \[/ && !feito {print "## [" v "] — " d; print ""; print "- " r; print ""; feito=1} {print}' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
}

case "${1:-atual}" in
  atual) atual ;;
  verificar) verificar ;;
  definir)
    nova="${2:?informe X.Y.Z}"; aplicar "$nova"
    grep -q "^## \[$nova\]" CHANGELOG.md || abrir_changelog "$nova" "${3:-PREENCHER}"
    echo "versão definida: $nova" ;;
  bump)
    tipo="${2:-patch}"; resumo="${3:-PREENCHER}"
    v="$(atual)"
    case "$v" in
      *-alpha.*)
        # pré-release: só "bump alpha"; para sair dela, "definir X.Y.Z"
        [ "$tipo" = "alpha" ] || { echo "versão $v é alpha: use 'bump alpha' (ou 'definir X.Y.Z' ao concluir a MAJOR)"; exit 2; }
        n="${v##*-alpha.}"; nova="${v%-alpha.*}-alpha.$((n + 1))" ;;
      *)
        ma="${v%%.*}"; r="${v#*.}"; mi="${r%%.*}"; pa="${r#*.}"
        case "$tipo" in
          patch) pa=$((pa + 1)) ;;
          minor) mi=$((mi + 1)); pa=0 ;;
          major) ma=$((ma + 1)); mi=0; pa=0 ;;
          alpha) echo "versão $v não é alpha: comece com 'definir X.0.0-alpha.1'"; exit 2 ;;
          *) echo "tipo inválido: $tipo (patch|minor|major|alpha)"; exit 2 ;;
        esac
        nova="$ma.$mi.$pa" ;;
    esac
    aplicar "$nova"
    abrir_changelog "$nova" "$resumo"
    echo "$v → $nova (complete a entrada do CHANGELOG.md antes do commit)"
    ;;
  tag)
    v="$(atual)"
    # só as versões "zeradas" (X.Y.0: etapa concluída ou MAJOR) ganham tag; os PATCH não
    case "$v" in *-*) echo "sem tag para v$v (pré-release)"; exit 0 ;; *.*.0) ;; *) echo "sem tag para v$v (só as versões X.Y.0 ganham tag)"; exit 0 ;; esac
    if git rev-parse -q --verify "refs/tags/v$v" >/dev/null; then echo "tag v$v já existe"; exit 0; fi
    git tag -a "v$v" -m "v$v — $(git log -1 --format=%s)"
    echo "tag v$v criada em $(git rev-parse --short HEAD)"
    ;;
  *) sed -n '2,13p' "$0"; exit 2 ;;
esac
