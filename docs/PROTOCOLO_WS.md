# Protocolo de difusão em tempo real — WebSocket e REST (versão 1)

Contrato entre o `uv-servico` (`--http <endereço:porta>`) e quem acompanha a apuração: o site de exemplo (etapa 5),
integradores, jornais. Implementação: `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/difusao.rs`; cliente de referência
e de carga: `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bin/uv-carga.rs`. Decisão de arquitetura: ADR-026 em
`/mnt/d/git/lukeboh/urna-velox/docs/ARCHITECTURE_DECISIONS.md`.

O **conteúdo** de todo resultado é o do arquivo publicado em `<saida>/publicacao/` — o formato da etapa 1
(`/mnt/d/git/lukeboh/urna-velox/docs/FORMATO_RESULTADO.md`), **byte a byte**, sem campo novo dentro dele. O protocolo só
acrescenta um envelope em volta.

## 1. Endereços

| | |
|---|---|
| WebSocket | `ws://<host>:<porta>/ws` (texto UTF-8, um objeto JSON por mensagem) |
| REST | `GET /api/v1/estado`, `/api/v1/topicos`, `/api/v1/resultado/<topico>`, `/api/v1/eventos`; desde a `0.4.1`, `/api/v1/marcos` e `/api/v1/nomes`; desde a `0.4.8`, só no replay com `--controle-replay`, `GET /api/v1/replay`, `POST /api/v1/replay/velocidade` e (`0.4.9`) `POST /api/v1/replay/posicao` (§11) |
| Página | desde a `0.4.1`: `GET /` (página de resultados de exemplo, embutida no binário) e `GET /static/<arquivo>` (§10) |

> **Acréscimos da versão 1 (etapa 5, `0.4.1`)** — nada mudou no que já existia; quem lê ignora o que não conhece:
> `op: marcos` (§3), mensagem `marco` (§4.5), `GET /api/v1/marcos` e `GET /api/v1/nomes` (§6), página embutida (§10).
> **`0.4.8`/`0.4.9`:** bloco `replay` na mensagem `estado` e controle do replay — velocidade e posição na noite (§11) — só contra o site simulado.
> **`0.6.11` (etapa 7):** tópicos `tse:<tópico>` e mensagem `tse` (§4.6), a comparação com o site de resultados do TSE; erro `comparacao_desligada`.
> **`0.7.7` (etapa 8):** mensagem `previsao` (§4.7) — a estimativa, com faixa, do horário do "matematicamente" e do fim da
> apuração — e `GET /api/v1/previsao/<topico>` (§6). Vai para quem está inscrito no tópico; nada muda na inscrição.
> **`0.8.28` (M-003, relógio do sistema):** os horários do Urna Velox passam a ser os do **relógio do sistema** (a data e a
> hora reais em que ele totalizou, também no simulado), separados dos do TSE. Acréscimos: no resultado, `totalizado_em`
> (`docs/FORMATO_RESULTADO.md`; o `ultimo_bu` continua sendo o horário do TSE); em `evento` e `marco`, `sistema` (ISO com
> ms; `null` no histórico de uma retomada, cujo horário não ficou gravado; o `horario` continua sendo o do BU no TSE); no
> `estado` e no `GET /api/v1/estado`, o bloco `relogio_sistema` = `{"agora", "noite0", "sistema0", "acelerar"}` — a
> âncora: o instante `noite0` da noite do TSE corresponde a `sistema0` no sistema, e um instante `t` do TSE equivale a
> `sistema0 + (t − noite0) / acelerar` (1 na noite real; 500 no modo rápido), o que permite medir a distância entre as
> duas apurações mesmo num simulado de outro dia; na `previsao` (§4.7), `relogio: "sistema"` e todos os horários (e o
> `historico`, em segundos com fração) no relógio do sistema — a conta é feita na escala da noite com o relógio do
> sistema, não com o do site; o `previsoes.csv` continua na escala da noite. A curva (§6) passa a ter `t` no relógio do
> sistema (segundos com fração), `relogio: "sistema"` e `passo_s` = um minuto da noite nesse relógio (0,12 s a 500×).

Exemplo de partida: `uv-servico … --http 127.0.0.1:8080`. Sem `--http` não há camada de rede de entrada. O serviço
**não tem TLS nem autenticação**: para expor fora da máquina, use um proxy reverso (`/mnt/d/git/lukeboh/urna-velox/docs/OPERACAO.md` §7).

## 2. Tópicos

Um tópico = uma abrangência × um cargo, com o nome do arquivo publicado sem a eleição e a extensão:
`br-c0001` (Presidente, Brasil), `sp-c0003` (Governador, SP), `ba34134-c0011` (Prefeito, Camaçari), `zz-c0001`
(Presidente, exterior). `GET /api/v1/topicos` lista os existentes. O servidor só aceita conexões depois da publicação
a frio da partida (todas as abrangências, `seq` 0 ou o da retomada): todo tópico listado já tem um resultado.

## 3. Cliente → servidor

Mensagens de texto JSON, **no máximo 4 KB** cada:

```json
{"op":"inscrever","topicos":["br-c0001","sp-c0001"]}
{"op":"cancelar","topicos":["sp-c0001"]}
{"op":"eventos"}
{"op":"marcos"}
{"op":"mapa","desde":182345,"epoca":1793037600123}
```

* `inscrever`: passa a receber os tópicos (e os eventos das circunscrições deles). Para cada tópico novo, o servidor
  responde um `instantaneo` com o resultado atual e o histórico de eventos do tópico. Inscrever de novo num tópico já
  inscrito manda outro `instantaneo` (é assim que um cliente pede para "ressincronizar").
* `cancelar`: deixa de receber.
* `eventos`: recebe **todos** os eventos (de qualquer circunscrição), começando pelo histórico, mesmo sem tópico.
* `marcos` (desde a `0.4.1`): recebe **todos** os marcos da noite (§4.5): o histórico inteiro e o último
  `faltam_para_mat` de cada circunscrição, e depois cada marco novo. Pedir de novo não repete o histórico.
  Desde a `0.8.11` (B-014), `{"op":"marcos","faltam":["br-c0001","sp71072-c0011"]}` limita o `faltam_para_mat` aos
  tópicos da lista (até o limite de tópicos; os demais marcos continuam todos): no modo rápido as 5.574 circunscrições
  de 2024 mandavam milhares desses marcos por minuto. Pedir de novo troca a lista e manda o valor atual dos tópicos que
  entraram nela; sem `faltam` (ou `null`), todos, como antes; `faltam` que não é lista → `erro` `faltam_invalido`.
