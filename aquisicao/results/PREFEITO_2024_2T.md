# Prefeito — 2º turno de 2024: coleta pela API, conferência e simulação da noite

Execução de 12/10/2026 (`v0.0.19`), i7-14700, WSL2. Eleição de 27/10/2024: pleito 453, eleição 620, **51 municípios em 20 UFs**. Desta vez tudo veio pela API do site de resultados (`aux.json` + BU), como na noite da eleição, para medir os tempos da aquisição.

## 1. Coleta e conferência

| item | resultado |
|---|---|
| coleta pela API (`coleta_nacional.sh`, `UV_PLEITO=453`) | **97.392 BUs** de 98.863 seções (1.471 com aux.json 404) em **34 min 44 s**; 46,7 seções/s, 94,2 req/s, no máximo 95 num mesmo segundo, **0 HTTP 429**; latência p50 do aux.json 486 ms, do BU 481 ms |
| arquivos oficiais por município (`baixar_oficial_municipal.py`) | 51, em 5 s |
| conferência por município (`comparar_municipal.py --eleicoes 620:11`) | **51 de 51 idênticos** (votos de cada candidato, brancos, nulos, nulos técnicos, comparecimento, seções) e **102 de 102 situações** idênticas ([comparacao_municipal_2024_2t.md](comparacao_municipal_2024_2t.md)) |
| totalização (BUs em memória) | 0,011 s com 28 threads, 0,022 s numa thread (0,22 µs por BU: só um cargo no BU) |
| retotalizados (data de totalização posterior a 27/10/2024) | **2**, só contados: SP 66192 (13/12/2024) e SP 66893 (14/11/2024) |

## 2. Simulação da noite: quanto antes o Urna Velox define cada município

`scripts/simular_noite_municipal.py`, BU a BU, com os dados reais desta coleta:

* **Disponibilidade no site:** horário de disponibilização de cada seção no `cs.json` (`da`/`ha`), preservado em 96.477 seções (915 republicadas em 29/09/2026 recebem recebimento + atraso mediano, que é 0 s).
* **Recebimento no TSE:** `dr`/`hr` do `aux.json`. Em 25.839 seções ele é **posterior** à disponibilização (lotes recarregados; ex.: ES 56995, zona 53, todo marcado 20:12:24 e publicado às 17:45): vale a disponibilização.
* **Cenário site** (observador externo): o coletor relê os 20 `cs.json` a cada 30 s, baixa as seções novas em fila com teto de 95 req/s (2 por seção) e cada seção leva as latências **medidas nesta coleta**.
* **Cenário rede** (Urna Velox recebendo os BUs junto com o TSE): cada BU entra no instante do recebimento.
* **Latência do Urna Velox** (totalização + publicação do município): medida com `uv-coletor replay --cargos 11` alimentado com os horários simulados, a 60×: p50 0,65 ms, **p99 1,73 ms** (usado na simulação), máx. 10,4 ms; os totais do replay conferem com o oficial (51/51).
* **Matematicamente eleito:** primeiro instante em que a vantagem do 1º colocado sobre o 2º supera o eleitorado ainda não apurado (`e.te` do arquivo oficial menos os aptos dos BUs já totalizados).
* **TSE:** data/hora de totalização do arquivo oficial (`dt`/`ht`), que está na **hora local** da UF; convertida para a de Brasília (AM, MS, MT e RO: +1 h).

Em todos os 51 municípios o candidato matematicamente eleito pelo Urna Velox é o eleito oficial. Antecedência em relação à totalização do TSE (49 municípios; os 2 retotalizados ficam fora):

| cenário | marco | mínimo | **mediana** | média | máximo | municípios em que o TSE chegou antes |
|---|---|---:|---:|---:|---:|---:|
| site público, 95 req/s | **matematicamente eleito** | 1 min 25 s | **26 min 01 s** | 29 min 41 s | 2 h 47 min 50 s | 0 |
| site público, 95 req/s | apuração completa | −2 s | **4 min 44 s** | 6 min 03 s | 18 min 16 s | 1 (Santarém/PA, por 2 s: o último BU, recebido às 20:42, só apareceu no `cs.json` às 20:50) |
| sem limite de taxa | **matematicamente eleito** | 1 min 46 s | **26 min 30 s** | 30 min 07 s | 2 h 48 min 10 s | 0 |
| sem limite de taxa | apuração completa | 27 s | **5 min 49 s** | 6 min 41 s | 18 min 31 s | 0 |

