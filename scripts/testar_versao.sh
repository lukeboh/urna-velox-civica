#!/usr/bin/env bash
# Teste do scripts/versao.sh numa cópia temporária do repositório (não altera nada aqui).
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
mkdir -p "$T/scripts" "$T/motor" "$T/aquisicao" "$T/bench/rust" "$T/bench/js"
cp "$RAIZ/scripts/versao.sh" "$T/scripts/"
cp "$RAIZ/VERSION" "$RAIZ/CHANGELOG.md" "$T/"
cp "$RAIZ/Cargo.lock" "$T/"
cp "$RAIZ/motor/Cargo.toml" "$T/motor/"
cp "$RAIZ/aquisicao/Cargo.toml" "$T/aquisicao/"
cp "$RAIZ/bench/rust/Cargo.toml" "$RAIZ/bench/rust/Cargo.lock" "$T/bench/rust/"
cp "$RAIZ/bench/js/package.json" "$RAIZ/bench/js/package-lock.json" "$T/bench/js/"
cd "$T"
v0="$(bash scripts/versao.sh atual)"
saida="$(bash scripts/versao.sh bump patch "teste")"
echo "$saida"
v1="$(bash scripts/versao.sh atual)"
case "$saida" in "$v0 → $v1"*) ;; *) echo "FALHA: mensagem do bump"; exit 1 ;; esac
[ "$v1" != "$v0" ] || { echo "FALHA: versão não mudou"; exit 1; }
bash scripts/versao.sh verificar
bash scripts/versao.sh bump minor "teste" > /dev/null
case "$(bash scripts/versao.sh atual)" in *.0) ;; *) echo "FALHA: minor não zerou o patch"; exit 1 ;; esac
bash scripts/versao.sh verificar
echo "OK: versao.sh"
