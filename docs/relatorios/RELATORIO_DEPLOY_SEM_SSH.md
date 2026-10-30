# Relatório — Implantação e Operação nos Nós da OCI sem a Porta 22 e sem o Portal

*Data: 15–16/10/2026 · Versão: `v0.6.5` (revisado na `v0.6.6`) · Referência: [docs/handoffs/HANDOFF_DEPLOY_SEM_SSH.md](../handoffs/HANDOFF_DEPLOY_SEM_SSH.md)*

> **Revisão da `0.6.6` (16/10/2026).** Na conferência deste relatório, quatro afirmações não correspondiam ao que
> está nos nós e foram corrigidas no texto: (1) o `sudoers` do `ocarun` **não** é uma lista fechada — na prática
> equivale a root (§2.2 e §7); (2) os 83/83 citados no §4.1 vêm da saída do simulado de **15/10** (binário
> `2.0.0-alpha.9`), não de uma execução na `0.6.2`; (3) a amostra com `--ordem` parou em **19.062** BUs, não ~22.000;
> (4) o `uv-atualizador` da `0.6.5` reaplicava o manifesto a cada 60 s mesmo sem série nova, o que desfazia o
> `60_simulado.sh`/`21_parar.sh` em até um minuto — corrigido na `0.6.6` (§4.2 e §7).

---

## 1. Resumo Executivo

As duas partes previstas no [HANDOFF_DEPLOY_SEM_SSH.md](../handoffs/HANDOFF_DEPLOY_SEM_SSH.md) foram implementadas e validadas nos dois nós Ampere A1 (`uv-no-a` e `uv-no-b`) da região `sa-saopaulo-1`, operando a partir do WSL (`Ubuntu-24.04`, usuário `dev`) na estação de operação sem utilizar conexões SSH de saída (porta 22) e sem depender do portal web da OCI:

- **Parte A (Run Command + Object Storage via `TRANSPORTE=oci`):** transporte intercambiável (`ssh` | `oci`) em [`deploy/oci/lib.sh`](../../deploy/oci/lib.sh), preservando código de saída, `stdout`, `stderr` e `stdin` (`em`), envio de arquivos ao nó (`para`) e coleta de arquivos do nó (`de`) através do bucket privado `urna-velox-implantacao` com **Instance Principal** (`/opt/urna-velox/bin/uv-bucket`).
- **Parte B (Atualização Pull com Manifesto Assinado Ed25519):** agente `/opt/urna-velox/bin/uv-atualizador` ([`deploy/oci/servidor/uv-atualizador`](../../deploy/oci/servidor/uv-atualizador)) executado a cada 60 s via `urna-velox-atualizador.timer`, publicado localmente por [`deploy/oci/70_publicar.sh`](../../deploy/oci/70_publicar.sh) e validado por [`deploy/oci/testar_atualizador.sh`](../../deploy/oci/testar_atualizador.sh).

---

## 2. O que foi criado na OCI (Recursos, Políticas e Bootstrap)

### 2.1. Identidade e Acesso (IAM) e Armazenamento

| Recurso | Tipo | Nome / Identificador | Configuração |
|---|---|---|---|
| **OCI CLI local** | Ferramenta WSL (`dev`) | `~/lib/oracle-cli` (`~/.local/bin/oci` v3.95.0) | Configurada em `~/.oci/config` (permissão `0600`, fora do repositório) autenticando como `luciano.bohnert@gmail.com` (`ocid1.user.oc1..<id>`) na região `sa-saopaulo-1`. |
| **Grupo Dinâmico** | IAM Dynamic Group | `uv-nos-dg` (`ocid1.dynamicgroup.oc1..<id>`) | Regra de correspondência fechada nos OCIDs das duas instâncias: `Any {instance.id = 'ocid1.instance.oc1.sa-saopaulo-1.<id-no-a>', instance.id = 'ocid1.instance.oc1.sa-saopaulo-1.<id-no-b>'}`. |
| **Política IAM** | IAM Policy (root) | `uv-nos-policy` (`ocid1.policy.oc1..<id>`) | Declarações restritas à execução do Run Command na própria instância e ao bucket `urna-velox-implantacao` (ver listagem abaixo). |
| **Bucket Privado** | Object Storage | `urna-velox-implantacao` (namespace `<namespace>`) | `publicAccessType: NoPublicAccess` (sem acesso público e sem PAR). Regra de ciclo de vida `apagar-apos-7-dias` ativa (`DELETE objects` após 7 dias). |

