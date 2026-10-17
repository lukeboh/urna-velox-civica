# Formato do resultado (versão 1)

Resultado de um **cargo** numa **abrangência**, como o motor `urna-velox` publica: um objeto JSON por
(eleição, cargo, abrangência). É o contrato do WebSocket da etapa 4 e do site de exemplo da etapa 5; os campos
conferidos com a divulgação oficial do TSE têm o nome do campo oficial ao lado.

Quem gera: `urna_velox::resultado::Publicador` (`valor`/`json`/`gravar`). Quem usa hoje: `uv-motor totalizar
--saida`, o replay da noite (`uv-coletor replay`, pasta `publicacao/`) e as conferências (`uv-motor conferir`,
`uv-motor conferir-publicacao`), que comparam cada publicação com o `-u.json` oficial correspondente.

## Arquivo

`<abr>-c<cargo:4>-e<eleição:6>.json` — o mesmo padrão dos arquivos oficiais, sem o `-u`. Ex.: `br-c0001-e006257.json`
(Presidente, Brasil, 1º turno de 2026), `ba34134-c0011-e000620.json` (Prefeito, Camaçari, 2º turno de 2024).
O arquivo é trocado de uma vez (escrita em temporário + `rename`): quem lê nunca vê um arquivo pela metade.

## Exemplo

Publicação encerrada (fim da apuração) do 2º turno de 2024 em BA 34134, gravada pelo replay:

```json
{"v":1,"eleicao":620,"turno":2,"cargo":11,"abr":"ba34134","circ":"ba34134","seq":97392,
 "ultimo_bu":"2024-11-01T00:00:16-03:00","encerrada":true,"secoes":{"total":546,"apuradas":540},
 "eleitorado":205865,"apurado":205865,"comparecimento":165447,"vb":1954,"vn":5143,"vnt":0,"validos":158350,
 "legenda":0,"estado":"definido",
 "cand":[{"n":13,"v":80626,"pct":50.92,"dvt":"Válido","st":"Eleito",
          "mat":{"estado":"eleito","seq":76236,"em":"2024-10-27T18:17:24-03:00"}},
         {"n":44,"v":77724,"pct":49.08,"dvt":"Válido","st":"Não eleito","mat":null}]}
```

## Campos

