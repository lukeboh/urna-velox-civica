# Handoff — Urna Velox: Etapa 6, ensaio geral (`0.6.0`, alvo 23/10/2026)

> **Renumeração de 16/10/2026 — leia antes.** Este é agora o handoff da **etapa 9 (`0.9.0`)**, o ensaio geral (até
> 16/10, etapa 7, `0.7.0`; até 15/10, etapa 6, `0.6.0`). Entraram antes dele duas etapas de usabilidade:
> **comparação Urna Velox × TSE** (etapa 7, `0.7.0`, [HANDOFF_0.7.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.7.0.md))
> e **previsão do matematicamente e do fim da apuração** (etapa 8, `0.8.0`,
> [HANDOFF_0.8.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.8.0.md)). Onde o texto abaixo diz "etapa 6" ou
> "etapa 7", `0.6.0` ou `0.7.0` referindo-se ao ensaio, leia "etapa 9" e `0.9.0`. Os itens do ensaio podem andar **em
> paralelo** com as etapas 7 e 8 (PATCH sobre a série corrente: `0.6.x`, `0.7.x` ou `0.8.x`), mas o `bump minor` →
> `0.9.0` só vem depois da `0.8.0`, e os ensaios completos que fecham a etapa **incluem as duas telas novas** (comparação
> ligada e previsão visível). A etapa 5 (`v0.5.0`) e a 6 (`v0.6.0`) já estão fechadas.
>
> **Renumeração de 15/10/2026.** A etapa 6 passou a ser a escalabilidade segura (federação), concluída na `v0.6.0`. O
> ensaio agora também cobre a federação: o critério A completo
> pendente da etapa 6 (1º turno de 2026 a 60× com 3 nós, plano em
> [HANDOFF_0.6.0.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.6.0.md), etapa A). Mapa completo no
> [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md).

*Preparado em 14/10/2026, no fim da sessão que levou a etapa 5 até a `0.4.13`. **Ponto de partida: a `0.4.x` mais
recente** (`git fetch` + `git log`), com a etapa 5 ainda aberta: falta a validação independente e o fechamento descritos
em [HANDOFF_0.4.10.md](/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.4.10.md) §3. **Feche a etapa 5 antes** (ou siga com
ela aberta só se o Luciano decidir). Quem executa esta etapa fica em PATCH (`0.5.1`, `0.5.2`… se a `0.5.0` já tiver
saído; senão `0.4.x`) e entrega um relatório; o `bump minor` para `0.6.0` só depois da validação independente descrita no
fim (ou por decisão explícita do Luciano). Para iniciar a sessão, cole este documento ou peça para lê-lo.*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` (Windows, Git, PowerShell) =
`/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`: scripts, cargo); dados = `/home/dev/uvreal`; ensaios da etapa 5 =
`/home/dev/uvreal/etapa5`; acervo de dados abertos = `/mnt/d/dados/tse-arquivos-urna` (= `D:\dados\tse-arquivos-urna`).
Ensaios desta etapa em `/home/dev/uvreal/etapa6` (novo). Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.9.0.md`.

## Contexto

Até a etapa 5 o Urna Velox tem: o motor (`urna-velox`), o catálogo pré-eleição (`uv-coletor preparar`), o serviço da
noite inteira (`uv-servico`, contra o site real ou o `uv-simulador`), a publicação por WebSocket/REST
([PROTOCOLO_WS.md](/mnt/d/git/lukeboh/urna-velox/docs/PROTOCOLO_WS.md)) e a página de resultados embutida no binário,
com marcos, curva da apuração e histórico do letreiro
([RELATORIO_0.5.0.md](/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.5.0.md),
[site/README.md](/mnt/d/git/lukeboh/urna-velox/site/README.md)). Cada peça foi medida e conferida com o oficial
separadamente; o replay controlável (`--controle-replay`, até 300×, linha do tempo com retotalização local) existe só
contra o simulador.

