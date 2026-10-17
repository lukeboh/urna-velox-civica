#!/usr/bin/env bash
# Cobertura de testes de unidade do código Rust (cargo-llvm-cov). Instala a ferramenta se faltar.
# Saída: resumo por arquivo no terminal e relatório HTML em ~/uvreal/cobertura/<crate>/html.
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
UV_WORK="${UV_WORK:-$HOME/uvreal}"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! cargo llvm-cov --version >/dev/null 2>&1; then
  rustup component add llvm-tools-preview >/dev/null 2>&1
  cargo install cargo-llvm-cov --locked -q
fi
for crate in motor aquisicao bench/rust; do
  nome="$(basename "$crate")"
  echo "== $crate"
  ( cd "$RAIZ/$crate" && CARGO_TARGET_DIR="$UV_WORK/target-cov/$nome" cargo llvm-cov --release --summary-only 2>&1 | grep -vE '^\s*(Compiling|Finished|Running|Downloaded|Downloading)' | sed -n '/^Filename/,$p;/test result/p' )
  ( cd "$RAIZ/$crate" && CARGO_TARGET_DIR="$UV_WORK/target-cov/$nome" cargo llvm-cov report --release --html --output-dir "$UV_WORK/cobertura/$nome" >/dev/null 2>&1 ) || true
done
