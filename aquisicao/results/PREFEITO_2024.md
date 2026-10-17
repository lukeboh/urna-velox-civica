# Prefeito e consultas populares — 1º turno de 2024: totalização, conferência e desempenho

Execução de 12/10/2026 (`v0.0.18`), i7-14700 (28 threads), WSL2. Eleição municipal de 06/10/2024: pleito 452, eleição 619 (Prefeito, cargo 11; Vereador, cargo 13) e 5 consultas populares (eleições 633–637, cargos 51–55). Entrada: **478.026 BUs** de 26 UFs (sem DF e sem exterior), dos zips de dados abertos guardados no acervo local, com os horários de recebimento tirados dos `aux.json` ([AQUISICAO §1.1.1](../../docs/AQUISICAO.md)).

## 1. Aquisição

| etapa | volume | tempo | observação |
|---|---|---:|---|
| zips de dados abertos, 25 UFs (`gerenciar_acervo.py`, acervo em `D:\dados\tse-arquivos-urna`) | 65,1 GB | 46,7 min de download; 1 h 34 min com derivação e validação ASN.1 de amostra | 23,3 MB/s em média; 1 requisição por UF; AC já estava no acervo |
| importação dos 26 zips originais para `.uvpk` (`uv-coletor zip --local`) | 478.026 BUs (478.025 `-bu.dat` + 1 `-busa.dat`, AM) | 13,5 min | leitura do disco D: pelo WSL; sem rede |
| `aux.json` de todas as seções (`uv-coletor baixar --so-aux`) | 500.341 seções; 478.026 com BU, 22.315 com aux.json 404 | 1 h 33 min | 89,7 req/s, no máximo 90 num mesmo segundo, **0 HTTP 429** |

Arquivos oficiais por município: 5.569 de Prefeito e 5 das consultas, baixados a 14,4 req/s em 6 min 27 s (`baixar_oficial_municipal.py`), todos HTTP 200.

## 2. Conferência com a divulgação oficial

`uv-coletor totalizar --totais-municipio --anuladas metricas_aux.csv` → `comparar_municipal.py` ([comparacao_municipal_2024_1t.md](comparacao_municipal_2024_1t.md)).

| item conferido, por município | resultado |
|---|---|
| Prefeito: votos de cada candidato (`vap`), brancos (`vb`), nulos (`vn`), nulos técnicos (`vnt`), comparecimento (`e.c`), seções totalizadas (`s.st`) × BUs | **5.569 de 5.569 municípios idênticos** |
| Prefeito: situação de cada candidato (eleito / 2º turno / não eleito), calculada com os votos do Urna Velox | **15.032 de 15.032 idênticas** (5.479 eleitos, 50 municípios com 2º turno, 31 municípios sem eleito) |
| Consultas populares: votos por opção, brancos, nulos, comparecimento, seções e opção vencedora | **5 de 5 idênticas** (Belo Horizonte, São Luís, Governador Edison Lobão, São Luiz, Dois Lajeados) |
| conferências do totalizador | 0 cargos inválidos, 0 votáveis desconhecidos, 0 duplicados, 0 fora do cadastro, 0 identificações divergentes, 0 erros de leitura |

Em **9 municípios** a totalização oficial não está finalizada (`tf = "n"`; em 4 de Sergipe, "Aguarde reprocessamento da eleição") e o arquivo traz a situação vazia: os votos foram conferidos (idênticos), a situação não. São GO 93998, GO 96610, MA 07137, MA 07951, MA 08222 e SE 31054 (Aracaju), 31119, 31313, 31950.

**Regras que o arquivo oficial aplica e o comparador reproduz:**

* **Maioria simples** nos municípios com até 200 mil eleitores (`e.te` do arquivo oficial); acima disso, **maioria absoluta** ou 2º turno entre os dois mais votados. A maioria absoluta é medida sobre a votação nominal inteira, inclusive a de candidatos anulados depois da eleição (o 2º turno foi decidido com esses votos: MG 54011).
* **Ninguém é eleito** (`esae = "s"`, eleição suplementar) quando o candidato com a maior votação nominal teve os votos anulados, ou quando os votos anulados passam de 50 % da votação nominal (Código Eleitoral, art. 224): 31 municípios.
* Só concorrem candidatos com destinação "Válido"; empate, o mais idoso.
* **Seção anulada:** MA 07315, zona 23, seção 354 tem no `aux.json` a situação "Anulada" e o BU "Excluído". A seção conta em `s.st` e no comparecimento, mas os 315 votos dela vão para os nulos (`vsan`). Sem `--anuladas`, esse município diverge em 304 votos (nominais e brancos).
* Votos em número sem candidato apto vêm nominais no BU e o TSE os soma em `vnt` (121 municípios; maior caso: SC 81833, nº 15, 21.021 votos).

