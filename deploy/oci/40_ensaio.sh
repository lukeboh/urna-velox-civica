#!/usr/bin/env bash
# 40 — Ensaio dos dois nós na OCI, sem tocar no TSE: o 2º turno de 2024 inteiro (51 municípios, ~97 mil BUs) servido pelo
# uv-simulador no nó A, com limite de 100 req/s POR IP DE ORIGEM; os dois nós federados coletam dele, sincronizam pela
# rede privada da VCN e cada um confere com os oficiais no fim. Usa pastas e unidades próprias (/srv/urna-velox/ensaio,
# uv-ensaio-*): não mexe na configuração de produção. Uso: ./40_ensaio.sh [--manter]  (--manter: não limpa no fim)
#
# Como o nó B chega ao simulador (que só escuta em 127.0.0.1 do A): uma ponte socat no B (127.0.0.1:18080) → rede
# privada → socat no A (IP privado:FED_PORTA+1) → 127.0.0.1:18080. Para o simulador, o B chega de 127.0.0.1 e o A sai de
# 127.0.0.2 (--ip-local): IPs de origem distintos, cada um com o seu limite, e o eco de IP de cada um confere.
source "$(dirname "$0")/lib.sh"
MANTER=0; [ "${1:-}" = "--manter" ] && MANTER=1
[ "$FED_REDE" = privado ] || erro "o ensaio usa a rede privada da VCN (FED_REDE=privado)"
ACEL="${ENSAIO_ACEL:-10}"; PSIM=18080; PPONTE=$((FED_PORTA + 1))
E=/srv/urna-velox/ensaio; D="$UVREAL/2024-2t"
APRIV=$(no_var A IP_PRIVADO); BPRIV=$(no_var B IP_PRIVADO)

for N in A B; do
  em "$N" "systemctl is-active --quiet urna-velox" && erro "o serviço de produção está rodando no nó $N: pare com ./21_parar.sh antes do ensaio"
done

limpar() {
  msg "limpando (unidades do ensaio e regra temporária do firewall)"
  for N in A B; do em "$N" "sudo systemctl stop uv-ensaio-no uv-ensaio-sim uv-ensaio-ponte 2>/dev/null; sudo systemctl reset-failed 'uv-ensaio-*' 2>/dev/null; true"; done
  em A "sudo iptables -D INPUT -p tcp -s $BPRIV --dport $PPONTE -j ACCEPT 2>/dev/null; true"
}
[ "$MANTER" = 1 ] || trap limpar EXIT
limpar >/dev/null 2>&1 || true

msg "1. dados do 2º turno de 2024 para o nó A (~120 MB; só na primeira vez)"
for f in dados metricas_2t.csv oficial preparo/candidatos.json; do [ -e "$D/$f" ] || erro "falta $D/$f (UVREAL no nos.env)"; done
if ! em A "test -f $E/2024-2t/.completo"; then
  TAR_A="/dev/shm/uv-ensaio-2024-2t-a-$$.tar.gz"
  tar -C "$D" -czf "$TAR_A" dados metricas_2t.csv oficial preparo/candidatos.json
  para A "$TAR_A" /tmp/uv-ensaio-2024-2t-a.tar.gz
  rm -f "$TAR_A"
  em A "sudo mkdir -p $E/2024-2t && sudo tar -C $E/2024-2t -xzf /tmp/uv-ensaio-2024-2t-a.tar.gz && sudo rm -f /tmp/uv-ensaio-2024-2t-a.tar.gz && sudo touch $E/2024-2t/.completo && sudo chown -R urnavelox:urnavelox $E"
fi
TAR_B="/dev/shm/uv-ensaio-2024-2t-b-$$.tar.gz"
tar -C "$D" -czf "$TAR_B" preparo/candidatos.json
para B "$TAR_B" /tmp/uv-ensaio-2024-2t-b.tar.gz
rm -f "$TAR_B"
em B "sudo mkdir -p $E/2024-2t && sudo tar -C $E/2024-2t -xzf /tmp/uv-ensaio-2024-2t-b.tar.gz && sudo rm -f /tmp/uv-ensaio-2024-2t-b.tar.gz && sudo chown -R urnavelox:urnavelox $E"

msg "2. federacao.json do ensaio (A sai por 127.0.0.2; B chega pela ponte como 127.0.0.1)"
cat > "$AQUI/gerado/federacao-ensaio.json" <<EOF
{ "versao": 1, "teto_coleta_rps": 95,
  "nos": [ { "id": "$(no_var A ID)", "orgao": "Ensaio A", "ip_saida": "127.0.0.2", "endereco": "$APRIV:$FED_PORTA" },
           { "id": "$(no_var B ID)", "orgao": "Ensaio B", "ip_saida": "127.0.0.1", "endereco": "$BPRIV:$FED_PORTA" } ] }