* `"escopo":"uf"` em `eventos` e `marcos` (desde a `0.8.11`, B-014): dos eventos e marcos de fora dos tópicos
  inscritos, só os do Brasil e das UFs (abrangência de até 2 letras ou sem abrangência); os das circunscrições
  municipais só chegam para os tópicos inscritos. É o que a página usa (celular). Sem `escopo`, todos, como antes.
* `tse:<tópico>` em `inscrever`/`cancelar` (desde a `0.6.11`): a comparação com o TSE (§4.6). A inscrição liga a
  busca do arquivo de resultado do TSE desse tópico no servidor; conta no limite de tópicos.
* `mapa` (desde a `0.9.19`): as seções que acenderam no mapa das seções (§4.8) depois de `desde` (quantas o cliente
  já tem, do `GET /api/v1/mapa/estado` ou das mensagens anteriores), e depois cada lote novo. `epoca` diferente da do
  processo → uma mensagem `{"tipo":"mapa","epoca":…,"reiniciar":true}` (o cliente busca a fotografia de novo e pede
  outra vez). Servidor sem o mapa → `erro` `mapa_indisponivel`. Pedir de novo troca o `desde`.
* Nada além disso: o cliente não muda estado, não força conferência, não lê arquivo.

Limites (por conexão; padrão, ajustáveis na linha de comando do serviço): até **200 tópicos**; até **20 mensagens/s**
(rajada de 40). Cada mensagem acima do limite → `erro` `taxa` (e é descartada); na 40ª acima do limite a conexão cai
(fechamento 1008, motivo `taxa`). Mensagem maior que 4 KB, binária ou JSON inválido → `erro` (as maiores que 4 KB
derrubam a conexão no próprio leitor do WebSocket: fechamento 1009). Nenhum erro de um cliente afeta os outros.

## 4. Servidor → cliente

Toda mensagem tem `tipo`. Instantes em **µs Unix** (relógio de parede do servidor).

### 4.1. `instantaneo` e `atualizacao`

```json
{"tipo":"atualizacao","topico":"br-c0001","seq":480125,"secoes_apuradas":480125,"ver":3172,"t_bu":1791248122123456,"t_envio":1791248122124301,"resultado":{…arquivo publicado…}
}
```

| campo | significado |
|---|---|
| `topico` | nome do tópico |
| `seq` | BUs aplicados ao estado quando o resultado foi calculado (= `resultado.seq`). Numa conexão, **nunca diminui** por tópico |
| `secoes_apuradas` | = `resultado.secoes.apuradas` (consistente com os totais: ADR-005) |
| `ver` | nº da versão do tópico neste processo do serviço (1, 2, 3… a cada publicação do tópico). Recomeça depois de um reinício do serviço — use `seq` para ordenar |
| `t_bu` | instante em que chegou o BU mais recente do lote que gerou o resultado; `null` se o lote não teve BU (ex.: só seções não instaladas, partida) |
| `t_envio` | instante em que o servidor pôs a mensagem para envio (a mesma para todos os clientes: a mensagem é montada **uma vez** e compartilhada) |
| `resultado` | o arquivo publicado, **idêntico byte a byte** (inclusive o `\n` final) |

**Como extrair os bytes exatos do arquivo:** o envelope é sempre `…,"resultado":` + bytes do arquivo + `}`. O
`resultado` é o último campo: os bytes do arquivo são do primeiro caractere depois de `"resultado":` até o penúltimo
da mensagem. Quem não precisa dos bytes pode simplesmente interpretar o JSON.

* `instantaneo`: a resposta a uma inscrição (o valor atual do tópico). Tem `t_envio` próprio.
* `atualizacao`: a cada publicação que mudou o tópico. O servidor guarda **só o último valor** por tópico: um cliente que
  não acompanha recebe o valor mais recente, nunca uma fila (ver `aviso` `seq_pulado`).

### 4.2. `evento`

```json
{"tipo":"evento","n":2,"classe":"matematicamente","seq":469818,"horario":"2026-10-04 21:31:05","abrangencia":"br","cargo":1,"candidato":13,"estado":"2t","topico":"br-c0001","t_envio":1791248122130000}
```

* `classe` `matematicamente`: uma linha do `eventos.csv`, com os mesmos campos (`seq`, `horario`, `abrangencia`,
  `cargo`, `candidato` — `null` para evento da circunscrição —, `estado`: `eleito`, `2t`, `matematicamente_eleito`,
  `matematicamente_2t`, ou com o prefixo `revogado:`), mais `motivo` nas revogações.
* `classe` `apuracao`: `estado` `apuracao_encerrada` ou `apuracao_reaberta` (com `motivo`), `abrangencia` e `cargo` `null`.
  Vão para toda conexão.
* `n`: número do evento neste processo do serviço (0, 1, 2…); a conexão nunca recebe o mesmo `n` duas vezes.
* Quem recebe: inscritos no tópico da circunscrição do evento (`<abrangencia>-c<cargo:4>`) e quem pediu `eventos`.

### 4.3. `estado`

Na conexão, a cada mudança (apurando ↔ encerrada, pausa por 429 e retomada, conferência concluída, novo tipo de
alerta) e pelo menos a cada 10 s. É o mesmo JSON de `GET /api/v1/estado`, com `"tipo":"estado"`:

```json
{"tipo":"estado","versao":"0.3.2","estado":"acompanhando","encerrada":false,"pleito":3220,"relogio_site":"04/10/2026 21:31:05",
 "secoes":{"catalogo":517179,"aplicadas":469818,"pendentes":0},"ultimo_bu":{"seq":469818,"site":"04/10/2026 21:31:04"},
 "pausa_429":{"ativa":false,"retomada_em":null},"conferencias":0,
 "alertas":{"destinacao_presumida":1},"clientes":1000,"t_envio":1791248122000000}
```

