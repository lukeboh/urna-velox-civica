#!/usr/bin/env bash
# uv-ciclo — ciclo diário de simulações com dados reais (docs/CICLO_DIARIO.md).
#
# Roda nos dois nós da OCI em rotação contínua e circular:
#   452 (1ºT 2024) -> 453 (2ºT 2024) -> 3220 (1ºT 2026) -> 3221 (2ºT 2026) -> 452...
# Após cada totalização concluída (apuracao.encerrada == true), o sistema mantém o resultado
# em exibição e inicia a próxima eleição pontualmente na próxima hora fechada (ex: 17h, 18h, 19h...).
# O pleito 3221 fica em exibição por 1 hora fechada (modo espera) e é substituído na seguinte.
# Para cada eleição simulada (452, 453, 3220), os modos alternam entre rápido (500×) e cadência (1×).
#
#   uv-ciclo.sh avancar [pleito] [modo] # avança para o próximo da rotação (chamado pelo vigia na hora fechada)
#   uv-ciclo.sh iniciar <pleito> [rapido|cadencia] # iniciação manual
#   uv-ciclo.sh acelerar               # (nó A) termina a noite simulada sem intervalo
#   uv-ciclo.sh parar                  # para tudo e arquiva (sem religar a produção)
#   uv-ciclo.sh estado                 # unidades, rotação, execução atual e histórico
set -uo pipefail
export TZ=America/Sao_Paulo
source /etc/urna-velox/ciclo.env      # PAPEL (A|B), NO_ID, A_PRIV, B_PRIV, FED_PORTA, BUCKET, NS, INICIO, FIM, CAND_3221
C=/srv/urna-velox/ciclo; AC=/srv/urna-velox/acervo; BIN=/opt/urna-velox/bin
PSIM=18080; PPONTE=$((FED_PORTA + 1)); ACEL_RAPIDO=500
mkdir -p $C/historico
chown -R urnavelox:urnavelox $C 2>/dev/null || true
log() { echo "$(date -Iseconds) [$NO_ID] $*" | tee -a $C/ciclo.log >&2; }

declare -A CICLO=([452]=ele2024 [453]=ele2024 [3220]=ele2026 [3221]=ele2026)
declare -A NOME=([452]="1º turno de 2024" [453]="2º turno de 2024" [3220]="1º turno de 2026" [3221]="2º turno de 2026")
declare -A DATA=([452]="06/10/2024" [453]="27/10/2024" [3220]="04/10/2026" [3221]="25/10/2026")
declare -A CORTE=([452]="07/10/2024 02:00:00" [453]="28/10/2024 02:00:00" [3220]="05/10/2026 02:00:00")
# tempo máximo em cadência a 1× antes de acelerar no Nó A (3h45 para 452 e 3220; 2h para 453)
declare -A TEMPO_CADENCIA_S=([452]=13500 [453]=7200 [3220]=13500)

proximo_pleito() {
  case "${1:-}" in
    452)  echo 453 ;;
    453)  echo 3220 ;;
    3220) echo 3221 ;;
    3221) echo 452 ;;
    *)    echo 452 ;;
  esac
}

modo_do_pleito() {
  local p="${1:-}"
  [ "$p" = 3221 ] && { echo "espera"; return; }
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
  [ -n "${CICLO[$P]:-}" ] || { log "pleito desconhecido: $P"; exit 2; }
  printf 'uv-ciclo %s %s\n' "$P" "$(date -Iseconds)" > /etc/urna-velox/controle-manual   # o uv-atualizador não mexe
  parar_unidades
  arquivar
  # CICLO_CARIMBO vem do vigia (hora do disparo) ou do 63_ciclo.sh (minuto do comando): o mesmo nos dois nós, para
  # A e B gravarem a execução com o mesmo ID (e as evidências caírem no mesmo ciclo/<ID>/ do bucket)
  local CARIMBO="${CICLO_CARIMBO:-$(date +%Y%m%d-%H%M)}"
  [[ "$CARIMBO" =~ ^[0-9]{8}-[0-9]{4}$ ]] || CARIMBO=$(date +%Y%m%d-%H%M)
  local ID; ID="$CARIMBO-$P-$MODO"; [ "$P" = 3221 ] && ID="$CARIMBO-3221-espera"
  [ -e "$C/historico/$ID" ] && ID="$ID-$(date +%S)"   # carimbo repetido (dois comandos no mesmo minuto)
  mkdir -p $C/atual/{dados,saida,federacao,sim}
  echo "$ID" > $C/atual/id
  # os dois nós começam juntos: espera o minuto seguinte ao disparo (os dois arquivam no primeiro minuto)
  local ESPERA=$(( 60 - ($(date +%s) - T0) )); [ "$ESPERA" -gt 0 ] && [ -z "${SEM_ESPERA:-}" ] && sleep $ESPERA
  chown -R urnavelox:urnavelox $C 2>/dev/null || true
  local N=$C/atual ROT ACEL RPS
  if [ "$P" = 3221 ]; then
    # 2º turno de 2026: só os candidatos já carregados (sem coleta) até a apuração real de 25/10; comparação com o TSE
    # só da tela aberta (uma busca a cada 30–60 s por tópico, regra de 16/10/2026)
    printf 'pleito=3221\nmodo=espera\ninicio=%s\n' "$(date -Iseconds)" > $N/params
    systemd-run --quiet --unit uv-ciclo-no --uid urnavelox -p WorkingDirectory=$N -p LimitNOFILE=65536 -p Restart=on-failure -p RestartSec=10 \
      -p StandardOutput=append:$N/servico.out -p StandardError=append:$N/servico.err \
      $BIN/uv-servico --pleito 3221 --ciclo ele2026 --candidatos "$CAND_3221" --dados $N/dados --saida $N/saida \
      --sem-coleta --continuar --http 0.0.0.0:8080 --aceitar-incompleto
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
      --http 0.0.0.0:8080 --continuar --simulacao "$ROT" "${IPL[@]}"
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
    # Chamado pelo uv-ciclo-vigia na hora fechada ou manualmente pelo operador
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
    [ -n "$P" ] || P=452
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
      P=452
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