**Objetivo desta etapa:** provar que **tudo junto** aguenta a noite, de ponta a ponta, **sem intervenção**, e deixar a
operação do 2º turno de 2026 (25/10, pleito 3221) pronta: noites reais reproduzidas pelo WebSocket e pela página, falhas
provocadas, o atraso entre o BU disponível e o BU aplicado medido e explicado, runbook e binário. Fechar a etapa leva à
`1.0.0` (majoritários completos), que é o que vai para a noite de 25/10.

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes (versionamento, rede, verificação).
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — "Etapa 6" e "`1.0.0`".
3. `/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.4.10.md` — onde a etapa 5 parou, ferramentas de teste da última sessão.
4. `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.3.0.md`, `RELATORIO_0.4.0.md` e `RELATORIO_0.5.0.md` — critérios
   A, B, C, D1–D4, E1/E2 e os números que não podem piorar; o §7 do da etapa 5 tem o achado do atraso.
5. `/mnt/d/git/lukeboh/urna-velox/docs/OPERACAO.md` — a operação como está documentada hoje (base do runbook).
6. `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/PREFEITO_2024_2T.md` — o modelo da comparação "matematicamente
   eleito × totalização do TSE" (mediana de 26 min antes do TSE no 2º turno de 2024).
7. Código: `aquisicao/src/servico.rs` (laço do motor, `reconstruir`, `gravador`, `saude`), `aquisicao/src/coletor.rs`
   (ritmos de `cs.json`, limites), `aquisicao/src/simulador.rs`, `aquisicao/src/difusao.rs`,
   `aquisicao/scripts/simular_site.sh`.

## O que fazer

1. **Atraso disponibilização → BU aplicado** (ROADMAP etapa 6, achado da `0.4.9`). Nos ensaios contínuos a 60× da
   etapa 5 os eventos saíam em média 11 min da noite depois da disponibilização do BU que os decidiu (até 46 min no
   pico; Governador de SP decidido com BUs disponíveis até 19:21 e publicado às 20:07). Suspeita: os roteiros usavam
   `--intervalo-cs 30/aceleração`, que o serviço divide de novo pela aceleração, relendo os 28 `cs.json` (31 MB) sem
   parar. Fazer:
   * medir por seção a disponibilização no `cs.json` × a hora do site em que o BU foi aplicado (p50/p99/máx.) num
     ensaio a 1× de 1 h no pico (18:30–19:30 de 04/10) e na noite a 60× com o `--intervalo-cs` padrão;
   * se passar de um intervalo de `cs.json` + alguns segundos, achar o gargalo (leitura dos `cs.json`, fila de coleta,
     teto de req/s, motor) e corrigir;
   * pôr o atraso na `saude.json` (e no `estado` do WebSocket, por acréscimo) para ser olhado na noite.
2. **Ensaios de ponta a ponta pelo WebSocket e pela página**, com conferência oficial no fim:
   * 1º turno de 2026 a 60× (noite inteira) e um trecho a 1× (≥ 1 h no pico), com 1.000 clientes (`uv-carga`) + a
     página aberta em algumas abas;
   * 2º turno de 2024 a 10×, pelo `uv-simulador` com os BUs já baixados (**não** baixar BUs de novo do TSE);
   * no fim de cada um: tela × oficial (`site/testes/conferir.py`), marcos × diário (`conferir_marcos.py`), conferência
     do serviço (83/83 e 832/832 em 2026; 51/51 e 102/102 em 2024), nenhuma revogação.
3. **Simulação da noite com "matematicamente eleito" para os cargos de 2026**, comparada com a totalização do TSE, como
   em `PREFEITO_2024_2T.md` (quanto tempo antes do TSE cada circunscrição ficou decidida; mediana e pior caso).
4. **Falhas provocadas durante os ensaios** (o simulador tem parâmetros para quase todas): queda de rede e do site,
   rajada de 429 (pausa ≥ 10 min e retomada sozinha), `kill -9` e reinício do serviço (a página volta sozinha),
   BU reenviado idêntico e diferente, seção anulada depois de publicada. Nada de duplicado nem perdido; os números
   finais continuam iguais ao oficial.
5. **Dois ensaios completos seguidos sem intervenção e sem divergência** (o critério da etapa): o mesmo roteiro, duas
   vezes, do zero, sem ninguém tocar no serviço.
