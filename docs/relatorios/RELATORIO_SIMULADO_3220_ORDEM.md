# Simulado do 3220 na ordem de recebimento do TSE (16/10/2026)

*Item da etapa 9 (ensaio geral, `0.9.0`) do [ROADMAP](../../ROADMAP.md). Registrado na `0.6.8`. Evidências em
`D:\dados\urna-velox\simulado-3220\resultado-20261016-ordem` (`ORDEM.md`, `no-A/`, `no-B/`).*

## Como rodou

* Nós da OCI `uv-no-a` (coleta, `<ip-público-no-a>`) e `uv-no-b` (só recebe do par), binário `0.6.5`, operados do WSL
  com `TRANSPORTE=oci` (sem porta 22).
* `./60_simulado.sh --confirmo-tse --combinado` às 07:13; coleta de 07:18 a 10:16 (~2 h 58 min) contra a CDN do TSE.
  Fila ordenada pelo `recebido_tse` de cada seção (`ordem_3220.csv`, tirado dos `aux.json`; `uv-servico --ordem`).
* Trava manual ligada nos dois nós durante todo o simulado (`controle_manual`): o atualizador não interferiu.
* Conferência pedida à mão (arquivo `CONFERIR` na saída) às 10:24, porque a apuração não encerrou (ver abaixo).
  Relatório: `./61_relatorio.sh`.

## Resultado

| medida | nó A | nó B |
|---|---|---|
| BUs aplicados | 499.207 (todos os publicados) | 499.207 (recebidos do A) |
| requisições ao TSE | `aux.json` 499.207 + BU 499.207, a ~94,6 req/s (teto 95) | 0 |
| HTTP 429 | 0 | 0 |
| conferência com os `-u.json` oficiais | 81/83 | 81/83 |
| situações dos candidatos | 832 idênticas, 0 divergentes | idem |
| matematicamente | 55/55 circunscrições decididas, 0 revogações, 0 diferentes do oficial | idem |

**Ordem de totalização × recebimento no TSE** (nó A; o B é igual):

| | 16/10 (`--ordem`) | 15/10 (sem ordem, UF por UF alfabética) |
|---|---|---|
| atraso no recebimento p50 / p99 | **1 s / 291 s** | 244.842 s / — |
| deslocamento de posição p50 / p99 | **6 / 25** | 158.050 / — |
| fora de ordem | 13,24 % | — |
| trocas vizinhas | 6,38 % | 49,41 % |
| ordem de início das UFs | ZZ AC TO DF MA AP RO PA PR AM MG … | AC AL AM AP BA CE … |

A ordem de início das UFs acompanha a do primeiro BU recebido no TSE (`MA TO ZZ AC AP DF PA PR RO AM MG …`) com
pequenas trocas na largada (a fila é por seção, e as primeiras UFs têm BUs com segundos de diferença).

## As 2 diferenças e o encerramento

As duas divergências da conferência são só o **número de seções totalizadas** de Presidente:

* `br-c0001-e006257-u.json`: oficial 499.248, Urna Velox 499.207;
* `zz-c0001-e006257-u.json`: oficial 1.351, Urna Velox 1.310.

Os votos são idênticos. A diferença são **41 seções do exterior (ZZ)** que estão no catálogo (`candidatos.json`), não
são agregadas e nunca tiveram BU publicado pelo TSE (não aparecem no `aux.json` nem no `ordem_3220.csv`); o oficial
as conta como totalizadas. Exemplos: município 29424 seção 54, município 99384 seção 2876 (lista completa com
`D:\dados\uv-tmp\faltam41.py`).

Consequência no serviço: ele só encerra quando cada abrangência atinge o número de seções esperado, e essas 41 não
são tratadas como não instaladas (`nao_instaladas` = 0). Então ZZ e BR nunca fecham, a tela fica em 99,99 %
("499.207 de 499.248"), a publicação não vira "encerrada" e a conferência automática do fim não roda. No 2º turno o
mesmo pode acontecer com seções do exterior sem BU.

**Corrigido na `0.6.10`** (16/10). O `aux.json` dessas 41 seções diz "Nao instalada" (coleta de 04/10,
`metricas_nacional.csv`), e o oficial final tem `s.sni` = 41 e `e.esni` = 423. O serviço já sondava as seções sem
disponibilização, mas só depois de 30 min (hora do site) sem BU novo — o simulado parou 18 min depois do último BU — e
pedia também as ~1.364 agregadas (404). Regra do Luciano: a seção sem BU continua esperada até o TSE dizer que não foi
instalada; então sai das que faltam e conta como totalizada, como no oficial. Agora a sondagem é por UF, sem esperar
silêncio, sem as agregadas (`nsp` do `cs.json`) e repetida a cada 5 min por seção (`docs/operacao/OPERACAO.md`, "Sondagem").

## Observações

* A antecedência sobre a totalização do TSE sai negativa (~ −11 dias) porque é um replay de 16/10 de uma eleição de
  04/10; não é medida da noite.
* Alerta esperado nos dois nós: destinação dos candidatos presumida "Válido" (os arquivos do 3220 não trazem `dvt`).
* Os nós ficaram ligados no fim, com a trava manual, mostrando o resultado. Para voltar ao pleito de produção:
  `./60_simulado.sh --restaurar`.