Só com `--controle-replay` (site simulado, §11) o `estado` traz também `"replay": {"acelerar": 60.0, "pausado": false,
"min": 1, "max": 300.0, "sessao": "65dd…", "busca": null}` e é reenviado na hora em que ele muda (o relógio vivo fica
em `GET /api/v1/replay`).

`alertas` repassa todos os tipos de alerta do serviço com a contagem (ex.: `secoes_esperadas_incoerentes` — o % de
seções de algumas UFs pode estar errado —, `destinacao_presumida`, `http429`, `reenvio_diferente`, `anulacao`,
`revogacao`). O texto de cada alerta fica no `alertas.csv` do serviço.

### 4.4. `aviso` e `erro`

```json
{"tipo":"aviso","motivo":"seq_pulado","topico":"br-c0001","de":470000,"para":470950,"juntadas":4}
{"tipo":"aviso","motivo":"desligamento"}
{"tipo":"erro","motivo":"topico_desconhecido","detalhe":"xx-c0001"}
```

* `seq_pulado`: a conexão não acompanhou e `juntadas` versões intermediárias do tópico foram descartadas desde o aviso
  anterior; a mensagem seguinte é o valor atual. **Nada a fazer**: o `resultado` é sempre completo (não é diferença).
  No máximo **um aviso por tópico a cada 5 s** por conexão (com a soma das juntadas). É normal também em clientes
  rápidos: na noite acelerada o `br-c0001` muda dezenas de vezes por segundo e o servidor manda a versão mais nova
  disponível no momento de escrever; `juntadas` muito maior que as mensagens recebidas é que indica cliente lento.
* `desligamento`: o serviço está parando (`SIGTERM`); a conexão será fechada (1001). Reconecte com espera.
* `erro`: motivos `json_invalido`, `op_desconhecida`, `topico_desconhecido`, `limite_topicos`, `taxa`, `binario` e
  (desde a `0.6.11`) `comparacao_desligada` (inscrição `tse:` num serviço com `--sem-tse`).

### 4.5. `marco` (desde a `0.4.1`)

Marcos e curiosidades da noite que só o servidor sabe com exatidão (qual seção entrou em cada lote, a conta do
"matematicamente"). Gerados na thread do motor (`/mnt/d/git/lukeboh/urna-velox/aquisicao/src/marcos.rs`), gravados
também em `<saida>/marcos.csv` (a conferência e a retomada usam esse arquivo: depois de um reinício o histórico volta).
Vão para quem pediu `{"op":"marcos"}`. Nada expõe pessoa: só seção, zona e município.

Do ensaio do 2º turno de 2024 a 60× (`/home/dev/uvreal/etapa5/fumaca1/saida/marcos.csv`; `n`, `restante` e `t_envio`
ilustrativos):

```json
{"tipo":"marco","n":2,"classe":"primeira_secao","seq":1,"horario":"2024-10-27 17:00:34","abrangencia":"am02550","cargo":null,"topico":null,
 "uf":"am","municipio":2550,"municipio_nome":"MANAUS","zona":58,"secao":934,"t_envio":1791946003120000}
{"tipo":"marco","n":31,"classe":"secao_decisiva","seq":14641,"horario":"2024-10-27 17:29:24","abrangencia":"pa04278","cargo":11,"topico":"pa04278-c0011",
 "estado":"matematicamente_eleito","uf":"pa","municipio":4278,"municipio_nome":"BELÉM","zona":29,"secao":636,"lote_bus":1,"t_envio":…}
{"tipo":"marco","n":57,"classe":"faltam_para_mat","seq":1,"horario":"2024-10-27 17:00:34","abrangencia":"am02550","cargo":11,"topico":"am02550-c0011",
 "faltam":1445701,"restante":…,"estado":"apurando","t_envio":…}
{"tipo":"marco","n":40,"classe":"virada","seq":655,"horario":"2024-10-27 17:07:50","abrangencia":"ce13730","cargo":11,"topico":"ce13730-c0011",
 "estado":"13:55","candidato":13,"anterior":55,"valor":1,"t_envio":…}
```

| `classe` | quando | campos próprios |
|---|---|---|
| `primeira_secao` | primeira seção totalizada do país (`br`), de cada UF e de cada circunscrição municipal (= primeira linha `aplicado` do `secoes.log` da abrangência) | `uf`, `municipio`, `municipio_nome`, `zona`, `secao` |
| `secao_decisiva` | a circunscrição virou "matematicamente eleito / 2º turno" (`estado`): a **última seção aplicada no lote que pertence à circunscrição**. O motor avalia por lote: a 1× os lotes têm 1–3 BUs; a 60× podem ter dezenas, e qualquer seção da circunscrição no mesmo lote poderia ser "a decisiva" (`lote_bus` = BUs do lote) | `estado`, `uf`, `municipio`, `municipio_nome`, `zona`, `secao`, `lote_bus` |
| `faltam_para_mat` | quantos eleitores ainda precisam ser apurados até o "matematicamente" ficar garantido, se o resto votar no pior caso (`urna_velox::matematica::faltam_para_mat`). Mandado quando muda, no máximo 1×/s por circunscrição, e **0 no `seq` exato do evento**. Nunca sobe enquanto só entram BUs (só com BU retirado/substituído ou mais votado não apto, art. 224). `faltam: null` = não se decide nem com 0 restantes (`estado` `indefinido`). **Não tem histórico**: guarda-se só o último de cada circunscrição; `n` é uma versão crescente própria | `faltam`, `restante`, `estado` (`apurando`, `matematicamente_eleito`, `matematicamente_2t`, `indefinido`) |
| `virada` | o 1º colocado de uma circunscrição mudou desde a verificação anterior (1×/s), depois de 1 % do eleitorado apurado | `candidato`, `anterior`, `valor` (nº da virada nesta disputa), `estado` = `"<novo>:<anterior>"` |
| `pct_secoes` | 25, 50, 75, 90, 99 e 100 % das seções apuradas do Brasil e de cada UF (conferido a cada 5 s; mesmas contas de `secoes` da publicação; `seq`/`horario` = os do momento da conferência) | `valor` (%), `secoes`, `total` |
| `bu_marco` | o BU nº 10 mil, 50 mil, 100 mil, 250 mil, 500 mil, 750 mil, 1 milhão da noite | `valor`, `uf`, `municipio`, `municipio_nome`, `zona`, `secao` |
| `ultima_secao` | a apuração encerrou: a última seção aplicada | `uf`, `municipio`, `municipio_nome`, `zona`, `secao` |

