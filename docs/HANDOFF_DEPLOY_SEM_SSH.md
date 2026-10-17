# Handoff — Implantação nos nós da OCI sem a porta 22 e sem o portal

*Preparado em 15/10/2026, na `v0.6.2`, a pedido do Luciano. Para iniciar a sessão, cole este documento ou peça para
lê-lo inteiro. Leia antes as regras em `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md`. Quem executa trabalha na `main`, em
PATCH (`scripts/versao.sh bump patch` a cada commit), e entrega um relatório no fim (seção 8).*

## 1. O problema

Os dois nós da federação rodam na OCI e hoje só são operados por SSH: todos os roteiros de
`/mnt/d/git/lukeboh/urna-velox/deploy/oci/` passam pelas funções `em` (comando remoto) e `para` (cópia de arquivo) de
`/mnt/d/git/lukeboh/urna-velox/deploy/oci/lib.sh`, que usam `ssh`/`scp` na porta 22. **Na rede de onde se opera a
porta 22 de saída é bloqueada**, e o Luciano não quer depender de estar logado no portal da OCI (Cloud Shell, Bastion,
console). Ele precisa implantar versões novas e operar os nós (iniciar, parar, simulado, relatório) a partir do
computador de operação.

**Não contornar o firewall da rede de onde se opera**: nada de SSH na porta 443, `sslh`, Tailscale, túneis Cloudflare/ngrok ou VPN
própria. Funcionariam, mas violam a política de segurança dessa rede. A solução tem de usar só **HTTPS de saída**
para serviços já permitidos.

**Medido em 15/10/2026, no computador de operação:** HTTPS de saída alcança
`https://iaas.sa-saopaulo-1.oraclecloud.com/` (API de computação da OCI, HTTP 404 = alcançável),
`https://objectstorage.sa-saopaulo-1.oraclecloud.com/` (Object Storage) e `https://api.github.com/` (200). A **OCI CLI
não está instalada** (nem no Windows, nem no WSL do usuário `dev`).

## 2. O que existe hoje

| item | onde |
|---|---|
| Roteiros de implantação e operação | `/mnt/d/git/lukeboh/urna-velox/deploy/oci/` (`00_criar_oci.sh`, `compilar_local_arm64.sh`, `10_implantar.sh`, `20_iniciar.sh`, `21_parar.sh`, `30_verificar.sh`, `40_ensaio.sh`, `50_atualizar.sh`, `60_simulado.sh`, `61_relatorio.sh`) |
| Passo a passo da implantação original | `/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_IMPLANTACAO_OCI.md` |
| Configuração local dos nós (fora do Git) | `/mnt/d/git/lukeboh/urna-velox/deploy/oci/nos.env` (cópia do worktree antigo; IDs, IPs, órgãos, pleito, RPS, chave SSH) |
| Binários aarch64 da `0.6.2`, prontos | `/mnt/d/git/lukeboh/urna-velox/deploy/oci/gerado/uv-binarios-aarch64.tar.gz` (gerado por `compilar_local_arm64.sh`: só executáveis e `VERSION`, sem código-fonte) |
| Insumos do simulado do 3220 | `/mnt/d/dados/urna-velox/simulado-3220/` (`candidatos.json` com as agregadas marcadas, `ordem_3220.csv`) |
| No servidor | binários em `/opt/urna-velox/bin/`, config em `/etc/urna-velox/` (`no.env`, `federacao.json`), dados em `/srv/urna-velox/` (`preparo/`, `saida/`, `dados/`, `federacao/`), serviço systemd `urna-velox` (usuário `urnavelox`), iniciado por `/opt/urna-velox/bin/uv-iniciar.sh` |

