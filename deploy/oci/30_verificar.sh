#!/usr/bin/env bash
# 30 — Estado dos dois nós: serviço, versão, saúde (seções, rede, 429), federação (quem participa, IP medido, cota,
# sincronização), alcance entre os pares, disco e últimos alertas. Uso: ./30_verificar.sh [saida]
# (saida = pasta do serviço no servidor; padrão /srv/urna-velox/saida; o ensaio usa /srv/urna-velox/ensaio/no/saida)
source "$(dirname "$0")/lib.sh"
SAIDA="${1:-/srv/urna-velox/saida}"
for N in A B; do
  O=$(outro_de "$N")
  msg "nó $N — $(no_var $N ID) — $(host_de $N)"
  em "$N" "bash -s" <<EOF
echo "serviço: \$(systemctl is-active urna-velox) · versão: \$(cat /opt/urna-velox/VERSION 2>/dev/null) · disco livre: \$(df -h /srv | awk 'NR==2{print \$4}')"
echo "linha de comando: \$(ps -o args= -C uv-servico | grep -oE -- '--(intervalo-cs|intervalo-cs-ocioso|rps|tse-intervalo) [^ ]+' | tr '\n' ' ' || true)"
S=$SAIDA/saude.json
if [ -f \$S ]; then
python3 - \$S <<'PY'
import json, sys
s = json.load(open(sys.argv[1])); f = s.get("federacao", {}); r = s["rede"]; sc = s["secoes"]; z = f.get("sincronia", {})
print(f"estado {s['estado']} · relógio do site {s['relogio_site']} · aplicadas {sc['aplicadas']}/{sc['catalogo']} · pendentes {sc['pendentes']} · fila {s['fila']}")
print(f"rede: {r['req_s_atual']} req/s agora (teto {r['teto_req_s']}) · aux {r['aux']} · BUs baixados {r['bu_baixados']} · HTTP 429: {r['http429']} · pausa {r['pausa_restante_s']} s")
if f:
    print(f"federação: participa={f['participa']} · IP medido {f['ip_saida_medido']} · cota {f['rps_cota']} req/s · ativos {[a['no'] for a in f['ativos']]} · fora {[x['no']+':'+x['motivo'] for x in f['fora']]}")
    print(f"  publicados {f['publicados']} · recebidos {f['recebidos']} · já aplicados {f['ja_aplicados']} · delegadas {f['delegadas_agora']} · assumidas {f['assumidas']} · rejeitados {f['rejeitados']}")
    print(f"  sincronia: lotes copiados {z.get('lotes_copiados')} · por repasse {z.get('por_repasse')} · servidas {z.get('servidas')} (304: {z.get('servidas_304')}) · erros {z.get('buscas_erro')}")
al = s.get("alertas", {})
if al: print("alertas:", al)
PY
else echo "sem \$S (o serviço ainda não rodou)"; fi
echo "alcance do par $(no_var $O ID) em $(endereco_de $O): \$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 http://$(endereco_de $O)/fed/v1/nos/$(no_var $O ID).json)"
if [ -f $SAIDA/atualizador.json ]; then
  python3 -c "import json,sys; a=json.load(open(sys.argv[1])); print(f\"atualizador: status={a.get('status')} ({a.get('motivo')}) · série={a.get('serie')} · desejado={a.get('estado_desejado')} · em={a.get('atualizado_em')}\")" $SAIDA/atualizador.json
fi
echo "controle manual: \$(cat /etc/urna-velox/controle-manual 2>/dev/null || echo 'não (o atualizador segue o manifesto)')"
tail -n 3 $SAIDA/alertas.csv 2>/dev/null | sed 's/^/alerta: /' || true
EOF
done