Campos comuns: `n` (nº do marco neste processo; recomeça depois de um reinício — deduplique por
`classe`+`abrangencia`+`cargo`+`seq`+`secao`+`valor`), `classe`, `seq`, `horario` (do BU, como nos eventos),
`abrangencia`, `cargo` (`null` nos marcos de abrangência), `topico` (`<abrangencia>-c<cargo:4>` ou `null`), `t_envio`.

### 4.6. `tse` — comparação com o site de resultados do TSE (desde a `0.6.11`, etapa 7)

Para ver a totalização do Urna Velox ao lado da do TSE. O cliente se inscreve em **`tse:<tópico>`** (o mesmo tópico da
§2, com o prefixo): `{"op":"inscrever","topicos":["tse:br-c0001"]}`. **É essa inscrição que liga a busca no
servidor:** enquanto houver pelo menos uma conexão inscrita em `tse:br-c0001`, o `uv-servico` busca no site do TSE o
arquivo de resultado desse tópico (`br-c0001-e006257-u.json`) no máximo uma vez a cada `--tse-intervalo` (padrão 30 s,
de 30 a 60; hora do site), não importa quantos clientes, e manda a mesma mensagem a todos; sem inscrito, nenhuma
requisição. `cancelar` com `tse:<tópico>` desfaz. Conta no limite de tópicos por conexão (§3). Sem a comparação
(`--sem-tse`): `erro` `comparacao_desligada`.

Na inscrição, uma mensagem com `"instantaneo": true` (o último arquivo, ou o estado `aguardando`); depois, uma a cada
arquivo novo do TSE ou mudança de `estado`:

```json
{"tipo":"tse","instantaneo":false,"topico":"tse:br-c0001","ver":7,"estado":"ok","motivo":null,"arquivo":"br-c0001-e006257-u.json",
 "buscado":"04/10/2026 20:42:01","novo_em":"04/10/2026 20:41:31",
 "dados":{"geracao":"04/10/2026 20:41:10","totalizacao":"04/10/2026 20:41:08","andamento":"p","md":"s","finalizada":false,
   "secoes":{"total":499248,"totalizadas":412000,"nao_instaladas":0},"eleitorado":158745502,"comparecimento":101000000,"abstencao":25000000,
   "vb":1500000,"vn":2900000,"vnt":0,"validos":96600000,
   "cand":[{"n":13,"v":44500000,"pct":46.07,"pos":1,"e":true,"st":"","dvt":"Válido"}, …]},
 "t_envio":1791248521000000}
```

| campo | significado |
|---|---|
| `estado` | `aguardando` (inscrito, primeira busca sem resposta), `ok` (arquivo obtido e confirmado — 200 ou 304 — há no máximo dois intervalos), `desatualizado` (sem resposta válida do TSE há mais de dois intervalos), `indisponivel` (404, erro de rede ou arquivo ilegível; `motivo` `404`, `erro`, `ilegivel`; ou sem vaga: `limite_topicos`), `pausado` (pausa da rede depois de um HTTP 429: nenhuma requisição ao TSE até o fim dela) |
| `arquivo` | nome do `-u.json` no TSE |
| `buscado` | hora do site da última resposta do TSE (200 ou 304) |
| `novo_em` | hora do site em que esta versão do arquivo (`geracao`) foi vista pela primeira vez |
| `dados` | `null` até o primeiro arquivo; depois, o resumo do **último arquivo válido** (fica mesmo com `estado` `indisponivel`/`desatualizado`/`pausado`) |
| `dados.geracao` | data e hora de geração do arquivo no TSE (`dg` `hg`) |
| `dados.totalizacao` | data e hora da totalização (`dt` `ht`) |
| `dados.andamento`, `md`, `finalizada` | `and` (`n`, `p`, `f`), `md` (matematicamente definido: `e` eleito, `s` 2º turno, `n`), `tf` = "s" |
| `dados.secoes` | `ts`, `st` (inclui as não instaladas, como no Urna Velox), `sni` |
| `dados.cand[]` | `n` número, `v` votos (`vap`), `pct` % dos votos a votáveis concorrentes (`pvapn`; sem ele, votos ÷ válidos), `pos` posição (1 = primeiro), `e` eleito/2º turno no arquivo, `st` situação (só com a totalização final), `dvt` destinação; **em ordem de `pos`** |
| `dados.cand[].pos` | posição pelos votos do arquivo; empate como no motor (o mais idoso à frente, `dt`), depois o `seq` e o número. O `seq` do TSE não é usado sozinho: nos arquivos de UF ele pode ser a ordem nacional |

Regra de rede (combinada com o Luciano em 16/10/2026; `docs/HANDOFF_0.7.0.md`): só o `-u.json` da abrangência e cargo
inscritos; no máximo uma busca por intervalo por tópico; no máximo `--tse-max-topicos` (padrão 200) tópicos buscados ao
mesmo tempo — o tópico novo espera vaga, `indisponivel` com `motivo` `limite_topicos`; `If-None-Match` e
`If-Modified-Since` (304 sem corpo); a mesma rede da coleta (100 req/s do órgão, pausa ≥ 10 min depois de um 429), com
prioridade abaixo dela. Na federação, cada nó busca para os seus clientes. A saúde (`saude.json`) tem o bloco
`tse_resultados` (tópicos ativos, inscritos, estados, buscas, req/s, 200/304/404, erros, `fora_do_intervalo` — a prova
da regra, sempre 0 — e a idade do arquivo mais antigo).

### 4.7. `previsao` — estimativa do "matematicamente" e do fim da apuração (desde a `0.7.7`, etapa 8)

