# Aquisição de BUs — `uv-coletor`

Fase de **aquisição** do Urna Velox: recebe ou recupera os Boletins de Urna (BUs) **brutos** publicados pelo TSE e os entrega ao núcleo de totalização. Arquitetura, decisões e medições: [docs/AQUISICAO.md](../docs/AQUISICAO.md) (ADR-013 a ADR-018 em [docs/ARCHITECTURE_DECISIONS.md](../docs/ARCHITECTURE_DECISIONS.md)). Termos técnicos: [docs/GLOSSARIO.md](../docs/GLOSSARIO.md).

```text
resultados.tse.jus.br ──▶ uv-coletor ──▶ pacotes .uvpk (um por UF)  ──▶ núcleo (lote)
  cs.json → aux.json → bu.dat     └────▶ canal em memória ──────────▶ núcleo (em fluxo)
cdn.tse.jus.br (zip dados abertos) ┘
```

## Como repetir o teste real

Um comando compila, coleta BUs de verdade, totaliza, confere com um decodificador independente e gera o relatório com gráficos.

**Linux / WSL2**

```bash
./aquisicao/teste-real.sh            # completo: ~35 min, quase todo esperando a rede
./aquisicao/teste-real.sh --rapido   # curto: ~3 min
UV_NACIONAL=1 ./aquisicao/teste-real.sh   # coleta TODAS as 517 mil seções (~9 GB, várias horas)
```

**Windows (PowerShell, usa o WSL2)**

```powershell
.\aquisicao\teste-real.ps1
.\aquisicao\teste-real.ps1 -Rapido
.\aquisicao\teste-real.ps1 -Distro Ubuntu-24.04
```

Resultados em **`aquisicao/results/RELATORIO.md`**, com gráficos em `aquisicao/results/graficos/`.

> **Limite de uso: no máximo 100 requisições/s.** O `uv-coletor` impõe esse teto em código (`TETO_RPS`, sem rajadas). Acima de ~200 req/s a borda do CDN (Akamai) responde **HTTP 429** e põe o IP de saída numa "caixa de penalidade" de ~8 minutos; em rede corporativa esse IP é compartilhado por muita gente (NAT).

### Pré-requisitos

| Item | Se faltar |
|---|---|
| Linux ou WSL2, `curl`, compilador C | `sudo apt install curl build-essential` |
| Python 3 + venv | `sudo apt install python3 python3-venv` |
| Rust estável | instalado automaticamente em `~/.cargo` pelo `setup.sh` |
| Acesso a `resultados.tse.jus.br` e `cdn.tse.jus.br` | — |
| Disco | ~1 GB em `~/uvreal` (≈ 9 GB com `UV_NACIONAL=1`) |

### Etapas

| Etapa | Script | O que faz |
|---|---|---|
| 1 | `scripts/setup.sh` | compila o `uv-coletor` e roda os testes de unidade; cria o Python com `asn1tools` e `matplotlib` |
| 2 | `scripts/descobrir.sh` | lista pleitos/eleições (`ele-c.json`) e as seções de todas as UFs (`cs.json`) |
| 3 | `scripts/varredura.sh` | vazão e latência × concorrência e HTTP/1.1 × HTTP/2, sempre com teto de 100 req/s |
| 4 | `scripts/coleta_uf.sh` | UF de referência (DF) completa: API, zip só com os BUs (HTTP Range), zip inteiro, acompanhamento por `ETag` e coleta com totalização em fluxo |
| 5 | `scripts/coleta_multi.sh` | coleta sustentada de várias UFs com controle adaptativo (ou todas, com `UV_NACIONAL=1`); registra as requisições iniciadas por segundo |
| 6 | `scripts/totalizar.sh` | monta o catálogo e totaliza os BUs reais por nº de threads (mesma métrica do `bench/`) |
| 7 | `scripts/conferir.sh` | decodifica os mesmos BUs com `asn1tools` + `spec/bu.asn1` oficial e compara total a total |
| 8 | `scripts/relatorio.py` | `RELATORIO.md` + gráficos |

### Variáveis de ambiente

