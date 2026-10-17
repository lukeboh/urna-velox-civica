# Handoff — implantação de dois nós do Urna Velox em servidores Arm da OCI

*Preparado em 14/10/2026 a pedido do Luciano. Objetivo: subir **dois nós federados** do Urna Velox (`uv-servico
--federacao`, branch `feature/2.0-escalabilidade`, a partir da `2.0.0-alpha.7`) em duas instâncias **Ampere A1 (Arm)**
da Oracle Cloud, com o máximo automatizado. Tudo está em [`deploy/oci/`](../deploy/oci/); este documento é o passo a
passo. Para continuar numa sessão nova: leia este arquivo, o [HANDOFF_SESSAO_2026-10-14.md](HANDOFF_SESSAO_2026-10-14.md)
e o [HANDOFF_0.6.0.md](HANDOFF_0.6.0.md).*

> **Estado honesto:** os scripts foram escritos e conferidos (sintaxe, `shellcheck`), e a parte mais delicada do ensaio
> (o nó B chegando ao simulador por uma ponte, com IP de origem diferente, e a federação entre endereços distintos) foi
> reproduzida no WSL (§5.3). **Nada disso rodou ainda numa conta da OCI**: a primeira execução real é a do §3–§5, e os
> comandos da OCI CLI (`00_criar_oci.sh`) podem pedir ajuste fino de parâmetros. Faça a primeira vez acompanhando a
> saída.

> **Quando (Luciano, 15/10):** a primeira implantação na OCI fica para **depois da versão RC da 2.0**. Nos clientes
> hospedados, cada nó é uma máquina com IP reservado exclusivo (ADR-029), como estes scripts já fazem.

## 1. Antes de começar — três decisões

1. **De que órgão é cada servidor?** O limite do TSE é **100 req/s por órgão**, contado por IP de saída (`CLAUDE.md`).
   Dois servidores do **mesmo órgão** não podem coletar cada um a 100 req/s.
   * Órgãos diferentes (cada um com seu IP): `NO_A_COLETA=true` e `NO_B_COLETA=true` — a coleta da federação
     soma os dois IPs (~190 req/s), o que, pelo benchmark, põe o atraso abaixo de 1 min no p90.
   * Mesmo órgão (ex.: os dois servidores são do Luciano): **`NO_B_COLETA=false`**. O B fica fora da partição, nunca
     pede nada ao TSE, recebe tudo do A e totaliza (réplica: resiliência, segunda fonte de resultado, ensaio de
     federação). O `10_implantar.sh` recusa a combinação "mesmo órgão + os dois coletando", e o `federacao.json` também
     (`"coleta": false` foi criado para isso, `2.0.0-alpha.7`).
2. **Qual versão roda na noite?** A 2.0 é alpha. Contra o site real do TSE, **nada da 2.0 roda antes de 26/10/2026 sem
   combinar** (o 2º turno é 25/10; `CLAUDE.md` também pede acordo para coletar durante uma apuração). O
   `20_iniciar.sh` recusa o site real sem `--confirmo-tse`, e antes de 26/10 sem `--combinado`. Até lá: ensaio (§5).
3. **Exposição ao público:** fora deste escopo. A difusão fica só em `127.0.0.1:8080` em cada nó; a exposição pública é
   a etapa B (direção proposta: cada nó serve os arquivos e uma CDN faz cache na frente; a confirmar).

## 2. O que vai ser criado

| | Nó A | Nó B |
|---|---|---|
| Instância | `VM.Standard.A1.Flex`, 2 OCPU, 12 GB, disco 100 GB, Ubuntu 24.04 aarch64 | idem |
| IP público | **reservado** (é o IP de saída que a CDN do TSE vê e que vai no `federacao.json`) | idem |
| IP privado | `10.0.1.10` (sub-rede `10.0.1.0/24`) | `10.0.1.11` |
| Portas abertas | 22 (só do seu IP) · 7700 federação (só da sub-rede) · 7701 ponte do ensaio (só da sub-rede) | idem |
| Serviço | `urna-velox.service` (systemd) → `uv-iniciar.sh` → `uv-servico --federacao` | idem |
| Pastas | `/srv/urna-velox/{dados,saida,federacao,preparo,ensaio}`, `/etc/urna-velox/`, `/opt/urna-velox/bin/` | idem |

