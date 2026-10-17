#!/usr/bin/env bash
# Fecha a coleta nacional: catálogo → totalização → comparação com a divulgação oficial, com resumo curto.
# Uso: fechar_nacional.sh [pasta dos .uvpk] [pasta de saída]
set -uo pipefail
UV_WORK="${UV_WORK:-$HOME/uvreal}"
D="${1:-$UV_WORK/dados}"; O="${2:-$UV_WORK/resultado}"
bash "$(dirname "$0")/totalizar_comparar.sh" "$D" "$O" > "$O.log" 2>&1
grep -E "^catálogo|BUs em memória|^conferência|real,28,|^comparação|divergência" "$O.log"
echo "abrangências conferidas sem divergência: $(grep -c '✅' "$O/comparacao_oficial.md")"
grep -E '❌' "$O/comparacao_oficial.md" || true
grep -E '\| (BR|MG|ZZ) \|' "$O/comparacao_oficial.md"
