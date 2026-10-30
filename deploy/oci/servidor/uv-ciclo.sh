#!/usr/bin/env bash
# uv-ciclo — ciclo diário de simulações com dados reais (docs/operacao/CICLO_DIARIO.md).
#
# Roda nos dois nós da OCI em rotação contínua e circular:
#   452 (1ºT 2024) -> 453 (2ºT 2024) -> 3220 (1ºT 2026) -> 3221 (2ºT 2026) -> 452...
# Após cada totalização concluída (apuracao.encerrada == true), o sistema mantém o resultado
# em exibição e inicia a próxima eleição pontualmente na próxima virada: a meia hora redonda (17h00, 17h30...; desde a
# 1.0.35, antes a hora fechada; PASSO_MIN no ciclo.env).
# Para cada eleição (452, 453, 3220 e, desde 26/10/2026, 3221), os modos alternam entre rápido (500×) e cadência (1×).
# O replay repete a noite real como foi, inclusive as pausas do TSE (no 3221, 2 h sem BU de 17:09 a 19:08).
# O modo "espera" (vitrine sem coleta, com $CAND_3221) continua aceito, mas não entra mais na rotação.
#
#   uv-ciclo.sh avancar [pleito] [modo] # avança para o próximo da rotação (chamado pelo vigia na virada da meia hora)
#   uv-ciclo.sh iniciar <pleito> [rapido|cadencia|aovivo] # iniciação manual (aovivo: pleito em curso, contra o TSE)
#   uv-ciclo.sh acelerar               # (nó A) termina a noite simulada sem intervalo
#   uv-ciclo.sh parar                  # para tudo e arquiva (sem religar a produção)
#   uv-ciclo.sh estado                 # unidades, rotação, execução atual e histórico
#
# Multipleito (1.1, desde a 1.0.35): os pleitos da rotação e os dados de cada um (ciclo, nome, data, corte, duração da
# cadência) vêm do pleitos.json e do acervo.json de cada acervo (uv_pleitos.py, ao lado deste script); todo pleito
# encerrado com o acervo pronto (/srv/urna-velox/acervo/<pleito>/.completo, montado pelo uv-pleitos.sh) entra.
set -uo pipefail
export TZ=America/Sao_Paulo
source /etc/urna-velox/ciclo.env      # PAPEL (A|B), NO_ID, A_PRIV, B_PRIV, FED_PORTA, BUCKET, NS, INICIO, FIM, CAND_3221
C=/srv/urna-velox/ciclo; AC=/srv/urna-velox/acervo; BIN=/opt/urna-velox/bin
PSIM=18080; PPONTE=$((FED_PORTA + 1)); ACEL_RAPIDO=500
mkdir -p $C/historico
chown -R urnavelox:urnavelox $C 2>/dev/null || true
log() { echo "$(date -Iseconds) [$NO_ID] $*" | tee -a $C/ciclo.log >&2; }

# ciclo, nome, data, corte e tempo máximo em cadência a 1× antes de acelerar no nó A (uv_pleitos.py: os valores de
# antes para 452, 453, 3220 e 3221 — 3h45, 2h, 3h45 e 4h15 —; para os outros, o acervo.json do montar_acervo.sh)
declare -A CICLO NOME DATA CORTE TEMPO_CADENCIA_S
info_pleito() {
  local k v
  [ -n "${CICLO[$1]:-}" ] && return 0
  while IFS='=' read -r k v; do
    case "$k" in
      ciclo) CICLO[$1]=$v ;; nome) NOME[$1]=$v ;; data_br) DATA[$1]=$v ;; corte) CORTE[$1]=$v ;; tempo_cadencia_s) TEMPO_CADENCIA_S[$1]=$v ;;
    esac
  done < <(python3 "$BIN/uv_pleitos.py" info "$1" 2>/dev/null)
  [ -n "${CICLO[$1]:-}" ]
}

proximo_pleito() {
  python3 "$BIN/uv_pleitos.py" seguinte "${1:-}" 2>/dev/null || echo 452
}

primeiro_pleito() {
  local p; p=$(python3 "$BIN/uv_pleitos.py" replays 2>/dev/null | head -1); echo "${p:-452}"
}

