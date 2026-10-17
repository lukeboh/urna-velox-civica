#!/usr/bin/env bash
# Uso do disco da distro pelo Urna Velox (regras no CLAUDE.md, "Disco"). Rode ANTES e DEPOIS de cada ensaio.
#
#   scripts/disco.sh                 relatório: espaço da distro, .vhdx no C:, o que é regenerável e onde
#   scripts/disco.sh --curto         uma linha (o compilar_dev.sh usa); código 3 se o regenerável passar de 5 GB
#   scripts/disco.sh --limpar        apaga só o regenerável (lista abaixo) e roda o fstrim
#   scripts/disco.sh arquivar <dir>  copia as evidências do ensaio (saida/, logs, resumos; sem dados/ nem federacao/)
#                                    para D:\dados\urna-velox\ensaios\<data>-<nome> e apaga <dir> da distro
#
# Regenerável (pode apagar sem perder evidência):
#   * dados/, federacao/ e armazem/ dentro das pastas de ensaio de $UV_WORK (etapa*/, simulacao-site/, teste*/,
#     ensaios/): cópias dos .uvpk e do diário que o uv-servico grava (os originais ficam em $UV_WORK/dados e no
#     acervo do D:) e os lotes trocados entre os nós;
#   * builds antigos em $UV_WORK/target-* que nenhum roteiro usa (ficam target, target-dev e target-cov);
#   * temporários uv-* em /tmp, /dev/shm e no TMPDIR de processos que já terminaram.
# Nunca toca: $UV_WORK/dados, 2024-1t/, 2024-2t/, 2026-2t/, acervo-*, preparo*, oficial/, config/, métricas (entradas).
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
D_ENSAIOS="${UV_ENSAIOS_D:-/mnt/d/dados/urna-velox/ensaios}"
LIMITE_GB="${UV_DISCO_LIMITE_GB:-5}"
RAIZES=(etapa3 etapa4 etapa5 simulacao-site ensaios teste9)
BUILDS_ANTIGOS=(target-c50fix target-exp target-v040 target-v2)

regeneraveis() { # caminhos regeneráveis, um por linha
  local r
  for r in "${RAIZES[@]}"; do
    [ -d "$UV_WORK/$r" ] || continue
    find "$UV_WORK/$r" -maxdepth 3 -type d \( -name dados -o -name federacao -o -name armazem \) -prune -print 2>/dev/null
  done
  for r in "${BUILDS_ANTIGOS[@]}"; do [ -d "$UV_WORK/$r" ] && echo "$UV_WORK/$r"; done
  local d n pid
  for d in /tmp/uv-* /dev/shm/uv-* /dev/shm/uv2-tmp/uv-* "${TMPDIR:-/tmp}"/uv-*; do
    [ -e "$d" ] || continue
    n=$(basename "$d"); pid=$(echo "$n" | sed -nE 's/^uv-[a-z0-9]+-([0-9]+)-.*/\1/p')
    # sem PID no nome: só se tiver mais de 1 dia; com PID: só se o processo não existe mais
    if [ -n "$pid" ]; then [ -d "/proc/$pid" ] || echo "$d"
    elif [ -n "$(find "$d" -maxdepth 0 -mmin +1440 2>/dev/null)" ]; then echo "$d"; fi
  done | sort -u
}

tamanho_kb() { [ $# -eq 0 ] && { echo 0; return; }; du -sk "$@" 2>/dev/null | awk '{s+=$1} END {print s+0}'; }

vhdx_gb() { # tamanho do .vhdx desta distro no Windows (vazio se não der para saber)
  command -v powershell.exe >/dev/null || return 0
  powershell.exe -NoProfile -Command "\$k=Get-ChildItem HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss | ForEach-Object { Get-ItemProperty \$_.PSPath } | Where-Object DistributionName -eq '$WSL_DISTRO_NAME'; if (\$k) { \$b=\$k.BasePath -replace '^\\\\\\\\\\?\\\\',''; \$v=Get-ChildItem \$b -Filter *.vhdx | Select-Object -First 1; if (\$v) { [math]::Round(\$v.Length/1GB,1) } }" 2>/dev/null | tr -d '\r'
}

case "${1:-}" in
  --curto)
    mapfile -t L < <(regeneraveis)
    kb=$(tamanho_kb "${L[@]}"); gb=$((kb / 1048576))
    usado=$(df -BG --output=used / | tail -1 | tr -d ' G')
    echo "disco: distro usa ${usado} GB; regenerável ${gb} GB em ${#L[@]} pasta(s)$([ "$gb" -ge "$LIMITE_GB" ] && echo " — rode scripts/disco.sh --limpar")"
    [ "$gb" -ge "$LIMITE_GB" ] && exit 3 || exit 0 ;;
  --limpar)
    mapfile -t L < <(regeneraveis)
    [ ${#L[@]} -eq 0 ] && { echo "nada regenerável para apagar"; exit 0; }
    kb=$(tamanho_kb "${L[@]}")
    printf 'apagando %s pasta(s), %s GB:\n' "${#L[@]}" "$((kb / 1048576))"
    for p in "${L[@]}"; do echo "  $p"; rm -rf -- "$p"; done
    # devolve os blocos livres ao .vhdx (sem isto o espaço só volta ao C: com a compactação)
    sudo -n fstrim -v / 2>/dev/null || echo "(fstrim precisa de sudo: rode 'sudo fstrim -v /' para o .vhdx poder encolher)"
    df -h / | tail -1 ;;
  arquivar)
    dir="${2:?informe a pasta do ensaio}"; [ -d "$dir" ] || { echo "não existe: $dir"; exit 1; }
    dest="$D_ENSAIOS/$(date +%Y%m%d-%H%M)-$(basename "$dir")"
    mkdir -p "$dest"
    # evidências: tudo menos dados/, federacao/ e armazem/ (regeneráveis)
    (cd "$dir" && find . \( -type d \( -name dados -o -name federacao -o -name armazem \) -prune \) -o -type f -print0 \
      | tar --null -T - -czf "$dest/evidencias.tar.gz")
    tar -tzf "$dest/evidencias.tar.gz" > "$dest/conteudo.txt"
    echo "evidências em $dest ($(du -sh "$dest/evidencias.tar.gz" | cut -f1), $(wc -l < "$dest/conteudo.txt") arquivos)"
    rm -rf -- "$dir" && echo "apagada da distro: $dir" ;;
  "")
    echo "== distro $WSL_DISTRO_NAME: $(df -h --output=used,avail / | tail -1 | awk '{print $1 " usados, " $2 " livres"}')"
    v=$(vhdx_gb); [ -n "$v" ] && echo "== .vhdx no Windows: $v GB (só encolhe com a compactação; ver CLAUDE.md, Disco)"
    echo "== \$UV_WORK ($UV_WORK), maiores:"; du -xsh "$UV_WORK"/* 2>/dev/null | sort -h | tail -8 | sed 's/^/   /'
    mapfile -t L < <(regeneraveis)
    echo "== regenerável: $(( $(tamanho_kb "${L[@]}") / 1048576 )) GB em ${#L[@]} pasta(s) (apague com --limpar)"
    for p in "${L[@]}"; do printf '   %6s  %s\n' "$(du -sh "$p" 2>/dev/null | cut -f1)" "$p"; done ;;
  *) sed -n '2,9p' "$0"; exit 2 ;;
esac