| Variável | Padrão | Efeito |
|---|---|---|
| `UV_WORK` | `~/uvreal` | binário, pacotes `.uvpk`, catálogo, zip e CSVs grandes (fora do repositório) |
| `UV_OUT` | `aquisicao/results` | resultados |
| `UV_CICLO` / `UV_PLEITO` | `ele2026` / `3220` | pleito a coletar (3220 = 1º turno de 2026) |
| `UV_UF_REF` | `df` | UF da coleta completa e da conferência |
| `UV_UFS_VARREDURA` | `sp,mg,ba,rj` | UFs sorteadas na varredura |
| `UV_UFS_EXTRA` | `ac,al,ap,pb,rn,rr,se,to` | UFs da coleta sustentada (42,9 mil seções) |
| `UV_CONC` | `1 4 16 32 64` | concorrências da varredura |
| `UV_RPS_MAX` | `100` | teto de requisições/s (o `uv-coletor` nunca passa de 100) |
| `UV_NACIONAL` | `0` | `1` = coleta sustentada de todas as UFs |
| `UV_RAPIDO` | `0` | `1` = execução curta |

## Totalização completa e conferência com a divulgação oficial

Scripts para coletar todos os BUs do pleito, totalizar e comparar com os arquivos oficiais de divulgação do TSE (Presidente, Governador e Senador). `uv-coletor totalizar --cargos 5` totaliza só os cargos pedidos (os demais cargos do BU são pulados sem alarme); `uv-coletor replay --cargos 1,3,5` escolhe os cargos publicados a cada ciclo.

Desde a v0.0.27 a totalização, a situação dos candidatos e a conferência também estão no motor (`motor/`, crate `urna-velox`, binário `uv-motor`): `uv-motor conferir --dados … --oficial … --metricas …` faz as conferências sem Python, e `uv-motor noite` mede quando cada circunscrição fica matematicamente decidida. O replay publica no formato do motor ([docs/FORMATO_RESULTADO.md](../docs/FORMATO_RESULTADO.md)); com `--oficial <pasta dos -u.json>` a publicação traz o "matematicamente eleito / 2º turno" a cada ciclo (eventos em `replay_eventos.csv`) e, no fim, a situação final, que `uv-motor conferir-publicacao --publicacao <saída>/publicacao --oficial …` confere com o oficial. `uv-coletor filtrar` recorta pacotes por município ou seção (fixtures dos testes).