* A apuração completa do Urna Velox sai milissegundos depois do último BU; a do TSE sai, na mediana, **5 min 49 s depois do último recebimento** (mín. 27 s, máx. 18 min 31 s). É essa a vantagem no cenário rede.
* Pelo site público a coleta não forma fila no 2º turno (pico de 2.562 BUs/min, abaixo da capacidade de ~2.830 seções/min a 95 req/s): o atraso é o ciclo de releitura do `cs.json` (até 30 s) mais as duas requisições (~1 s). Por isso os dois cenários quase coincidem.
* Maior caso: **São Paulo** (9,3 milhões de eleitores, 26.513 seções), matematicamente eleito às 18:42:34, **1 h 14 min 44 s** antes da totalização do TSE (19:57:19); apuração completa 17 min 18 s antes.

Tabela completa, por município: [prefeito_2024_2t_municipios.md](prefeito_2024_2t_municipios.md) (dados em [simulacao_prefeito_2024_2t.csv](simulacao_prefeito_2024_2t.csv), resumo em [simulacao_prefeito_2024_2t_resumo.json](simulacao_prefeito_2024_2t_resumo.json)).

**Limites da simulação.** A disponibilização no `cs.json` é a do arquivo republicado do site (não um registro da noite); o atraso de propagação no CDN além dela não é conhecido; o eleitorado restante usa o `e.te` do arquivo oficial (num sistema real viria do cadastro, antes da eleição); a hora do TSE é a da última totalização do município, não a de uma eventual divulgação de "matematicamente eleito" pelo próprio TSE, que não é publicada.

## 3. Como repetir

```bash
cd aquisicao
W=~/uvreal/2024-2t
python3 scripts/baixar_oficial_municipal.py --ciclo ele2024 --eleicao 620 --cargo 11 --dest $W/oficial --rps 10
UV_CICLO=ele2024 UV_PLEITO=453 UV_DADOS=$W/dados UV_SAIDA=$W UV_RPS=95 \
  UV_UFS=am,ba,ce,es,go,ma,mg,ms,mt,pa,pb,pe,pr,rj,rn,ro,rs,se,sp,to scripts/coleta_nacional.sh 2t
uv-coletor catalogo --dados $W/dados --saida $W/resultado/catalogo.txt --vagas ""
uv-coletor totalizar --dados $W/dados --catalogo $W/resultado/catalogo.txt --anuladas $W/metricas_2t.csv --totais-municipio $W/resultado/totais_municipio.csv
python3 scripts/comparar_municipal.py --totais $W/resultado/totais_municipio.csv --oficial $W/oficial --dados $W/dados --eleicoes 620:11
python3 scripts/simular_noite_municipal.py --dados $W/dados --metricas $W/metricas_2t.csv --oficial $W/oficial \
  --eleicao 620 --cargo 11 --dia 27/10/2024 --latencia-uv-ms 1.73 --saida $W/sim
uv-coletor replay --dados $W/dados --catalogo $W/resultado/catalogo.txt --chegadas $W/sim/chegadas_site.csv --acelerar 60 --cargos 11 --saida $W/replay-site-60
```

## Refeito com o motor (v0.0.26)

A simulação foi refeita com o motor `urna-velox` (`uv-motor noite`), que calcula o matematicamente eleito com a regra
completa (empate pela idade, art. 224, eleitorado restante) e confere com a situação oficial:

```text
uv-motor noite --dados ~/uvreal/2024-2t/dados --oficial ~/uvreal/2024-2t/oficial --chegadas ~/uvreal/2024-2t/sim/chegadas_site.csv \
  --anuladas ~/uvreal/2024-2t/metricas_2t.csv --dia 27/10/2024 --latencia-ms 1.726 --saida noite-2t-site
uv-motor noite … --chegadas ~/uvreal/2024-2t/metricas_2t.csv --disponivel-antes …   # BUs junto com o TSE
```

| cenário | matematicamente eleito antes do TSE (49 municípios) | apuração completa antes do TSE |
|---|---|---|
| site público (coleta a 95 req/s) | mediana **1.561 s (26 min)**, mín. 85 s, máx. 10.071 s | mediana 285 s |
| BUs junto com o TSE | mediana 1.590 s, mín. 106 s, máx. 10.090 s | mediana 349 s |

Os mesmos números da simulação Python; 51 de 51 municípios decididos, nenhuma revogação, todos iguais ao oficial.