Custo: cabe no **Always Free** da OCI (até 4 OCPUs e 24 GB de A1 e 200 GB de disco somados; IPs reservados e tráfego de
saída dentro da franquia). Disco: na escala do Brasil, cada nó guarda os BUs nos pacotes **e** nos lotes da federação
(~8 GB cada na noite de 2026) — 100 GB sobram.

```
                Internet (TSE: resultados.tse.jus.br)
                  ▲ ≤ 95 req/s (IP A)        ▲ ≤ 95 req/s (IP B) — ou 0, se NO_B_COLETA=false
        ┌─────────┴─────────┐       ┌─────────┴─────────┐
        │ nó A  uv-servico  │◄─────►│ nó B  uv-servico  │   sincronização direta (HTTP só de leitura,
        │ 10.0.1.10:7700    │ 7700  │ 10.0.1.11:7700    │   ETag/304, repasse) — sem storage compartilhado
        └───────────────────┘       └───────────────────┘
          cada nó: todos os BUs, totaliza tudo, confere com os oficiais no fim
```

## 3. Passo a passo

Tudo a partir do WSL (`Ubuntu-24.04`, usuário `dev`), na pasta da branch:

```bash
cd /mnt/d/git/lukeboh/urna-velox-v2/deploy/oci
cp nos.env.exemplo nos.env && nano nos.env      # §3.1
ssh-keygen -t ed25519 -f ~/.ssh/oci_urna_velox   # se ainda não tiver uma chave para a OCI
```

### 3.1 Preencher o `nos.env`
Ids, órgãos (§1.1), `NO_x_COLETA`, IPs (depois do §3.2), `PLEITO`/`CICLO` da eleição, `ECO_IP`
(`https://checkip.amazonaws.com`: o nó só coleta se o IP medido for o declarado). Para o `00_criar_oci.sh`, também
`OCI_COMPARTIMENTO`, `ADMIN_CIDR` (seu IP/32; descubra com `curl -s checkip.amazonaws.com`), `SSH_CHAVE_PUB` e,
opcionalmente, `OCI_AD`, `OCI_OCPUS`, `OCI_MEMORIA_GB`, `OCI_DISCO_GB`.

### 3.2 Criar a infraestrutura — pela CLI (`00_criar_oci.sh`) **ou** pelo console

**CLI:** instale e configure a OCI CLI (`bash -c "$(curl -L https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.sh)"`
e `oci setup config`, com a chave de API enviada no console → *Perfil → Chaves de API*). Depois `./00_criar_oci.sh`:
cria (ou reaproveita, pelo nome) VCN `uv-vcn`, internet gateway, rota, security list `uv-seguranca`, sub-rede `uv-sub`,
as instâncias `uv-no-a`/`uv-no-b` com IP privado fixo e um **IP público reservado** para cada uma, e imprime as linhas
`NO_x_IP_PUBLICO`/`NO_x_IP_PRIVADO` para colar no `nos.env`. Erro comum: *Out of host capacity* (A1 do Always Free
esgota) — rode de novo mais tarde ou troque `OCI_AD`.

**Console (alternativa):**
1. *Networking → Virtual Cloud Networks → Start VCN Wizard → "Create VCN with Internet Connectivity"* (VCN
   `10.0.0.0/16`, sub-rede pública `10.0.1.0/24`).
2. Na *Default Security List* da sub-rede pública: ingress TCP 22 só do seu IP (troque o `0.0.0.0/0` padrão) e TCP
   7700–7701 com origem `10.0.1.0/24`.
