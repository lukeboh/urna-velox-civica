#!/usr/bin/env bash
# Passo 1 — confere dependências, compila o uv-coletor e prepara o Python (asn1tools + matplotlib).
source "$(dirname "$0")/comum.sh"
titulo "Conferindo dependências"
need() { command -v "$1" >/dev/null || falha "$1 não encontrado. $2"; }
need curl "sudo apt install curl"
need python3 "sudo apt install python3 python3-venv"
if ! command -v cargo >/dev/null; then
  [ "${UV_SEM_INSTALAR:-0}" = "1" ] && falha "cargo (Rust) não encontrado."
  need cc "O Rust precisa de um compilador C: sudo apt install build-essential"
  titulo "Instalando o Rust (rustup) em ~/.cargo — só para este usuário"
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
  PATH="$HOME/.cargo/bin:$PATH"
fi
echo "rust: $(rustc --version)"
titulo "Compilando o uv-coletor (release, LTO)"
(cd "$AQ" && CARGO_TARGET_DIR="$UV_WORK/target" cargo build --release --locked -q) || falha "falha ao compilar"
(cd "$AQ" && CARGO_TARGET_DIR="$UV_WORK/target" cargo test --release --locked -q 2>&1 | tail -2)
titulo "Preparando Python (asn1tools para a conferência, matplotlib para os gráficos)"
if [ ! -x "$PY" ]; then python3 -m venv "$UV_WORK/.venv" || falha "python3 -m venv (instale python3-venv)"; fi
"$PY" -c "import asn1tools, matplotlib" 2>/dev/null || "$PY" -m pip install -q asn1tools matplotlib || falha "pip install"
titulo "Conferindo acesso ao servidor de resultados"
# HTTP 429 aqui = o IP está na "caixa de penalidade" da borda do CDN (dura ~8 min): espera, 1 tentativa por minuto
for i in $(seq 1 15); do
  st=$(curl -s -m 20 -o /dev/null -w "%{http_code}" "https://resultados.tse.jus.br/oficial/ele2026/arquivo-urna/3220/config/ac/ac-p003220-cs.json")
  [ "$st" = "200" ] && break
  [ "$st" = "429" ] || falha "sem acesso a resultados.tse.jus.br (HTTP $st)"
  echo "  HTTP 429: IP em penalidade no CDN; nova tentativa em 60 s ($i/15)"; sleep 60
done
[ "$st" = "200" ] || falha "o CDN continua devolvendo 429 depois de 15 min"
echo "pronto."
