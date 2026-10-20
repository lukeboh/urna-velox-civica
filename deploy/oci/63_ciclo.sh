#!/usr/bin/env bash
# 63 — Ciclo diário de simulações com dados reais nos dois nós (docs/CICLO_DIARIO.md).
#
#   ./63_ciclo.sh --instalar            # binários novos, uv-ciclo, vigia, telemetria nos dois nós
#   ./63_ciclo.sh --scripts             # atualiza só os scripts (uv-ciclo.sh, uv-ciclo-vigia.py...) sem trocar binários
#   ./63_ciclo.sh --iniciar <pleito> [rapido|cadencia]   # inicia eleição agora, nos dois nós
#   ./63_ciclo.sh --avancar             # avança para o próximo da rotação nos dois nós
#   ./63_ciclo.sh --binarios            # só troca os executáveis nos dois nós, sem parar nada
#   ./63_ciclo.sh --reiniciar [A|B]     # reinicia o uv-servico do nó (padrão: os dois), retomando de onde parou
#   ./63_ciclo.sh --estado              # unidades, rotação, execução atual, últimas linhas do ciclo.log, histórico
#   ./63_ciclo.sh --telemetria [pasta]  # baixa do bucket as evidências (padrão: /mnt/d/dados/urna-velox/ciclo-diario)
#   ./63_ciclo.sh --remover             # para o ciclo e vigia (a produção do 3221 volta com 20_iniciar.sh)
#
# O acervo (/srv/urna-velox/acervo/<pleito>) é enviado uma vez, à parte (BUs já baixados do TSE: nada é baixado de
# novo). Nas simulações, nenhuma requisição ao TSE; o limite de 100 req/s é só para buscar arquivos no TSE.
source "$(dirname "$0")/lib.sh"
ACAO="${1:-}"; shift || true
INICIO_CICLO=2026-10-18; FIM_CICLO=2026-10-23
NS_BUCKET="${OCI_NAMESPACE:-}"

ns() {
  [ -n "$NS_BUCKET" ] && { echo "$NS_BUCKET"; return; }
  "${OCI_PY:-python3}" -c "import oci; c=oci.config.from_file(); print(oci.object_storage.ObjectStorageClient(c).get_namespace().data)"
}

case "$ACAO" in
--instalar)
  G="$AQUI/gerado"; S="$AQUI/servidor"
  [ -f "$G/uv-binarios-aarch64.tar.gz" ] || erro "falta $G/uv-binarios-aarch64.tar.gz (./compilar_local_arm64.sh)"
  NSV=$(ns)
  for N in A B; do
    msg "nó $N: arquivos do ciclo"
    T=$(mktemp -d)
    cat > "$T/ciclo.env" <<EOF