6. **Runbook da noite** (`/mnt/d/git/lukeboh/urna-velox/docs/RUNBOOK_NOITE.md`, novo): instalação a partir do binário,
   `candidatos.json` do 3221 (como regerar e conferir), comando exato do `uv-servico` para a noite real (sem
   `--controle-replay`), portas, o que olhar na `saude.json` e na página, o que fazer em cada alerta
   (`secoes_esperadas_incoerentes`, `destinacao_presumida`, pausa por 429, `replay_bu_ausente`…), como encerrar e
   conferir. Escrito para quem não acompanhou o desenvolvimento.
7. **Binário Linux pronto:** build release reproduzível (`cargo build --release` com o perfil do repositório), versão no
   `--version`, `sha256` publicado no runbook; a página embutida (`site/dist`) compilada antes.
8. **Cobertura de testes do motor** medida (ex.: `cargo llvm-cov`) e as partes críticas cobertas (leitor de BU,
   agregador, situação, matematicamente); registrar o número (a meta de ≥ 80 % é da `1.x`, aqui basta medir e cobrir o
   crítico).
9. **Documentação:** relatório `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.6.0.md` (o que foi feito, critérios,
   números medidos, riscos, como validar), ROADMAP marcado com a versão, sem concluir a etapa; CHANGELOG em cada
   commit.

## Critério (ROADMAP)
- Dois ensaios completos seguidos sem intervenção e sem divergência (2026 a 60× e 2024 a 10×), com as falhas
  provocadas no meio, conferência oficial idêntica e a página mostrando os mesmos números no fim.
- Atraso disponibilização → BU aplicado medido, explicado e, a 1×, dentro de um intervalo de `cs.json` + alguns
  segundos no p99 (ou o motivo documentado e aceito pelo Luciano).
- Nada medido piorou: BU → publicação com 0 clientes (B), BU → cliente p99 < 50 ms com 1.000 clientes (D1), CLS 0 na
  página, motor `--ignored` 13/13, serviço `--ignored` 9/9.
- Runbook e binário prontos; cobertura do motor registrada.

## Regras inegociáveis
- **Rede (TSE):** ≤ 100 req/s por órgão e por IP de saída, somando tudo; depois de **qualquer** 429, parar tudo
  ≥ 10 min; nada de vários IPs, proxies ou processos do mesmo órgão em paralelo. **Esta etapa roda contra o
  `uv-simulador`.** Uso do site real: só `uv-coletor preparar --pleito 3221` (≤ 5 req/s), **uma vez por dia até 24/10**.
  Qualquer outra coisa contra o site real, pergunte ao Luciano antes.
- **Nada contra o site real em 25/10 e 26/10** sem combinar (o CLAUDE.md pede acordo para coletar durante uma apuração).
- **O que está medido não pode piorar sem explicação** (critérios das etapas 3, 4 e 5).
- **Motor sem rede e sem `tokio`** (`cargo tree -p urna-velox -e normal`). Executáveis começam com `uv-`.
- **Versionamento** como no CLAUDE.md: `compilar_dev.sh` sem avisos, motor `--ignored` 13/13 e serviço `--ignored` 9/9
  antes dos commits que mexem no serviço; `bash /mnt/d/git/lukeboh/urna-velox/scripts/versao.sh bump patch` (sem
  resumo na linha de comando); CHANGELOG sem `PREENCHER`; `git add` (a `aquisicao/acervo/` é versionada desde a
  `0.4.13`); `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG` (gravado com a ferramenta de escrita e relido);
  `git push --follow-tags`; **não faça `bump minor`**. Identidade `Luciano Bohnert <luciano.bohnert@gmail.com>`.
  Outra sessão pode estar commitando: `git fetch` + `git log` antes de numerar.
- Termos: "**matematicamente eleito**" / "matematicamente 2º turno"; os dados dos candidatos são "**candidatos**".
- Documentos novos com **caminhos absolutos**.