| campo | tipo | significado | oficial |
|---|---|---|---|
| `v` | inteiro | versão do formato (1). Mudança incompatível sobe a versão | — |
| `eleicao` | inteiro | código da eleição (ex.: 6257 Presidente 2026, 6259 Governador/Senador 2026, 619/620 Prefeito 2024; consultas: o código da consulta); 0 sem candidatos | `ele` |
| `turno` | inteiro | 1 ou 2 | `t` |
| `cargo` | inteiro | código do cargo no BU (1 Presidente, 3 Governador, 5 Senador, 11 Prefeito; consultas: 25–99) | `carg[].cd`, `perg[].cd` |
| `abr` | texto | abrangência: `br`, UF em minúsculas (`df`; `zz` = exterior) ou UF + município com 5 dígitos (`ac01031`) | `cdabr` |
| `circ` | texto | **circunscrição** do cargo que contém a abrangência — onde se decide quem é eleito: `br` (Presidente), a UF (Governador, Senador), o município (Prefeito, consultas). Abrangências menores repetem a situação da circunscrição, como na divulgação | — |
| `seq` | inteiro | nº de BUs aplicados ao estado quando a publicação foi feita (cresce sempre; ordena as publicações) | — |
| `ultimo_bu` | texto ou `null` | horário (Brasília, ISO 8601) do último BU aplicado; `null` na totalização em lote | — |
| `encerrada` | booleano | `true` só na publicação final (totalização encerrada): é a única que traz a situação final (`st`) | — |
| `secoes.total` | inteiro | seções da abrangência (catálogo + não instaladas conhecidas), **sem as agregadas** (desde a `0.4.9`; seções juntadas a uma principal, sem urna nem BU próprio — o TSE não as conta; se uma vier com BU, passa a contar) | `s.ts` |
| `secoes.apuradas` | inteiro | seções totalizadas: com BU aplicado + não instaladas (o TSE as conta totalizadas, sem votos) | `s.st` |
| `eleitorado` | inteiro ou `null` | eleitorado da abrangência (lista de candidatos); `null` se não se conhece | `e.te` |
| `apurado` | inteiro | eleitores aptos das seções já totalizadas (soma do `qtdEleitoresAptos` dos BUs) | `e.est` (aprox.) |
| `comparecimento` | inteiro | comparecimento | `e.c` |
| `vb` | inteiro | votos em branco | `v.vb` |
| `vn` | inteiro | votos nulos (inclui todos os votos de seção anulada pela Junta) | `v.vn` |
| `vnt` | inteiro | nulos técnicos: votos digitados em número que não está na lista de candidatos | `v.vnt` |
| `validos` | inteiro | votos dos candidatos aptos (destinação "Válido…") | `v.vv` |
| `legenda` | inteiro | votos de legenda (0 nos cargos majoritários; > 0 indica BU inesperado) | — |
| `estado` | texto | ver abaixo | — |
| `cand` | lista | candidatos (ou opções da consulta) da circunscrição, em ordem de votos (empate: o mais idoso; depois o número) | `cand[]`, `resp[]` |
| `cand[].n` | inteiro | número na urna (nas consultas, o da opção) | `n` |
| `cand[].v` | inteiro | votos na abrangência | `vap` |
| `cand[].pct` | número ou `null` | `v / validos × 100`, 2 casas; `null` para não aptos | `pvap` |
| `cand[].dvt` | texto ou `null` | destinação dos votos: "Válido", "Válido sub judice", "Anulado", "Anulado sub judice"… | `dvt` |
| `cand[].st` | texto | situação final: "Eleito", "2º turno", "Não eleito"; **vazia** enquanto a publicação não é encerrada ou se a totalização oficial não foi finalizada | `st` |
| `cand[].mat` | objeto ou `null` | estado matemático do candidato, quando já não pode mudar: `estado` ("eleito" ou "2t"), `seq` e `em` (horário do BU que o disparou). Só existe no acompanhamento em fluxo (replay, serviço) | — |

### `estado`

| valor | quando |
|---|---|
| `apurando` | resultado ainda pode mudar (ou totalização oficial não finalizada, `tf = n`) |
| `matematicamente_eleito` | todas as vagas da circunscrição já têm eleito garantido, qualquer que seja o voto do eleitorado restante |
| `matematicamente_2t` | haverá 2º turno: nenhum apto alcança mais a maioria absoluta (os dois finalistas aparecem em `cand[].mat` quando garantidos) |
| `definido` | publicação encerrada, com eleito(s) ou 2º turno |
| `sem_eleito` | publicação encerrada sem eleito (art. 224 do Código Eleitoral: o mais votado teve os votos anulados, ou os anulados passam de 50 % da votação nominal) |

As regras de situação e do "matematicamente" estão em `motor/src/situacao.rs` e `motor/src/matematica.rs`
(resumo no [CHANGELOG](../CHANGELOG.md), versões 0.0.25 e 0.0.26). Um "matematicamente" nunca é revogado: os
testes reproduzem as noites de 2026 e 2024 e conferem isso.

## Eventos (precursor do WebSocket)

O replay grava `replay_eventos.csv`, uma linha por mudança de estado matemático, na ordem em que aconteceram:

```text
seq,horario,circunscricao,cargo,candidato,estado
76236,2024-10-27T18:17:24-03:00,ba34134,11,13,eleito
76236,2024-10-27T18:17:24-03:00,ba34134,11,,matematicamente_eleito
```

`candidato` vazio = evento da circunscrição. Estados: `eleito`, `2t` (candidato); `matematicamente_eleito`,
`matematicamente_2t` (circunscrição); com o prefixo `revogado:` se um estado deixasse de valer (não acontece; o
replay falha se acontecer).

## Compatibilidade

Campos novos podem ser acrescentados sem mudar `v`; quem lê deve ignorar campos desconhecidos. Renomear,
remover ou mudar o significado de um campo sobe `v`.