| Script | O que faz |
|---|---|
| `scripts/coleta_nacional.sh` | coleta pela API todas as UFs (inclui `zz`), retomando o que já está nos `.uvpk`; teto `UV_RPS` = 95 req/s, adaptativo a partir de 40 |
| `scripts/baixar_oficial.sh` | baixa, a 1 req/s, os resultados oficiais por abrangência: Presidente (br, 27 UFs, zz), Governador e Senador (27 UFs); `UV_CARGOS="1 3 5"`; pula o que já existe |
| `scripts/totalizar_comparar.sh <dados> <saída>` | catálogo → `uv-coletor totalizar` → `comparar_oficial.py`; gera `comparacao_oficial.md` |
| `scripts/comparar_oficial.py` | confere, por abrangência: votos de cada candidato (`vap`), brancos (`vb`), nulos (`vn`), nulos técnicos (`vnt`), comparecimento (`e.c`), nº de BUs × seções totalizadas (`s.st`) e a situação de cada candidato (`st`: eleito / 2º turno / não eleito, calculada com os votos do Urna Velox); `--cargos 1,3,5` |
| `scripts/totalizar_acervo.sh [raiz] [ano] [turno]` | mesma conferência a partir do acervo local de dados abertos (`bus/<ano>/<turno>t/<UF>.zip`), sem rede |
| `scripts/totalizar_subconjunto.sh "ac,df"` | mesma conferência em algumas UFs, sobre uma cópia dos `.uvpk` (serve com a coleta em andamento) |
| `scripts/sonda_oficial.sh` | sonda os endereços dos arquivos oficiais |
| `scripts/conferir_senador.sh [resultado]` | baixa (se faltar) e confere Presidente, Governador e Senador, inclusive a situação de cada candidato |
| `scripts/desempenho_cargo.sh ["5" "1" …]` | totalização por cargo (`totalizar --cargos`), 1 e 28 threads, com conferência oficial de cada variante |
| `scripts/replay_senador.sh` | replay da noite publicando só Senador (0× e 60×) e Presidente + Governador + Senador (60×) |
| `scripts/baixar_oficial_municipal.py --ciclo ele2024 --eleicao 619 --cargo 11 --dest …` | arquivos oficiais **por município** (Prefeito, Vereador, consultas), teto de 15–20 req/s, para no primeiro 429, pula o que já existe |
| `scripts/municipal_acervo.sh 2024 1 452 ~/uvreal/2024-1t 619:11,633:55,…` | eleição municipal a partir do acervo local (sem rede): importa os zips originais (`bu` e `busa`), catálogo, totalização e conferência por município |
| `scripts/comparar_municipal.py` | confere por município votos, brancos, nulos, nulos técnicos, comparecimento, seções e a situação de Prefeito (2º turno acima de 200 mil eleitores; ninguém eleito quando o mais votado foi anulado ou os anulados passam de 50 %) e das consultas |
| `scripts/desempenho_municipal.sh <trabalho> <eleição:cargo,…> "11" "todos"` | desempenho por cargo numa eleição municipal, com conferência por município |
| `scripts/simular_noite_municipal.py` | simulação da noite BU a BU (site público a ≤ 95 req/s ou sem limite de taxa): instante em que cada município fica **matematicamente eleito** e em que termina, contra a hora de totalização do TSE (convertida da hora local) |
| `scripts/chegadas_cs.py` | horários de disponibilização do `cs.json` no formato das métricas (alternativa ao `aux.json` quando não há republicação) |
| `scripts/progresso.py [--vigiar 60]` | percentual da coleta por UF, sem rede (BUs nos `.uvpk` × seções totalizadas no oficial) |
| `scripts/erros_coleta.py <metricas.csv>` | erros da coleta por tipo e por UF |
| `scripts/retomar_depois.sh HH:MM rótulo` | espera a quarentena depois de um 429 e retoma pulando as seções sem BU conhecidas |
| `scripts/fechar_nacional.sh` | `totalizar_comparar.sh` nos dados nacionais, com resumo curto |
| `scripts/ver_aux.sh uf mun zona secao` | mostra o aux.json de uma seção (1 requisição) |
| `scripts/compilar_dev.sh` | compila em `~/uvreal/target-dev`, sem tocar no binário de uma coleta em andamento |
| `scripts/replay_noite.sh`, `simular_aquisicao.py`, `varrer_teto_aquisicao.sh` | benchmark "noite da eleição" (replay e simulação da aquisição) |

**Retomada e HTTP 429.** O coletor tem disjuntor (`--max-429 N`, padrão 20): ao somar N respostas 429 ele para a coleta inteira, e a retomada pula o que já está nos pacotes. Numa retomada, passe as métricas anteriores em `--pular-sem-bu` (ou `UV_PULAR_SEM_BU` no `coleta_nacional.sh`): pedir de novo milhares de seções sem BU (aux.json 404) seguidas fez a borda responder 429 já a ~70 req/s.

**BU do Sistema de Apuração.** Seções apuradas fora da urna têm `…-busa.dat` (tipo `busa`) em vez de `…-bu.dat`; o coletor e o leitor de zip aceitam os dois (31 seções no 1º turno de 2026).

Arquivo oficial em 2026: `resultados.tse.jus.br/oficial/ele2026/<eleição>/dados/<abr>/<abr>-c<cargo>-e<eleição>-u.json` (6257/cargo 1 = Presidente; 6259/cargo 3 = Governador). O caminho `dados-simplificados/…-r.json` de 2022/2024 responde 404.

**Nulos técnicos.** Um voto digitado em número que não é de candidato apto (ex.: nº 28 para Presidente em 2026) vem no BU como voto **nominal** nesse número; na divulgação, o TSE o soma em `vnt` (nulos técnicos), separado de `vn`. O comparador confere `vnt` com a soma desses números. Candidatos "anulados sub judice" continuam na lista oficial com `vap` próprio.

## Uso direto do `uv-coletor`