Para cada tópico, o serviço estima **a que horas deve sair o "matematicamente"** (eleito ou 2º turno; só nos tópicos
que são a própria circunscrição) e **a que horas a apuração chega a 99 % e a 100 % das seções**, sempre como uma
**faixa** (P10–P90) e sempre como **estimativa** — nunca "resultado". O cálculo (`urna_velox::previsao`, método em
`/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.8.0.md` e a calibração em `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.8.0.md`)
roda numa thread própria a cada `--previsao-intervalo` segundos do relógio do serviço (padrão 30; no replay, o relógio
simulado), só para os tópicos que mudaram, e só com pelo menos 5 % dos aptos apurados e 10 min de apuração.

Quem está inscrito no tópico (§3) recebe, sem pedir nada: na inscrição, a última previsão com o histórico da noite
(`"instantaneo": true`); depois, uma mensagem a cada previsão nova do tópico (sem o histórico).

```json
{"tipo":"previsao","instantaneo":false,"topico":"br-c0001","abrangencia":"br","cargo":1,"circunscricao":true,"estimativa":true,
 "instante":"2026-10-25 18:44:00","situacao":"prevista","base":{"apurado":0.4691,"secoes":0.4853,"aptos_min":1410273},
 "mat":{"tipo":"matematicamente_eleito","p10":"2026-10-25 19:31:12","p50":"2026-10-25 19:52:40","p90":"2026-10-25 20:21:05","incerta":false,"eta":"2026-10-25 19:49:30",
   "prob_antes_fim":1.0,"apurado_p50":0.951},
 "fim99":{"p10":"2026-10-25 20:20:31","p50":"2026-10-25 20:52:06","p90":"2026-10-25 21:31:44","incerta":false,"eta":"2026-10-25 20:47:10"},
 "fim100":{"p10":"…","p50":"…","p90":"…","incerta":true,"eta":"…"},
 "chegou":{"fim99":null,"fim100":null},"definida":null,"confirmacao":null,"t_envio":1792975440123456}
```

| campo | significado |
|---|---|
| `instante` | hora (do relógio do serviço) em que a previsão foi feita |
| `situacao` | `prevista`; `suspensa` (desde a `0.7.8`: a chegada dos BUs parou no país, ver `parada`); ou `definida` (o "matematicamente" já saiu: `mat` vem `null` e `confirmacao` diz como a previsão se saiu) |
| `parada` | desde a `0.7.8`, só com `situacao` = `suspensa`: `{"desde":"aaaa-mm-dd hh:mm:ss","minutos":n}`. A parada começa quando o ritmo de chegada dos BUs no país nos últimos 5 min cai abaixo de 10 % do ritmo dos 30 min anteriores (entre 5 % e 97 % apurado) e termina quando volta a 30 % dele. Enquanto dura, nenhuma previsão nova é calculada: `mat`, `fim99` e `fim100` são os da última previsão normal, **adiados** pelo tempo parado (são o mais cedo possível, se a chegada voltasse agora). Mostre algo como "chegada de BUs parada desde hh:mm — previsão adiada". Com a volta, a previsão é recalculada e `parada` some |
| `base` | o que a previsão usou: fração dos aptos (`apurado`) e das seções (`secoes`) já apurados, e a velocidade em aptos por minuto (média móvel, meia-vida de 10 min) |
| `mat` | só na circunscrição ainda em aberto: `tipo` (`matematicamente_eleito` ou `matematicamente_2t`), a faixa `p10`–`p90` com a mediana `p50`, `incerta` (faixa maior que 3 h da noite), `eta` (B-052: horário pelo ritmo atual — o que falta até `apurado_p50` ÷ a velocidade média móvel —, como o ETA de um navegador de trânsito; com a faixa `incerta`, mostre ele: "por volta de …"; `null` só sem ritmo nenhum), `prob_antes_fim` (probabilidade de sair com pelo menos 1 % dos aptos ainda por apurar; perto de 0 = "deve ir até o fim") e `apurado_p50` (fração dos aptos com que deve sair) |
| `fim99`, `fim100` | horário (faixa) em que as seções apuradas chegam a 99 % e a 100 % das esperadas (sem agregadas); `null` quando já chegou (ver `chegou`). O 100 % depende das últimas seções (exterior, locais remotos): no Brasil costuma ficar `incerta`. Cada uma traz também o `eta` (horário pelo ritmo atual das seções) |
| `chegou` | quando o tópico chegou a 99 % e a 100 % das seções (`null` = ainda não) |
| `definida` | quando saiu o "matematicamente" da circunscrição (horário do BU, como nos eventos) |
| `confirmacao` | depois de `definida`: `saiu`, a última faixa prevista antes (`faixa`: `p10`, `p90`), se o horário real caiu nela (`dentro`) e `desde` quando todas as faixas previstas o continham (`null` se a última não continha) |
| `historico` | só no `instantaneo` (e no REST): `[[instante, p10, p50, p90], …]` das faixas do "matematicamente" ao longo da noite (no máximo um por minuto), em segundos civis de Brasília (a escala do `GET /api/v1/curva`) — para desenhar a faixa convergindo |

Horários em `"aaaa-mm-dd hh:mm:ss"` (Brasília, como `horario` dos eventos). A previsão é guardada só na memória do
serviço: depois de um reinício, recomeça (precisa de 10 min de série de novo). O registro de cada rodada fica em
`<saida>/previsoes.csv`; a saúde (`saude.json`) tem o bloco `previsao` (tópicos, rodadas, custo em ms).

### 4.8. `mapa` — mapa das seções (desde a `0.9.19`)

O servidor não sabe de geografia: dá a cada seção do catálogo um **índice** (a posição na lista ordenada pela chave
`município << 32 | zona << 16 | seção`, a mesma do `GET /api/v1/mapa/secoes`) e conta quais já foram aplicadas, na ordem
em que acenderam. A seção acende na **primeira** aplicação do BU (substituição e anulação não mudam o ponto); na
retomada, as seções do diário acendem de uma vez, com a faixa da disponibilização. Seção fora do catálogo (descoberta
na noite) não tem índice: fica de fora (contada em `fora`). A página põe cada índice no mapa com a geografia de
`/static/mapa.json` (`site/geo/README.md`).

```json
{"tipo":"mapa","epoca":1793037600123,"de":182345,"n":182351,"s":[40211,171,40212,171,301877,172,12,58,13,58,90001,172]}
```