# gerado por 63_ciclo.sh em $(date -Iseconds)
PAPEL=$N
NO_ID=$(no_var $N ID)
A_PRIV=$(no_var A IP_PRIVADO)
B_PRIV=$(no_var B IP_PRIVADO)
FED_PORTA=$FED_PORTA
BUCKET=$OCI_BUCKET
NS=$NSV
INICIO=$INICIO_CICLO
FIM=$FIM_CICLO
CAND_3221=/srv/urna-velox/preparo/candidatos.json
PLAYLIST_URL=$([ "$N" = A ] && echo http://127.0.0.1:8090/api/v1/playlist || echo "https://${URL_DOMINIO:-urnavelox.com.br}/api/v1/playlist")
ELEICOES_OFICIAIS=${ELEICOES_OFICIAIS:-2026-10-25}
EOF
    tar -C "$T" -czf "$T/ciclo.tar.gz" ciclo.env
    tar -C "$S" -czf "$T/scripts.tar.gz" uv-ciclo.sh uv-telemetria.py uv-ciclo-resumo.py uv-ciclo-vigia.py uv-ciclo.service
    para "$N" "$T/ciclo.tar.gz" /tmp/uv-implantar/uv-ciclo-env.tar.gz
    para "$N" "$T/scripts.tar.gz" /tmp/uv-implantar/uv-ciclo-scripts.tar.gz
    para "$N" "$G/uv-binarios-aarch64.tar.gz" /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
    rm -rf "$T"
    msg "nó $N: instalando (o serviço de produção e o ciclo param durante a troca dos binários)"
    em "$N" "set -e; printf 'uv-ciclo instalação %s\n' \"\$(date -Iseconds)\" | sudo tee /etc/urna-velox/controle-manual >/dev/null
      sudo systemctl stop urna-velox uv-ciclo-no uv-ciclo-sim uv-ciclo-tel uv-ciclo-vigia 2>/dev/null || true
      rm -rf /tmp/uv-ext && mkdir -p /tmp/uv-ext/b /tmp/uv-ext/s /tmp/uv-ext/e
      tar -xzf /tmp/uv-implantar/uv-binarios-aarch64.tar.gz -C /tmp/uv-ext/b
      tar -xzf /tmp/uv-implantar/uv-ciclo-scripts.tar.gz -C /tmp/uv-ext/s
      tar -xzf /tmp/uv-implantar/uv-ciclo-env.tar.gz -C /tmp/uv-ext/e
      sudo install -o urnavelox -g urnavelox -m 0755 /tmp/uv-ext/b/bin/* /opt/urna-velox/bin/
      sudo install -m 0644 /tmp/uv-ext/b/VERSION /opt/urna-velox/VERSION
      sudo install -m 0755 /tmp/uv-ext/s/uv-ciclo.sh /tmp/uv-ext/s/uv-telemetria.py /tmp/uv-ext/s/uv-ciclo-resumo.py /tmp/uv-ext/s/uv-ciclo-vigia.py /opt/urna-velox/bin/
      sudo install -m 0644 /tmp/uv-ext/s/uv-ciclo.service /etc/systemd/system/
      sudo install -m 0644 /tmp/uv-ext/e/ciclo.env /etc/urna-velox/ciclo.env
      sudo rm -rf /tmp/uv-ext /tmp/uv-implantar/uv-ciclo-*.tar.gz /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
      sudo systemctl daemon-reload
      sudo systemctl disable --now uv-ciclo.timer 2>/dev/null || true
      echo \"versão: \$(/opt/urna-velox/bin/uv-servico --version)\"
      /opt/urna-velox/bin/uv-servico --help | grep -q -- '--simulacao' && echo 'uv-servico com --simulacao: ok'
      for c in socat python3 curl; do command -v \$c >/dev/null && echo \"\$c: ok\" || echo \"\$c: FALTA\"; done
      test -s /srv/urna-velox/preparo/candidatos.json && echo 'candidatos do 3221: ok' || echo 'candidatos do 3221: FALTA'
      ls /srv/urna-velox/acervo 2>/dev/null | tr '\n' ' '; echo"
  done ;;
--scripts)
  S="$AQUI/servidor"
  for N in A B; do
    msg "nó $N: enviando scripts do ciclo (sem tocar nos binários)"
    T=$(mktemp -d)
    tar -C "$S" -czf "$T/scripts.tar.gz" uv-ciclo.sh uv-telemetria.py uv-ciclo-resumo.py uv-ciclo-vigia.py uv-ciclo.service
    para "$N" "$T/scripts.tar.gz" /tmp/uv-implantar/uv-ciclo-scripts.tar.gz
    rm -rf "$T"
    msg "nó $N: instalando scripts"
    em "$N" "set -e
      rm -rf /tmp/uv-ext && mkdir -p /tmp/uv-ext/s
      tar -xzf /tmp/uv-implantar/uv-ciclo-scripts.tar.gz -C /tmp/uv-ext/s
      sudo install -m 0755 /tmp/uv-ext/s/uv-ciclo.sh /tmp/uv-ext/s/uv-telemetria.py /tmp/uv-ext/s/uv-ciclo-resumo.py /tmp/uv-ext/s/uv-ciclo-vigia.py /opt/urna-velox/bin/
      sudo install -m 0644 /tmp/uv-ext/s/uv-ciclo.service /etc/systemd/system/
      sudo rm -rf /tmp/uv-ext /tmp/uv-implantar/uv-ciclo-scripts.tar.gz
      sudo systemctl daemon-reload
      sudo systemctl disable --now uv-ciclo.timer 2>/dev/null || true
      sudo chown -R urnavelox:urnavelox /srv/urna-velox/ciclo 2>/dev/null || true
      sudo systemctl restart uv-ciclo-vigia 2>/dev/null || true
      echo 'scripts instalados com sucesso no nó $N'"
  done ;;
--playlist)
  # Playlist de simulados (docs/CICLO_DIARIO.md): uv-playlist no nó A, rota no Caddy dos dois nós, segredo comum aos
  # vigias e PLAYLIST_URL/ELEICOES_OFICIAIS no ciclo.env. Não para nada do ciclo: a apuração no ar segue; o vigia é
  # reiniciado (retoma a decisão salva) para passar a consultar a playlist na próxima virada.
  S="$AQUI/servidor"; G="$AQUI/gerado"; mkdir -p "$G"
  DOM="${1:-${URL_DOMINIO:-urnavelox.com.br}}"
  ELEI="${ELEICOES_OFICIAIS:-2026-10-25}"
  [ -s "$G/playlist.segredo" ] || { head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$G/playlist.segredo"; chmod 600 "$G/playlist.segredo"; }
  for N in A B; do
    if [ "$N" = A ]; then URL=http://127.0.0.1:8090/api/v1/playlist; else URL="https://$DOM/api/v1/playlist"; fi
    PAR=$(no_var "$(outro_de "$N")" IP_PRIVADO)
    msg "nó $N: playlist ($URL; eleições oficiais $ELEI)"
    T=$(mktemp -d)
    cp "$G/playlist.segredo" "$S/uv-playlist.py" "$S/uv-playlist.html" "$S/uv-playlist.service" "$S/Caddyfile.template" \
       "$S/uv-ciclo-vigia.py" "$S/uv-ciclo.sh" "$T/"
    # o instalador roda como root no nó (uv-operar.sh, como o 02_configurar_borda.sh); valores já substituídos aqui
    cat > "$T/instalar_playlist.sh" <<EOF
set -e
D=/tmp/uv-pl
install -o root -g urnavelox -m 0640 \$D/playlist.segredo /etc/urna-velox/playlist.segredo
install -m 0755 \$D/uv-playlist.py \$D/uv-ciclo-vigia.py \$D/uv-ciclo.sh /opt/urna-velox/bin/
install -m 0644 \$D/uv-playlist.html /opt/urna-velox/bin/
sed -i -e '/^PLAYLIST_URL=/d' -e '/^ELEICOES_OFICIAIS=/d' /etc/urna-velox/ciclo.env
printf 'PLAYLIST_URL=%s\nELEICOES_OFICIAIS=%s\n' '$URL' '$ELEI' >> /etc/urna-velox/ciclo.env
if [ "$N" = A ]; then
  install -m 0644 \$D/uv-playlist.service /etc/systemd/system/
  systemctl daemon-reload; systemctl enable uv-playlist >/dev/null 2>&1; systemctl restart uv-playlist
  sleep 2; curl -s --max-time 5 http://127.0.0.1:8090/api/v1/playlist/saude && echo ' (uv-playlist no ar)'
fi
if command -v caddy >/dev/null; then
  sed -e 's/{\\\$DOMINIO:localhost}/$DOM/g' -e 's/{\\\$PAR_IP:127.0.0.1}/$PAR/g' \$D/Caddyfile.template > /etc/caddy/Caddyfile.novo
  if caddy validate --config /etc/caddy/Caddyfile.novo --adapter caddyfile >/dev/null 2>&1; then
    cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.antes-playlist 2>/dev/null || true
    mv /etc/caddy/Caddyfile.novo /etc/caddy/Caddyfile
    # o "admin off" do Caddyfile desliga a API que o reload usa: o reload falha e o Caddy fica com a configuração
    # antiga (19/10/2026). Nesse caso, reinicia (~1 s fora do ar; a página reconecta sozinha)
    if systemctl reload caddy 2>/dev/null; then echo 'Caddy recarregado com a rota /playlist'
    else systemctl restart caddy && echo 'Caddy reiniciado com a rota /playlist (o reload foi recusado)'; fi
  else
    echo 'Caddyfile novo INVÁLIDO: o Caddy ficou como estava'; caddy validate --config /etc/caddy/Caddyfile.novo --adapter caddyfile 2>&1 | tail -3
  fi
fi
systemctl restart uv-ciclo-vigia 2>/dev/null || true
echo "nó $N: playlist instalada"
EOF
    tar -C "$T" -czf "$T/playlist.tar.gz" playlist.segredo uv-playlist.py uv-playlist.html uv-playlist.service \
        Caddyfile.template uv-ciclo-vigia.py uv-ciclo.sh instalar_playlist.sh
    para "$N" "$T/playlist.tar.gz" /tmp/uv-implantar/uv-playlist.tar.gz
    rm -rf "$T"
    em "$N" "rm -rf /tmp/uv-pl && mkdir -p /tmp/uv-pl && tar -xzf /tmp/uv-implantar/uv-playlist.tar.gz -C /tmp/uv-pl \
      && sudo /opt/urna-velox/bin/uv-operar.sh bash /tmp/uv-pl/instalar_playlist.sh; R=\$?
      sudo rm -rf /tmp/uv-pl /tmp/uv-implantar/uv-playlist.tar.gz; exit \$R"
  done
  msg "confira: https://$DOM/playlist" ;;
--binarios)
  G="$AQUI/gerado"; [ -f "$G/uv-binarios-aarch64.tar.gz" ] || erro "falta $G/uv-binarios-aarch64.tar.gz (./compilar_local_arm64.sh)"
  for N in A B; do
    para "$N" "$G/uv-binarios-aarch64.tar.gz" /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
    em "$N" "set -e; rm -rf /tmp/uv-ext && mkdir -p /tmp/uv-ext && tar -xzf /tmp/uv-implantar/uv-binarios-aarch64.tar.gz -C /tmp/uv-ext
      sudo install -o urnavelox -g urnavelox -m 0755 /tmp/uv-ext/bin/* /opt/urna-velox/bin/ && sudo install -m 0644 /tmp/uv-ext/VERSION /opt/urna-velox/VERSION
      sudo rm -rf /tmp/uv-ext /tmp/uv-implantar/uv-binarios-aarch64.tar.gz
      echo \"nó $N: \$(/opt/urna-velox/bin/uv-servico --version) instalado (vale no próximo reinício do serviço ou no próximo horário)\""
  done ;;
--reiniciar)
  ALVOS="${1:-A B}"; [ "$ALVOS" = ambos ] && ALVOS="A B"
  # anota a hora em atual/reinicios_intencionais: o RESUMO.json não conta este reinício como queda (B-003)
  for N in $ALVOS; do em "$N" "date -Iseconds | sudo tee -a /srv/urna-velox/ciclo/atual/reinicios_intencionais >/dev/null; sudo systemctl restart uv-ciclo-no uv-ciclo-tel uv-ciclo-vigia && sleep 8 && systemctl is-active uv-ciclo-no && tail -3 /srv/urna-velox/ciclo/atual/servico.err"; done ;;
--iniciar)
  P="${1:?pleito}"; M="${2:-rapido}"
  CARIMBO=$(TZ=America/Sao_Paulo date +%Y%m%d-%H%M)   # o mesmo ID da execução nos dois nós
  msg "trocando para o $P ($M) nos dois nós (execução $CARIMBO-$P)"
  for N in A B; do em "$N" "sudo systemd-run --quiet --collect --unit uv-ciclo-manual-\$(date +%s) --setenv=CICLO_CARIMBO=$CARIMBO /opt/urna-velox/bin/uv-ciclo.sh iniciar $P $M && echo 'nó $N: disparado'" & done
  wait ;;
--avancar)
  CARIMBO=$(TZ=America/Sao_Paulo date +%Y%m%d-%H%M)
  msg "avançando para a próxima eleição da rotação nos dois nós (carimbo $CARIMBO)"
  for N in A B; do em "$N" "sudo systemd-run --quiet --collect --unit uv-ciclo-avancar-\$(date +%s) --setenv=CICLO_CARIMBO=$CARIMBO /opt/urna-velox/bin/uv-ciclo.sh avancar && echo 'nó $N: comando avancar disparado'" & done
  wait ;;
--estado)
  for N in A B; do msg "nó $N"; em "$N" "/opt/urna-velox/bin/uv-ciclo.sh estado 2>&1; f=/srv/urna-velox/ciclo/atual/saida/saude.json; [ -f \$f ] && python3 -c \"import json;s=json.load(open('\$f'));print('estado',s['estado'],'·',s.get('relogio_site'),'· aplicadas',s['secoes']['aplicadas'],'/',s['secoes'].get('publicadas_cs'),'· fila',s.get('fila'),'· rss',s['recursos']['rss_mb'],'MB ·',(s.get('simulacao') or {}).get('rotulo'))\""; done
  # os dois nós devem estar na mesma execução (mesmo pleito, modo e carimbo); se não, a federação está dessincronizada
  IA=$(em A "cat /srv/urna-velox/ciclo/atual/id 2>/dev/null" 2>/dev/null | tail -1); IB=$(em B "cat /srv/urna-velox/ciclo/atual/id 2>/dev/null" 2>/dev/null | tail -1)
  if [ -n "$IA" ] && [ "$IA" = "$IB" ]; then msg "nós sincronizados: $IA"
  elif [ "${IA#*-*-}" = "${IB#*-*-}" ] && [ -n "$IA" ]; then aviso "mesmo pleito e modo, carimbos diferentes (A: $IA · B: $IB)"
  else aviso "nós DESSINCRONIZADOS (A: ${IA:-nada} · B: ${IB:-nada}): realinhe com ./63_ciclo.sh --iniciar <pleito> <modo>"; fi ;;
--telemetria)
  DEST="${1:-/mnt/d/dados/urna-velox/ciclo-diario}"; mkdir -p "$DEST"
  OCI_BUCKET="$OCI_BUCKET" NSV=$(ns) DEST="$DEST" "${OCI_PY:-python3}" - <<'PY'
import oci, os, tarfile
c = oci.config.from_file(); o = oci.object_storage.ObjectStorageClient(c)
ns, b, dest = os.environ['NSV'], os.environ['OCI_BUCKET'], os.environ['DEST']
inicio, n = None, 0
while True:
    r = o.list_objects(ns, b, prefix='ciclo/', start=inicio).data
    for ob in r.objects:
        alvo = os.path.join(dest, ob.name[len('ciclo/'):])
        if os.path.exists(alvo): continue
        os.makedirs(os.path.dirname(alvo), exist_ok=True)
        with open(alvo, 'wb') as f:
            for ch in o.get_object(ns, b, ob.name).data.raw.stream(1 << 20, decode_content=False): f.write(ch)
        with tarfile.open(alvo) as t: t.extractall(os.path.join(dest, '_extraido', os.path.basename(alvo)[:-7]))
        n += 1; print('baixado', ob.name)
    inicio = r.next_start_with
    if not inicio: break
print(n, 'novos em', dest)
PY
  ;;
--remover)
  for N in A B; do em "$N" "sudo systemctl disable --now uv-ciclo.timer 2>/dev/null || true; sudo systemd-run --quiet --unit uv-ciclo-parar-\$(date +%s) /opt/urna-velox/bin/uv-ciclo.sh parar; echo 'nó $N: ciclo desligado'"; done
  aviso "a trava manual continua: para voltar à produção do 3221, confira o no.env e use ./20_iniciar.sh" ;;
*) erro "uso: $0 --instalar | --scripts | --iniciar <pleito> [rapido|cadencia] | --avancar | --binarios | --reiniciar [A|B] | --estado | --telemetria [pasta] | --remover" ;;
esac
