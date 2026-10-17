#!/usr/bin/env bash
# Passo 2 — descobre pleitos/eleições e as seções de todas as UFs (cs.json).
source "$(dirname "$0")/comum.sh"
titulo "Pleitos e eleições publicados"
"$UC" eleicoes | tee "$UV_OUT/eleicoes.txt" | grep -E "pleito|$UV_CICLO"
titulo "Seções por UF (pleito $UV_PLEITO)"
"$UC" secoes "${ALVO[@]}" --ufs todas --cache "$DADOS/config" --csv "$UV_WORK/secoes.csv" | tee "$UV_OUT/secoes.txt"
{ echo "# $(date -Is)"; lscpu | grep -E "Model name|^CPU\(s\)"; free -m | head -2; uname -r; "$UC" --version
  curl -s -o /dev/null -w "ip_local %{local_ip}  servidor %{remote_ip}  tls %{time_appconnect}s\n" https://resultados.tse.jus.br/oficial/comum/config/ele-c.json
  echo "UV_CONC=\"$UV_CONC\" UV_RPS_MAX=$UV_RPS_MAX UV_UF_REF=$UV_UF_REF UV_NACIONAL=$UV_NACIONAL"; } > "$UV_OUT/machine.txt"