```bash
B=~/uvreal/target/release/uv-coletor
$B eleicoes                                   # pleitos e eleições publicados
$B secoes --ufs todas                         # seções por UF (517.179 no 1º turno de 2026)
$B baixar --ufs df --dados ~/uvreal/dados     # DF inteiro em ~/uvreal/dados/df.uvpk (retoma se interrompido)
$B baixar --ufs ac,rr -c 128 --adaptativo --rps-inicial 40 --rps 150 --metricas m.csv
$B acompanhar --ufs df --intervalo 30 --rodadas 0   # noite da eleição: relê cs.json (If-None-Match) e baixa o que mudou
$B zip --uf df --modo faixas                  # dados abertos: só os BUs do zip, por HTTP Range
$B catalogo --dados ~/uvreal/dados            # municípios, seções e candidatos → catalogo.txt
$B totalizar --dados ~/uvreal/dados --threads "1 4 8" --totais totais.csv
$B baixar --ufs df --sem-gravar --totalizar --catalogo ~/uvreal/dados/catalogo.txt   # coleta + totalização em fluxo
$B baixar --so-aux --ciclo ele2024 --pleito 452 --ufs ac,al,am --dados x --metricas m.csv   # só os aux.json (recebimento), sem BU
$B totalizar --dados d --anuladas m.csv --totais-municipio tm.csv   # seções anuladas contam como nulos; totais por município
$B inspecionar o03220df9701200150300-bu.dat   # imprime um BU
$B exportar --pacote ~/uvreal/dados/df.uvpk --saida bus/ --n 20
```

Opções de rede (em `baixar`, `zip`, `acompanhar`): `-c/--concorrencia` (requisições em andamento), `--conexoes` (clientes HTTP/2 independentes), `--rps` (teto de requisições/s), `--adaptativo` + `--rps-inicial` (AIMD), `--http1`, `--tentativas`, `--timeout`. Saída: `--dados`, `--sem-gravar`, `--totalizar`, `--catalogo`, `--threads-totalizacao`, `--metricas` (CSV por seção), `--resumo` (CSV por execução), `--rotulo`.

## Estrutura

| Caminho | Conteúdo |
|---|---|
| `src/tse.rs` | endereços e formatos JSON do site de resultados (`ele-c`, `cs`, `aux`), escolha do hash, nomes de arquivo |
| `src/coletor.rs` | motor HTTP: clientes HTTP/2, limitador de taxa (fixo ou AIMD), novas tentativas, coleta concorrente, métricas |
| `src/zip.rs` | dados abertos: diretório central por HTTP Range (com zip64), faixas por BU, download completo |
| `src/pacote.rs` | formato `.uvpk` (escrita só de acréscimo, retomada, leitura por `mmap`) |
| `src/bu.rs` | leitor BER verificado do BU real (envelope + `EntidadeBoletimUrna`) |
| `src/totaliza.rs` | catálogo, estado denso (`AtomicU32`), agregador, consolidação por UF, CSV de totais |
| `src/main.rs` | CLI `uv-coletor` |
| `spec/bu.asn1` | especificação oficial do BU (pacote "formato-arquivos-de-bu-rdv-e-assinatura-digital" do TSE, eleições 2026) |
| `scripts/` | etapas do teste real |
| `results/` | resultados da execução de referência |

## Formato `.uvpk`

```text
arquivo : "UVPK" 0x01 0 0 0
registro: tamanho_bu:u32 pleito:u32 municipio:u32 zona:u16 secao:u16 uf:[u8;2] situacao:u8 origem:u8 sha256_vsc:[u8;32] bu:[u8]
```

Little-endian, cabeçalho de 52 bytes por registro, BU exatamente como veio do TSE. `situacao` = 1 se o aux.json marcava "Totalizado"; `origem` = 1 (API) ou 2 (zip de dados abertos); `sha256_vsc` é o hash do diretório do BU (SHA-256 do arquivo de assinaturas `.vsc`), para auditoria posterior. Um registro incompleto no fim do arquivo é descartado ao reabrir.

## Formato de `results/coleta_resumo.csv`

`modo, rotulo, ufs, secoes, ok, erros, concorrencia, conexoes, http, rps_limite, segundos, secoes_por_s, req_por_s, mb, mb_por_s, lat_aux_p50_ms, lat_bu_p50_ms, lat_p95_ms, lat_p99_ms, tentativas_extras, http429, lag_totalizacao_ms, adaptativo, rps_final, data`

* `erros` inclui as seções sem BU publicado (aux.json 404: seções agregadas ou sem votação), ~1–3 % das listadas no `cs.json`.
* latências por requisição; uma seção = 2 requisições (aux.json + bu.dat).
* `lag_totalizacao_ms`: na coleta em fluxo, quanto a totalização terminou depois do último BU recebido.
* `max_req_em_1s`: maior número de requisições iniciadas num mesmo segundo (precisa ser ≤ 100).
