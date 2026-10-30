# Relatório da etapa 7 — comparação Urna Velox × TSE (`0.7.0`)

*17/10/2026. Plano: [HANDOFF_0.7.0.md](../handoffs/HANDOFF_0.7.0.md). Trabalho da `0.6.10` à `0.6.24` (a `0.6.15`–`0.6.20`, `0.6.22` e `0.6.23` são de outra sessão: licença e cópia pública); evidências arquivadas em
`D:\dados\urna-velox\ensaios\20261017-14xx-etapa7-*` (`scripts/disco.sh arquivar`).*

## Resumo

| Critério do handoff | Resultado | Ensaio |
|---|---|---|
| 1. Simulador 2024-2T a 10×, 10 abas | 10/10 telas idênticas ao oficial nas duas colunas, 51/51, CLS 0 | `etapa7-f2024` (0.6.21) |
| 1. Simulador 2026-1T a 60×, 10 abas | 10/10 telas idênticas, 83/83, **CLS 0 nas 10 abas** | `etapa7-f2026` (0.6.24) |
| 2. Rede | `fora_do_intervalo` 0 em todos; 0 buscas de arquivo fora das telas; 0 HTTP 429; BU → publicação p99 23,5 ms (2026) e 3,0 ms (2024) | os quatro |
| 3. Site real, ≥ 30 min, 3 telas | 3220: 3/3 telas iguais aos arquivos, 240 buscas (237 × 304), 0 HTTP 429; 3221: 3/3, 243 buscas (240 × 304), 0 HTTP 429 | `etapa7-real3220`, `etapa7-real3221` (0.6.21) |
| 4. Marco "TSE confirmou" × `eventos.csv` | 10/10 (2024) e 8/8 (2026) com o horário do `eventos.csv` | `etapa7-f2024`, `etapa7-f2026` |

## O que entrou

* **0.6.10 — não instaladas.** As 41 seções do exterior sem BU do 1º turno de 2026 (`s.sni` = 41, `e.esni` = 423 no
  `br-c0001-e006257-u.json` final) continuam esperadas até o `aux.json` dizer "Nao instalada"; então saem das que
  faltam e contam como totalizadas, como no oficial (regra do Luciano, 16/10). A apuração encerra com 499.248 seções.
* **0.6.11 — servidor e protocolo.** `aquisicao/src/tse_resultados.rs`: o `uv-servico --http` busca o `-u.json` só dos
  tópicos inscritos em `tse:<tópico>`, ≤ 1 vez por `--tse-intervalo` (30–60 s, hora do site), cache compartilhado,
  `If-None-Match`/`If-Modified-Since`, prioridade abaixo da coleta, pausa de 429; mensagem `tse` com estado
  `aguardando`/`ok`/`desatualizado`/`indisponivel`/`pausado`; bloco `tse_resultados` no `saude.json`;
  `Oficial::de_bytes` no motor (conferências sem mudança).
* **0.6.12 — TSE simulado.** `uv-simulador --tse-simulado`: `-u.json` progressivos com atraso de divulgação
  log-normal (mediana e p90 de `Opcoes::padrao` em `tse_simulado.rs`), byte a byte o oficial no fim. Posição do TSE pelos votos (o `seq` do arquivo
  não é a ordem).
* **0.6.13 — página.** Controle "Urna Velox · Ambos · TSE" (`localStorage` em `try/catch`, `?comparar=`), colunas lado
  a lado com posição, %, votos e diferença, linha destacada quando a ordem diverge, cabeçalho com as seções de cada
  fonte e a idade do arquivo do TSE, marco "TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min
  antes", celular sem rolagem horizontal.
* **0.6.14** — roteiro do site real (`ensaio_tse_real.sh`), desempate da posição como no motor, `desatualizado` na
  hora do site, `--sem-coleta` sem a conferência do fim (que baixaria todos os `-u.json`).
* **0.6.21** — `--sem-coleta` aplica os BUs do acervo (sem isso, a coluna do Urna Velox saía zerada no ensaio do site
  real) e selo de posição do TSE com largura fixa.
* **0.6.24** — `site/dist` regerado com o selo novo (o binário embute o `dist`: a correção da 0.6.21 não tinha chegado
  ao serviço) e `ensaio_tse_real.sh` com `--aceitar-incompleto` no 3221 e parada imediata se o serviço não subir.

## Critério 1 e 4 — simulador

**2024-2T a 10×** (`etapa7-f2024`): 10 abas (ambos, só TSE e alternando; duas no celular), 5 municípios. No fim, as
duas colunas iguais ao oficial em 10/10, conferência do serviço 51/51, CLS 0, sem rolagem horizontal. Marcos "TSE
confirmou" 10/10 com o horário do `eventos.csv`, o Urna Velox 2–4 min antes do TSE.

**2026-1T a 60×** (`etapa7-f2026`): 10 abas em Brasil/Presidente, SP/Governador, MG/Senador, RJ/Governador e
exterior/Presidente. 10/10 idênticas, 83/83, **CLS 0 nas 10 abas**. Marcos 8/8 com o `eventos.csv` (o exterior não
tem "matematicamente").

* **CLS.** No ensaio de 16/10 (`etapa7-c2026`, 0.6.13) o CLS máximo foi 3,4·10⁻⁵, sempre no mesmo elemento
  (`tse-l1`, 112 ↔ 117 px): o selo de posição do TSE tinha `min-width` e alargava na passagem de "9º" para "10º"
  (Presidente tem 12 candidatos). Largura fixa (32 px; 24 px no celular) na 0.6.21; a primeira repetição ainda deu
  2,5·10⁻⁵ porque o `site/dist` não tinha sido regerado; com o `dist` novo (0.6.24), CLS 0.
