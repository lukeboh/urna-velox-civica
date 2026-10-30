# Handoff — Urna Velox: Etapa 3, operação automatizada (`0.3.0`, alvo 19/10/2026)

*Preparado em 12/10/2026 e publicado na `v0.2.1`, sobre a `v0.2.0` (etapa 2 concluída: tag `v0.2.0`, `origin/main` em `3ad4758`). Para iniciar a sessão, cole este documento ou peça para lê-lo. **Quem executa este handoff para na `0.2.x` e entrega um relatório; o `bump minor` para `0.3.0` só acontece depois da validação independente descrita no fim.***

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` no Windows (Git, PowerShell) = `/mnt/d/git/lukeboh/urna-velox` no WSL (scripts, cargo); dados = `/home/dev/uvreal` (WSL); acervo de dados abertos = `/mnt/d/dados/tse-arquivos-urna` (= `D:\dados\tse-arquivos-urna`). Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.3.0.md`.

## Contexto
O Urna Velox é um motor de totalização eleitoral em Rust (repo `lukeboh/urna-velox`, branch `main`). Hoje existem todas as peças, mas soltas:
- **Motor** (`/mnt/d/git/lukeboh/urna-velox/motor/`, crate `urna-velox`, binário `uv-motor`): leitor de BU, catálogo, estado denso, agregador, situação dos candidatos, **matematicamente eleito / matematicamente 2º turno** (`acompanhamento.rs`, `noite.rs`), publicação JSON (`resultado.rs`, formato em `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/FORMATO_RESULTADO.md`), conferência com os oficiais (`conferencia.rs`).
- **Candidatos pré-eleição** (etapa 2): `uv-coletor preparar` gera o `candidatos.json` (`/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/FORMATO_CANDIDATOS.md`); o do 2º turno de 2026 (pleito 3221) está em `/home/dev/uvreal/2026-2t/preparo/candidatos.json` — **ainda incompleto**: os `cs.json` de SE e TO davam 404 em 12/10.
- **Aquisição** (`/mnt/d/git/lukeboh/urna-velox/aquisicao/`, crate `uv-aquisicao`, binário `uv-coletor`): `coletor.rs` (HTTP/2, limitador GCRA com teto de 100 req/s, AIMD, disjuntor de 429, `If-None-Match`), `acompanhar` (relê os `cs.json` e baixa as seções novas), `baixar`, `replay` (injeta BUs no horário de chegada e publica), gravador `.uvpk` (`motor/src/pacote.rs`).

**O problema desta etapa:** na noite de 25/10 alguém precisaria encadear à mão `preparar`, `acompanhar`, `replay`/`noite` e a conferência, e nada disso retoma sozinho depois de uma queda. **Objetivo:** um único serviço que, a partir do `candidatos.json`, roda a noite inteira sem intervenção — acompanha o site, coleta dentro dos limites, totaliza em fluxo, calcula o matematicamente, publica, se recupera de falhas e confere com os oficiais no fim.

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes (versionamento, rede, verificação).
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — seções "Etapa 3" e "Metas de desempenho".
3. `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` — de 0.1.0 a 0.2.0.
4. `/mnt/d/git/lukeboh/urna-velox/docs/relatorios/VALIDACAO_0.2.0.md` — como a etapa anterior foi reprovada e corrigida (os mesmos critérios valem aqui: nada de teste tautológico, números medidos, não afirmados).
5. `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/AQUISICAO.md` §1.1, §2 e §5; `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/PREFEITO_2024_2T.md` (cenários "site" e "rede", ciclo de releitura do `cs.json`); `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/ARCHITECTURE_DECISIONS.md` ADR-003, 005, 006, 012, 014, 015, 016.
6. Código: `aquisicao/src/main.rs` (`Cmd::Acompanhar`, `iniciar_consumidor`, `relatar`), `aquisicao/src/coletor.rs`, `aquisicao/src/tse.rs` (`Aux::escolher_bu`, `ConfigSecoes::secoes`), `aquisicao/src/replay.rs`, `aquisicao/src/preparo.rs`; `motor/src/estado.rs` (`Estado::aplicar`, bitset `vistos`), `pacote.rs`, `acompanhamento.rs`, `noite.rs`, `resultado.rs` (`Publicador`, `gravar_atomico`), `conferencia.rs`, `metricas.rs`.
7. Testes: `/mnt/d/git/lukeboh/urna-velox/motor/tests/` (`comum/mod.rs`, `integracao.rs`, `matematica.rs`).