3. *Compute → Instances → Create* (duas vezes): imagem **Canonical Ubuntu 24.04** (aarch64), *shape*
   **VM.Standard.A1.Flex** 2 OCPU / 12 GB, a sub-rede pública, *Private IPv4* `10.0.1.10` / `10.0.1.11`, **sem** IP
   público efêmero, sua chave pública SSH, *boot volume* 100 GB.
4. *Networking → IP Management → Reserved Public IPs → Reserve*, duas vezes; em cada instância, *Attached VNICs →
   IPv4 Addresses → Edit → Reserved public IP*.
5. Anote os IPs no `nos.env`.

### 3.3 Implantar — `./10_implantar.sh`
Faz, em ordem (idempotente; rodar de novo atualiza):
0. confere SSH, arquitetura (`aarch64`) e a regra do órgão (§1.1);
1. gera `gerado/federacao.json` (os dois nós, órgão, IP público declarado, endereço de sincronização, `coleta`) e
   `gerado/no-A.env`/`no-B.env`;
2. empacota o código da branch com `git archive` (**sem precisar de GitHub nos servidores**);
3. em cada servidor (`servidor/preparar.sh`): pacotes, usuário `urnavelox`, pastas, Rust, `chrony` com o servidor de
   tempo da OCI (169.254.169.254), limites de rede, e **abre a porta 7700 no `iptables` só para o IP do par** (a imagem
   Ubuntu da OCI rejeita tudo menos SSH por padrão — além da security list, é preciso abrir no sistema);
4. compila no A (`servidor/compilar.sh`, ~15–25 min com LTO em 2 OCPUs) e copia os binários para o B;
5. instala `federacao.json`, `no.env`, `uv-iniciar.sh` e `urna-velox.service` (habilitado, **não iniciado**);
6. monta o `candidatos.json` do pleito **uma vez, no A** (`uv-coletor preparar`, ≤ 5 req/s) e copia para o B;
7. mede o IP de saída de cada nó pelo eco e avisa se for diferente do declarado.

`./10_implantar.sh --sem-compilar` refaz só configuração e serviço.

## 4. Verificar — `./30_verificar.sh`
Para cada nó: estado do serviço, versão, disco; da `saude.json`: estado, relógio do site, seções aplicadas, fila,
req/s, `aux`, BUs baixados, **HTTP 429** e pausa; federação: **participa**, IP medido, cota, ativos, fora (com o motivo:
`morto`, `em_pausa`, `ip_divergente`, `so_recebe`), publicados/recebidos/delegadas/assumidas; sincronização (lotes
copiados, por repasse, servidas, erros); **alcance do par** (`GET /fed/v1/nos/<par>.json` deve dar 200 ou 304);
últimos alertas.

## 5. Ensaio na OCI, sem o TSE — `./40_ensaio.sh`

### 5.1 O que faz
Roda o **2º turno de 2024 inteiro** (51 municípios, ~97 mil BUs) com o `uv-simulador` no nó A, limite de **100 req/s
por IP de origem**, a `ENSAIO_ACEL` (padrão 10×, ~25 min): envia os dados (~120 MB, só na primeira vez) de
`$UVREAL/2024-2t`, sobe ponte, simulador e os dois nós em unidades próprias (`uv-ensaio-*`, pastas em
`/srv/urna-velox/ensaio`, **sem tocar na configuração de produção**), acompanha a cada minuto e, no fim, mostra a
conferência de cada nó, os duplicados, os 429 e as requisições por IP de origem, e limpa tudo (`--manter` deixa as
unidades no ar para investigar). Recusa rodar se o serviço de produção estiver ativo.

### 5.2 Como o B chega ao simulador
O simulador só escuta em `127.0.0.1` do A, e o serviço só aceita aceleração contra servidor local (trava de segurança
contra o TSE). Então: ponte no B (`127.0.0.1:18080`) → rede privada → ponte no A (`10.0.1.10:7701`) →
`127.0.0.1:18080`. Para o simulador, o B chega de `127.0.0.1` e o A sai de `127.0.0.2` (`--ip-local`): dois IPs de
origem, cada um com o seu limite, e o eco de IP de cada nó confere com o `federacao.json` do ensaio.