| campo | significado |
|---|---|
| `epoca` | identifica o processo (ms da partida): outro processo, outra numeração das seções acesas |
| `de`, `n` | as seções acesas de `de` (inclusive) a `n` (exclusive) na ordem em que acenderam |
| `s` | pares `[índice, faixa]`; `faixa` = horário do BU no TSE em faixas de 6 min do dia (1 = 00:00–00:06, 171 = 17:00–17:06, 240 = 23:54–24:00) |
| `total` | (desde a `1.0.2`, B-064) seções do mapa, o mesmo total da página: a lista não tem as agregadas do `candidatos.json`, e as que se revelam agregadas na noite saem do total (índices em `ret` no `GET /api/v1/mapa/estado`). Mudou: o cliente relê a fotografia |

No máximo uma mensagem a cada 250 ms por conexão (juntando os lotes) e até 8.192 pares por mensagem (o histórico de
quem pede atrasado vai em várias, em ordem). `de` maior que o que o cliente tem = lacuna: busque a fotografia de novo.
A saúde (`saude.json`, bloco `difusao`) tem `mapa`: `secoes`, `acesas`, `fora_do_catalogo` e `fotografias`.

## 5. Vivacidade e reconexão

* O servidor manda **ping** a cada 20 s; sem nenhum quadro do cliente (pong ou mensagem) por 60 s, a conexão cai
  (motivo `sem_pong`).
* Cliente lento: o servidor junta versões (último valor por tópico). Se a conexão ficar **30 s** sem conseguir
  escrever, ou acumular mais de **4 MB** no buffer de escrita, cai (motivo `cliente_lento`).
* **Ao reconectar** (queda da rede, reinício do serviço, `desligamento`): espere 1 s, 2 s, 4 s… (até 30 s, com
  variação aleatória), reinscreva os tópicos e, se quiser, `eventos`. O `instantaneo` traz o estado atual: substitua o
  que tinha **se `seq` for maior ou igual**; o histórico de eventos vem de novo (descarte os repetidos por
  `seq`+`abrangencia`+`cargo`+`candidato`+`estado` — o `n` recomeça depois de um reinício do serviço).
* Depois de um `kill -9` e reinício, o serviço retoma do diário: o `seq` do `instantaneo` é ≥ ao último que o cliente
  recebeu antes da queda.

## 6. REST (só leitura, `Access-Control-Allow-Origin: *`)

| rota | resposta |
|---|---|
| `GET /api/v1/estado` | o JSON da §4.3 (sem `tipo`/`t_envio`), `Cache-Control: no-cache` |
| `GET /api/v1/topicos` | `[{"topico":"br-c0001","arquivo":"br-c0001-e006257.json","seq":480125}, …]` |
| `GET /api/v1/resultado/<topico>` | o arquivo publicado (mesmos bytes), `ETag: "<seq>"`, `Cache-Control: public, max-age=1`; com `If-None-Match` igual → **304** sem corpo; tópico inexistente → 404 |
| `GET /api/v1/eventos` | lista dos eventos (os objetos da §4.2, sem `tipo`/`t_envio`) |
| `GET /api/v1/marcos` | (desde a `0.4.1`) os marcos com histórico e o último `faltam_para_mat` de cada circunscrição (os objetos da §4.5, sem `tipo`/`t_envio`), `no-cache` |
| `GET /api/v1/nomes` | (desde a `0.4.1`) nomes para exibir, do `candidatos.json` (arquivos públicos do TSE, sem requisição nova): `{"pleito", "ufs": {"sp": "SÃO PAULO", "zz": "EXTERIOR"…}, "municipios": {"sp71072": "SÃO PAULO"…}, "eleicoes": [{"codigo", "turno", "cargos": [{"cargo", "vagas", "consulta", "abrangencias": [...], "circunscricoes": [{"abr", "eleitorado", "candidatos": [{"n", "nome_urna", "nome", "partido", "vices": [{"tp", "nome_urna", "partido"}]}]}]}]}]}`; `ETag` fixo por processo (304 com `If-None-Match`), `Cache-Control: public, max-age=60`. `tp` dos vices como no `-u.json`: `v` = vice, `1`/`2` = suplentes |
| `GET /api/v1/curva/<topico>` | (desde a `0.4.10`) a curva da apuração do tópico desde o começo da noite (ou do último reinício do processo), para a página desenhar o gráfico inteiro ao abrir ou recarregar: `{"topico", "passo_s", "relogio", "pontos": [[t, apuradas, total, [[n, pct], …]], …]}`. `t` = desde a `0.8.28` (M-003) o `totalizado_em` do resultado (relógio do sistema), em segundos civis de Brasília com fração (como `Date.UTC` da hora local; antes, o horário do último BU no TSE); `passo_s` = 60 s da noite no relógio do sistema (60 / aceleração), `apuradas`/`total` = `secoes`, e o `pct` de **todos** os candidatos (uma virada não troca as linhas de dono). Um ponto por 60 s de noite (o último é provisório e anda até completar o passo), amostrado a cada publicação do tópico; só memória, nada em disco. Tópico ainda sem publicação (ou inexistente) → 404 `{"erro":"topico_desconhecido"}`; `no-cache` |
| `GET /api/v1/mapa/secoes` | (desde a `0.9.19`) a lista das seções do catálogo do mapa (§4.8), fixa por processo: `{"v":1,"n":N,"ufs":["ac",…],"m":[município, uf, nº de zonas, zona, nº de seções, 1ª seção, diferença, diferença, …, zona, …, município, …]}` (`uf` = posição em `ufs`; seções de cada zona em ordem, a primeira inteira e as outras como diferença da anterior); o índice de uma seção é a posição dela nessa sequência. `ETag` (304 com `If-None-Match`), `Cache-Control: public, max-age=300`; sem o mapa → 503 |
| `GET /api/v1/mapa/estado` | (desde a `0.9.19`) fotografia do mapa das seções: `{"v":1,"epoca","total","n","fora","ret","h"}`, com `h` = um byte por índice em base64 (0 = apagada; senão a faixa da §4.8), `total` = seções do mapa sem as agregadas e `ret` = índices que saíram do total por se revelarem agregadas na noite (desde a `1.0.2`, B-064) e `n` = quantas acenderam (o `desde` do `op: mapa`); refeita no máximo 1×/s, `Cache-Control: public, max-age=5`; sem o mapa → 503 |
| `GET /api/v1/previsao/<topico>` | (desde a `0.7.7`) a última previsão do tópico com o histórico (o objeto da §4.7, sem `tipo`/`t_envio`/`instantaneo`); sem previsão ainda (pouco apurado, serviço com `--previsao-intervalo 0`) ou tópico inexistente → 404 `{"erro":"sem_previsao"}`; `no-cache` |

