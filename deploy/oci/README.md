# Implantação de dois nós do Urna Velox na OCI (Arm / Ampere A1)

Passo a passo completo, decisões e operação: **[docs/HANDOFF_IMPLANTACAO_OCI.md](../../docs/HANDOFF_IMPLANTACAO_OCI.md)** e
**[docs/RELATORIO_DEPLOY_SEM_SSH.md](../../docs/RELATORIO_DEPLOY_SEM_SSH.md)** (operação sem porta 22 e sem portal).

Resumo (do WSL/Linux, nesta pasta):

```bash
cp nos.env.exemplo nos.env && nano nos.env   # órgãos, coleta, IPs, OCIDs, TRANSPORTE=ssh|oci, pleito
./00_criar_oci.sh        # opcional: rede + 2 instâncias A1 + IPs reservados pela OCI CLI (ou pelo console)
./compilar_local_arm64.sh # compilação cruzada local (zero código na OCI: gera gerado/uv-binarios-aarch64.tar.gz)
./10_implantar.sh --sem-compilar # instala binários, configurações, serviço systemd e timer do uv-atualizador (não inicia coleta)
./30_verificar.sh        # estado dos dois nós (serviço, saúde, federação, alcance do par e uv-atualizador)
./40_ensaio.sh           # ensaio na OCI com o simulador (2º turno de 2024), sem tocar no TSE
./20_iniciar.sh --confirmo-tse [--combinado]  # só quando combinado (nada da 2.0 contra o TSE antes de 26/10/2026)
./21_parar.sh · ./50_atualizar.sh [--sem-compilar]
./02_configurar_borda.sh <dominio> # configura Caddy (TLS Let's Encrypt + proxy reverso) e Security List da OCI
./60_simulado.sh --confirmo-tse --combinado   # simulado do 3220 contra a CDN, na ordem de recebimento no TSE
./61_relatorio.sh                             # conferência, 429 e ordem de totalização × recebimento (ORDEM.md)
./60_simulado.sh --restaurar                  # volta o no.env ao pleito de produção
./70_publicar.sh [--estado parado|producao|simulado] [--combinado-ate ISO] [--confirmo-tse] # publicação pull assinada (Ed25519)
./testar_atualizador.sh                       # testes locais do uv-atualizador (assinatura, SHA-256, anti-rollback, combinado_ate)
```

### Exposição Pública e Balanceamento (`./02_configurar_borda.sh`)
- **URL Pública:** Servida via Caddy com terminação TLS/HTTPS (certificado Let's Encrypt automático) na porta 443 e redirecionamento de 80.
- **Topologia de Borda:** Cada nó escuta em `0.0.0.0:8080` apenas para a rede privada (`10.0.1.0/24`) e localhost. O Caddy de cada nó distribui as requisições e conexões WebSocket (`/ws`) 50/50 entre o nó local e o nó parceiro com health check ativo em `/api/v1/estado`.
- **DNS / Domínio:** Domínio registrado via Registro.br (`urnavelox.com.br`) apontando para o IP público do Nó A (`<ip-público-no-a>`) com balanceamento para o Nó B.
- **Segurança e Segredos:** Nenhum segredo ou chave privada é embutido no repositório. As chaves de assinatura do manifesto permanecem na máquina local (`~/.ssh/`), as credenciais OCI no `~/.oci/config` e as variáveis do ambiente no `nos.env` (ignorado pelo git).


### Operação sem a porta 22 (`TRANSPORTE=oci` e Pull por Manifesto Assinado)
- **Parte A (`TRANSPORTE=oci` no `nos.env`):** todos os roteiros (`10` a `61`) operam por HTTPS de saída (porta 443) usando o plugin **Compute Instance Run Command** da OCI CLI e o bucket privado `urna-velox-implantacao` com **Instance Principal** (`/opt/urna-velox/bin/uv-bucket`), sem SSH e sem portal web.
- **Parte B (`./70_publicar.sh` + `uv-atualizador`):** publica no bucket o pacote de binários e o `manifesto.json` assinado com **Ed25519** (`ssh-keygen -Y sign`, chave privada `~/.ssh/uv_manifesto_ed25519` exclusiva da máquina local). O timer `urna-velox-atualizador.timer` (a cada 1 min em cada nó) confere a assinatura (`/etc/urna-velox/signatarios_permitidos`), o número de série (anti-rollback), o SHA-256 do pacote e a trava `combinado_ate` contra o TSE, atualizando um nó de cada vez em até 5 min sem SSH e sem Run Command.
- **Manifesto × operação manual (desde a `0.6.6`):** o atualizador só aplica o manifesto quando a **série é nova**
  (o `combinado_ate` é cobrado sempre). Enquanto existir `/etc/urna-velox/controle-manual` ele não mexe no nó: a trava é
  criada por `20_iniciar.sh` e `60_simulado.sh` e removida por `21_parar.sh` e `60_simulado.sh --restaurar`. O segundo
  nó do `federacao.json` espera 180 s depois do `emitido_em` antes de aplicar. `--estado simulado` só liga o serviço
  com o `no.env` que estiver no nó (o manifesto não leva `candidatos.json`, `ordem` nem `PLEITO`) — para simulado com a
  ordem do TSE use `60_simulado.sh`. O bucket apaga objetos em 7 dias: republique antes do 2º turno.

**Arquive `saida`, `dados` e `federacao` sempre juntos** (o `60_simulado.sh` já faz isso). Quando só a `saida` é
trocada, o diário novo e o storage da federação ficam de execuções diferentes. Até a `0.8.0`, isso levou o nó A a
republicar o diário inteiro e a morrer por falta de memória a cada partida. Desde a `0.8.2`, o serviço não republica
nesse caso e dá o alerta `federacao_diario_desalinhado`. Antes de religar um nó, confira para onde o `no.env` dele
aponta (`BASE`, `PLEITO`): ao voltar, ele retoma a coleta.

O simulado (`60`) usa os insumos de `D:\dados\urna-velox\pleitos\2026-10-04-3220\simulado` (pastas de pleito no
formato `AAAA-MM-DD-código`; `pasta_pleito` no `lib.sh`) (`candidatos.json` do 3220 e `ordem_3220.csv`,
o `recebido_tse` de cada seção tirado dos `aux.json`); a saída anterior de cada nó vai para
`/srv/urna-velox/anteriores/<data>` (nada é apagado). Exige o binário `0.6.1` ou mais novo (`uv-servico --ordem`).