### 5.3 Já provado no WSL (14/10)
`D:\dados\uv2-scripts\ensaio_topologia.sh` reproduz essa topologia numa máquina só (ponte em Python no lugar do
`socat`; "IPs privados" `127.0.0.3`/`127.0.0.4`; AM e RO a 120×): resultado em §9.

### 5.4 Critério para dar a implantação por boa
Os dois nós **2/2 (ou N/N) idênticos ao oficial**, 0 duplicados, **0 HTTP 429**, no simulador nenhum IP acima de
100 × aceleração por segundo, e cada nó com lotes copiados do outro (`30_verificar.sh`). Repita a 60× (1º turno de
2026) quando o critério A completo da etapa A for rodado (dados maiores: ~8 GB por nó).

## 6. Iniciar na noite — `./20_iniciar.sh --confirmo-tse [--combinado]`
Checklist antes:
* [ ] combinado com o Luciano (antes de 26/10/2026 a trava exige `--combinado`); a noite do 2º turno é 25/10;
* [ ] `NO_x_COLETA` coerente com os órgãos (§1.1); `30_verificar.sh` mostra os dois IPs medidos = declarados;
* [ ] `candidatos.json` do pleito certo (`PLEITO=3221` = 2º turno de 2026) e sem `incompleto` relevante;
* [ ] ensaio do §5 aprovado com o binário que vai rodar;
* [ ] `chronyc tracking` com desvio pequeno nos dois.

Inicia os dois juntos (eles se enxergam nos primeiros *heartbeats* antes de coletar) e mostra o estado. Contra o TSE o
serviço **não sai sozinho** depois da conferência (um BU atrasado reabre a publicação): pare com `./21_parar.sh`.

## 7. Operação
| Precisa de… | Faça |
|---|---|
| ver o estado | `./30_verificar.sh` |
| logs | `ssh … sudo tail -f /srv/urna-velox/saida/servico.log` (JSON por linha), `alertas.csv`, `stderr.log` |
| parar / retomar | `./21_parar.sh [A|B]` / `./20_iniciar.sh …` — a retomada relê pacotes e diário, nada se perde; o nó que volta recebe do par o que perdeu |
| atualizar o código | `./50_atualizar.sh` — compila no A sem parar nada, troca um nó por vez e **só reinicia quem estava rodando** |
| um nó caiu | o outro detecta em ~30 s (`fora: morto`) e **assume as seções dele**; o `systemd` reinicia o caído (`Restart=on-failure`) |
| HTTP 429 | o nó atingido pausa 10 min e sai da partição; o outro assume; nada a fazer (conferir no `30_verificar.sh`) |
| IP divergente | o nó não coleta (só recebe): confira se o IP público reservado está atribuído e se o `nos.env` está certo; refaça `10_implantar.sh --sem-compilar` |
| disco | `df -h /srv`; os lotes da federação ficam em `/srv/urna-velox/federacao` (podem ser apagados com o serviço parado, depois da eleição) |

## 8. Arquivos

