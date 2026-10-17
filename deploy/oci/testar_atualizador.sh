#!/usr/bin/env bash
# testar_atualizador.sh — Testes automatizados locais do uv-atualizador (Critério da Parte B de HANDOFF_DEPLOY_SEM_SSH.md):
#   1. Manifesto válido assinado (Ed25519) + pacote íntegro -> atualiza a versão e grava status=ok;
#   2. Manifesto com assinatura errada (adulterado ou outra chave) -> recusado (assinatura_invalida);
#   3. Pacote com SHA-256 errado -> recusado (sha256_invalido), binário antigo preservado;
#   4. Manifesto mais antigo que o atual (série menor) -> recusado (serie_antiga, anti-rollback);
#   5. Manifesto com combinado_ate vencido contra o TSE -> para o serviço imediatamente e grava bloqueado (combinado_vencido).
#   6. Série já aplicada -> não reaplica o estado (um serviço ligado à mão continua ligado) (0.6.6);
#   7. Trava /etc/urna-velox/controle-manual -> o atualizador não mexe em nada; sem a trava, aplica a série nova (0.6.6);
#   8. Nó que não é o primeiro do federacao.json -> espera UV_ATRASO_PAR_S depois do emitido_em (0.6.6).
set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ATUALIZADOR="$AQUI/servidor/uv-atualizador"

TMP="$(mktemp -d /dev/shm/uv-teste-atualizador-XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/opt/bin" "$TMP/etc" "$TMP/saida" "$TMP/chaves" "$TMP/pkg/bin"
echo "0.6.1" > "$TMP/opt/VERSION"
printf '#!/usr/bin/env bash\necho "uv-servico 0.6.1"\n' > "$TMP/opt/bin/uv-servico"
chmod 0755 "$TMP/opt/bin/uv-servico"
echo "active" > "$TMP/opt/servico_estado_teste"

cat > "$TMP/etc/no.env" <<EOF
NO_ID=no-a
PLEITO=3221
CICLO=ele2026
AMBIENTE=oficial
BASE=https://resultados.tse.jus.br
EOF

# Chaves Ed25519 de teste (descartáveis, em /dev/shm)
ssh-keygen -t ed25519 -N "" -C "manifesto@urna-velox" -f "$TMP/chaves/oficial" >/dev/null
ssh-keygen -t ed25519 -N "" -C "manifesto@urna-velox" -f "$TMP/chaves/intruso" >/dev/null
PUB_OFICIAL="$(awk '{print $1, $2}' "$TMP/chaves/oficial.pub")"
printf 'manifesto@urna-velox namespaces="urna-velox-manifesto" %s\n' "$PUB_OFICIAL" > "$TMP/etc/signatarios_permitidos"

# Pacote 0.6.2 de teste
echo "0.6.2" > "$TMP/pkg/VERSION"
printf '#!/usr/bin/env bash\necho "uv-servico 0.6.2"\n' > "$TMP/pkg/bin/uv-servico"
chmod 0755 "$TMP/pkg/bin/uv-servico"
tar -czf "$TMP/pacote-0.6.2.tar.gz" -C "$TMP/pkg" bin VERSION
SHA_OK="$(sha256sum "$TMP/pacote-0.6.2.tar.gz" | awk '{print $1}')"

cat > "$TMP/etc/federacao.json" <<EOF
{ "versao": 1, "nos": [ { "id": "no-a" }, { "id": "no-b" } ] }
EOF

rodar() {
  UV_ATUALIZADOR_TESTE=1 \
  UV_FED_JSON="$TMP/etc/federacao.json" \
  UV_CONTROLE_MANUAL="$TMP/etc/controle-manual" \
  UV_OPT_DIR="$TMP/opt" \
  UV_NO_ENV="$TMP/etc/no.env" \
  UV_SIGNATARIOS="$TMP/etc/signatarios_permitidos" \
  UV_ATUALIZADOR_SERIE="$TMP/etc/manifesto_serie" \
  UV_ATUALIZADOR_ESTADO="$TMP/saida/atualizador.json" \
  UV_MANIFESTO_LOCAL="$TMP/manifesto.json" \
  python3 "$ATUALIZADOR"
}

assinar() {
  local chave="$1"
  rm -f "$TMP/manifesto.json.sig"
  ssh-keygen -Y sign -q -f "$chave" -n urna-velox-manifesto "$TMP/manifesto.json"
}

gerar_manifesto() {
  local serie="$1" versao="$2" sha="$3" estado="$4" comb="$5" emitido="${6:-$(date -u +%Y-%m-%dT%H:%M:%S+00:00)}"
  cat > "$TMP/manifesto.json" <<EOF
{
  "esquema": 1,
  "serie": $serie,
  "emitido_em": "$emitido",
  "versao": "$versao",
  "pacote": {
    "local": "$TMP/pacote-0.6.2.tar.gz",
    "sha256": "$sha"
  },
  "combinado_ate": "$comb",
  "nos": {
    "no-a": {
      "estado": "$estado",
      "base": "https://resultados.tse.jus.br"
    },
    "no-b": {
      "estado": "$estado",
      "base": "https://resultados.tse.jus.br"
    }
  }
}
EOF
}