## Decisões já tomadas (mude só se o Luciano pedir)
- **Onde fica o serviço:** novo binário **`uv-servico`** no crate `uv-aquisicao` (`/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bin/uv-servico.rs` ou `src/servico/`), reaproveitando `coletor.rs` e `tse.rs`. O **motor continua sem rede e sem `tokio`** (ADR-012/014): o serviço conversa com ele por canal e threads nativas. O ROADMAP fala em `uv-motor servir`; atualize o texto para `uv-servico` no primeiro commit.
- **Site simulado para os critérios:** os BUs de 2024 2º turno e de 2026 1º turno já estão em `/home/dev/uvreal` e **não podem ser baixados de novo** do TSE. Os critérios rodam contra um **site simulado local**, novo binário **`uv-simulador`** (mesmo crate), que serve em `127.0.0.1` exatamente os caminhos do site real (`<base>/<ambiente>/<ciclo>/arquivo-urna/<pleito>/config/<uf>/<uf>-p<pleito>-cs.json`, `aux.json`, `-bu.dat`, `-u.json` dos oficiais), liberando cada seção no seu horário de disponibilização, com relógio acelerado. O serviço aponta para ele com `--base http://127.0.0.1:<porta>`: **o mesmo código que vai rodar na noite**, sem caminho especial.
- **Teto de taxa:** contra o TSE, teto de 100 req/s (padrão 95) e pausa ≥ 10 min depois de qualquer 429, sem exceção. Só quando a base é `127.0.0.1`/`localhost` o serviço aceita teto = 95 × aceleração e pausa de 429 escalada — e isso tem de estar no código (validação do host), não só na documentação.