**Declarações exatas da política `uv-nos-policy`:**
```text
Allow dynamic-group uv-nos-dg to use instance-agent-command-execution-family in tenancy where request.instance.id=target.instance.id
Allow dynamic-group uv-nos-dg to read buckets in tenancy where target.bucket.name='urna-velox-implantacao'
Allow dynamic-group uv-nos-dg to manage objects in tenancy where target.bucket.name='urna-velox-implantacao'
Allow service objectstorage-sa-saopaulo-1 to manage object-family in tenancy where target.bucket.name='urna-velox-implantacao'
Allow group Administrators to manage instance-agent-command-family in tenancy
Allow group Administrators to manage objects in tenancy where target.bucket.name='urna-velox-implantacao'
```

### 2.2. Instâncias e Bootstrap do Agente (`ocarun`)

- **Nó A (`uv-no-a`):** `ocid1.instance.oc1.sa-saopaulo-1.<id-no-a>` (`<ip-público-no-a>` / `10.0.1.10`) — plugin `Compute Instance Run Command` ativo (`ENABLED`).
- **Nó B (`uv-no-b`):** `ocid1.instance.oc1.sa-saopaulo-1.<id-no-b>` (`<ip-público-no-b>` / `10.0.1.11`) — plugin `Compute Instance Run Command` ativo (`ENABLED`).
- **Intervalo de consulta do Run Command:** na imagem Ubuntu 24.04 da OCI, `/etc/oracle-cloud-agent/plugins/runcommand/config.yml` traz por padrão `pollCommandInterval: '4m'`. Foi ajustado nos dois nós (e incorporado em `servidor/preparar.sh`) para `pollCommandInterval: '5s'`, reduzindo a latência de cada comando remoto de ~2–4 min para ~8–12 s.
- **Privilégios do usuário `ocarun` (`/etc/sudoers.d/urna-velox-ocarun`):** a intenção era uma lista fechada no lugar de `NOPASSWD: ALL`, validada com `visudo -cf`. **O efeito, porém, é equivalente a root:** a lista inclui `/opt/urna-velox/bin/uv-operar.sh` (que faz `exec "$@"`), `tee`, `cp`, `sed`, `chmod`, `systemd-run` e `bash /tmp/uv-implantar/*` sem restrição de argumentos — qualquer um deles basta para escrever em qualquer arquivo ou rodar qualquer comando como root. A barreira real é o IAM (só quem está no grupo `Administrators` cria comandos do Run Command nas instâncias). Fechar de verdade exige trocar a lista por ações nomeadas do `uv-operar.sh` (pendência no §7). O comentário em `servidor/preparar.sh` diz isso desde a `0.6.6`.
- **Cliente de Object Storage com Instance Principal (`/opt/urna-velox/bin/uv-bucket`):** instalado em `/opt/urna-velox/oci-venv` usando `oci.auth.signers.InstancePrincipalsSecurityTokenSigner()`, sem armazenar nenhuma chave ou credencial nos servidores.

---

## 3. O que mudou nos Roteiros (`deploy/oci/`)

1. **[`deploy/oci/nos.env.exemplo`](../../deploy/oci/nos.env.exemplo) e `deploy/oci/nos.env`:**
   - Adicionadas as variáveis `TRANSPORTE=ssh|oci` (padrão `ssh`), `OCI_COMPARTIMENTO`, `OCI_BUCKET`, `OCI_NAMESPACE`, `NO_A_OCID` e `NO_B_OCID`.
