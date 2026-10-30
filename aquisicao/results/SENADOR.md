# Senador — 1º turno de 2026: totalização, conferência e desempenho

Execução de 12/10/2026 (`v0.0.13`), i7-14700 (28 threads), WSL2. Entrada: os **499.207 BUs** coletados do site de resultados ([AQUISICAO §4.3.1](../../docs/arquitetura/AQUISICAO.md)). Em 2026 cada UF elegeu **2 senadores**: cada eleitor deu 2 votos, e o leitor confere em cada BU que a soma dos votos do cargo é `comparecimento × 2`.

## 1. Conferência com a divulgação oficial

Arquivos oficiais: `resultados.tse.jus.br/oficial/ele2026/6259/dados/<uf>/<uf>-c0005-e006259-u.json` (27 arquivos, baixados a 1 req/s com `scripts/baixar_oficial.sh`). Comparação: `scripts/conferir_senador.sh` → `comparar_oficial.py` ([comparacao_oficial_nacional.md](comparacao_oficial_nacional.md)).

| item conferido | resultado |
|---|---|
| votos de cada candidato (`vap`), 27 UFs | **idênticos** |
| brancos (`vb`), nulos (`vn`), nulos técnicos (`vnt`), comparecimento (`e.c`) | **idênticos** nas 27 UFs |
| seções totalizadas (`s.st`) × BUs | **idênticos** nas 27 UFs |
| situação de cada candidato (eleito / não eleito), calculada com os votos do Urna Velox | **277 de 277** iguais à oficial |
| senadores eleitos | **54 de 54** ([lista](senador_eleitos_2026_1t.md)) |
| Presidente e Governador, na mesma rodada | 56 de 56 abrangências idênticas; situação de 348 + 169 candidatos idêntica |

Nulos técnicos de Senador (votos digitados em número sem candidato apto): AM nº 181 (57.518), DF nº 555 (6.323), MS nº 365 (2.773) — iguais ao `vnt` oficial.

**Regra da situação:** os `nv` (= 2) candidatos aptos mais votados são eleitos (maioria simples); empate resolvido pelo mais idoso. Só concorrem candidatos com destinação "Válido". Hoje a aptidão vem do próprio arquivo oficial; no motor ela virá do cadastro de candidaturas (ADR-018).

## 2. Desempenho da totalização

Mesma métrica do `bench/`: BUs já em memória (`mmap` dos `.uvpk`), tempo de parede, mediana de 5 repetições (`scripts/desempenho_cargo.sh`, dados em [desempenho_por_cargo.csv](desempenho_por_cargo.csv)). `--cargos 5` totaliza só o Senador; os outros cargos do BU são lidos e pulados.

| cargos totalizados | 1 thread | 28 threads | BUs/s (28 threads) | µs por BU (1 thread) | estado | conferência oficial |
|---|---:|---:|---:|---:|---:|---|
| **Senador** | **1,174 s** | **0,118 s** | **4,22 milhões** | **2,35** | **2,5 MB** | 27/27 idênticas |
| Presidente | 1,168 s | 0,106 s | 4,72 milhões | 2,34 | 2,6 MB | 29/29 |
| Governador | 1,231 s | 0,108 s | 4,61 milhões | 2,47 | 2,4 MB | 27/27 |
| Presidente + Governador + Senador | 1,211 s | 0,128 s | 3,90 milhões | 2,43 | 7,4 MB | 83/83 |
| todos os cargos do BU (inclui deputados) | 1,292 s | 0,125 s | 4,01 milhões | 2,59 | 34,3 MB | 83/83 |

* **O custo é o da leitura do BU, não o do cargo.** Totalizar só o Senador custa quase o mesmo que totalizar todos os cargos (2,35 contra 2,59 µs por BU numa thread): percorrer o BER domina, e somar os votos de um cargo a mais quase não pesa. O Senador acrescenta ~0,1 µs por BU ao custo de Presidente e Governador.
* O estado do Senador é pequeno (2,5 MB): 27 UFs × ~10 candidatos por município.
* Com 28 threads, a eleição inteira de Senador sai em **0,12 s**.

## 3. Replay da noite (do recebimento ao resultado publicado)

`uv-coletor replay --cargos 5`: cada BU entra no horário em que o TSE o recebeu (17:08 de 04/10 a 03:00 de 05/10, `dr/hr` dos `aux.json`), e o resultado de Senador das 27 UFs é republicado sempre que há BU novo (`scripts/replay_senador.sh`).

| cenário | publicações | ciclo de publicação p50 / p99 | latência p50 | p90 | p99 | máx. | conferência final |
|---|---:|---|---:|---:|---:|---:|---|
| **Senador, noite acelerada 60×** (carga 60 vezes mais densa; pico de 392 mil BUs/min) | 14.304 | 1,9 / 4,8 ms | **3,8 ms** | 4,7 ms | 11,3 ms | 20,4 ms | 27/27 idênticas |
| Senador, todos os BUs de uma vez (pior caso) | 1.192 | 1,3 / 2,8 ms | 1,00 s | 1,64 s | 1,77 s | 1,78 s | 27/27 idênticas |
| Presidente + Governador + Senador, 60× | 14.162 | 3,7 / 9,0 ms | 6,7 ms | 8,6 ms | 14,2 ms | 24,3 ms | 83/83 idênticas |
| *referência:* Presidente + Governador, 60× (`v0.0.10`) | 14.081 | 2,7 / 6,4 ms | 4,9 ms | 6,1 ms | 12,2 ms | 16,3 ms | 56/56 |

* A latência acompanha o custo de cada publicação, que cresce com o número de arquivos e de candidatos consolidados: 27 arquivos de Senador (ciclo p50 1,9 ms) contra 83 arquivos dos três cargos (3,7 ms). Mesmo com os três cargos, o p99 fica em 14 ms.

## 4. Como repetir

```bash
cd aquisicao
UV_CARGOS=5 scripts/baixar_oficial.sh                      # 27 arquivos oficiais (1 req/s; pula os que já existem)
scripts/conferir_senador.sh ~/uvreal/resultado             # conferência de Presidente, Governador e Senador
scripts/desempenho_cargo.sh "5" "1" "3" "1,3,5" "todos"    # desempenho por cargo
scripts/replay_senador.sh                                  # replays 0× e 60×
python3 scripts/senador_eleitos.py --totais ~/uvreal/resultado/totais_uf.csv --oficial ~/uvreal/oficial
```
