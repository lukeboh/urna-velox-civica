# M-002 — Vazão de ponta a ponta e topologia (medição de 18/10/2026)

Pedido do Luciano (docs/BUGS.md, M-002): medir onde vai o tempo em cada etapa, por nó, e avaliar uma topologia que
permita aumentar a vazão quando precisar. **Nada foi mudado nos nós** (decisão de 18/10: medir sem tocar; mudança de
topologia só depois de 25/10 e com decisão dele).

Fontes: as evidências do ciclo diário no bucket (`63_ciclo.sh --telemetria`: `telemetria.jsonl` a cada 15 s,
`secoes.log`, `saude.json`), analisadas por `aquisicao/scripts/medir_vazao.py`; a chegada dos BUs ao TSE nas noites
reais (linha do tempo, `recebido_tse` de cada seção), simulada por `aquisicao/scripts/fila_noite.py`. As execuções
medidas são da agenda fixa de 17–18/10 (antes da rotação contínua por hora fechada da 0.8.19); a rotação não muda o
que cada execução faz, só quando começa, então os números valem para ela.

## 1. Resposta curta

* **Na noite real, quem limita é o teto de 100 req/s por IP de saída, não a CPU nem a totalização.** Pela API são 2
  requisições por BU (`aux.json` + BU), então cada nó coleta no máximo ~50 BU/s. A chegada ao TSE tem pico de
  **115 BU/s** (1 min; 452) e **96 BU/s** (10 min). Com os **2 nós de hoje**, o atraso máximo de um BU fica em
  **< 1 min** (0,8 min no 452; 0,5 min no 3220). Com **1 nó**, sobe para **45 min** (452) e **38 min** (3220).
* **O risco da topologia atual é a falta de folga:** se um dos dois nós cair no pico (18h30–19h30), a coleta volta a
  1 nó e o atraso chega a ~40 min. Um **3º nó com IP próprio** mantém o atraso < 1 min mesmo com um nó fora (N+1).
* **No modo rápido das simulações, quem limita é a CPU** dos nós ARM de 2 núcleos: o par aplica ~1.100 BU/s (cada nó
  aplica os ~500 mil BUs; busca metade). O `uv-servico` usa ~1,1 núcleo, o simulador 0,19 (só no nó A) e o resto do
  sistema (núcleo do Linux, rede local, `socat` da ponte, não separados na telemetria) mais 0,25–0,35; a CPU do sistema
  fica em 76 % de média e 99 % de pico. A totalização em si é desprezível (`matematicamente`, `marcos` e `lote` somam
  menos de 1 % do tempo).
* **Disco:** a sincronia do diário (`fsync`) tem p99 de 3 a 10 s e máximos de 11 s no nó B — não atrasa a apuração
  (fica fora do caminho do BU), mas é o maior "soluço" medido.

## 2. A noite real: chegada ao TSE × capacidade de coleta

`fila_noite.py`: cada BU entra na fila no `recebido_tse` e sai à capacidade da federação (nós × (100 − cs.json) ÷
requisições por BU), em passos de 1 s. Supõe o BU disponível assim que chega ao TSE e nenhum 429.

| cenário | capacidade | 452 (1º t. 2024): atraso máx. · fila máx. · zera | 3220 (1º t. 2026): atraso máx. · fila máx. · zera |
|---|---|---|---|
| 1 nó, API | 49,6 BU/s | **44,9 min** · 133.581 · 20h16 | **37,8 min** · 112.198 · 20h13 |
| **2 nós, API (hoje)** | 99,1 BU/s | **0,8 min** · 4.684 · 18h49 | **0,5 min** · 3.028 · 18h56 |
| 3 nós, API | 148,7 BU/s | 0,0 min · 35 | 0,1 min · 839 |
| 4 nós, API | 198,3 BU/s | 0 | 0,0 min · 36 |
| 1 nó, 1 requisição por BU | 99,1 BU/s | 0,8 min | 0,5 min |
| 2 nós, 1 requisição por BU | 198,3 BU/s | 0 | 0,0 min |

No 2º turno (453: pico de 42 BU/s) um nó basta (atraso máx. 0,3 min).

## 3. As simulações: onde vai o tempo em cada nó

Execuções do ciclo de 18/10 no modo rápido (500×, sem teto de taxa contra o simulador local). "Partida" = serviço
no ar → 1º BU aplicado.

| execução | nó | BUs | partida | 50 % | 99 % | 100 % | BU/s (média) | CPU uv-servico | simulador | CPU sistema (méd./máx.) |
|---|---|---|---|---|---|---|---|---|---|---|
| 3220 13h (0.8.10) | A | 499.207 | 2,8 s | 216 s | 471 s | 473 s | 1.062 | 1,05 núcleo | 0,19 | 77 % / 99,5 % |
| 3220 13h (0.8.10) | B | 499.207 | 2,9 s | 231 s | 445 s | 447 s | 1.124 | 1,17 núcleo | — | 76 % / 99,2 % |
| 452 07h (0.8.4) | A | 478.026 | 2,8 s | 198 s | 410 s | 413 s | 1.166 | 1,02 núcleo | 0,19 | 76 % / 98,9 % |
| 453 11h (0.8.4) | A | 97.392 | 1,7 s | 17 s | 30 s | 65 s | 1.545 | 0,41 núcleo | 0,19 | 41 % / 96 % |