2. **[`deploy/oci/lib.sh`](../../deploy/oci/lib.sh):**
   - Mantido o comportamento original para `TRANSPORTE=ssh` (inclusive adicionando `de` como `scp` reverso).
   - Para `TRANSPORTE=oci`, `em`, `para` e `de` utilizam o SDK Python da OCI CLI (`~/lib/oracle-cli/bin/python3`) sobre HTTPS (porta 443):
     - `em <A|B> <comando>`: suporta `stdin` (via descritor `3<&0`, usado por `em "$N" "bash -s" <<EOF` no `30_verificar.sh`), captura separadamente `stdout` (fd 1), `stderr` (fd 2) e o código de saída exato (`$?`). Comandos ou saídas maiores que os limites de texto plano do Run Command (4 KB de entrada / 1 KB de saída) transitam automaticamente pelo bucket `urna-velox-implantacao` e são apagados no bloco `finally`.
     - `para <A|B> <origem> <destino>` e `de <A|B> <origem> <destino-local>`: transferem arquivos de qualquer tamanho pelo bucket usando Instance Principal (`uv-bucket get`/`put`) e removem o objeto temporário imediatamente após a cópia.
3. **[`deploy/oci/10_implantar.sh`](../../deploy/oci/10_implantar.sh), [`40_ensaio.sh`](../../deploy/oci/40_ensaio.sh), [`50_atualizar.sh`](../../deploy/oci/50_atualizar.sh) e [`61_relatorio.sh`](../../deploy/oci/61_relatorio.sh):**
   - Substituídos os pipes diretos entre nós (`em A "tar -czf - ..." | em B "tar -xzf -"`) por `de A` + `para B`.
   - Em `61_relatorio.sh`, substituídos os redirecionamentos `em "$N" "sudo cat ..." > arquivo` e `em "$N" "sudo gzip -c ..." > arquivo` por `de "$N"`.
   - Em `20_iniciar.sh`, `60_simulado.sh` e `10_implantar.sh`, ajustada a expressão regular de verificação das seções agregadas para `grep -Eq '"agregada":[[:space:]]*true'` (compatível com JSON formatado com espaço após os dois-pontos).
   - Em `10_implantar.sh`, se o `candidatos.json` do pleito 3221 nos nós ainda não tiver `"agregada": true`, envia o `candidatos.json` local já preparado (`~/uvreal/2026-2t/preparo/candidatos.json`, com as 17.787 seções agregadas marcadas).
4. **Parte B — [`deploy/oci/servidor/uv-atualizador`](../../deploy/oci/servidor/uv-atualizador), [`70_publicar.sh`](../../deploy/oci/70_publicar.sh) e [`testar_atualizador.sh`](../../deploy/oci/testar_atualizador.sh):**
   - **Escolha técnica e justificativa:** implementado `/opt/urna-velox/bin/uv-atualizador` em Python (sobre o ambiente `/opt/urna-velox/oci-venv` já instalado para Instance Principal) combinando `ssh-keygen -Y verify` (Ed25519 nativo do OpenSSH) e `sha256sum`. Isso evita adicionar dependências de assinatura/OCI aos binários Rust de apuração, permite que o atualizador substitua todos os binários `uv-*` em `/opt/urna-velox/bin/` sem conflito de auto-sobrescrita e funciona tanto via bucket OCI (Instance Principal) quanto via URL HTTPS estática (ex.: GitHub Releases).
   - O `30_verificar.sh` lê e exibe `/srv/urna-velox/saida/atualizador.json` em cada nó.

---

## 4. Resultados dos Critérios de Aceitação

### 4.1. Critério da Parte A (`TRANSPORTE=oci`, sem SSH e sem portal)

1. **`./10_implantar.sh --sem-compilar`:**
   - Executado integralmente do WSL com `TRANSPORTE=oci`: preparou ambos os servidores, instalou a `0.6.2` (`/opt/urna-velox/bin/uv-servico --version` reportou `uv-servico 0.6.2` nos nós A e B), instalou o `candidatos.json` do pleito 3221 com as 17.787 seções agregadas marcadas e conferiu os IPs de saída (`<ip-público-no-a>` e `<ip-público-no-b>`).
2. **`./30_verificar.sh`:**
   - Exibiu o estado completo dos dois nós, saúde, federação e alcance HTTP 200 entre os pares na sub-rede privada `10.0.1.0/24`.