* **SP com o TSE 7–8 min "antes" do Urna Velox.** Não é defeito da comparação: a 60×, o pico da noite (≈ 100 mil
  seções entre 19h e 19h30) passa do que o simulador serve na máquina (~5.700 req/s reais), e entre 19h e 20h10 o
  Urna Velox ficou 15 a 45 min atrás na hora do site (medido no `secoes.log` de `etapa7-c2026`); o TSE simulado aplica
  o atraso sobre o recebimento real. Do pico em diante o atraso cai para 20–60 s e os marcos voltam a ficar 3–31 min à
  frente do TSE. Na noite real o Urna Velox coleta a ≤ 95 req/s com o atraso de disponibilização de ~1 s
  ([RELATORIO_SIMULADO_3220_ORDEM.md](RELATORIO_SIMULADO_3220_ORDEM.md)).

## Critério 2 — rede

| Ensaio | Buscas da comparação | Curtas* | Fora das telas | Depois de fechar | HTTP 429 | `fora_do_intervalo` | BU → publicação p99 |
|---|---|---|---|---|---|---|---|
| f2024 (5 arquivos) | 1.808 | 0 | 0 | 1** | 0 | 0 | 3,0 ms (exec. inteira 1,4 ms) |
| f2026 (5 arquivos) | 3.271 | 8 | 0 | 0 | 0 | 0 | 23,5 ms (exec. inteira 25,7 ms) |
| real3220 (3 telas) | 240 | — | 0 | — | 0 | 0 | — |
| real3221 (3 telas) | 243 | — | 0 | — | 0 | 0 | — |

\* buscas do mesmo arquivo a menos de 0,95 intervalo no log do simulador: são as da **conferência do fim**, que baixa
todos os `-u.json` (e repete a cada minuto enquanto o oficial ainda está incompleto); o contador do próprio serviço,
`fora_do_intervalo`, que só vê as buscas da comparação, ficou em 0.
\*\* um 304 do PA 2,3 s depois de fechar as abas (tolerância do analisador: 2 s): a busca já estava agendada quando a
conexão caiu; a seguinte não aconteceu.

**BU → publicação sem regressão.** Referência mais próxima: o ensaio E1 da etapa 5 (2026-1T a 60× com 10 abas, sem
comparação, 0.4.0): p99 das últimas 50 mil 34,9 ms, execução inteira 17,3 ms. Com a comparação ligada: 23,5 / 25,7 ms
(0.6.24), 29,3 / 35,9 ms (0.6.13). Tudo abaixo de 50 ms; a medida da execução inteira oscila de um ensaio para outro
(o pico da noite a 60× satura a máquina). Uma medida igual-para-igual exigiria repetir o mesmo ensaio com `--sem-tse`.

## Critério 3 — site real

`ensaio_tse_real.sh --confirmo-tse --combinado` (horário liberado pelo Luciano em 16/10, "a qualquer tempo"):
`uv-servico --sem-coleta --http` contra `https://resultados.tse.jus.br` e 3 abas (Brasil/Presidente em "ambos" no PC,
um governador em "só TSE" e outro alternando, no celular). Nenhum `cs.json`, `aux.json` ou BU: a única coisa que foi ao
TSE foram as buscas da comparação.

* **3220** (1º turno, encerrado; partida dos pacotes do acervo, 499.207 BUs, votos de Presidente idênticos ao oficial):
  40 min, 240 buscas — 3 × 200 e 237 × 304 (o `If-None-Match` funciona na CDN real) —, 0 erros, 0 HTTP 429; telas
  Brasil 499.207 de 499.248 seções (as 41 não instaladas não se sabem sem coleta) e TSE 100 %, AC 2.270/2.270, AL
  7.101/7.101; 0 divergências; CLS 0.
* **3221** (2º turno, pré-apuração): 40 min, 243 buscas — 3 × 200 e 240 × 304 —, 0 erros, 0 HTTP 429; telas zeradas nas
  duas fontes, iguais aos arquivos; CLS 0.
* Os ensaios duram 40 min (limite do teste) e não 30: o teste espera a apuração do Urna Velox encerrar, o que sem coleta
  não acontece. O marco "TSE confirmou" sai com minutos negativos no 3220 porque o Urna Velox partiu já com o resultado
  final; ali ele não mede nada.

## Achados e pendências

* **404 de `/api/v1/curva/<tópico>`** em todas as abas, no carregamento: o servidor responde 404 enquanto o tópico não
  tem nenhum ponto de curva (desde a 0.4.10), a página trata e semeia a curva depois. Benigno; aparece só como erro no
  console. Pode virar 200 com `pontos: []` se incomodar.
* **Primeira tentativa do site real (16/10)** parou antes de testar: o serviço partia com 0 BUs (sem diário) e o
  Chromium morria com a sessão do WSL. ≤ 2 buscas ao TSE. Corrigido na 0.6.21.
* **3221 com `candidatos.json` incompleto** (falta o `cs.json` de TO, preparado em 12/10): o serviço recusa sem
  `--aceitar-incompleto`. Para a noite de 25/10, rodar `uv-coletor preparar` de novo antes (item do ensaio geral).
* **Commits com título errado:** `81c8462` (0.6.10) e `059faaa` (0.6.14) saíram com o título do commit anterior (arquivo
  de mensagem antigo). O conteúdo e o CHANGELOG estão certos; o histórico não foi reescrito.

## Fechamento

Critérios 1 a 4 atingidos. Fechamento autorizado pelo Luciano em 17/10/2026: `0.7.0`.