modo_do_pleito() {
  local p="${1:-}"
  for rf in "$C/rotacao.json" "$C/atual/rotacao.json"; do
    if [ -f "$rf" ]; then
      local m
      m=$(python3 -c "import json; r=json.load(open('$rf')); print(r.get('historico_modos',{}).get('$p',''))" 2>/dev/null || true)
      if [ "$m" = rapido ]; then echo cadencia; return; fi
      if [ "$m" = cadencia ]; then echo rapido; return; fi
    fi
  done
  local ult
  ult=$(ls -1td "$C/historico"/*-"$p"-* 2>/dev/null | head -1 || true)
  if [[ "$ult" =~ rapido ]]; then
    echo cadencia
  elif [[ "$ult" =~ cadencia ]]; then
    echo rapido
  else
    echo rapido
  fi
}

# depois de uma partida que não pegou (aovivo sem candidatos, sem configuração): o vigia volta, para tentar de novo
reerguer_vigia() {
  systemd-run --quiet --unit uv-ciclo-vigia -p Restart=on-failure -p StandardError=append:$C/atual/vigia.err \
    python3 $BIN/uv-ciclo-vigia.py $C $NO_ID 10 2>/dev/null || true
}

parar_unidades() {
  systemctl stop uv-ciclo-tel uv-ciclo-vigia uv-ciclo-no uv-ciclo-sim uv-ciclo-acel.timer uv-ciclo-acel 2>/dev/null
  systemctl stop urna-velox 2>/dev/null
  systemctl reset-failed 'uv-ciclo-*' 2>/dev/null
  true
}

arquivar() {
  [ -f $C/atual/id ] || { rm -rf $C/atual; return 0; }
  local ID; ID=$(cat $C/atual/id); local H=$C/historico/$ID S=$C/atual/saida
  mkdir -p $H
  for f in saude.json conferencia.json conferencia.md eventos.csv alertas.csv servico.log; do [ -f $S/$f ] && cp $S/$f $H/; done
  [ -f $S/secoes.log ] && gzip -c $S/secoes.log > $H/secoes.log.gz
  for f in id params telemetria.jsonl servico.out servico.err reinicios_intencionais sim/simulador_resumo.json sim/simulador.err; do [ -f $C/atual/$f ] && cp $C/atual/$f $H/; done
  python3 $BIN/uv-ciclo-resumo.py $C/atual $H > $H/RESUMO.json 2> $H/resumo.err || log "resumo de $ID falhou (ver resumo.err)"
  gzip -f $H/telemetria.jsonl 2>/dev/null
  if [ -n "${BUCKET:-}" ]; then
    tar -C $C/historico -czf /tmp/uv-ciclo-$ID-$NO_ID.tar.gz $ID && \
      $BIN/uv-bucket put "$BUCKET" "ciclo/$ID/$NO_ID.tar.gz" /tmp/uv-ciclo-$ID-$NO_ID.tar.gz "$NS" >/dev/null 2>&1 \
      && log "evidências de $ID no bucket (ciclo/$ID/$NO_ID.tar.gz)" || log "envio de $ID ao bucket falhou (fica em $H)"
    rm -f /tmp/uv-ciclo-$ID-$NO_ID.tar.gz
  fi
  log "arquivado $ID em $H: $(python3 -c "import json,sys;r=json.load(open(sys.argv[1]));print(r.get('veredito'),'·',r.get('resumo',''))" $H/RESUMO.json 2>/dev/null)"
  # o regenerável (cópias dos BUs, lotes da federação, publicação) não fica para a eleição seguinte
  rm -rf $C/atual
}

pontes() { # ponte do B até o simulador do A pela rede privada (como no 40_ensaio.sh)
  if [ "$PAPEL" = A ]; then
    iptables -C INPUT -p tcp -s "$B_PRIV" --dport $PPONTE -j ACCEPT 2>/dev/null || iptables -I INPUT 1 -p tcp -s "$B_PRIV" --dport $PPONTE -j ACCEPT
    systemctl is-active --quiet uv-ciclo-ponte || systemd-run --quiet --unit uv-ciclo-ponte -p Restart=always \
      socat TCP-LISTEN:$PPONTE,bind=$A_PRIV,fork,reuseaddr TCP:127.0.0.1:$PSIM
  else
    systemctl is-active --quiet uv-ciclo-ponte || systemd-run --quiet --unit uv-ciclo-ponte -p Restart=always \
      socat TCP-LISTEN:$PSIM,bind=127.0.0.1,fork,reuseaddr TCP:$A_PRIV:$PPONTE
  fi
}

federacao_json() {
  cat > /etc/urna-velox/federacao-ciclo.json <<EOF
{ "versao": 1, "teto_coleta_rps": 5000,
  "nos": [ { "id": "no-a", "orgao": "Simulação A", "ip_saida": "127.0.0.2", "endereco": "$A_PRIV:$FED_PORTA", "teto_rps": 5000,
             "acordo": "simulação local com o uv-simulador (nenhuma requisição ao TSE; regra de 17/10/2026)" },
           { "id": "no-b", "orgao": "Simulação B", "ip_saida": "127.0.0.1", "endereco": "$B_PRIV:$FED_PORTA", "teto_rps": 5000,
             "acordo": "simulação local com o uv-simulador (nenhuma requisição ao TSE; regra de 17/10/2026)" } ] }
EOF
}

iniciar() {
  local P="$1" MODO="${2:-rapido}" T0; T0=$(date +%s)
  info_pleito "$P" || { log "pleito desconhecido: $P (fora do pleitos.json e sem acervo.json)"; exit 2; }
  printf 'uv-ciclo %s %s\n' "$P" "$(date -Iseconds)" > /etc/urna-velox/controle-manual   # o uv-atualizador não mexe
  parar_unidades
  arquivar
  # CICLO_CARIMBO vem do vigia (hora do disparo) ou do 63_ciclo.sh (minuto do comando): o mesmo nos dois nós, para
  # A e B gravarem a execução com o mesmo ID (e as evidências caírem no mesmo ciclo/<ID>/ do bucket)
  local CARIMBO="${CICLO_CARIMBO:-$(date +%Y%m%d-%H%M)}"
  [[ "$CARIMBO" =~ ^[0-9]{8}-[0-9]{4}$ ]] || CARIMBO=$(date +%Y%m%d-%H%M)
  local ID; ID="$CARIMBO-$P-$MODO"
  [ -e "$C/historico/$ID" ] && ID="$ID-$(date +%S)"   # carimbo repetido (dois comandos no mesmo minuto)
  mkdir -p $C/atual/{dados,saida,federacao,sim}
  echo "$ID" > $C/atual/id
  # os dois nós começam juntos: espera o minuto seguinte ao disparo (os dois arquivam no primeiro minuto)
  local ESPERA=$(( 60 - ($(date +%s) - T0) )); [ "$ESPERA" -gt 0 ] && [ -z "${SEM_ESPERA:-}" ] && sleep $ESPERA
  chown -R urnavelox:urnavelox $C 2>/dev/null || true
  local N=$C/atual ROT ACEL RPS
  # multipleito (1.1): a página mostra todos os pleitos (GET /api/v1/pleitos), do pleitos.json do uv-pleitos.sh
  local PLT=(); [ -s /srv/urna-velox/pleitos/pleitos.json ] && $BIN/uv-servico --help 2>/dev/null | grep -q -- '--pleitos' \
    && PLT=(--pleitos /srv/urna-velox/pleitos/pleitos.json)
  # multipleito (1.1): as séries de consulta dos encerrados (/consulta/*), geradas pelo uv-pleitos.sh
  [ -d /srv/urna-velox/consulta ] && $BIN/uv-servico --help 2>/dev/null | grep -q -- '--consulta' && PLT+=(--consulta /srv/urna-velox/consulta)
  if [ "$MODO" = espera ]; then
    # vitrine (até 25/10/2026, a espera do 2º turno): só os candidatos já carregados, sem coleta; comparação com o TSE
    # só da tela aberta (uma busca a cada 30–60 s por tópico, regra de 16/10/2026). Fora da rotação desde 26/10/2026.
    [ "$P" = 3221 ] || { log "modo espera só existe para o 3221"; exit 2; }
    printf 'pleito=3221\nmodo=espera\ninicio=%s\n' "$(date -Iseconds)" > $N/params
    systemd-run --quiet --unit uv-ciclo-no --uid urnavelox -p WorkingDirectory=$N -p LimitNOFILE=65536 -p Restart=on-failure -p RestartSec=10 \
      -p StandardOutput=append:$N/servico.out -p StandardError=append:$N/servico.err \
      $BIN/uv-servico --pleito 3221 --ciclo ele2026 --candidatos "$CAND_3221" --dados $N/dados --saida $N/saida \
      --sem-coleta --continuar --http 0.0.0.0:8080 --aceitar-incompleto "${PLT[@]}"
  elif [ "$MODO" = aovivo ]; then
    # multipleito (1.1): pleito em curso no ele-c.json, totalizado ao vivo contra o site do TSE (autorizado pelo Luciano
    # em 29/10/2026, CLAUDE.md). A configuração do nó é a da produção (/etc/urna-velox/no.env e federacao.json: servidor,
    # ≤ 95 req/s, IP de saída, o B só recebendo quando os dois são do mesmo órgão), pelo uv-iniciar.sh; daqui vêm só o
    # pleito, o ciclo, as UFs do pleito e as pastas desta execução.
    [ -s /etc/urna-velox/no.env ] && [ -s /etc/urna-velox/federacao.json ] \
      || { log "aovivo: falta a configuração de produção do nó (/etc/urna-velox/no.env e federacao.json)"; reerguer_vigia; exit 4; }
    local UFSP BASE_TSE AMB_TSE; UFSP=$(python3 "$BIN/uv_pleitos.py" ufs "$P" 2>/dev/null || true)
    BASE_TSE=$(. /etc/urna-velox/no.env; echo "${BASE:-https://resultados.tse.jus.br}")
    AMB_TSE=$(. /etc/urna-velox/no.env; echo "${AMBIENTE:-oficial}")
    printf 'pleito=%s\nmodo=aovivo\nufs=%s\ninicio=%s\n' "$P" "${UFSP:-todas}" "$(date -Iseconds)" > $N/params
    mkdir -p $N/preparo $AC/$P/fontes-aovivo; chown -R urnavelox:urnavelox $N $AC/$P 2>/dev/null || true
    # candidatos.json do pleito (≤ 5 req/s, para no 1º HTTP 429; código 2 = fontes ainda incompletas: o serviço aceita)
    sudo -u urnavelox $BIN/uv-coletor preparar --base "$BASE_TSE" --ambiente "$AMB_TSE" --ciclo "${CICLO[$P]}" --pleito "$P" \
      --saida $N/preparo --fontes $AC/$P/fontes-aovivo > $N/preparo/preparar.log 2>&1; local RCP=$?
    chown -R urnavelox:urnavelox $N $AC/$P 2>/dev/null || true
    # reserva: o catálogo pré-eleição montado pelo uv-pleitos.sh antes da noite (§3.6)
    if [ ! -s $N/preparo/candidatos.json ] && [ -s $AC/$P/preparo/candidatos.json ]; then
      cp $AC/$P/preparo/candidatos.json $N/preparo/candidatos.json; chown urnavelox:urnavelox $N/preparo/candidatos.json 2>/dev/null || true
      log "aovivo: preparo da noite falhou (código $RCP); usando o catálogo pré-eleição de $AC/$P/preparo"; RCP=2
    fi
    if [ ! -s $N/preparo/candidatos.json ]; then
      log "aovivo: candidatos.json do $P não saiu (código $RCP; $N/preparo/preparar.log); o vigia tenta de novo em 10 min"
      reerguer_vigia; exit 5
    fi
    [ "$RCP" = 0 ] || log "aovivo: candidatos.json do $P com fontes incompletas (código $RCP): o serviço aceita e segue"
    systemd-run --quiet --unit uv-ciclo-no --uid urnavelox -p WorkingDirectory=$N -p LimitNOFILE=65536 -p Restart=on-failure -p RestartSec=10 \
      -p StandardOutput=append:$N/servico.out -p StandardError=append:$N/servico.err \
      --setenv=UV_PLEITO="$P" --setenv=UV_CICLO="${CICLO[$P]}" --setenv=UV_CANDIDATOS=$N/preparo/candidatos.json \
      --setenv=UV_DADOS=$N/dados --setenv=UV_SAIDA=$N/saida --setenv=UV_ARMAZEM=$N/federacao --setenv=UV_UFS="$UFSP" \
      --setenv=UV_EXTRA="--continuar --aceitar-incompleto" \
      $BIN/uv-iniciar.sh
    log "AO VIVO: $P (${NOME[$P]}, eleição de ${DATA[$P]}) contra $BASE_TSE, UFs ${UFSP:-todas}"
  else
    local D=$AC/$P
    [ -f $D/.completo ] || { log "acervo $D incompleto"; exit 3; }
    # rótulo do selo da página: a eleição (com o dia em que os BUs foram de fato coletados) e o modo, em palavras de
    # quem chega sem contexto (playlist de simulados: "tempo real" / "mesma ordem, acelerado")
    if [ "$MODO" = rapido ]; then ACEL=$ACEL_RAPIDO; ROT="${NOME[$P]} (eleição de ${DATA[$P]}) · mesma ordem de chegada dos BUs, acelerado"
    else ACEL=1; ROT="${NOME[$P]} (eleição de ${DATA[$P]}) · em tempo real, no ritmo em que os BUs chegaram ao TSE"; fi
    RPS=2000   # sem o teto do TSE: o servidor é o simulador local (regra de 17/10/2026)
    printf 'pleito=%s\nmodo=%s\naceleracao=%s\nrps_por_no=%s\nrotulo=%s\ninicio=%s\n' "$P" "$MODO" "$ACEL" "$RPS" "$ROT" "$(date -Iseconds)" > $N/params
    federacao_json
    pontes
    if [ "$PAPEL" = A ]; then
      local TSE=(); [ "$MODO" = cadencia ] && TSE=(--tse-simulado $D/candidatos.json)
      # rápido: o relógio do simulador acompanha os nós (no máximo 1 % das seções disponibilizadas à espera), para a
      # curva, a previsão e a ordem seguirem a noite em vez de o relógio chegar ao fim antes dos BUs (B-011)
      [ "$MODO" = rapido ] && TSE+=(--acompanhar-servico 1)
      systemd-run --quiet --unit uv-ciclo-sim --uid urnavelox -p WorkingDirectory=$N/sim -p LimitNOFILE=65536 -p Restart=on-failure -p RestartSec=5 \
        -p StandardOutput=append:$N/sim/simulador.out -p StandardError=append:$N/sim/simulador.err \
        $BIN/uv-simulador --dados $D/dados --cs $D/cs --metricas $D/linha-do-tempo.csv --oficial $D/oficial \
        --pleito $P --ciclo ${CICLO[$P]} --acelerar $ACEL --horario recebido --corte "${CORTE[$P]}" --porta $PSIM \
        --log $N/sim/simulador.log "${TSE[@]}"
      for _ in $(seq 1 180); do curl -s -o /dev/null --max-time 2 http://127.0.0.1:$PSIM/eco-ip && break; sleep 1; done
      log "simulador do $P no ar ($MODO, ${ACEL}×): $(curl -s --max-time 5 http://127.0.0.1:$PSIM/controle)"
    else
      for _ in $(seq 1 240); do curl -s -o /dev/null --max-time 2 http://127.0.0.1:$PSIM/eco-ip && break; sleep 1; done
    fi
    local IPL=(); [ "$PAPEL" = A ] && IPL=(--ip-local 127.0.0.2)
    systemd-run --quiet --unit uv-ciclo-no --uid urnavelox -p WorkingDirectory=$N -p LimitNOFILE=65536 -p Restart=on-failure -p RestartSec=10 \
      -p StandardOutput=append:$N/servico.out -p StandardError=append:$N/servico.err \
      $BIN/uv-servico --pleito $P --ciclo ${CICLO[$P]} --candidatos $D/candidatos.json --dados $N/dados --saida $N/saida \
      --base http://127.0.0.1:$PSIM --aceleracao-local $ACEL --rps $RPS --ordem $D/linha-do-tempo.csv \
      --federacao /etc/urna-velox/federacao-ciclo.json --no $NO_ID --armazem $N/federacao --federacao-escuta 0.0.0.0:$FED_PORTA \
      --http 0.0.0.0:8080 --continuar --simulacao "$ROT" "${IPL[@]}" "${PLT[@]}"
    if [ "$PAPEL" = A ] && [ "$MODO" = cadencia ]; then
      local TC=${TEMPO_CADENCIA_S[$P]:-13500}
      systemd-run --quiet --unit uv-ciclo-acel --on-active="${TC}s" $BIN/uv-ciclo.sh acelerar \
        && log "aceleração do fim em cadência agendada para ${TC}s"
    fi
  fi
  systemd-run --quiet --unit uv-ciclo-tel --uid urnavelox -p Restart=on-failure \
    -p StandardError=append:$N/telemetria.err python3 $BIN/uv-telemetria.py $N $NO_ID 15
  systemd-run --quiet --unit uv-ciclo-vigia -p Restart=on-failure \
    -p StandardError=append:$N/vigia.err python3 $BIN/uv-ciclo-vigia.py $C $NO_ID 10
  log "iniciado $ID"
}

case "${1:-}" in
  avancar)
    # Chamado pelo uv-ciclo-vigia na virada (meia hora redonda) ou manualmente pelo operador
    P="${2:-}"; M="${3:-}"
    if [ -z "$P" ]; then
      for rf in "$C/rotacao.json" "$C/atual/rotacao.json"; do
        if [ -f "$rf" ]; then
          P=$(python3 -c "import json; r=json.load(open('$rf')); print(r.get('proximo',{}).get('pleito',''))" 2>/dev/null || true)
          M=$(python3 -c "import json; r=json.load(open('$rf')); print(r.get('proximo',{}).get('modo',''))" 2>/dev/null || true)
          [ -n "$P" ] && break
        fi
      done
    fi
    [ -n "$P" ] || P=$(primeiro_pleito)
    [ -n "$M" ] || M=$(modo_do_pleito "$P")
    log "avançando para próximo pleito da rotação: $P ($M)"
    iniciar "$P" "$M" ;;
  horario)
    DIA=$(date +%F)
    if [[ "$DIA" < "$INICIO" || "$DIA" > "$FIM" ]]; then log "fora do período do ciclo ($INICIO a $FIM): nada a fazer"; exit 0; fi
    if [ -f "$C/atual/params" ]; then
      P_ATUAL=$(grep '^pleito=' "$C/atual/params" | cut -d= -f2)
      P=$(proximo_pleito "$P_ATUAL")
    else
      P=$(primeiro_pleito)
    fi
    M=$(modo_do_pleito "$P")
    iniciar "$P" "$M" ;;
  iniciar) iniciar "$2" "${3:-rapido}" ;;
  acelerar)
    R=$(curl -s -X POST --max-time 10 "http://127.0.0.1:$PSIM/controle/velocidade?x=$ACEL_RAPIDO"); log "fim da noite sem intervalo (${ACEL_RAPIDO}×): $R" ;;
  parar) parar_unidades; arquivar; log "parado" ;;
  estado)
    systemctl --no-pager --plain list-units 'uv-ciclo*' | head -15
    [ -f $C/atual/id ] && echo "atual: $(cat $C/atual/id)"
    for rf in "$C/rotacao.json" "$C/atual/rotacao.json"; do
      if [ -f "$rf" ]; then
        python3 -c "import json; r=json.load(open('$rf')); pr=r.get('proximo',{}); at=r.get('atual',{}); print('rotação: atual', at.get('pleito'), '(', at.get('modo'), ') concluído=', at.get('concluido'), '| próximo', pr.get('pleito'), '(', pr.get('modo'), ') disparo=', pr.get('disparo'))" 2>/dev/null && break || true
      fi
    done
    tail -5 $C/ciclo.log 2>/dev/null || tail -5 $C/atual/vigia.log 2>/dev/null || true
    ls $C/historico | tail -8 ;;
  *) echo "uso: $0 avancar [pleito] [modo]|iniciar <pleito> [rapido|cadencia]|acelerar|parar|estado"; exit 2 ;;
esac