Infraestrutura (do `docs/RELATORIO_SIMULADO_1T_2026.md` e do `nos.env`): região `sa-saopaulo-1`; VCN `uv-vcn`
(`10.0.0.0/16`), sub-rede `uv-sub` (`10.0.1.0/24`); `uv-no-a` (Ampere A1, 2 OCPU, 12 GB, IP privado `10.0.1.10`, coleta)
e `uv-no-b` (IP privado `10.0.1.11`, só recebe: `coleta: false`, mesmo órgão do A); Ubuntu 24.04; disco de 100 GB. A
implantação original foi feita de outro usuário do WSL (`/home/<outro-usuário>`, com a chave SSH em
`~/.ssh/ssh-key-2026-06-11-oci`); confira com o Luciano qual usuário e qual máquina usar.

## 3. Solução pedida, em duas partes

### Parte A — para já (o simulado de 16/10): Run Command da OCI + Object Storage

O **Oracle Cloud Agent** (já instalado nas imagens da OCI) tem o plugin **Compute Instance Run Command**: um comando
enviado pela API da OCI é executado na instância pelo agente, que busca a tarefa por HTTPS de saída. Nenhuma porta de
entrada, nenhum login no portal: a OCI CLI autentica com **chave de API** do usuário. Arquivos grandes (binários,
`candidatos.json` de 40 MB, diário de seções de ~100 MB) vão por um **bucket do Object Storage**; a instância lê e
grava no bucket com **instance principal** (sem segredo na máquina).

**Tarefas:**

1. **OCI CLI no computador de operação**, no WSL que roda os roteiros (instalação oficial da Oracle; `oci setup config`
   com uma chave de API do usuário do Luciano — ele mesmo cola a chave pública no perfil, uma vez; a chave privada fica
   em `~/.oci/`, **nunca no repositório**, nunca em mensagem). Confira `oci iam region list` funcionando pela rede de
   onde se opera (proxy, se houver).
2. **IAM** (pela CLI, uma vez; os nomes exatos de famílias de recurso e verbos **confira na documentação oficial da
   OCI** antes de criar, não confie de memória):
   - grupo dinâmico com as duas instâncias (por OCID ou pelo compartimento);
   - política para o grupo dinâmico executar comandos do Run Command (`instance-agent-command-execution-family`) e
     ler/gravar objetos **só no bucket do Urna Velox** (condição `target.bucket.name`);
   - política para o grupo do usuário gerenciar comandos (`instance-agent-command-family`) e objetos desse bucket.
3. **Plugin Run Command ativado** nas duas instâncias (pela CLI: `oci compute instance update` com o
   `agent-config` do plugin "Compute Instance Run Command"; confira se a imagem já o traz ligado). O agente executa os
   comandos como o usuário `ocarun`: dê a ele, por `sudoers` em `/etc/sudoers.d/`, **só** o necessário (`systemctl
   start|stop|status urna-velox`, instalar em `/opt/urna-velox` e `/etc/urna-velox`, ler `/srv/urna-velox/saida`) — ou,
   se ficar complexo, um único roteiro root `/opt/urna-velox/bin/uv-operar.sh <ação>` liberado no sudoers. Esse passo
   de bootstrap precisa de um acesso inicial à máquina: faça-o **pelo próprio Run Command** se o `ocarun` já tiver sudo
   na imagem, ou peça ao Luciano para rodá-lo uma vez por SSH de onde a porta 22 funciona (de outra rede).
4. **Bucket** `urna-velox-implantacao` (nome a confirmar), privado, no compartimento do projeto, com regra de ciclo de
   vida apagando objetos com mais de 7 dias. Nada de PAR (URL pré-autenticada) pública de longa duração.