| Arquivo | Onde roda | Função |
|---|---|---|
| `deploy/oci/nos.env.exemplo` | local | variáveis (`TRANSPORTE=ssh\|oci`, `NO_A_OCID`, `NO_B_OCID`, `OCI_BUCKET`; copie para `nos.env`, fora do git) |
| `deploy/oci/lib.sh` | local | funções comuns (`em`, `para`, `de` sobre SSH ou OCI Run Command + Object Storage, endereços, travas) |
| `deploy/oci/00_criar_oci.sh` | local (OCI CLI) | rede, firewall da nuvem, IPs reservados, 2 instâncias A1 |
| `deploy/oci/compilar_local_arm64.sh` | local (WSL) | compilação cruzada local (`aarch64-unknown-linux-gnu`), zero código-fonte na nuvem |
| `deploy/oci/10_implantar.sh` | local | prepara, instala binários (`--sem-compilar`), configura `federacao.json`, `candidatos.json`, `urna-velox` e `uv-atualizador` |
| `deploy/oci/20_iniciar.sh` / `21_parar.sh` | local | inicia (com travas) / para |
| `deploy/oci/30_verificar.sh` | local | estado dos dois nós (serviço, saúde, federação, alcance do par e `uv-atualizador`) |
| `deploy/oci/40_ensaio.sh` | local | ensaio com o simulador na OCI |
| `deploy/oci/50_atualizar.sh` | local | atualização um nó por vez (`--sem-compilar` ou compilando no A) |
| `deploy/oci/60_simulado.sh` / `61_relatorio.sh` | local | simulado do 3220 contra a CDN na ordem de recebimento (`--ordem`) e relatório (`ORDEM.md`) |
| `deploy/oci/70_publicar.sh` | local | publica pacote e `manifesto.json` assinado com Ed25519 no bucket para atualização pull automática |
| `deploy/oci/testar_atualizador.sh` | local | 8 testes automatizados do `uv-atualizador` (assinatura, SHA-256, atualização, anti-rollback, `combinado_ate`, série repetida, trava manual, atraso do par) |
| `deploy/oci/servidor/preparar.sh` | servidor (root) | pacotes, usuário, `uv-bucket` (Instance Principal), `uv-operar.sh`, sudoers de `ocarun`, relógio, `iptables`, limites |
| `deploy/oci/servidor/uv-atualizador` | servidor (timer) | agente pull que valida manifesto Ed25519, SHA-256, série e `combinado_ate` a cada 1 min |
| `deploy/oci/servidor/uv-iniciar.sh` | servidor (systemd) | monta a linha de comando a partir de `/etc/urna-velox/no.env` |
| `deploy/oci/servidor/urna-velox.service` | servidor | unidade systemd (reinício, limites, endurecimento) |

## 9. Registro do que foi conferido
* `bash -n` e `shellcheck -S warning` em todos os scripts: sem erros (14/10 e 16/10).
* Código: `"coleta": false` por nó (`config.rs`, `estado.rs`; motivo `so_recebe`), com 2 testes novos; federação
  22/22 testes de unidade.
* Topologia do ensaio no WSL (14/10, `ensaio_topologia.sh`, AM e RO a 120×, ponte em Python no lugar do `socat`): o B
  chega ao simulador como `127.0.0.1`, o A sai por `127.0.0.2`; os dois nós **2/2 idênticos ao oficial**, 5.102 BUs cada,
  `aux.json` 2.643 + 2.459 = **5.102** (cada seção uma vez), **0 HTTP 429**, ~100 lotes copiados por nó, máximo de
  1.012 req num segundo por IP (limite 12.000 a 120×).
* Implantação e operação na OCI sem porta 22 e sem portal (15–16/10, [docs/RELATORIO_DEPLOY_SEM_SSH.md](RELATORIO_DEPLOY_SEM_SSH.md)):
  `TRANSPORTE=oci` (Run Command + Object Storage com Instance Principal) e agente pull `uv-atualizador` com manifesto
  Ed25519 (`70_publicar.sh`) validados nos dois nós (`uv-no-a` e `uv-no-b`).

## 10. Pendências e riscos
* O critério A completo (2026 a 60× com 2 ou 3 nós) não foi rodado; a OCI é um bom lugar para ele (`40_ensaio.sh` com
  outro conjunto de dados — adaptar o passo 1 e o pleito).
* Autenticação entre os nós (etapa C): hoje a porta 7700 é protegida só por rede (security list + `iptables` com o IP
  do par). Não exponha a 7700 à internet.
* Exposição pública dos resultados: etapa B (a confirmar).