Nada mais: fora o controle de velocidade do replay (§11, que só existe contra o site simulado), nenhuma rota escreve, lista pastas ou usa caminho vindo do cliente (`<topico>` só é procurado na tabela
de tópicos). Como o REST se comporta como arquivo estático (`ETag` + 304 + `max-age` curto), um proxy/CDN na frente
funciona sem mudança (ADR-021).

## 7. Como medir a latência

Na mesma máquina do serviço (mesmo relógio): **BU → cliente** = `agora − t_bu`; **servidor → cliente** =
`agora − t_envio`. O `uv-carga` faz isso para N conexões e grava histogramas. Em máquinas diferentes, só com relógios
sincronizados (NTP/PTP) e somando o erro deles. Não compare com o `relogio_site` (no simulador ele corre acelerado).

## 8. Limites do servidor (opções do `uv-servico`)

| opção | padrão | |
|---|---|---|
| `--http <end:porta>` | (desligado) | liga a camada; exemplo `127.0.0.1:8080` |
| `--max-clientes` | 5.000 | conexões WebSocket simultâneas; acima disso, `503` no *handshake* |
| `--http-threads` | 4 | threads do runtime da camada HTTP (separadas da rede de coleta e do motor) |
| tempo para o *handshake* | 10 s | cabeçalhos HTTP incompletos depois disso: a conexão TCP é fechada |
| tópicos por conexão | 200 | |
| mensagens recebidas | 20/s, rajada 40 | |
| tamanho da mensagem recebida | 4 KB | |
| buffer de escrita por conexão | 4 MB | acima disso, `cliente_lento` |
| `--tse-intervalo` | 30 s | comparação com o TSE (§4.6): segundos (hora do site) entre duas buscas do `-u.json` de um tópico inscrito; de 30 a 60 (fora disso, o serviço não parte) |
| `--tse-max-topicos` | 200 | tópicos `tse:` buscados ao mesmo tempo; acima disso, o novo espera vaga (`indisponivel`, `limite_topicos`) |
| `--sem-tse` | (ligada) | desliga a comparação: nenhum tópico `tse:` |
| `--previsao-intervalo` | 30 s | previsão (§4.7): segundos do relógio do serviço entre duas rodadas; 0 desliga |
| `--previsao-threads` | 4 | threads da rodada de previsão (fora da thread do motor) |

## 9. Exemplos reais

Mensagens recebidas pelo `uv-carga` no ensaio D4 (2º turno de 2024 a 10×, `/home/dev/uvreal/etapa4/D4/amostras.jsonl`,
binário `0.3.3`), na ordem em que chegaram; o `resultado` vai inteiro (é o arquivo publicado, com o `\n` final antes
do `}`).

Ao conectar, o `estado` (o `relogio_site` é `01/01/1970 00:00:00` até a primeira resposta do site):

```json
{"versao":"0.3.2","estado":"acompanhando","encerrada":false,"pleito":453,"relogio_site":"01/01/1970 00:00:00","secoes":{"catalogo":98863,"publicadas_cs":0,"aplicadas":0,"nao_instaladas":0,"pendentes":0},"ultimo_bu":{"seq":0,"site":null},"pausa_429":{"ativa":false,"retomada_em":null,"pausas":0},"conferencias":0,"alertas":{"destinacao_presumida":1},"clientes":0,"tipo":"estado","t_envio":1791931973927043}
```

O `instantaneo` da inscrição em `am02550-c0011` (Prefeito de Manaus), antes do primeiro BU:

```json
{"tipo":"instantaneo","topico":"am02550-c0011","seq":0,"secoes_apuradas":0,"ver":1,"t_bu":null,"t_envio":1791931974216785,"resultado":{"v":1,"eleicao":620,"turno":2,"cargo":11,"abr":"am02550","circ":"am02550","seq":0,"ultimo_bu":null,"encerrada":false,"secoes":{"total":3992,"apuradas":0},"eleitorado":1446122,"apurado":0,"comparecimento":0,"vb":0,"vn":0,"vnt":0,"validos":0,"legenda":0,"estado":"apurando","cand":[{"n":70,"v":0,"pct":null,"dvt":"Válido","st":"","mat":null},{"n":22,"v":0,"pct":null,"dvt":"Válido","st":"","mat":null}]}
}
```

A primeira `atualizacao` (o primeiro BU da noite; `t_envio − t_bu` = 0,44 ms):

```json
{"tipo":"atualizacao","topico":"am02550-c0011","seq":1,"secoes_apuradas":1,"ver":2,"t_bu":1791931979822490,"t_envio":1791931979822928,"resultado":{"v":1,"eleicao":620,"turno":2,"cargo":11,"abr":"am02550","circ":"am02550","seq":1,"ultimo_bu":"2024-10-27T17:00:34-03:00","encerrada":false,"secoes":{"total":3992,"apuradas":1},"eleitorado":1446122,"apurado":384,"comparecimento":296,"vb":4,"vn":9,"vnt":0,"validos":283,"legenda":0,"estado":"apurando","cand":[{"n":70,"v":160,"pct":56.54,"dvt":"Válido","st":"","mat":null},{"n":22,"v":123,"pct":43.46,"dvt":"Válido","st":"","mat":null}]}
}
```

Um aviso de versão juntada e o primeiro evento da noite (com `op: eventos`, vêm os de todas as circunscrições):

```json
{"tipo":"aviso","motivo":"seq_pulado","topico":"am02550-c0011","de":417,"para":419,"juntadas":1}
{"n":0,"classe":"matematicamente","seq":14575,"horario":"2024-10-27 17:29:21","abrangencia":"pa04278","cargo":11,"candidato":15,"estado":"eleito","topico":"pa04278-c0011","tipo":"evento","t_envio":1791932152520511}
```