5. **Transporte trocável no `lib.sh`**: `TRANSPORTE=ssh|oci` no `nos.env` (padrão `ssh`, para nada mudar para quem tem
   SSH). Com `oci`:
   - `em <A|B> <comando>` → cria o comando no Run Command para o OCID do nó (`NO_A_OCID`/`NO_B_OCID` novos no `nos.env`),
     espera terminar (com tempo limite e intervalo de consulta razoáveis), devolve **stdout, stderr e o código de saída**
     do comando remoto como se fosse `ssh` (os roteiros dependem do código de saída: `em A "test -s ..." || erro ...`).
     Saídas maiores que o limite do Run Command vão pelo bucket.
   - `para <A|B> <origem> <destino>` → envia o arquivo ao bucket e manda o nó baixá-lo (instance principal) para o
     destino.
   - **o contrário de `para`** (hoje o `61_relatorio.sh` faz `em N "sudo cat ..." > arquivo` e `em N "gzip -c ..." >
     arquivo`): uma função `de <A|B> <origem> <destino-local>` que passa pelo bucket, e o `61_relatorio.sh` passa a usá-la
     (com SSH, `de` = `scp` ao contrário).
   - stdin para o remoto (o `30_verificar.sh` faz `em "$N" "bash -s" <<EOF ... EOF`): resolva com `em` lendo o stdin
     local quando houver e mandando junto como roteiro.
   - Os pipes entre nós (`em A "tar -czf - ..." | em B "tar -xzf -"`, em `10_implantar.sh` e `50_atualizar.sh`)
     precisam de outro caminho com `oci` (o A grava no bucket, o B lê): reescreva esses trechos em cima de `de`/`para`.
6. **Nenhum roteiro muda de comportamento** além do transporte: as travas continuam (`--confirmo-tse`, `--combinado`,
   checagem das agregadas no `candidatos.json`, guarda da saída anterior em `/srv/urna-velox/anteriores/<data>`).

**Critério da Parte A:** do computador de operação, sem porta 22 e sem portal, com `TRANSPORTE=oci`:
`./10_implantar.sh --sem-compilar` instala a `0.6.2` nos dois nós (`/opt/urna-velox/bin/uv-servico --version` mostra
`0.6.2`), `./30_verificar.sh` mostra o estado dos dois, e `./60_simulado.sh --confirmo-tse --combinado` /
`./61_relatorio.sh` / `./60_simulado.sh --restaurar` funcionam. Com `TRANSPORTE=ssh` (de outra rede) tudo continua igual.

### Parte B — definitiva: os nós buscam a versão (pull) com pacote assinado

Para não depender de nenhuma máquina do Luciano (nem de credencial de API no computador de operação) e para servir
aos clientes futuros (cada órgão com o seu nó, CLAUDE.md), cada nó roda um **atualizador** que consulta, por HTTPS de
saída, um **manifesto assinado** e se ajusta a ele:

- Manifesto JSON (`manifesto.json` + assinatura), publicado no bucket (ou em GitHub Releases do
  `lukeboh/urna-velox`): versão, URL e SHA-256 do pacote de binários aarch64, e o **estado desejado** de cada nó
  (`parado` | `produção` | `simulado`, pleito, arquivos de insumo com SHA-256). Assinatura **Ed25519** (ex.: `minisign`
  ou `ssh-keygen -Y sign`) com a chave privada **só** no computador do Luciano; a pública vai no nó na implantação. Sem
  assinatura válida, o nó não muda nada e registra o motivo.
- `uv-atualizador` (um executável `uv-*` em Rust no workspace, ou um roteiro shell pequeno — decida e justifique) num
  timer do systemd a cada 1–2 min: baixa o manifesto, confere a assinatura, compara com o estado atual, baixa e confere o
  pacote, troca os binários como o `50_atualizar.sh` faz (um nó de cada vez: o B espera o A estar saudável) e ajusta o
  serviço. Registra cada passo num arquivo de estado lido pelo `30_verificar.sh`.
- **Trava de segurança:** um estado que coleta do site real do TSE só vale com um campo `combinado_ate` (data/hora) no
  manifesto, dentro do prazo; vencido, o nó para a coleta. Nada contra o TSE antes de 26/10/2026 sem combinar com o
  Luciano (CLAUDE.md). Um manifesto mais antigo que o atual (número de série) é recusado (sem *rollback* por reenvio).
- Roteiro local `deploy/oci/70_publicar.sh` que monta o manifesto, assina e envia.

**Critério da Parte B:** publicar a `0.6.x` seguinte só com `./70_publicar.sh` e ver os dois nós nela em até 5 min, sem
SSH nem Run Command; manifesto com assinatura errada, pacote com SHA-256 errado e manifesto antigo recusados (testes);
`combinado_ate` vencido para a coleta.