(No 452 das 07h o `uv-servico` do nó B reiniciou no meio — números do B fora da tabela; é o B-006/B-003.)

Custo por etapa no 3220 (nó A; `custos_ms`/`latencias_ms` do `saude.json`, n × p50 ≈ tempo somado):

| etapa | n | p50 | p99 | máx. |
|---|---|---|---|---|
| busca do `aux.json` (simulador local) | 249.051 | 14 ms | 929 ms | 1,5 s |
| busca do BU | 249.038 | 13 ms | 934 ms | 1,5 s |
| BU buscado → publicado no site (`bu_publicacao`) | 499.207 | 71 ms | 1,6 s | 1,7 s |
| publicação das abrangências | 5.699 | 14 ms | 96 ms | 165 ms |
| gravação do diário | 4.243 | 19 ms | 215 ms | 2,9 s |
| sincronia do disco (`fsync`) | 231 | 237 ms | 3,1 s | 6,7 s (B: 11 s) |
| lote da federação | 5.698 | 7 ms | 112 ms | 0,8 s |
| `matematicamente` / marcos / faltam | 5.686 | < 0,1 ms | 6 ms | 75 ms |

Outras medidas:

* **CPU por BU:** ~1 ms de CPU do `uv-servico` por BU aplicado (506 s de CPU para 499 mil BUs, metade buscados pelo
  próprio nó). A capacidade do par cresce com núcleos; com 2 núcleos, o limite está em ~1.100 BU/s.
* **cs.json no modo rápido:** o intervalo de 30 s do cs.json é dividido pela aceleração (30 s ÷ 500 = 60 ms), o que dá
  ~270 cs.json/s por nó só de consulta (126 mil requisições, quase todas 304, em 8 min). É artefato da simulação (na
  noite real são ~1 req/s), mas consome CPU do serviço e do simulador nas simulações — e, depois de encerrada, a
  apuração continua consultando no mesmo ritmo (no 452 das 07h, mais 1,3 milhão de 304 entre o encerramento, às
  07h09, e a última amostra, às 07h52: ~500 por segundo, sem nenhum BU novo).
* **Federação:** trabalho repetido (BU recebido do par já aplicado) de **0,1 %** quando ninguém reinicia; **14 % e
  38 %** no 452 das 07h, em que o B reiniciou. Cada nó lê os lotes do par e aplica todos os BUs.
* **Disco:** `fsync` com p99 de 3 s (A) a 10 s (B); o pior caso (24 s) foi no 453 em cadência de 17/10.
* **Partida:** 3 s do serviço no ar ao 1º BU (catálogo de 500 mil seções carregado em 0,7 s).

## 4. Opções de topologia

| opção | o que muda | ganho | custo / risco |
|---|---|---|---|
| **A. Manter 2 nós** | nada | noite real com atraso máx. < 1 min | sem folga: um nó fora no pico → ~40 min de atraso |
| **B. 3º nó (IP próprio)** | mais um nó na federação | N+1: atraso < 1 min mesmo com um nó fora; +50 % de coleta | os 2 nós atuais (A1, 2 OCPU e 12 GB cada, `00_criar_oci.sh`) já usam os 4 OCPU / 24 GB do Always Free da OCI: um 3º seria pago ou exigiria redividir (p. ex. 2+1+1 OCPU, o que reduz a CPU das simulações); precisa respeitar a regra do teto por IP de saída (acima de 100 req/s há HTTP 429, ADR-028) |
| **C. 1 requisição por BU** | deixar de depender do `aux.json` para achar o BU (investigar; as faixas Range do zip fazem 1 req/BU, AQUISICAO.md) | dobra a coleta sem nó novo; 2 nós viram 198 BU/s | depende do que o TSE publica na noite; trabalho de código e de medição |
| **D. Simulações mais leves** | cs.json com intervalo mínimo (p. ex. 1 s) no modo rápido; parar de consultar depois de encerrada; simulador fora do nó A | devolve no nó A os 0,19 núcleo do simulador e a parte do cs.json (não separada; a medir antes de mudar) | só afeta as simulações (o B-001 também: ordem no modo rápido) |
| **E. Mais núcleos por nó** | forma maior | modo rápido/reprocessamento mais rápido (escala com a CPU) | não muda a noite real (o teto é de requisições) |

Recomendação: para 25/10, **A** está dentro do necessário; **B** é a forma de ter folga contra a queda de um nó no pico
e **C** a de ganhar coleta sem mais máquinas. **D** é barata e só melhora as simulações. Nenhuma foi feita: a escolha
é do Luciano, e nada muda nos nós antes de 25/10 sem decisão dele.

## 5. Como repetir

```
cd deploy/oci && ./63_ciclo.sh --telemetria                       # evidências do bucket para o D:
python3 aquisicao/scripts/medir_vazao.py /mnt/d/dados/urna-velox/ciclo-diario/<execução> [--json x.json]
python3 aquisicao/scripts/fila_noite.py /mnt/d/dados/urna-velox/linha-do-tempo/452.csv [--json x.json]
```