## Regras inegociáveis
- **Rede (TSE):** no máximo **100 req/s** somando tudo; depois de **qualquer HTTP 429**, parar tudo ≥ 10 min (o IP de saída pode ser compartilhado); nada de vários IPs, proxies ou processos em paralelo. **Nesta etapa não baixe BUs do TSE.** O único uso do site real é o teste do item 7 (pleito 3221 antes da eleição: só `ele-c.json`, `cs.json` e `-u.json`, ≤ 5 req/s em média). Orçamento total de rede real da etapa: ~5 mil requisições; se precisar de mais, pare e pergunte ao Luciano.
- **Não rodar nada contra o site real em 25/10 e 26/10** sem combinar (noite e dia seguinte da eleição).
- **Separação:** motor sem rede; quem faz rede é o `uv-aquisicao`.
- **Todos os executáveis começam com `uv-`** (`uv-servico`, `uv-simulador`).
- **Sem regressão:** antes de cada commit, `bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` (sem avisos) e `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (13 testes, ~20 s); registre no CHANGELOG. Um número que mudar exige explicação no CHANGELOG antes de mudar o teste.
- **Versionamento a cada commit:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/versao.sh bump patch` (sem resumo na linha de comando), preencher a entrada do `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (o hook rejeita `PREENCHER`), `git add` (nunca `aquisicao/acervo/`), `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG`, `git push --follow-tags`. Commits `0.2.2`, `0.2.3`, … **Não faça `bump minor`.** Identidade git `Luciano Bohnert <luciano.bohnert@gmail.com>`. Mensagens `área: resumo`, em português. **Grave o `UV_COMMIT_MSG` com a ferramenta de escrita de arquivo e releia antes do commit** (na v0.1.5 um `Set-Content` falhou e o commit saiu com a mensagem anterior).
- Termos: "**matematicamente eleito**" / "matematicamente 2º turno"; os dados dos candidatos são "**candidatos**" (nunca "cadastro").
- Documentos novos com **caminhos absolutos**.

## Ambiente
- Windows + WSL2 `Ubuntu-24.04`, usuário `dev`, i7-14700 (28 threads), 15 GB no WSL. Comandos pelo Desktop Commander (`start_process`, PowerShell). **Aspas quebram no PowerShell:** escreva o comando num script `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh` (ou `.py`) e rode `wsl -d Ubuntu-24.04 -- bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh`. Cada chamada expira em ~60 s; para jobs longos (simulações da noite), rode em segundo plano com `nohup … > <log> 2>&1 &` e acompanhe o log. Para editar arquivos com acentos, scripts Python com `encoding="utf-8"` ou `edit_block`.
- Binários de desenvolvimento: `/home/dev/uvreal/target-dev/release/{uv-motor,uv-coletor,uv-servico,uv-simulador}`.
- Git pelo PowerShell (`cd D:\git\lukeboh\urna-velox; git …`).

### O que já existe localmente (não baixe de novo)
| conjunto | o quê | onde |
|---|---|---|
| 2024, 2º turno (pleito 453, eleição 620, 51 municípios em 20 UFs) | BUs `.uvpk`; `cs.json` com `da`/`ha` (republicados em 29/09/2026); métricas com `recebido_tse`, `disponivel_cs`, `situacao`; oficiais `-u.json`; `candidatos.json` pré-eleição; chegadas do cenário site | `/home/dev/uvreal/2024-2t/dados` (+ `dados/config/*-cs.json`), `/home/dev/uvreal/2024-2t/metricas_2t.csv`, `/home/dev/uvreal/2024-2t/oficial`, `/home/dev/uvreal/2024-2t/preparo/candidatos.json`, `/home/dev/uvreal/2024-2t/sim/chegadas_site.csv` |
| 2026, 1º turno (pleito 3220) | BUs, `cs.json`, métricas (`metricas_multi.csv`, `metricas_nacional.csv`, `metricas_retomada.csv`), 83 oficiais, `candidatos.json` (`--cargos 1,3,5`) | `/home/dev/uvreal/dados`, `/home/dev/uvreal/config`, `/home/dev/uvreal/metricas_*.csv`, `/home/dev/uvreal/oficial`, `/home/dev/uvreal/preparo_2026_1t/candidatos.json` |
| 2024, 1º turno (pleito 452) | idem, 5.569 municípios + 5 consultas | `/home/dev/uvreal/2024-1t/{dados,oficial,metricas_aux.csv,preparo/candidatos.json}` |
| 2026, 2º turno (pleito 3221) | `candidatos.json` incompleto (SE/TO), fontes em cache | `/home/dev/uvreal/2026-2t/preparo/` |
| exemplos reais de `aux.json` | formato para o simulador | `/home/dev/uvreal/2024-1t/descoberta/` (`…-aux.json`) |

## O que fazer

### 1. Estado do motor: aplicar, retirar e substituir um BU
Hoje `Estado::aplicar` (`/mnt/d/git/lukeboh/urna-velox/motor/src/estado.rs`) só soma, e o bitset `vistos` trata um segundo BU da mesma seção como duplicado. Na noite há dois casos que exigem mais:
- **Reenvio:** o `aux.json` da seção passa a apontar outro hash (BU novo). Se o conteúdo for idêntico (mesmo SHA-256), nada muda nos totais; se for diferente, o BU anterior sai e o novo entra, com **alerta** (ADR-006).
- **Anulação depois de totalizada:** o BU da seção vira "Excluído" no `aux.json` (caso real: MA 07315/0023/0354, 2024) — os votos passam a contar como nulos, como no oficial (hoje isso só funciona se a anulação for conhecida antes, via métricas).

Implemente retirar/substituir no motor (ex.: o mesmo agregador com sinal negativo, `fetch_sub`, mantendo `aptos` e `secoes` coerentes) e guarde por seção qual BU está valendo (hash). Testes de unidade com BUs das fixtures: aplicar → retirar volta exatamente ao estado zero (todos os contadores, inclusive `aptos`, `secoes` e o checksum); substituir A por B = aplicar só B; anular depois = aplicar com a seção anulada desde o início. O acompanhamento do matematicamente precisa reavaliar a circunscrição afetada; uma declaração revogada por causa de anulação/reenvio vira **alerta** com o motivo, não erro silencioso.

### 2. Site simulado (`uv-simulador`)
Servidor HTTP local (pode usar `axum` ou `hyper` no crate `uv-aquisicao`; nunca dentro do motor) que lê os dados locais de um conjunto e serve, com relógio acelerado (`--acelerar N`, `--inicio "dd/mm/aaaa hh:mm:ss"`):
- `ele-c.json` e `cs.json` de cada UF **reconstruídos no instante simulado**: uma seção só aparece com `da`/`ha` depois do seu horário de disponibilização (`disponivel_cs` das métricas; sem ele, `recebido_tse`); `ETag` muda quando o conteúdo muda e `If-None-Match` devolve 304.
- `aux.json` de cada seção (formato real, ver os exemplos em `/home/dev/uvreal/2024-1t/descoberta/`; o hash do diretório sai do SHA-256 do `.vsc` guardado no registro do `.uvpk`, `tse::hash_dir_do_sha256`), 404 antes da disponibilização; `-bu.dat` com os bytes do `.uvpk`; seções "Não instalada" e "Excluído" como nas métricas.
- os `-u.json` oficiais (para a conferência do fim), só depois do último BU.
- **Injeção de falhas** por parâmetro: rajada de HTTP 429 num intervalo; latência e erros 5xx aleatórios (semente fixa); reenvio de uma seção com BU idêntico (hash novo); anulação de uma seção depois de publicada; queda total do servidor por N segundos.
- Um registro (`simulador.log`) de cada requisição recebida, para provar taxas e contagens.

### 3. Serviço (`uv-servico`)
`uv-servico --pleito <P> --candidatos <candidatos.json> --dados <pasta> --saida <pasta> [--base …] [--ambiente oficial|simulado] [--ciclo …] [--intervalo-cs 30] [--rps 95]`:
- **Partida:** carrega o `candidatos.json` (recusa arquivo com `incompleto` não vazio, salvo `--aceitar-incompleto`, com alerta), monta catálogo e candidatos, **recarrega o que já está nos `.uvpk` da pasta `--dados`** (retomada) e o diário de seções (abaixo).
- **Acompanhamento:** relê os `cs.json` de todas as UFs do pleito a cada `--intervalo-cs` com `If-None-Match`; seção nova ou com disponibilização alterada → fila de coleta. **Seção sem disponibilização (`da`/`ha` vazios) não gera requisição** (antes da eleição o 3221 tem 494 mil seções listadas e nenhuma publicada: não pode virar 494 mil 404).
- **Coleta:** `aux.json` → escolhe o BU (`Aux::escolher_bu`) → `-bu.dat`; mesmo limitador, AIMD e disjuntor do `coletor.rs`; teto 95 req/s contra o TSE; **depois de um 429, pausa global ≥ 10 min** (contador e horário de retomada visíveis na saúde). Seção com hash igual ao já aplicado não é baixada de novo.
- **Totalização em fluxo** com o motor (item 1), **acompanhamento do matematicamente** a cada lote e **publicação** com o `Publicador` em `<saida>/publicacao/` (gravação atômica; `seq` = BUs aplicados; cadência configurável, padrão ≤ 1 s) e eventos (`<saida>/eventos.csv`, mesmo formato do `uv-motor noite`).
- **Persistência e retomada:** BUs em `.uvpk` append-only (nunca reescritos); **diário de seções** append-only (`<saida>/secoes.log`: chave, hash, SHA-256, situação, disponibilização, hora de aplicação, ação = aplicado/substituído/anulado/retirado) que decide, na retomada, qual BU de cada seção está valendo. `kill -9` em qualquer instante e reinício: nenhum BU perdido, nenhum contado duas vezes, mesmo resultado final; o `acompanhar` atual marca as seções já coletadas com disponibilização vazia e nunca mais percebe reenvio — não repita isso.
- **Fim da apuração:** quando todas as seções do catálogo estiverem resolvidas (BU aplicado, não instalada ou anulada) ou sob comando (arquivo-sinal `<saida>/CONFERIR` ou `--conferir-a-cada <min>`): publicação encerrada (`encerrada: true`, situação final) e **conferência automática** — baixa os `-u.json` das abrangências do `candidatos.json` (≤ 5 req/s; no simulador, locais), confere votos, `vnt`, comparecimento, seções e situação com `conferencia.rs`, grava `<saida>/conferencia.md` e `.json`. Divergência = alerta, não queda do serviço.
- **Saúde e métricas:** `<saida>/saude.json` regravado a cada ≤ 5 s (seções publicadas/coletadas/aplicadas por UF, fila, req/s atual e média, latências p50/p99 de `aux` e BU, 429 e pausa, último BU e horário, BU → publicação em ms, alertas por tipo, versão do binário, uso de memória) e log estruturado em linhas JSON (`<saida>/servico.log`), sem um registro por BU no nível padrão. O HTTP de saúde é da etapa 4; aqui tudo em arquivo.
- **Parada limpa:** `Ctrl+C`/`SIGTERM` termina o lote, grava publicação, diário e saúde, e sai com código 0.

### 4. Seções anuladas, não instaladas e reenvios como no oficial
A situação vem do `aux.json` de cada seção, no momento da coleta e nas releituras: "Totalizado" → aplica; "Não instalada" → conta em `s.st` sem BU; "Excluído"/anulada → votos como nulos (inclusive depois de já aplicada, item 1); reenvio → substitui (item 1). Descubra e documente no relatório quais valores de `st` (seção e hash) aparecem nos dados locais de 2024 e 2026 e como cada um é tratado; um valor desconhecido vira alerta e a seção fica pendente.

### 5. Critérios com o site simulado (o coração da etapa)
Rode e registre no relatório (comando exato, duração, contagens do `simulador.log` e da `saude.json`, conferência):
- **A — 2º turno de 2024, do início ao fim**, simulador a 10× (teto local 950 req/s): 51/51 arquivos idênticos e 102/102 situações, conferidos pela **conferência automática do próprio serviço** e também por `uv-motor conferir-publicacao`; 0 HTTP 429 provocado pelo serviço; 0 duplicados; nenhuma revogação; mediana do matematicamente comparável à do cenário site de `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/PREFEITO_2024_2T.md` (explique a diferença).
- **B — 1º turno de 2026**, simulador a 60×, `candidatos.json` de `/home/dev/uvreal/preparo_2026_1t/`: 83/83 e 832/832; 3 eventos de Presidente; 0 revogações; BU → publicação < 1 ms no p50 (meta do ROADMAP) medido no serviço.
- **C — Falhas** (pode ser no 2024 2º turno a 60×): (1) `kill -9` do serviço três vezes em pontos diferentes, reinício e resultado final idêntico ao de A; (2) rajada de 429 → pausa (escalada no simulador) e retomada sem perda, 0 requisições durante a pausa (provado pelo `simulador.log`); (3) reenvio idêntico → totais inalterados, alerta; (4) anulação depois de publicada → votos da seção viram nulos, publicação corrigida, alerta, e a conferência continua batendo com a seção anulada do oficial quando ela existir; (5) queda do simulador por 2 min → retomada sozinha.
- Transforme A (reduzido a algumas UFs, se preciso, para caber em ~1 min) e os casos de C em **testes automatizados** `--ignored` (simulador e serviço no mesmo processo de teste ou como processos filhos com porta aleatória). Os testes de unidade do item 1 rodam sempre.

### 6. Desempenho e recursos
Meça no B: CPU média do serviço, memória residente (meta do motor < 150 MB; registre o total do serviço), BU → publicação (p50/p99) e quanto o matematicamente e a gravação dos JSON custam por lote. Se a publicação de todas as abrangências a cada lote for o gargalo (2024 1º turno tem 5.574 arquivos), publique só as abrangências que mudaram no lote.

### 7. Teste real, sem BUs: pleito 3221 antes da eleição
Com o binário final, rode o serviço contra o **site real** (`--pleito 3221`, `/home/dev/uvreal/2026-2t/preparo/candidatos.json`, `--aceitar-incompleto` enquanto SE/TO faltarem) por ≥ 30 min com `--intervalo-cs 60`: só `cs.json` (maioria 304) e nenhum `aux.json` (nenhuma seção publicada), 0 HTTP 429, `saude.json` coerente, publicação a frio idêntica à do `uv-motor totalizar --candidatos` (43 arquivos, "apurando"). Antes, rode `uv-coletor preparar --pleito 3221` de novo (≤ 5 req/s): se os `cs.json` de SE e TO já existirem, o `candidatos.json` fica completo — registre.

### 8. Documentação
- `/mnt/d/git/lukeboh/urna-velox/docs/operacao/OPERACAO.md` (novo): como preparar e rodar o serviço na noite (comandos completos para o 3221, pastas, o que olhar na `saude.json`, o que fazer em cada alerta, como retomar, como forçar a conferência). É o embrião do runbook da etapa 6.
- `ARCHITECTURE.md` (seção do serviço e do simulador; fluxo cs → aux → BU → motor → publicação), `docs/arquitetura/AQUISICAO.md` (modo serviço), ADR novo para o simulador e para retirar/substituir BU.

## Entregáveis
- Código (motor: item 1; `uv-aquisicao`: `uv-servico`, `uv-simulador`), testes, `docs/operacao/OPERACAO.md`, documentação atualizada, CHANGELOG a cada commit, ROADMAP com os itens da etapa 3 marcados com a versão (sem marcar a etapa como concluída) e o critério reescrito para o site simulado + teste real do 3221.
- **Relatório final** `/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.3.0.md`: o que foi feito; os critérios A, B, C e o teste real com comandos exatos e números medidos; desempenho; valores de `st` encontrados e tratamento; riscos para a noite de 25/10 e o que ficou pendente. Termine com a lista de commits/tags.

## Fora do escopo
HTTP/WebSocket e REST (etapa 4), site (etapa 5), ensaio geral e runbook final (etapa 6), proporcionais (2.0), verificação do `.vsc` e assinaturas (1.x), baixar BUs do TSE.

## Armadilhas conhecidas
- `cs.json` muda no CDN a cada ~48 s na noite; o `If-None-Match` evita baixar de novo, mas cada releitura conta no orçamento de req/s (28 UFs por ciclo).
- Milhares de 404 seguidos provocam 429 mesmo abaixo de 100 req/s (por isso a regra de não pedir seção sem disponibilização).
- O último BU de um município pode aparecer no `cs.json` minutos depois do recebimento no TSE (Santarém/PA, 2024).
- Hora: `aux.json`/`cs.json` em Brasília; `dt`/`ht` dos oficiais na hora local da UF (`noite::fuso`).
- O BU traz todas as eleições do pleito (no 3221: Presidente + Governador em 5 UFs + suplementares + consultas); cargos municipais têm candidatos por município.
- Exterior (`zz`) só vota para Presidente; seções não instaladas não têm BU mas contam em `s.st`.
- `cs.json` de 2024 foi republicado em 29/09/2026: a disponibilização dele não é a da noite (915 seções no 2º turno). O simulador deve usar as métricas da coleta como fonte do horário.
- Destinação presumida "Válido" no 3221 (os `-u.json` pré-apuração não trazem `dvt`): o serviço deve avisar na partida quando `destinacao_presumida` for verdadeiro.
- Disco: os `.uvpk` de uma eleição geral têm ~1 GB; verifique espaço na partida e alerte.

## Validação (será feita por outra sessão, independente)
Quem validar vai, sem confiar no relatório: ler o diff desde `v0.2.0`; rodar `compilar_dev.sh`, os testes normais e os `--ignored`; rodar de novo os critérios A e B com o simulador e comparar com os números do relatório; repetir pelo menos dois casos de C (`kill -9` e anulação depois de publicada); conferir no `simulador.log` as taxas, o comportamento depois do 429 e a ausência de requisições para seções sem disponibilização; conferir que o teto de 100 req/s e a pausa de 10 min não podem ser afrouxados contra um host que não seja local; rodar o teste real do 3221 por alguns minutos; conferir versionamento (`scripts/versao.sh verificar`, tags, CHANGELOG) e a regra `uv-`. Só então sobe `bump minor` → `0.3.0`.