No fim: `evento` `apuracao_encerrada` (`classe` `apuracao`, para todos), as `atualizacao` com `"encerrada":true` e a
situação final (`st`), `estado` com `"encerrada":true`, `aviso` `desligamento` e o fechamento 1001.

## 10. Página de resultados embutida (desde a `0.4.1`)

`GET /` serve a página de exemplo da etapa 5 e `GET /static/<arquivo>` os recursos dela, de uma **tabela fixa
embutida no binário** (`/mnt/d/git/lukeboh/urna-velox/aquisicao/src/pagina.rs`; nenhum caminho do cliente chega ao
disco; fora da tabela → 404). Cabeçalhos: `Content-Security-Policy: default-src 'self'; connect-src 'self'; img-src
'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`, `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`; `index.html` com `no-cache`, estáticos com `public, max-age=31536000, immutable`
(o `index.html` os cita com `?v=<versão>`). A página só usa o que está neste documento (`/ws`, `/api/v1/nomes`,
`/api/v1/topicos`; desde a `0.8.44`, B-026, também `/api/v1/resultado/<topico>` e `/api/v1/estado`: com o `/ws`
recusado ou caindo 3 vezes seguidas, consulta a cada ~60 s o tópico da tela e o estado, com `If-None-Match`, e tenta
o WebSocket de novo a cada ~2 min; `?modo=http` na URL força esse modo, para conferir) e serve de modelo para integrar: fontes em
`/mnt/d/git/lukeboh/urna-velox/site/` (`site/README.md`).

## 11. Controle do replay: velocidade e posição na noite (desde a `0.4.8`; só contra o site simulado)

Com `uv-servico … --base http://127.0.0.1:<porta do uv-simulador> --http … --controle-replay` (recusado contra
qualquer servidor que não seja local, e sem `--http`), a noite simulada se controla como um vídeo. Roteiro pronto:
`aquisicao/scripts/simular_site.sh` (OPERACAO §8). Tempos: segundos na hora de Brasília, na escala de
`urna_velox::metricas::segundos` (o horário civil de Brasília lido como UTC: `new Date(t * 1000).toISOString()` dá o
horário de Brasília).

| rota | o quê |
|---|---|
| `GET /api/v1/replay` | `{"acelerar": 60.0, "pausado": false, "min": 1, "max": 300.0, "sessao": "65dd…", "busca": null, "t": 1791140400, "relogio": "04/10/2026 19:00:00", "inicio": 1791133664, "fim": 1791165600, "real": 60}`, `no-store`; sem `--controle-replay` → 404. `acelerar` = a velocidade escolhida; `real` = a do simulador agora (diferente durante uma busca); `t`/`relogio` = relógio do simulador (lido a cada 250 ms); `inicio`/`fim` = começo e fim da noite (primeira e última disponibilização); `busca` = `{"de", "alvo"}` durante um avanço; `sessao` muda a cada reinício |
| `POST /api/v1/replay/velocidade` | corpo `{"acelerar": x}` com `x` = 0 (pausa: o relógio do simulador para) ou de 1 a **300** (desde a `0.4.9`; medido: a 200× a fila de coleta fica em até 4,5 mil seções no pico, a 300× em até 68 mil — ~3,5 s de atraso, recuperado em seguida —, a 600× em 152 mil); resposta igual à do `GET` |
| `POST /api/v1/replay/posicao` | (desde a `0.4.9`) corpo `{"t": segundos}`, limitado a `inicio`–`fim`. **Instante já visto** (para trás, ou para a frente até onde a simulação já chegou): `{"reinicio": true, …}` — o serviço para, guarda o diário das seções no diário-mestre do replay (`<dados>/replay_diario.log`), apaga `--saida` e recomeça, **retotalizando do disco** (os BUs já baixados estão nos pacotes de `--dados`) todos os BUs com disponibilização até o instante, em ordem cronológica, em blocos de 30 s da noite — eventos e marcos com o horário do bloco —, sem nenhuma requisição; o simulador vai para o instante e a apuração segue ao vivo dali; a `sessao` muda e as páginas se recarregam. **Além do que já foi visto**: retotaliza até o ponto mais distante visto e, dali, **busca**: o simulador anda a 300× (até 900× com a fila de coleta quase vazia) até o instante e lá volta sozinho à velocidade escolhida |

Medido no 1º turno de 2026 (14/10/2026): retotalizar até as 18:30 (168 mil BUs) leva ~2 s e a página volta em 4–8 s;
até as 19:30 (411 mil), 5 s; a noite inteira (499 mil BUs), ~6 s de retotalização e ~10 s até a página voltar (até 16 s
com a máquina ocupada por outros ensaios) — e confere com os oficiais (83/83, 832/832).
Os eventos da retotalização são os mesmos 144 dos ensaios contínuos, com o horário da disponibilização do bloco que os
decidiu — em média alguns minutos antes do horário dos ensaios contínuos a 60×, que inclui a fila de coleta.

O serviço comanda o simulador por caminhos que não existem no site real: `POST /controle/velocidade?x=N[&ate=T&depois=M]`
(com `ate`/`depois`: anda a `N`× até `T` e lá passa a `M`×), `POST /controle/posicao?t=T[&x=N]` e `GET /controle`. O
relógio do simulador não salta ao mudar de velocidade e para sozinho 6 h depois dos oficiais. As esperas do serviço
(intervalo dos `cs.json`, novas tentativas) seguem a velocidade atual, inclusive as que já estavam correndo.

Segurança: sem `Access-Control-Allow-Origin` — só a própria página comanda o replay (corpo `application/json`, senão
415; `Origin` diferente do endereço da página → 403; outra origem precisaria da pré-verificação CORS, que dá 405);
valores fora da faixa → 400; simulador sem resposta → 502. **Ir a um instante já visto apaga `--saida`**: use pastas
descartáveis. Exemplos:

```
curl -X POST -H 'Content-Type: application/json' -d '{"acelerar":60}' http://localhost:18080/api/v1/replay/velocidade
curl -X POST -H 'Content-Type: application/json' -d '{"t":1791140400}' http://localhost:18080/api/v1/replay/posicao
```
