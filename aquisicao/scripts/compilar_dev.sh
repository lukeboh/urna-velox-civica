#!/usr/bin/env bash
# Compila e testa o workspace (binários `uv-motor` e `uv-coletor`) num diretório de build separado (target-dev),
# sem mexer no binário que estiver rodando uma coleta longa em ~/uvreal/target.
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cd "$RAIZ"
# disco da distro (CLAUDE.md, "Disco"): avisa se sobrou regenerável de ensaios anteriores
bash "$RAIZ/scripts/disco.sh" --curto || echo "!! limpe antes de ensaiar: bash $RAIZ/scripts/disco.sh --limpar"
export CARGO_TARGET_DIR="$UV_WORK/target-dev"
cargo build --release --workspace 2>&1 | grep -E '^(error|warning)|-->|Finished' | head -40
cargo test --release --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | head -20
ls -la "$CARGO_TARGET_DIR/release/uv-coletor" "$CARGO_TARGET_DIR/release/uv-motor"
