#!/usr/bin/env bash
# Ativa os hooks versionados em .githooks/ (bump obrigatório e tag automática). Rode uma vez por clone.
set -eu
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
git -C "$RAIZ" config core.hooksPath .githooks
chmod +x "$RAIZ/.githooks/"* "$RAIZ/scripts/versao.sh" 2>/dev/null || true
git -C "$RAIZ" config push.followTags true
echo "hooks ativos: $(git -C "$RAIZ" config core.hooksPath); push.followTags=$(git -C "$RAIZ" config push.followTags)"
