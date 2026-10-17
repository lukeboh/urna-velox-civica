#!/usr/bin/env bash
# compilar_local_arm64.sh — Compilação cruzada local (WSL/Linux x86_64 -> Linux aarch64) dos binários do Urna Velox.
# Garante que NENHUM código-fonte (.rs, .toml) precise ser enviado para a nuvem da Oracle.
# Gera deploy/oci/gerado/uv-binarios-aarch64.tar.gz contendo estritamente os executáveis e VERSION.
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
GERADO="$AQUI/gerado"
TARGET="aarch64-unknown-linux-gnu"
LINKER="aarch64-linux-gnu-gcc"
STRIP="aarch64-linux-gnu-strip"

msg() { printf '\n\033[1;36m== %s\033[0m\n' "$*"; }
aviso() { printf '\033[1;33m!! %s\033[0m\n' "$*"; }
erro() { printf '\033[1;31mXX %s\033[0m\n' "$*" >&2; exit 1; }

msg "1. Conferindo ferramentas de compilação cruzada"
command -v rustup >/dev/null 2>&1 || { [ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"; }
command -v cargo >/dev/null 2>&1 || erro "cargo não encontrado. Instale com: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
command -v "$LINKER" >/dev/null 2>&1 || erro "$LINKER não encontrado. Instale com: sudo apt-get install -y gcc-aarch64-linux-gnu"

# Garante que o target aarch64 está instalado no rustup
if ! rustup target list --installed | grep -q "^$TARGET$"; then
  msg "Instalando target $TARGET no rustup..."
  rustup target add "$TARGET"
fi

msg "2. Compilando em modo release para $TARGET (local)"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$LINKER"
cd "$RAIZ"
cargo build --release --target "$TARGET" -p uv-aquisicao -p urna-velox

msg "3. Montando pacote de binários limpo (sem fontes)"
PACOTE_DIR="$(mktemp -d /tmp/uv-binarios-XXXXXX)"
trap 'rm -rf "$PACOTE_DIR"' EXIT

mkdir -p "$PACOTE_DIR/bin"
BINS=(uv-servico uv-coletor uv-simulador uv-carga uv-motor)

for b in "${BINS[@]}"; do
  SRC_BIN="$RAIZ/target/$TARGET/release/$b"
  [ -f "$SRC_BIN" ] || erro "binário $SRC_BIN não foi gerado!"
  cp "$SRC_BIN" "$PACOTE_DIR/bin/$b"
  if command -v "$STRIP" >/dev/null 2>&1; then
    "$STRIP" "$PACOTE_DIR/bin/$b"
  fi
done

cp "$RAIZ/VERSION" "$PACOTE_DIR/VERSION"

mkdir -p "$GERADO"
DEST_TAR="$GERADO/uv-binarios-aarch64.tar.gz"
tar -czf "$DEST_TAR" -C "$PACOTE_DIR" bin VERSION

msg "4. Verificação do pacote gerado"
ls -lh "$DEST_TAR"
tar -ztvf "$DEST_TAR"

msg "5. Arquitetura dos binários"
for b in "${BINS[@]}"; do
  file "$PACOTE_DIR/bin/$b"
done

msg "Sucesso! Pacote gerado em: $DEST_TAR"
msg "Para implantar na OCI sem código-fonte: cd deploy/oci && ./10_implantar.sh --sem-compilar"