3. **`./60_simulado.sh --confirmo-tse --combinado`:**
   - Parou os dois nós, moveu a saída anterior para `/srv/urna-velox/anteriores/20261016-000600`, enviou `candidatos.json` (39 MB) e `ordem_3220.csv` (18 MB) pelo bucket e iniciou os dois nós na `0.6.2` com `--ordem`.
   - As 499.207 seções não agregadas entraram na fila ordenadas por `recebido_tse` (`858 aplicadas + 498.348 na fila` na primeira leitura, a ~95 req/s no nó A e 0 req/s externos no nó B, `HTTP 429: 0`).
4. **`./61_relatorio.sh`:**
   - Validado tanto sobre a saída completa anterior (`resultado-antes-0.6.2` = saída do simulado de **15/10**, binário `2.0.0-alpha.9`, sem `--ordem`: **83/83 abrangências idênticas ao oficial**, 0 situações divergentes, 0 revogações, 499.207 BUs aplicados) quanto sobre a amostra da `0.6.2` com `--ordem` (`resultado-amostra-0.6.2`, 19.062 BUs aplicados):
     - Antes da `0.6.1`/`0.6.2` (sem `--ordem`): atraso no recebimento p50 **244.842 s**, deslocamento p50 **158.050**, UFs em ordem alfabética (`AC AL AM AP...`).
     - Com a `0.6.2` (`--ordem`): atraso no recebimento p50 **1 s** (p99 **56 s**), deslocamento p50 **6** (p99 **25**), UFs iniciadas na ordem do primeiro BU recebido no TSE (`ZZ MA AC AP TO RO DF PA PR AM MG...`).
5. **`./60_simulado.sh --restaurar`:**
   - Parou os dois nós e restaurou `/etc/urna-velox/no.env` para o pleito de produção (`PLEITO=3221`).

### 4.2. Critério da Parte B (`./70_publicar.sh` + `uv-atualizador` + testes de segurança)

1. **Testes automatizados de segurança (`./testar_atualizador.sh` — 5/5 aprovados na `0.6.5`; 8/8 na `0.6.6`, com os três novos listados no fim):**
   - Manifesto com assinatura Ed25519 inválida (chave intrusa): **recusado** (`assinatura_invalida`), versão inalterada.
   - Pacote com SHA-256 divergente: **recusado** (`sha256_invalido`), versão inalterada.
   - Manifesto válido assinado + SHA-256 correto: **atualizado** (`status: ok`).
   - Manifesto com número de série antigo (`99 < 100`): **recusado** (`serie_antiga`, proteção anti-rollback).
   - Manifesto com `combinado_ate` vencido contra o TSE: **serviço parado imediatamente** e bloqueado (`combinado_vencido`).
   - *(0.6.6)* Manifesto já aplicado (mesma série) com o serviço parado à mão: **não religa** (`em_dia`). Na `0.6.5` só a série *menor* era recusada; a *igual* era reaplicada a cada 60 s.
   - *(0.6.6)* Com `/etc/urna-velox/controle-manual` presente: atualizador **não toca em nada** (`controle_manual`). A trava é criada por `20_iniciar.sh` e `60_simulado.sh` e removida por `21_parar.sh` e `60_simulado.sh --restaurar`.
   - *(0.6.6)* Nó que não é o primeiro do `federacao.json` espera `UV_ATRASO_PAR_S` (180 s) depois do `emitido_em` antes de aplicar (`aguardando_par`), para que um pacote ruim não derrube os dois nós no mesmo minuto.
2. **Publicação real da `0.6.5` nos nós da OCI apenas com `./70_publicar.sh`:**
   - Com os nós na `0.6.2`, foi executado exclusivamente `./70_publicar.sh --estado parado` (enviando o pacote `0.6.5` e o `manifesto.json` assinado ao bucket, sem SSH e sem Run Command).
   - Em **~75 segundos** (< 5 min), o `urna-velox-atualizador.timer` nos dois nós baixou o manifesto, validou a assinatura Ed25519 e o SHA-256 e atualizou ambos os nós para a `0.6.5`.

---

## 5. Como Testar e Operar

