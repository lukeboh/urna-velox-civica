#!/usr/bin/env bash
# Passo 1 — verifica e prepara as dependências e compila os três benchmarks.
#   * Rust: se `cargo` não existir, instala o Rust só para o seu usuário (rustup, em ~/.cargo; não usa sudo)
#   * Java 21+ e Node 20+: precisam estar instalados (o script só confere)
#   * Bouncy Castle (jar) e asn1js (npm): baixados para $UV_WORK
#   * Python + matplotlib: num ambiente virtual em $UV_WORK/.venv
# Defina UV_SEM_INSTALAR=1 para apenas conferir, sem instalar nada.
source "$(dirname "$0")/comum.sh"

titulo "Conferindo dependências"
need() { command -v "$1" >/dev/null || falha "$1 não encontrado. $2"; }
need java  "Instale um JDK 21+ (ex.: sudo apt install openjdk-21-jdk)."
need javac "Instale um JDK 21+ (ex.: sudo apt install openjdk-21-jdk)."
need node  "Instale o Node.js 20+ (ex.: https://nodejs.org)."
need npm   "Instale o npm junto com o Node.js."
need curl  "sudo apt install curl"
need python3 "sudo apt install python3 python3-venv"
JV=$(java -version 2>&1 | sed -n 's/.*version "\([0-9]*\).*/\1/p' | head -1)
[ "${JV:-0}" -ge 21 ] || falha "Java $JV encontrado; é preciso 21 ou superior."
NV=$(node -v | sed 's/v\([0-9]*\).*/\1/')
[ "$NV" -ge 20 ] || falha "Node $NV encontrado; é preciso 20 ou superior."

if ! command -v cargo >/dev/null; then
  [ "${UV_SEM_INSTALAR:-0}" = "1" ] && falha "cargo (Rust) não encontrado."
  need cc "O Rust precisa de um compilador C para ligar os binários: sudo apt install build-essential"
  titulo "Instalando o Rust (rustup) em ~/.cargo — só para este usuário"
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
  PATH="$HOME/.cargo/bin:$PATH"
fi
echo "rust:   $(rustc --version)"
echo "java:   $(java -version 2>&1 | head -1)"
echo "node:   $(node -v)"
command -v bun >/dev/null && echo "bun:    $(bun -v) (opcional, será medido)"

titulo "Compilando o benchmark Rust (release, LTO)"
(cd "$BENCH/rust" && cargo build --release --locked -q) || falha "falha ao compilar o Rust"

titulo "Baixando dependências das variantes com biblioteca"
[ -s "$UV_WORK/bcprov.jar" ] || curl -sSfL -o "$UV_WORK/bcprov.jar" \
  https://repo1.maven.org/maven2/org/bouncycastle/bcprov-jdk18on/1.78.1/bcprov-jdk18on-1.78.1.jar || falha "download do Bouncy Castle"
mkdir -p "$UV_WORK/js" "$UV_WORK/java"
cp "$BENCH/js/"*.mjs "$BENCH/js/package.json" "$BENCH/js/package-lock.json" "$UV_WORK/js/"
(cd "$UV_WORK/js" && npm ci --silent --no-audit --no-fund) || falha "npm ci"

titulo "Compilando o benchmark Java"
javac -cp "$UV_WORK/bcprov.jar" -d "$UV_WORK/java" "$BENCH/java/Bench.java" "$BENCH/java/Mem.java" || falha "javac"

titulo "Preparando Python + matplotlib (gráficos)"
if [ ! -x "$PY" ]; then python3 -m venv "$UV_WORK/.venv" || falha "python3 -m venv (instale python3-venv)"; fi
"$PY" -c "import matplotlib" 2>/dev/null || "$PY" -m pip install -q matplotlib || falha "pip install matplotlib"
echo "pronto."