echo "== Teste 1: manifesto com assinatura errada (chave intrusa) -> recusado"
gerar_manifesto 100 "0.6.2" "$SHA_OK" "parado" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/intruso"
if rodar; then echo "FALHA: aceitou assinatura inválida"; exit 1; fi
grep -q '"assinatura_invalida' "$TMP/saida/atualizador.json"
[ "$(cat "$TMP/opt/VERSION")" = "0.6.1" ]
echo "  OK: assinatura inválida recusada; versão permaneceu 0.6.1"

echo "== Teste 2: pacote com SHA-256 errado -> recusado"
gerar_manifesto 100 "0.6.2" "0000000000000000000000000000000000000000000000000000000000000000" "parado" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
if rodar; then echo "FALHA: aceitou SHA-256 inválido"; exit 1; fi
grep -q '"sha256_invalido' "$TMP/saida/atualizador.json"
[ "$(cat "$TMP/opt/VERSION")" = "0.6.1" ]
echo "  OK: pacote com SHA-256 errado recusado; versão permaneceu 0.6.1"

echo "== Teste 3: manifesto válido assinado + SHA-256 correto -> atualiza para 0.6.2"
gerar_manifesto 100 "0.6.2" "$SHA_OK" "producao" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
rodar
grep -q '"status": "ok"' "$TMP/saida/atualizador.json"
[ "$(cat "$TMP/opt/VERSION")" = "0.6.2" ]
[ "$(cat "$TMP/opt/servico_estado_teste")" = "active" ]
echo "  OK: atualizado para 0.6.2 e serviço ativo"

echo "== Teste 4: manifesto antigo (série 99 < 100) -> recusado (anti-rollback)"
gerar_manifesto 99 "0.6.1" "$SHA_OK" "parado" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
if rodar; then echo "FALHA: aceitou manifesto antigo"; exit 1; fi
grep -q '"serie_antiga' "$TMP/saida/atualizador.json"
[ "$(cat "$TMP/opt/VERSION")" = "0.6.2" ]
echo "  OK: manifesto antigo (série 99 < 100) recusado"

echo "== Teste 5: combinado_ate vencido contra o TSE -> para a coleta imediatamente"
gerar_manifesto 101 "0.6.2" "$SHA_OK" "producao" "2020-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
if rodar; then echo "FALHA: aceitou combinado_ate vencido"; exit 1; fi
grep -q '"combinado_vencido' "$TMP/saida/atualizador.json"
[ "$(cat "$TMP/opt/servico_estado_teste")" = "inactive" ]
echo "  OK: combinado_ate vencido parou o serviço imediatamente"

servico() { cat "$TMP/opt/servico_estado_teste"; }
serie() { cat "$TMP/etc/manifesto_serie"; }

echo "== Teste 6: série já aplicada não reaplica o estado (serviço ligado à mão continua ligado)"
gerar_manifesto 102 "0.6.2" "$SHA_OK" "parado" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
rodar
[ "$(servico)" = "inactive" ] && [ "$(serie)" = "102" ]
echo "active" > "$TMP/opt/servico_estado_teste"   # alguém ligou o serviço à mão (sem a trava)
rodar; rodar                                      # dois ciclos do timer com a mesma série
[ "$(servico)" = "active" ] || { echo "FALHA: série igual derrubou o serviço ligado à mão"; exit 1; }
grep -q '"em_dia"' "$TMP/saida/atualizador.json"
echo "  OK: série 102 aplicada uma vez; depois, o serviço ligado à mão não foi derrubado"

echo "== Teste 7: trava de controle manual -> não mexe; sem a trava -> aplica a série nova"
echo "60_simulado.sh 2026-10-16T07:00:00-03:00" > "$TMP/etc/controle-manual"
gerar_manifesto 103 "0.6.2" "$SHA_OK" "parado" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
rodar
[ "$(servico)" = "active" ] && [ "$(serie)" = "102" ] || { echo "FALHA: mexeu no nó com a trava manual"; exit 1; }
grep -q '"controle_manual"' "$TMP/saida/atualizador.json"
rm -f "$TMP/etc/controle-manual"
rodar
[ "$(servico)" = "inactive" ] && [ "$(serie)" = "103" ]
echo "  OK: com a trava nada mudou; sem ela a série 103 foi aplicada (parado)"

echo "== Teste 8: o segundo nó espera ${UV_ATRASO_PAR_S:-180} s depois do emitido_em"
gerar_manifesto 104 "0.6.2" "$SHA_OK" "producao" "2099-01-01T00:00:00+00:00"
assinar "$TMP/chaves/oficial"
UV_NO_ID=no-b rodar
[ "$(servico)" = "inactive" ] && [ "$(serie)" = "103" ] || { echo "FALHA: o segundo nó não esperou"; exit 1; }
grep -q '"aguardando_par"' "$TMP/saida/atualizador.json"
gerar_manifesto 104 "0.6.2" "$SHA_OK" "producao" "2099-01-01T00:00:00+00:00" "$(date -u -d '-10 min' +%Y-%m-%dT%H:%M:%S+00:00)"
assinar "$TMP/chaves/oficial"
UV_NO_ID=no-b rodar
[ "$(servico)" = "active" ] && [ "$(serie)" = "104" ]
echo "  OK: emitido agora -> aguardando_par; emitido há 10 min -> aplicado no segundo nó"

echo "TODOS OS 8 TESTES DO UV-ATUALIZADOR PASSARAM"