EOF
for N in A B; do
  para "$N" "$AQUI/gerado/federacao-ensaio.json" /tmp/federacao-ensaio.json
  em "$N" "sudo rm -rf $E/no && sudo install -d -o urnavelox -g urnavelox $E/no $E/sim && sudo install -o urnavelox -m 0644 /tmp/federacao-ensaio.json $E/federacao.json"
done

msg "3. ponte e simulador (a $ACEL×, limite de 100 req/s por IP de origem)"
em A "sudo iptables -I INPUT 1 -p tcp -s $BPRIV --dport $PPONTE -j ACCEPT"
em A "sudo systemd-run --quiet --unit uv-ensaio-ponte socat TCP-LISTEN:$PPONTE,bind=$APRIV,fork,reuseaddr TCP:127.0.0.1:$PSIM"
em B "sudo systemd-run --quiet --unit uv-ensaio-ponte socat TCP-LISTEN:$PSIM,bind=127.0.0.1,fork,reuseaddr TCP:$APRIV:$PPONTE"
em A "sudo systemd-run --quiet --unit uv-ensaio-sim --uid urnavelox -p WorkingDirectory=$E/sim /opt/urna-velox/bin/uv-simulador \
  --dados $E/2024-2t/dados --cs $E/2024-2t/dados/config --metricas $E/2024-2t/metricas_2t.csv --oficial $E/2024-2t/oficial \
  --pleito 453 --ciclo ele2024 --acelerar $ACEL --limite-ip-rps 100 --porta $PSIM --log $E/sim/simulador.log"
for _ in $(seq 1 120); do em A "curl -s -o /dev/null http://127.0.0.1:$PSIM/eco-ip" && break; sleep 1; done
em B "curl -s --max-time 5 http://127.0.0.1:$PSIM/eco-ip; echo ' ← IP com que o B chega ao simulador'"

msg "4. os dois nós federados, juntos"
for N in A B; do
  IPL=""; [ "$N" = A ] && IPL="--ip-local 127.0.0.2"
  em "$N" "sudo systemd-run --quiet --unit uv-ensaio-no --uid urnavelox -p WorkingDirectory=$E/no -p LimitNOFILE=65536 \
    /opt/urna-velox/bin/uv-servico --pleito 453 --ciclo ele2024 --candidatos $E/2024-2t/preparo/candidatos.json \
    --dados $E/no/dados --saida $E/no/saida --base http://127.0.0.1:$PSIM --aceleracao-local $ACEL --rps 95 \
    --federacao $E/federacao.json --no $(no_var $N ID) --armazem $E/no/federacao --federacao-escuta 0.0.0.0:$FED_PORTA $IPL" &
done
wait

msg "5. acompanhando (a cada 60 s; o 2º turno a ${ACEL}× leva ~$((200 / ACEL + 3)) min)"
T0=$(date +%s)
while :; do
  sleep 60
  ATIVOS=0; for N in A B; do em "$N" "systemctl is-active --quiet uv-ensaio-no" && ATIVOS=$((ATIVOS + 1)); done
  "$AQUI/30_verificar.sh" "$E/no/saida" 2>/dev/null | grep -E '^(== nó|estado|rede|federação)' || true
  [ "$ATIVOS" = 0 ] && break
  [ $(( $(date +%s) - T0 )) -gt 10800 ] && { aviso "3 h sem terminar: parando"; break; }
done

msg "6. resultado"
OK=1
for N in A B; do
  echo "-- nó $N"
  em "$N" "python3 - $E/no/saida <<'PY'
import json, sys, os
s = sys.argv[1]
c = json.load(open(os.path.join(s, 'conferencia.json'))); h = json.load(open(os.path.join(s, 'saude.json')))
print(f\"conferência: {c['identicos']}/{c['arquivos']} idênticos · situações divergentes {c['situacoes']['divergentes']} · revogações {c['matematicamente']['revogacoes']}\")
print(f\"BUs aplicados {h['secoes']['aplicadas']} · duplicados {h['motor']['duplicados']} · aux {h['rede']['aux']} · HTTP 429 {h['rede']['http429']}\")
f = h['federacao']; print(f\"federação: publicados {f['publicados']} · recebidos {f['recebidos']} · lotes copiados {f['sincronia']['lotes_copiados']}\")
sys.exit(0 if c['identicos'] == c['arquivos'] and h['motor']['duplicados'] == 0 and h['rede']['http429'] == 0 else 1)
PY" || OK=0
done
echo "-- simulador (por IP de origem: requisições, máximo num segundo, 429)"
em A "python3 -c \"import json;s=json.load(open('$E/sim/simulador_resumo.json'));[print(' ',k,v) for k,v in s['por_ip'].items()]\""
[ "$OK" = 1 ] && msg "ENSAIO OK: os dois nós com o resultado idêntico ao oficial, sem 429" || aviso "ENSAIO COM PROBLEMA: veja $E/no/saida (servico.log, alertas.csv) nos nós"