## 4. Ordem sugerida

1. Parte A, passos 1–4 (CLI, IAM, plugin, bucket), com o Luciano presente para a chave de API e qualquer decisão de
   IAM. **Confirme com ele antes de criar qualquer recurso ou política na conta OCI.**
2. Parte A, passo 5 (transporte no `lib.sh`) e teste com comandos inofensivos (`uname -a`, `systemctl is-active
   urna-velox`, `cat /opt/urna-velox/VERSION`) antes de qualquer roteiro que mude algo.
3. Critério da Parte A. A partir daqui o simulado de 16/10 pode rodar pelo Run Command.
4. Parte B, depois do simulado.

## 5. Regras que valem aqui (de `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md`)

- Contra o TSE: no máximo 100 req/s **por órgão** (95 nas coletas longas), contados pelo IP de saída; depois de um 429,
  parar ≥ 10 min; um órgão não roda mais de um nó coletando (o B é do mesmo órgão do A: só recebe).
- Não coletar durante a apuração de uma eleição sem combinar. Os roteiros contra o TSE exigem `--confirmo-tse` e, antes
  de 26/10/2026, `--combinado`.
- **Zero código-fonte na nuvem**: só binários (`compilar_local_arm64.sh`). Um roteiro shell de operação no nó é
  aceitável; repositório Git, `.rs`, `.toml` não.
- Segredos (chave de API da OCI, chave de assinatura) nunca no repositório nem em mensagens.
- Cada commit sobe a versão (`scripts/versao.sh bump patch`, mensagem em `/mnt/d/git/lukeboh/urna-velox/.git/UV_COMMIT_MSG`
  gravada com write_file e `git commit -F`; `git push --follow-tags`); identidade `luciano.bohnert@gmail.com`.
- Armazenamento, inclusive temporário, em `D:\dados` (`/mnt/d/dados`), não na distro do WSL (o `ext4.vhdx` fica no C:,
  quase cheio).

## 6. Riscos e cuidados

- **Limites do Run Command** (tamanho do roteiro, da saída, tempo de execução): confira na documentação e projete
  `em`/`de` para não depender deles (saída grande e arquivos sempre pelo bucket).
- **`ocarun` com sudo amplo** é um privilégio grande: prefira a lista fechada ou o `uv-operar.sh`, e registre a escolha.
- **Política da rede**: a OCI CLI faz só HTTPS para domínios da Oracle; confirme com o Luciano que isso é aceitável na
  rede de onde se opera antes de instalar.
- O `nos.env` em `/mnt/d/git/lukeboh/urna-velox/deploy/oci/` foi copiado do worktree antigo; o campo `REPO` aponta para
  `/home/<outro-usuário>/urna-velox-v2` e só importa sem `--sem-compilar` (não use: compile local).
- O `candidatos.json` de produção (3221) **nos nós** foi gerado com um binário anterior à `0.4.9` e não marca as seções
  agregadas: o `20_iniciar.sh` agora recusa; antes do 2º turno ele precisa ser regerado (`uv-coletor preparar` atual,
  com os `cs.json` de SE e TO) e enviado aos dois nós.

## 7. Fora do escopo

Publicação estática na borda, quórum, Wasm (1.x); terceiro nó; mudar a partição ou a federação; qualquer coleta contra
o TSE além do simulado combinado.

## 8. Entrega

Relatório em `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_DEPLOY_SEM_SSH.md`: o que foi criado na OCI (recursos e
políticas, com os nomes), o que mudou nos roteiros, como testar, o resultado dos critérios A (e B, se feita) e o que
ficou pendente. Atualize `/mnt/d/git/lukeboh/urna-velox/deploy/oci/README.md`,
`/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_IMPLANTACAO_OCI.md` e o `nos.env.exemplo` (`TRANSPORTE`, `NO_A_OCID`,
`NO_B_OCID`, bucket). O Luciano vai pedir uma revisão independente antes de usar na produção.