## 3. Desempenho da totalização

BUs já em memória (`mmap` dos `.uvpk`), tempo de parede, mediana de 5 repetições (`scripts/desempenho_municipal.sh`, dados em [desempenho_por_cargo_2024_1t.csv](desempenho_por_cargo_2024_1t.csv)).

| cargos totalizados | 1 thread | 28 threads | BUs/s (28 threads) | µs por BU (1 thread) | estado | conferência |
|---|---:|---:|---:|---:|---:|---|
| **Prefeito** | **0,569 s** | **0,065 s** | **7,4 milhões** | **1,19** | **2,3 MB** | 5.569/5.569 |
| consultas (51–55) | 0,560 s | 0,061 s | 7,8 milhões | 1,17 | < 0,1 MB | 5/5 |
| Prefeito + consultas | 0,560 s | 0,066 s | 7,3 milhões | 1,17 | 2,3 MB | 5.574/5.574 |
| todos os cargos do BU (inclui Vereador) | 0,796 s | 0,085 s | 5,6 milhões | 1,66 | 6,1 MB | 5.574/5.574 |

* O BU municipal é menor que o de 2026 (2 cargos em vez de 6): **1,2 µs por BU** numa thread contra 2,3 µs em 2026.
* O estado é pequeno porque os candidatos ficam **por município** (5.569 listas): 1,59 milhão de contadores para todos os cargos, inclusive Vereador. Com as listas por UF (o layout de 2026) só o AC já precisava de 3 × mais contadores.
* Catálogo (municípios, seções e candidatos dos 478.026 BUs): 5,5 s, dos quais 5,2 s lendo os BUs.

## 4. Replay da noite (do recebimento ao resultado publicado)

`uv-coletor replay --cargos 11,51,52,53,54,55 --anuladas …`: cada BU entra no horário em que o TSE o recebeu (06/10/2024 17:01–23:59, `dr/hr` dos `aux.json`; os 325 reenvios posteriores entram às 23:59). Os cargos municipais são publicados **por município**, só nos municípios que receberam BU desde a publicação anterior.

| cenário | publicações | ciclo de publicação p50 / p99 | latência p50 | p90 | p99 | máx. | conferência final |
|---|---:|---|---:|---:|---:|---:|---|
| **noite acelerada 60×** (pico de 6.896 BUs/min × 60) | 17.606 | 0,31 / 4,3 ms | **2,4 ms** | 4,1 ms | 5,5 ms | 11,2 ms | 5.574/5.574 idênticos |
| todos os BUs de uma vez (pior caso) | 82 | 10,5 / 44,0 ms | 519 ms | 931 ms | 1,02 s | 1,02 s | 5.574/5.574 idênticos |

## 5. Como repetir

```bash
cd aquisicao
# BUs: zips de dados abertos no acervo (1 requisição por UF)
python3 acervo/gerenciar_acervo.py --raiz /mnt/d/dados/tse-arquivos-urna --anos 2024 --turnos 1 --rps 90
# horários de recebimento: só os aux.json (~1,5 h a 90 req/s)
uv-coletor baixar --so-aux --ciclo ele2024 --pleito 452 --ufs ac,al,…,to --dados ~/uvreal/2024-1t/aux \
  -c 128 --adaptativo --rps 90 --metricas ~/uvreal/2024-1t/metricas_aux.csv
# arquivos oficiais por município
python3 scripts/baixar_oficial_municipal.py --ciclo ele2024 --eleicao 619 --cargo 11 --dest ~/uvreal/2024-1t/oficial
# importação, catálogo, totalização e conferência (sem rede)
scripts/municipal_acervo.sh 2024 1 452 ~/uvreal/2024-1t 619:11,633:55,634:53,635:51,636:54,637:52
scripts/desempenho_municipal.sh ~/uvreal/2024-1t 619:11,633:55,634:53,635:51,636:54,637:52 "11" "51,52,53,54,55" "todos"
```