## Ambiente
Windows + WSL2, Desktop Commander com PowerShell. Comandos com aspas, `$` ou `pkill`/`pgrep` vão em scripts
`/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh` (dentro de `wsl -e bash -c "..."` o `pkill -f`/`pgrep -f` acha o
próprio comando e o mata). Processos longos com `setsid nohup … &` dentro de um script; chamadas ao computador às vezes
passam de 60 s e o comando continua rodando: confira antes de repetir. Ferramentas prontas da etapa 5 (todas em
`/mnt/d/git/lukeboh/urna-velox/.git/`): `uv_tmp_e5_build.sh` + `uv_tmp_esperar_build.sh` (página + `compilar_dev`),
`uv_tmp_sim.sh [velocidade]` (simulação na porta 18080), `uv_tmp_verif0410.sh` (motor e serviço `--ignored`),
`uv_tmp_e5_ensaio.sh` (E1/E2), `uv_tmp_e5_base.sh` (B em rodadas alternadas), `uv_tmp_cv_cdp.mjs` e
`uv_tmp_video.mjs` (fotos, CLS e vídeos no Chrome). Navegador para testes: **Chrome na porta 9333** (instância própria,
sem conta; comando no HANDOFF_0.4.10 §4). A **porta 9222 é o Chrome do Luciano**: não usar sem ele
pedir. **Não rode medidas de desempenho do serviço com a página animando no mesmo computador** (divide CPU): meça em
rodadas separadas ou registre que estavam juntos.

## Pendências herdadas (para a noite de 25/10)
- **`cs.json` de TO** do 3221 dava 404: o `candidatos.json` do 2º turno de 2026 está incompleto
  (`/home/dev/uvreal/2026-2t/preparo`, gerado com `--ignorar-destinacao --cargos 1,3,11,25,26,27`; o serviço precisa de
  `--aceitar-incompleto`). Rerrodar o `preparar` uma vez por dia até 24/10 e registrar.
- **`s.ts` parcial em 9 UFs** para Presidente (alerta `secoes_esperadas_incoerentes`): a página mostra o aviso; o
  runbook explica.
- **Destinação presumida "Válido"** no 3221: conferir cassações/indeferimentos antes da noite.
- **Seções agregadas** (`nsp`) já ficam fora do total (`0.4.9`); conferir que o `candidatos.json` final do 3221 tem o
  campo `agregada`.
- Validação independente das etapas 3, 4 e 5 não feita (concluídas por decisão do Luciano ou ainda abertas): se alguém
  fizer, pode ser em paralelo; nada incompatível no protocolo v1.

## Fora do escopo
TLS e autenticação no binário; CDN/distribuição massiva e federação (2.0); apps nativos; proporcionais (3.0);
auditoria criptográfica, exportações e empacotamento para Windows/macOS/Docker (`1.x`); baixar BUs do TSE.

## Armadilhas conhecidas
- **`--intervalo-cs` já é dividido pela aceleração dentro do serviço:** não divida de novo no roteiro (é a suspeita do
  atraso da etapa 5).
- **Contas com `i64::MIN` como "nunca":** `x - i64::MIN` estoura e, em release, dá negativo sem pânico (foi o defeito
  da curva depois de voltar no tempo, `0.4.11`). Use `saturating_sub` ou `Option`.
- **Rajadas juntam versões:** gravador e difusão guardam só a versão mais nova de cada arquivo; quem precisa de todas
  (curva) recebe à parte (`amostrar_curva_superada`). Qualquer medida nova por publicação tem de considerar isso.
- **`--controle-replay` só contra o simulador local** (o serviço recusa outra base); na noite real ele não existe.
- **A 60× o `br-c0001` muda ~100 vezes por segundo**; a 1× poucas. Ensaio a 1× é o que mais se parece com a noite.
- **Cache de página do WSL cheio** depois de uma noite de ensaios deixa o p99 ruidoso (8–18 ms no B com o mesmo
  binário): compare versões em rodadas alternadas, de preferência com a máquina recém-iniciada.
- **O `n` dos eventos recomeça** depois de reinício do serviço; deduplique por conteúdo (protocolo §5).

## Validação (por outra sessão, independente)
Quem validar vai, sem confiar no relatório: ler o diff desde o ponto de partida; rodar `compilar_dev.sh`, testes e
`--ignored`; repetir um dos dois ensaios completos com as falhas provocadas e conferir com o oficial; medir o atraso
disponibilização → BU aplicado por conta própria; seguir o runbook do zero numa pasta limpa, com o binário publicado
(conferir o `sha256`); abrir a página durante o ensaio (CLS, números finais); conferir versionamento e a regra `uv-`.
Só então `bump minor` → `0.6.0`, e depois a `1.0.0` (majoritários completos) para a noite de 25/10.