```bash
cd /mnt/d/git/lukeboh/urna-velox/deploy/oci

# 1. Verificar estado dos dois nós (via Run Command quando TRANSPORTE=oci)
./30_verificar.sh

# 2. Rodar os 5 testes locais de segurança do uv-atualizador
./testar_atualizador.sh

# 3. Publicar nova versão ou alterar estado desejado via pull (sem SSH e sem Run Command)
./compilar_local_arm64.sh
./70_publicar.sh --estado parado

# 4. Rodar o simulado completo do 3220 pelo Run Command
./60_simulado.sh --confirmo-tse --combinado
./30_verificar.sh
./61_relatorio.sh
./60_simulado.sh --restaurar
```

---

## 6. O que ficou pendente / Recomendações

- **Simulado completo de ~1h50m do pleito 3220 na `0.6.2+`:** o fluxo completo (`60_simulado.sh` → coleta com `--ordem` → `61_relatorio.sh` → `60_simulado.sh --restaurar`) foi validado na OCI até 19.062 BUs aplicados (confirmando a ordem de recebimento p50 = 1 s e 0 erros HTTP 429) e em seguida restaurado para `parado` (`PLEITO=3221`) para não manter coleta desacompanhada durante a madrugada. Quando desejar rodar as 499.207 seções até o fim com `--ordem`, basta executar `./60_simulado.sh --confirmo-tse --combinado`.
- **Antes da produção do 2º turno (25/10/2026):** mover ou arquivar `/srv/urna-velox/saida` da amostra do simulado e regerar o `candidatos.json` do pleito `3221` quando o TSE publicar o `to-p003221-cs.json` (atualmente ausente na CDN oficial).

---

## 7. Pendências registradas na revisão da `0.6.6`

1. **`sudo` do `ocarun` equivale a root.** Fechar de verdade: o `uv-operar.sh` passa a aceitar só ações nomeadas
   (`iniciar`, `parar`, `instalar-pacote <arquivo do bucket>`, `trocar-no-env <perfil>`...), cada uma com argumentos
   validados, e o `sudoers` fica com essa única entrada — sem `tee`, `cp`, `sed`, `chmod`, `systemd-run` nem
   `bash /tmp/...`. Até lá, a proteção é o IAM: ninguém fora de `Administrators` cria comandos do Run Command.
2. **`--estado simulado` do `70_publicar.sh` é só nominal.** O manifesto não leva insumos (`candidatos.json`,
   `ordem_*.csv`, `PLEITO`, `ORDEM`); ele apenas liga o serviço com o `no.env` que já estiver no nó. Para um simulado
   com a ordem do TSE, usar `60_simulado.sh` (que envia os insumos, grava `ORDEM=` e cria a trava manual).
3. **O manifesto some do bucket em 7 dias** (regra `apagar-apos-7-dias`). Sem manifesto, o atualizador só registra
   erro e não mexe nos serviços — mas também não aplica nada. Republicar com `70_publicar.sh` na semana do 2º turno
   (antes de 25/10) e conferir com `30_verificar.sh`.
4. **`combinado_ate` largo demais.** O manifesto `20261016091638` (emitido em 16/10 06:16) traz
   `estado: simulado` e `combinado_ate: 2026-10-25T23:59:59-03:00`, ou seja, autoriza coleta contra o TSE até o fim da
   noite do 2º turno. Para simulados, usar prazos de horas; para a produção, publicar um manifesto próprio.
5. **`pollCommandInterval: '5s'`** foi editado no `config.yml` do plugin do Oracle Cloud Agent. Uma atualização do
   agente pode reverter para `4m`; o sintoma é cada comando do `TRANSPORTE=oci` levar minutos. O `preparar.sh` reaplica.
6. **O `uv-atualizador` não vai no pacote de binários.** Ele é instalado pelo `preparar.sh`/`10_implantar.sh`; um
   pacote publicado pelo `70_publicar.sh` não o atualiza. A `0.6.6` foi instalada nos dois nós em 16/10 por
   `para` + `sudo install` via Run Command e conferida (`em_dia`, trava e liberação testadas no nó A).
7. **Chave de assinatura sem senha.** A chave criada na `0.6.5` (`~/.ssh/uv_manifesto_ed25519`) foi gerada com
   `-N ""`. Desde a `0.6.6`, `chave_manifesto` (em `lib.sh`) nunca gera chave sem senha e avisa se a existente não
   tiver. Pôr senha na atual: `ssh-keygen -p -f ~/.ssh/uv_manifesto_ed25519`.
