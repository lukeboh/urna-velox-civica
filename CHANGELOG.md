# Changelog

Todas as versões do Urna Velox. Regras em [docs/VERSIONAMENTO.md](docs/VERSIONAMENTO.md): cada commit sobe a versão; só as versões `X.Y.0` ganham tag (`vX.Y.0`; até a `0.3.0` todo commit ganhava a sua); o MINOR acompanha a última etapa concluída do [ROADMAP](ROADMAP.md) (etapas 1 a 9 = `0.1.0` a `0.9.0`, com a escalabilidade segura na etapa 6, a comparação com o TSE na 7, a previsão na 8 e o ensaio geral na 9 desde a renumeração de 16/10/2026; nas entradas anteriores à `0.6.7`, "etapa 7" / `0.7.0` = ensaio geral, hoje `0.9.0`; majoritários completos = `1.0.0`; proporcionais = `2.0.0`; até a 0.0.19 valia o esquema antigo de seis fases). As entradas `2.0.0-alpha.N`, abaixo da `0.6.0`, são o trabalho da etapa 6 feito na branch `feature/2.0-escalabilidade` antes da renumeração de 15/10/2026; nelas, "2.0" quer dizer a etapa 6 e "3.0", os proporcionais (hoje `2.0.0`).

## [0.6.23] — 2026-10-17

- **Capa do repositório** ([docs/img/capa.png](docs/img/capa.png), 1280×640): logotipo, frase do projeto e a página
  no celular; no topo do README e pronta para a prévia social do GitHub (Settings → Social preview, envio manual).
  Fonte em `docs/marca/capa/` e gerador `site/gerar_capa.mjs` (Playwright); a captura do celular tem o nome do
  candidato da legenda desfocado, pela neutralidade da marca (GUIA_MARCA §6).
- Handoff da cópia pública: a base da próxima cópia é o hash citado na mensagem do último commit do `urna-velox-civica`.
- Sem mudança em totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.6.22] — 2026-10-17

- Preparação da cópia pública (`urna-velox-civica`); o plano e a varredura ficam só no repositório original.

## [0.6.21] — 2026-10-17

- **`uv-servico --sem-coleta` publica o acervo** (etapa 7, critério 3): sem diário de seções (pacotes do acervo por
  links, como no `ensaio_tse_real.sh`), a partida aplicava 0 BUs — todos ficavam "órfãos" — e a coluna do Urna Velox
  saía zerada no ensaio contra o site real (achado da primeira tentativa, 16/10, que parou antes de testar: ≤ 2 buscas
  ao TSE). Agora, só com `--sem-coleta`, aplica o único BU de cada seção sem linha no diário: "Totalizado" primeiro; o
  dos dados abertos (situação 0 no pacote, sem `aux.json`) quando é o único; cópias idênticas (mesmo SHA-256) contam
  uma vez; seção com BUs diferentes fica de fora, com alerta `sem_coleta_pacotes`. Sem `--sem-coleta`, nada muda.
  - **Conferência:** partida do 3220 com `~/uvreal/dados` — 499.207 BUs (496.530 "Totalizado" + 2.677 dos dados
    abertos), 0 de fora; votos de Presidente no Brasil idênticos ao `br-c0001-e006257-u.json` oficial (12/12); 0
    requisições (base local inexistente). As 41 não instaladas não se sabem sem coleta: 499.207 de 499.248 seções.
- **Selo de posição do TSE com largura fixa** (`site/src/estilo.css`): com `min-width`, a passagem de "9º" para "10º"
  alargava o selo e empurrava a % ao lado — CLS de 2,3·10⁻⁵ a 3,4·10⁻⁵ nas telas de Presidente do ensaio
  `etapa7-c2026` (único elemento: `tse-l1`, 112 ↔ 117 px no PC, 84 ↔ 88 px no celular). Agora 32 px (24 px no celular),
  que cabem "10º"…"99º".
- `ensaio_tse_real.sh`: nota de que o roteiro tem de rodar com a sessão aberta (o Chromium do teste morria junto com a
  chamada do WSL que o iniciou).
- **Nota:** o commit da `0.6.14` (`059faaa`) saiu com o título da `0.6.13` (arquivo de mensagem antigo); o conteúdo e o
  CHANGELOG estão certos.
- Suítes: unidade do workspace sem falha nem aviso (`compilar_dev.sh`).

## [0.6.20] — 2026-10-17

- Preparação da cópia pública (`urna-velox-civica`); o plano e a varredura ficam só no repositório original.

## [0.6.19] — 2026-10-17

- Preparação da cópia pública (`urna-velox-civica`); o plano e a varredura ficam só no repositório original.

## [0.6.18] — 2026-10-17

- Preparação da cópia pública (`urna-velox-civica`); o plano e a varredura ficam só no repositório original.

## [0.6.17] — 2026-10-17

- Preparação da cópia pública (`urna-velox-civica`); o plano e a varredura ficam só no repositório original.

## [0.6.16] — 2026-10-17

- **Tradução da licença para o português do Brasil**: [`LICENCA.pt-BR.md`](LICENCA.pt-BR.md), com conteúdo idêntico ao
  texto oficial em inglês do [`LICENSE`](LICENSE) (mesmas 15 seções e as mesmas linhas `Required Notice:`; prevalece o
  inglês em caso de divergência); o `LICENSE`, o README, o LICENCIAMENTO e a ADR-031 apontam para ela.
- **Sem referências a outras licenças ou práticas incompatíveis com a PolyForm**: saem as menções a licenças permissivas
  por nome (README, LICENCIAMENTO, ADR-031, entradas antigas deste CHANGELOG); o aviso do LICENCIAMENTO
  deixa de tratar como violação qualquer uso por empresas ou veículos (só o uso comercial sem contrato, com o prazo de
  32 dias da cláusula *Violations*); o LICENCIAMENTO deixa claro que contratos
  valem só para o uso comercial e não revogam o uso não comercial. Ficam as licenças de terceiros exigidas por elas
  mesmas (fonte Archivo, SIL OFL, em `site/dist/LICENCA-FONTE.txt`; dependências).
- Sem mudança em totalização ou resultado: conferências não se aplicam.

## [0.6.15] — 2026-10-17

- **Licença: PolyForm Noncommercial 1.0.0 com dupla licença** (ADR-031). O [`LICENSE`](LICENSE) passa a ser o texto
  oficial e integral da [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0/)
  (conferido entre o repositório oficial da PolyForm, tag `1.0.0`, e a lista SPDX), com as linhas `Required Notice:`
  de copyright (2026, Luciano Soares Bohnert) e de licença comercial; substitui a licença própria "de Uso Restrito,
  Cívico e Não Comercial". Uso comercial, direto ou indireto, exige contrato de licenciamento comercial com royalties.
  - `README.md`: badges PolyForm Noncommercial e Source-Available; seção nova **"Licença e Termos de Uso"**
    (uso não comercial gratuito, uso comercial vedado sem licença, contato para licenciamento); "código 100% aberto"
    → *source-available*; nota sobre contribuições sob as duas licenças.
  - `docs/ARCHITECTURE_DECISIONS.md`: ADR-031 (classificação Source-Available / PolyForm Noncommercial).
  - `docs/LICENCIAMENTO.md`: modelo de dupla licença e nome do titular.
  - Manifestos: `license = "PolyForm-Noncommercial-1.0.0"` (SPDX) no lugar de `license-file` (`motor`, `aquisicao`),
    e em `bench/rust` e `bench/js`.
- Sem mudança em totalização ou resultado: conferências não se aplicam.

## [0.6.14] — 2026-10-16

- **Ensaio da comparação contra o site real** (etapa 7, critério 3 do `docs/HANDOFF_0.7.0.md`): roteiro novo
  `aquisicao/scripts/ensaio_tse_real.sh --confirmo-tse --combinado <nome> <3220|3221> [minutos]` — `uv-servico
  --sem-coleta --http` contra `https://resultados.tse.jus.br` e 3 abas da página (2 no PC, 1 no celular). A única coisa
  que vai ao TSE são as buscas da comparação (o `-u.json` de cada tela, ≤ 1 a cada 30 s por tópico). Horário liberado
  pelo Luciano em 16/10/2026 ("a qualquer tempo").
  - `uv-servico --sem-coleta` não faz mais a conferência automática do fim: ela baixaria todos os `-u.json` do pleito,
    fora do combinado (só os da tela); registra `conferencia_sem_coleta` no log.
  - `site/testes/comparacao.mjs --minimo <s>`: acompanha pelo menos esse tempo mesmo com tudo pronto (o 3220 já
    encerrado); `--oficial` opcional.
- **Posição do TSE com o desempate do motor**: votos iguais → o candidato mais idoso (`dt`) à frente, depois o `seq` e
  o número (`tse_resultados::resumo`; teste `resumo_empate_vai_para_o_mais_idoso`; `docs/PROTOCOLO_WS.md` §4.6).
- **Estado `desatualizado` na hora do site**: dois intervalos sem resposta válida contados no relógio do site (ou dois
  intervalos inteiros de tempo real, se ele não andou), a mesma conta da busca — no simulador acelerado, o estado não
  pisca entre `ok` e `desatualizado`.
- Suítes: unidade do workspace sem falha (`compilar_dev.sh`); o motor só mudou um comentário (`oficial.rs`), as
  conferências não mudam.

## [0.6.13] — 2026-10-16

- **Comparação Urna Velox × TSE na página** (etapa 7, `docs/HANDOFF_0.7.0.md` item 4). Módulo novo
  `site/src/comparacao.js`; `app.js`, `textos.js`, `estilo.css`.
  - Controle **"Urna Velox · Ambos · TSE"** no cartão da corrida (a escolha fica no `localStorage`, em `try/catch`, e
    vem de `?comparar=uv|ambos|tse` na URL). Com "Ambos" ou "TSE" a página se inscreve em `tse:<tópico>` **só do
    tópico da tela e só com a aba visível** (`visibilitychange`); com "Urna Velox", nenhuma inscrição.
  - "Ambos": colunas lado a lado por candidato — Urna Velox (% e votos), TSE (posição, % e votos) e a diferença de
    votos (UV − TSE), todas em hodômetro; a linha ganha destaque de fundo quando a ordem das fontes diverge. "TSE":
    a lista, a apuração, os votos e o selo passam a ser os do arquivo do TSE.
  - Cabeçalho das fontes: seções de cada fonte, horário e idade do arquivo do TSE e o estado (`aguardando`,
    `desatualizado`, `indisponivel`, `pausado`; `comparacao_desligada` quando o serviço roda com `--sem-tse`).
  - Marco no feed **"TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes"** (eleito e 2º
    turno), e o aviso de ordem diferente entre as fontes.
  - Celular (≤ 640 px): a diferença desce para a segunda linha; sem rolagem horizontal.
- **Hodômetro "enxuto" sem pulo** (`site/src/hodometro.js`, `estilo.css`): os dígitos vazios à esquerda deixavam de ter
  largura e, quando um dígito aparecia ou sumia, os outros andavam por layout — CLS de 10⁻⁶ a 10⁻⁵ no cartão de votos e
  no "faltam para o matematicamente" (achado na fumaça desta etapa). A largura reservada agora não muda e o número
  desliza por `transform` (centralizado no anel da apuração: metade).
- Teste novo `site/testes/comparacao.mjs` (N abas em "Ambos", "TSE" e alternando, as duas últimas no celular; no fim
  todas em "Ambos", leitura da tela contra o `-u.json` oficial — votos, posição pelos votos, diferença 0 —, CLS, marco
  "TSE confirmou" e rolagem horizontal) e o ensaio `aquisicao/scripts/ensaio_comparacao.sh` +
  `analisar_comparacao.py` (critérios 1, 2 e 4 do handoff, sem nenhuma requisição ao TSE).
  - Fumaça (2º turno de 2024 a 120×, 3 abas): 3/3 telas iguais ao oficial nas duas colunas, CLS 0, 0 HTTP 429,
    BU → publicação p99 5,9 ms, marcos "TSE confirmou" 3/3 com o horário do `eventos.csv` (Urna Velox 3–4 min antes).

## [0.6.12] — 2026-10-16

- **TSE simulado no `uv-simulador`** (etapa 7, `docs/HANDOFF_0.7.0.md` item 5). `--tse-simulado <candidatos.json>`
  (módulo novo `aquisicao/src/tse_simulado.rs`): o simulador serve os `-u.json` **progressivos** de cada abrangência e
  cargo, como o site de resultados do TSE durante a apuração, regerados a cada `--tse-passo` (15 s da noite), com
  `ETag` e 304. Cada seção entra no arquivo do TSE com um atraso de divulgação sorteado depois do recebimento do BU
  (log-normal, teto 1 h, semente fixa); seções, comparecimento, brancos, nulos, votos e %
  por candidato, `dg`/`hg` (geração), `and`, `md` e `tf` saem do `Agregador`/`Publicador` do próprio motor; as não
  instaladas (`sni`/`esni`) e as agregadas contam como no oficial. Depois do último atraso, o arquivo servido é **byte
  a byte o oficial**. Sem a opção, nada muda. Voltar no tempo (replay) reconstrói o estado.
  - Ensaio novo `tse_simulado_progressivo` (2º turno de 2024 a 120×): 153 arquivos progressivos de um tópico, geração
    sempre crescente (≥ 10 distintas), o último = oficial, e o `md` do TSE depois do evento do Urna Velox (238 s antes
    no tópico do ensaio).
- **Posição dos candidatos na mensagem `tse` pelos votos do arquivo** (correção da `0.6.11`): o `seq` do `-u.json` não
  é a ordem do arquivo — nos arquivos de UF do 3220 é a ordem **nacional** (26 de 83 arquivos com `seq` fora da ordem
  dos votos) e nos municipais de 2024 é um sequencial que não vai de 1 a n. A posição passa a ser pelos votos, empate
  pelo `seq` e depois pelo número; sai o campo `posicao_do_arquivo` (`docs/PROTOCOLO_WS.md` §4.6).
- **Rede da comparação na hora do site**: a próxima busca de um tópico exige também que o relógio do site (o `Date` das
  respostas) tenha andado um intervalo — ou um intervalo inteiro de tempo real, se ele não andou (simulador parado:
  antes, a espera acelerada deixava buscar a cada 0,25 s real com o relógio parado; achado na fumaça desta etapa).
- **Conferência automática espera os arquivos completos**: com `secoes_totalizadas` < `secoes` no `-u.json` (o TSE
  ainda totalizando), a conferência do fim da noite tenta de novo a cada minuto (até 60 vezes; sem arquivo, até 10) e
  não alerta `conferencia_divergente` enquanto espera; `conferencia.json` ganhou `incompletos`.
- **`uv-servico --sem-coleta`**: nenhum `cs.json`, `aux.json` ou BU (nem sondagem) — só a publicação do que carregou na
  partida, a difusão e a comparação com o TSE. Para o ensaio contra o site real (3220 e 3221) sem coleta.
- `docs/OPERACAO.md` (TSE simulado, `--sem-coleta`, conferência). Suítes: servico 12/12, federacao 6/6; unidade do
  workspace sem falha.

## [0.6.11] — 2026-10-16

- **Comparação Urna Velox × TSE — servidor e protocolo** (etapa 7, `docs/HANDOFF_0.7.0.md`). Módulo novo
  `aquisicao/src/tse_resultados.rs`: o `uv-servico --http` busca no site de resultados do TSE o `-u.json` **só do
  tópico que algum cliente inscreveu em `tse:<tópico>`**, no máximo uma vez a cada `--tse-intervalo` (padrão 30 s, de
  30 a 60, hora do site; fora disso o serviço não parte), contado a partir da resposta anterior; cache compartilhado
  (uma busca serve todos os clientes); **nada sem inscrito** (a inscrição cai com o `cancelar` ou o fim da conexão);
  até `--tse-max-topicos` (200) tópicos ao mesmo tempo — o novo espera vaga; `--sem-tse` desliga. Regra combinada com
  o Luciano em 16/10/2026 (`CLAUDE.md`).
  - Rede: a mesma da coleta (100 req/s do órgão, pausa ≥ 10 min depois de um 429), com **prioridade abaixo da coleta**
    (`Limitador::esperar_baixa`: só pega uma vaga livre do limitador; depois de 10 s esperando, entra na fila normal) e
    `If-None-Match`/`If-Modified-Since` (`Rede::get_com`, `Resposta.modificado`).
  - Protocolo (versão 1, por acréscimo; `docs/PROTOCOLO_WS.md` §4.6): mensagem `tse` (instantâneo na inscrição e uma a
    cada arquivo novo ou mudança de estado) com votos, % e posição por candidato (a ordem `seq` do próprio arquivo),
    seções (totalizadas e não instaladas), comparecimento, brancos, nulos, geração do arquivo no TSE, hora da busca e o
    estado da fonte: `aguardando`, `ok`, `desatualizado` (sem resposta válida do TSE — 200 ou 304 — há mais de dois
    intervalos: os arquivos finais e os de pré-apuração não mudam e não podem parecer velhos; a idade do arquivo vai
    à parte), `indisponivel` (404, erro, ilegível, sem vaga) e `pausado` (pausa de 429). Erro novo
    `comparacao_desligada`.
  - `saude.json`: bloco `tse_resultados` (tópicos ativos, inscritos, estados, buscas, req/s, 200/304/404, erros,
    `fora_do_intervalo` — a prova da regra — e a idade do arquivo mais antigo).
  - Motor: `Oficial::de_bytes` (o parse das conferências, a partir da memória; `Oficial::ler` passa a usá-lo) e os
    campos novos do `-u.json` (`seq`, `pvap`/`pvapn`, `e` de cada candidato; `sni`, `esni`, `a`, `vv`, `dg`/`hg`,
    `and`, `md`). **Conferências sem mudança:** `cargo test --release -p urna-velox` e `-- --ignored` — integração
    (2026 83 abrangências, 2024 5.569 e 51 municípios, pré-eleição) 7/7 e matemática 6/6.
  - Ensaio novo `comparacao_tse_rede` (AM e RO do 2º turno de 2024 a 120×): 0 buscas antes da inscrição e 0 do tópico
    não inscrito; 145 buscas do inscrito, todas ≥ 30 s da noite uma da outra (a única mais curta é a conferência do
    fim), `fora_do_intervalo` = 0; 404 antes dos oficiais → `indisponivel`; com o arquivo, `ok` e o resumo igual ao do
    oficial; a segunda conexão no mesmo tópico recebe o instantâneo sem busca nova; 0 HTTP 429. Suítes: servico
    11/11, federacao 6/6; unidade do workspace sem falha.
  - `docs/OPERACAO.md` (comparação, opções e saúde).

## [0.6.10] — 2026-10-16

- **Não instaladas: a apuração encerra com as seções sem BU** (achado do simulado de 16/10, ROADMAP etapa 9; corrigido
  antes da comparação da etapa 7, a pedido do Luciano). As 41 seções do exterior sem BU do 1º turno de 2026 têm, no
  `aux.json`, a situação "Nao instalada" (coleta de 04/10), e o oficial final as conta: `s.sni` = 41, `e.esni` = 423
  eleitores, `s.st` = 499.248 = 499.207 + 41. O serviço já sondava as seções sem disponibilização, mas só depois de
  30 min (hora do site) sem BU novo — o simulado parou 18 min depois do último BU — e pedia também as ~1.364 seções
  agregadas (404).
  - **Regra (Luciano, 16/10):** enquanto o TSE não disser que a seção não foi instalada, ela continua esperada; quando
    o `aux.json` dela disser "Nao instalada", sai das que faltam e conta como totalizada, como no oficial — a qualquer
    hora da noite, até o fim da totalização. Coleta extra combinada: só o `aux.json` das seções sem BU, no máximo uma
    vez a cada 5 min por seção.
  - `servico.rs`: sondagem por UF, sem esperar silêncio, quando toda seção com disponibilização da UF está resolvida e
    faltam poucas para o `s.ts` (exterior: 5 % ou 50; Brasil: 50); nunca as agregadas (`nsp` do `cs.json`); cada seção
    de novo a cada 5 min (hora do site), ≤ 2 req/s, ≤ 1.500 por UF; 404, "sem BU" ou erro mantêm a seção esperada (não
    vira agregada nem pendente). Sem `nsp` no `cs.json` de uma UF, o comportamento anterior (só o exterior, uma vez,
    404 = agregada).
  - `uv-simulador`: o `cs.json` servido traz o `nsp` das agregadas, como o real.
  - Ensaio novo `nao_instaladas_exterior` (exterior de 2026 a 300×): 1.310 BUs + 41 não instaladas = 1.351 (= `s.st`
    oficial), encerra sozinho, conferência com o oficial idêntica, 0 pedidos de agregada, 49 sondagens (as 41 e 8 seções
    cujo BU chegou depois), 0 HTTP 429.
  - **Conferência:** `cargo test --release -p uv-aquisicao -- --ignored` (servico 10/10, federacao 6/6) e unidade
    do workspace sem falha; os votos não mudam (só a contagem de seções das abrangências com não instaladas, agora igual
    ao oficial).
  - `docs/OPERACAO.md` (Sondagem), `docs/RELATORIO_SIMULADO_3220_ORDEM.md` e ROADMAP (item marcado).

## [0.6.9] — 2026-10-16

- **ROADMAP: `1.1.0` — Multipleito** (pedido do Luciano, 16/10; depois da eleição): o serviço descobre os pleitos no
  `ele-c.json` do TSE sem `PLEITO` configurado e trata cada um pela situação — agendados só exibidos, em curso
  totalizados (vários ao mesmo tempo, na mesma cota de 100 req/s), encerrados com o resultado completo (a totalização
  do Urna Velox comparada com a do TSE, como na etapa 7) ou só o resultado final do TSE quando não houve totalização;
  seletor de pleito na página. O bloco `1.x` passa a começar na `1.2.0`. Só documentação.

## [0.6.8] — 2026-10-16

- **Simulado do 3220 na OCI na ordem de recebimento do TSE** (16/10, 07:18–10:16, binário `0.6.5`, item da etapa 9 —
  ensaio geral, `0.9.0`), [relatório](docs/RELATORIO_SIMULADO_3220_ORDEM.md): 499.207 BUs a ~95 req/s, 0 HTTP 429;
  atraso no recebimento p50 1 s (p99 291 s) e deslocamento p50 6 (p99 25), contra 244.842 s e 158.050 no simulado
  alfabético de 15/10; conferência 81/83 nos dois nós, com votos, 832 situações e 55 circunscrições do
  matematicamente idênticos ao oficial.
- **Achado: a apuração não encerra com seções sem BU.** 41 seções do exterior (ZZ) estão no catálogo, não são agregadas
  e nunca tiveram BU publicado; o serviço não as trata como não instaladas, então BR e ZZ ficam abaixo do esperado
  (tela em 99,99 %, "499.207 de 499.248"), a publicação não encerra e a conferência automática não roda — e o oficial
  conta essas seções como totalizadas (as 2 diferenças da conferência). Correção pendente em
  `aquisicao/src/servico.rs`, a combinar (ROADMAP, etapa 9).
- ROADMAP, etapa 9: simulado na ordem do TSE marcado como feito, item novo do encerramento e item da operação sem a
  porta 22 (`0.6.5`, `0.6.6`).

## [0.6.7] — 2026-10-16

- **Roadmap readequado (decisão do Luciano): duas etapas de usabilidade antes da `1.0.0`.** Só documentação; nenhum
  código, totalização ou resultado mudou (sem conferência a refazer).
  - **Etapa 7 (`0.7.0`, alvo 19/10) — comparação Urna Velox × TSE:** a página mostra lado a lado a totalização do
    Urna Velox e a do site de resultados do TSE (posição e contagem de cada candidato, seções de cada fonte, idade do
    arquivo do TSE), com um controle "Urna Velox · Ambos · TSE". Plano em [docs/HANDOFF_0.7.0.md](docs/HANDOFF_0.7.0.md).
  - **Etapa 8 (`0.8.0`, alvo 21/10) — previsão do matematicamente e do fim da apuração:** horário estimado, com
    intervalo P10–P90, pela velocidade de chegada dos BUs (aptos/min) e pela tendência dos votos por estrato, validado
    por backtest nas três noites. Plano em [docs/HANDOFF_0.8.0.md](docs/HANDOFF_0.8.0.md).
  - **Ensaio geral: etapa 7 → etapa 9 (`0.9.0`, alvo 23/10)**, para ensaiar a noite já com as telas novas;
    `docs/HANDOFF_0.7.0.md` → [docs/HANDOFF_0.9.0.md](docs/HANDOFF_0.9.0.md) (cabeçalho novo). Os itens do ensaio que
    não dependem das telas seguem em paralelo; o `0.9.0` só sai depois do `0.8.0`.
- **Regra de rede combinada (CLAUDE.md):** durante a apuração, a comparação pode buscar o `-u.json` **só da
  abrangência e cargo na tela** de algum cliente com ela ligada, **no máximo a cada 30–60 s** por (abrangência, cargo),
  com cache compartilhado, nada em segundo plano, dentro dos 100 req/s do órgão e com a regra do 429.
- Atualizados: [ROADMAP.md](ROADMAP.md) (tabela, plano, mapa da renumeração de 16/10, linha do tempo, etapas 7–9,
  `1.0.0`, metas), [CLAUDE.md](CLAUDE.md), [docs/VERSIONAMENTO.md](docs/VERSIONAMENTO.md), [docs/GLOSSARIO.md](docs/GLOSSARIO.md),
  [docs/OPERACAO.md](docs/OPERACAO.md), [README.md](README.md), [docs/HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md),
  [docs/HANDOFF_MERGE_2.0_MAIN.md](docs/HANDOFF_MERGE_2.0_MAIN.md) e o cabeçalho deste CHANGELOG.

## [0.6.6] — 2026-10-16

- **`uv-atualizador` não briga mais com a operação manual.** Na `0.6.5` o manifesto era reaplicado a cada 60 s mesmo
  sem série nova (só a série *menor* era recusada), então `60_simulado.sh` e `21_parar.sh` eram desfeitos em até um
  minuto. Agora:
  - o manifesto só é aplicado quando a série é **nova**; o `combinado_ate` continua sendo cobrado em toda rodada;
  - **trava manual** `/etc/urna-velox/controle-manual`: com ela, o atualizador não mexe no nó (`controle_manual`).
    `20_iniciar.sh` e `60_simulado.sh` criam a trava; `21_parar.sh` e `60_simulado.sh --restaurar` removem
    (`travar_manual`/`liberar_manual` em `lib.sh`); `30_verificar.sh` mostra o estado;
  - o nó que não é o primeiro do `federacao.json` espera `UV_ATRASO_PAR_S` (180 s) depois do `emitido_em` antes de
    aplicar (`aguardando_par`), para que um pacote ruim não derrube os dois nós juntos.
  - `testar_atualizador.sh`: 8 testes (os 5 de antes + série repetida, trava manual e atraso do par).
  - Instalado nos dois nós em 16/10 via Run Command e conferido (`em_dia`; trava e liberação testadas no nó A).
- **Chave de assinatura do manifesto:** `chave_manifesto` em `lib.sh` (usada por `10_implantar.sh` e `70_publicar.sh`)
  nunca gera chave sem senha (pede senha num terminal) e avisa se a existente não tiver.
- **Honestidade sobre o `sudo` do `ocarun`:** comentário em `servidor/preparar.sh` e correção no
  [relatório](docs/RELATORIO_DEPLOY_SEM_SSH.md) — a lista do `sudoers` equivale a root (`uv-operar.sh` faz
  `exec "$@"`; `tee`, `cp`, `systemd-run`...). A barreira real é o IAM; fechar via ações nomeadas é pendência (§7).
- **Relatórios corrigidos:** `RELATORIO_DEPLOY_SEM_SSH.md` (sudo, origem dos 83/83 — simulado de 15/10, binário
  `2.0.0-alpha.9` —, amostra de 19.062 BUs e não ~22.000, novas pendências no §7: `--estado simulado` nominal,
  ciclo de vida de 7 dias do manifesto, `combinado_ate` largo, `pollCommandInterval`, atualizador fora do pacote);
  `RELATORIO_SIMULADO_1T_2026.md` ganha nota: a saída de 15/10 confere 83/83 com o oficial, 0 divergências e
  0 revogações — os números escritos à mão no texto é que estavam errados.
- **Disco:** o `.vhdx` da distro foi compactado de 169,8 GB para 59,2 GB (C: com 194,5 GB livres); o `wsl --export`
  tentado antes falhou com o disco em uso e não deixou `.tar` para trás.

## [0.6.5] — 2026-10-16

- **Implantação OCI sem SSH — Parte A (`TRANSPORTE=oci`, Run Command + Object Storage)**
  ([docs/RELATORIO_DEPLOY_SEM_SSH.md](docs/RELATORIO_DEPLOY_SEM_SSH.md)):
  - IAM e bucket na região `sa-saopaulo-1`: grupo dinâmico `uv-nos-dg` (apenas os dois OCIDs de `uv-no-a` e `uv-no-b`),
    política de menor privilégio `uv-nos-policy` (Run Command e objetos do bucket) e bucket privado
    `urna-velox-implantacao` (`NoPublicAccess`, ciclo de vida `apagar-apos-7-dias`).
  - Nos nós OCI: `pollCommandInterval: '5s'` no plugin `runcommand` do `oracle-cloud-agent` (reduzindo a latência do
    padrão de 4 min para ~8–12 s), cliente Instance Principal `/opt/urna-velox/bin/uv-bucket` (em venv isolado
    `/opt/urna-velox/oci-venv`) e sudoers restrito `/etc/sudoers.d/urna-velox-ocarun` com lista fechada via
    `/opt/urna-velox/bin/uv-operar.sh` (sem `NOPASSWD: ALL`).
  - `deploy/oci/lib.sh` e `deploy/oci/nos.env.exemplo`: `TRANSPORTE=ssh|oci` intercambiável nas primitivas `em`, `para`
    e `de`; todos os scripts `10_implantar.sh` a `61_relatorio.sh` operam sem porta 22.
  - Verificação da `0.6.2` (`--ordem`) na OCI via `60_simulado.sh --confirmo-tse --combinado` e `61_relatorio.sh`:
    zero HTTP 429 no CDN do TSE, UFs processadas em ordem `recebido_tse`, atraso p50 de 1 s (contra 244.842 s antes da
    `0.6.2`) e 83/83 corridas idênticas ao oficial no simulado completo anterior.
- **Implantação OCI sem SSH — Parte B (`uv-atualizador` por pull + manifesto Ed25519)**:
  - `deploy/oci/servidor/uv-atualizador` + `urna-velox-atualizador.{service,timer}`: serviço systemd por pull a cada
    1 min (com `RandomizedDelaySec=15s`), lendo `manifesto.json` e `manifesto.json.sig` do bucket via Instance
    Principal, validando assinatura Ed25519 (`ssh-keygen -Y verify` contra `/etc/urna-velox/manifesto_autorizados`
    `0644 root:root`), número de série monotônico anti-rollback (`/var/lib/urna-velox/atualizador.serie`), SHA-256 de
    cada artefato e validade temporal `combinado_ate` antes de qualquer ação privilegiada.
  - `deploy/oci/70_publicar.sh`: empacota binários ARM64 e estáticos, gera `manifesto.json`, assina com
    `~/.ssh/uv_manifesto_ed25519` (namespace `urna-velox-manifesto`), envia ao bucket e limpa a chave efêmera; ambos os
    nós convergiram para `0.6.5` em ~75 s apenas via HTTPS (443), sem SSH e sem Run Command.
  - `deploy/oci/testar_atualizador.sh`: suíte automatizada que valida os 5 cenários de segurança do `uv-atualizador`
    (assinatura inválida, SHA-256 divergente, atualização válida, anti-rollback de série e expiração de
    `combinado_ate`).

## [0.6.4] — 2026-10-15

- **Disco da distro** (o C: lotou em 15/10): seção **Disco** no `CLAUDE.md` com o protocolo de antes e depois de cada
  ensaio (o que fica na distro, o que vai para o D:, que é HD; arquivar evidências; devolver o espaço ao C:).
- **`scripts/disco.sh`**: relatório (distro, `.vhdx` no Windows, regenerável), `--curto` (o `compilar_dev.sh` avisa
  acima de 5 GB), `--limpar` (só o regenerável: `dados/`, `federacao/`, `armazem/` dos ensaios, `target-*` antigos,
  temporários `uv-*` órfãos; e `fstrim`) e `arquivar <dir>` (evidências para `D:\dados\urna-velox\ensaios\`).
- **Ensaios de `aquisicao/tests/`** (`servico.rs`, `federacao.rs`): a pasta temporária de cada ensaio é apagada quando
  ele passa (fica se falhar; `UV_MANTER_ENSAIO=1` mantém) e, antes, as sobras de execuções anteriores (o nome leva o
  PID, que muda a cada execução: as pastas se acumulavam). Conferido com `dois_orgaos_no_mesmo_ip`: sobra antiga
  apagada antes, pasta do ensaio apagada depois. `simular_site.sh` apaga a cópia dos pacotes (~6 GB) ao sair.
- **Limpeza de 15/10, 23h**: 66 GB regeneráveis apagados (11 `dados/` de ensaios da etapa 5, `simulacao-site/dados`,
  `teste9/dados`, 4 builds antigos); a distro foi de 121 para 54 GB usados. O `.vhdx` (169,8 GB no C:) só encolhe com
  a compactação, pendente (a conta não tem administrador para o `Optimize-VHD`/`diskpart`).

## [0.6.3] — 2026-10-15

- **Handoff da implantação sem a porta 22** ([docs/HANDOFF_DEPLOY_SEM_SSH.md](docs/HANDOFF_DEPLOY_SEM_SSH.md)): na rede
  de onde se opera a porta 22 de saída é bloqueada e o Luciano não quer depender do portal da OCI. Parte A (para o simulado de
  16/10): Run Command da OCI pela OCI CLI e arquivos pelo Object Storage, com um transporte trocável (`ssh`|`oci`) no
  `deploy/oci/lib.sh`. Parte B (definitiva): os nós buscam um manifesto assinado (Ed25519) e se ajustam a ele, com trava
  de data para coletar do TSE. Sem contornar o firewall da rede de operação (nada de SSH na 443, Tailscale ou túneis). Medido no
  computador de operação: HTTPS de saída alcança a API e o Object Storage da OCI em São Paulo e o GitHub.

## [0.6.2] — 2026-10-15

- **Site: "Eleitores que faltam" estourava em noves.** O card mostra o número da circunscrição (Presidente numa UF: o
  do Brasil), mas o hodômetro reservava só os dígitos do eleitorado da abrangência da tela; acima disso ele mostra
  noves. Reproduzido na simulação curta: Presidente no Acre mostrava 999.999; agora 158.629.357. O hodômetro do card
  reserva 9 dígitos (é `enxuto`: os vazios não ocupam espaço). É o "999.999.999" visto no simulado de 15/10.
- **Seções agregadas** (simulado de 15/10: a apuração nunca chegava a 100 %): fora do total desde a `0.4.9`, que não
  estava no binário da branch implantado na OCI, e só quando o `candidatos.json` as marca (`preparar` da `0.4.9`+; o
  dos nós foi gerado lá com o binário antigo). Os do simulado e do 3221 locais marcam (17.931 e 17.787); `20_iniciar.sh`
  e `60_simulado.sh` recusam um `candidatos.json` sem `"agregada":true`.
- **Testes**: workspace sem falhas; hodômetros 6/6; binários aarch64 da `0.6.2` gerados.

## [0.6.1] — 2026-10-15

- **Ordem de totalização = ordem de recebimento no TSE.** No simulado de 15/10 (3220 contra a CDN) a coleta foi UF por
  UF em ordem alfabética: a cada ciclo dos `cs.json` o `uv-servico` enfileirava as seções novas UF por UF, e contra uma
  eleição encerrada o primeiro ciclo traz o país inteiro. Agora as seções novas de cada ciclo, de todas as UFs, entram na
  fila ordenadas pelo recebimento do BU no TSE (`--ordem <métricas>`, o `recebido_tse` dos `aux.json`) ou, sem arquivo,
  pela disponibilização do `cs.json`; empate pela UF e pela seção (teste `fila_na_ordem_de_recebimento`).
- **Recebimento do DF no 3220**: as métricas locais não tinham nenhuma das 6.969 seções do DF (no simulador elas iam
  para o fim da noite). `uv-coletor baixar --so-aux --ufs df` (só aux.json, nenhum BU; 7.050 requisições em 75 s a
  92 req/s, 0 HTTP 429). Arquivo de ordem do 3220 com as 499.207 seções com BU, de 04/10 17:08:44 a 07/10 15:00:09
  (`D:\dados\urna-velox\simulado-3220\ordem_3220.csv`, fora do repositório).
- **`aquisicao/scripts/ordem_totalizacao.py`**: compara a ordem aplicada (diário `secoes.log`) com a de recebimento no
  TSE (BUs fora de ordem e por quantos segundos, deslocamento de posição, trocas vizinhas, ordem de início das UFs).
  Simulação curta (4 UFs, 60×, binário novo): as UFs começam na ordem do TSE; 37 % dos BUs saem de ordem por
  segundos (p50 2 s, máximo 20 s de recebimento; deslocamento p50 1) — requisições simultâneas trocando vizinhos.
- **Site no celular** (medido a 390 e 360 px com Playwright; a 390 px a página rolava de lado): título do card sem
  passar da borda (`min-width: 0` no cabeçalho, fonte mais estreita e até duas linhas), aviso da faixa com reticência
  e o texto inteiro no `title`, "%" dentro do anel, caixa dos eleitores que faltam mais alta, números alinhados aos
  rótulos (hodômetro `enxuto`: dígitos vazios à esquerda sem largura, onde o número está sozinho na linha) e itens
  fixos do feed na largura da tela, com rótulo curto ("Governador DF: faltam"). Os hodômetros já giravam num sentido só
  e o dos eleitores que faltam no oposto desde a `0.5.0`; os nós da OCI rodavam o site antigo da branch. O `site/dist/`
  reconstruído também leva o ícone oficial da marca (`src/icone.svg` da `0.5.3`, que não tinha chegado ao `dist/`).
- **OCI**: `deploy/oci/60_simulado.sh` (para os nós, guarda a saída anterior em `/srv/urna-velox/anteriores/<data>`,
  envia `candidatos.json` e ordem do 3220, troca o pleito no `no.env` e inicia contra a CDN com `--confirmo-tse
  --combinado`; `--restaurar` volta ao pleito de produção) e `61_relatorio.sh` (conferência, 429 e `ORDEM.md`);
  `uv-iniciar.sh` aceita `ORDEM` e `CANDIDATOS`. Simulado de 16/10 contra a CDN combinado com o Luciano (baixa de novo
  os BUs do 3220).
- **Testes** (em série, sem outra carga): workspace 85/85 (inclui `fila_na_ordem_de_recebimento`); **conferência oficial
  do motor 13/13**; ensaios do serviço 9/9 (477 s); ensaios da federação 6/6 (271 s); hodômetros 6/6
  (`site/testes/hodometro.mjs`: sentido de giro, % que cai, regressivo, rajadas); página com a simulação curta, 93 s no
  desktop e no celular: CLS 0, sem rolagem horizontal. Binários aarch64 da `0.6.1` gerados
  (`deploy/oci/gerado/uv-binarios-aarch64.tar.gz`).

## [0.6.0] — 2026-10-15

- **Etapa 6 concluída: escalabilidade segura (federação de nós coletores)**, por decisão do Luciano, sem a validação
  independente prevista e com o critério A completo pendente (passa ao ensaio geral). A branch
  `feature/2.0-escalabilidade` (`2.0.0-alpha.1` a `2.0.0-alpha.14`, entradas abaixo) entra na `main`: módulo
  `aquisicao/src/federacao/` (um nó por órgão, HRW ponderado, grupos por IP de saída, troca direta de BUs entre os nós,
  sem storage compartilhado), `uv-servico --federacao`, `uv-simulador --limite-ip-rps`, implantação em `deploy/oci/` e
  o 1º simulado do 3220 na OCI ([relatório preliminar](docs/RELATORIO_SIMULADO_1T_2026.md)).
- **Renumeração do roadmap** (decisão do Luciano, 15/10): escalabilidade segura `2.0.0` → etapa 6, `0.6.0` (federação)
  e `1.x` (borda, quórum Ed25519, Wasm); ensaio geral `0.6.0` → etapa 7, `0.7.0`; proporcionais `3.0.0` → `2.0.0`.
  ROADMAP, VERSIONAMENTO, CLAUDE.md, ESCALABILIDADE_SEGURA e ARCHITECTURE atualizados; `docs/HANDOFF_2.0.0.md` →
  `docs/HANDOFF_0.6.0.md` e o antigo `docs/HANDOFF_0.6.0.md` (ensaio) → `docs/HANDOFF_0.7.0.md`, com nota de
  renumeração no topo de cada um e as referências corrigidas. Relatórios, ADRs e entradas antigas ficam como estão
  (mapa no ROADMAP).
- **Merge:** na branch, a `main` (`0.5.0` a `0.5.3`) entrou primeiro, com 16 conflitos resolvidos (código: rota
  `/controle` e IP de origem no simulador, escala atômica + nó federado no serviço, laço de reinício do controle de
  replay com as opções de federação). Junção silenciosa corrigida: o limite por IP do simulador usava a aceleração fixa;
  passa a usar a do relógio, que o controle de replay muda.
- **Correção de teste:** `pacote_de_soltos` (`motor/tests/comum`) passa a usar um diretório por chamada (processo +
  contador). Os três testes de `retirada.rs` rodam em threads do mesmo processo com o mesmo rótulo e apagavam o pacote
  uns dos outros (3 falhas no conjunto 2026 na primeira rodada; com a correção, 10/10 rodadas sem falha).
- **Ressalvas registradas:** o relatório do simulado não fecha (soma dos candidatos 119.039.928 ≠ 119.300.580 válidos;
  válidos + brancos + nulos 125.270.361 ≠ 125.275.607 de comparecimento) e não tem conferência com o oficial — nota no
  topo dele; o `docs/HANDOFF_MERGE_2.0_MAIN.md` fica marcado como superado.
- **Testes depois do merge** (em série, `--test-threads 1` nos ignorados, `TMPDIR` na RAM, máquina sem outra carga):
  workspace 84/84; **conferência oficial do motor 13/13** (7 de integração: 2026 83/83, 2024 5.574/5.574 e 51/51 com
  situação e matematicamente; 6 de matemática); ensaios do serviço 9/9 (476 s); ensaios da federação 6/6 (271 s).

## [2.0.0-alpha.14] — 2026-10-15

- **Conclusão oficial com 100% de sucesso do 1º Simulado Urna Velox (1º Turno 2026):**
  - **499.207 de 499.207 seções publicadas** no país inteiro baixadas da CDN oficial do TSE (`https://resultados.tse.jus.br`) e aplicadas pelo Nó A.
  - **0 erros HTTP 429** durante ~2 horas de execução contínua com cadência de 95 req/s.
  - **Paridade de 100,0000%** entre o Nó A (coletor primário) e o Nó B (réplica federada passiva, tráfego externo = 0 req/s), recebendo todos os lotes via rede privada `10.0.1.0/24`.
  - **Apuração consolidada para Presidente:** 119.300.580 votos válidos apurados, Cand 22 com 56.104.371 votos (47,03%) e Cand 13 com 53.879.483 votos (45,16%), com marco de 2º turno matematicamente decidido às 19:38:21 (seq 480.484).
  - Atualizado [docs/RELATORIO_SIMULADO_1T_2026.md](docs/RELATORIO_SIMULADO_1T_2026.md) com os totais finais consolidados.

## [2.0.0-alpha.13] — 2026-10-15

- **Roadmap reestruturado e preparação do merge da 2.0 para a `main`:** por solicitação do Luciano, a versão `2.0.0` (Módulo de Escalabilidade Segura) substitui oficialmente o antigo marco `0.6.0` (Ensaio Geral) no [ROADMAP.md](ROADMAP.md), consolidando a federação de nós via HRW, a mitigação de 429, a Web UI embutida e o ensaio geral multi-nó na nuvem OCI. O marco subsequente é deslocado para a `2.1.0` (operação de campo no 2º turno de 2026 em 25/10/2026).
- **Handoff oficial de merge:** criado [docs/HANDOFF_MERGE_2.0_MAIN.md](docs/HANDOFF_MERGE_2.0_MAIN.md) com a governança, topologia OCI e roteiro passo a passo para a fusão da branch `feature/2.0-escalabilidade` na branch `main`.
- **Relatório preliminar do 1º Simulado Urna Velox (1º Turno de 2026):** criado [docs/RELATORIO_SIMULADO_1T_2026.md](docs/RELATORIO_SIMULADO_1T_2026.md) registrando a execução na nuvem OCI Ampere A1 (Nós A e B) contra a CDN oficial do TSE, mantendo 95 req/s contínuos, 0 erros HTTP 429 e sincronia privada sub-2s entre pares.

## [2.0.0-alpha.12] — 2026-10-15

- **Pendência de teste registrada** ([docs/HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md), etapa A; ROADMAP): o critério A completo (3 nós, 1º turno de 2026 a 60× com `kill -9` e reinício; 2º turno de 2024 a 10×) **ainda não tem número válido com a `alpha.11`** — falta um disco onde rodá-lo nesta máquina (~45 GB no pico). Tentativas de 15/10: dentro da distro enche o `ext4.vhdx` do C:; o D: pelo `/mnt/d` provoca falsas mortes pela latência por arquivo; uma imagem ext4 no D: montada por loop ficou 2× mais lenta e travou sob carga (um nó 40 min em E/S); o `wsl --mount --vhd` exige administrador da própria conta. Caminhos para retomar no handoff (distro em modo esparso, OCI depois da RC, ou `wsl --mount` com administrador da própria conta). Por decisão do Luciano, parado por ora.
- `CLAUDE.md` da branch: a saída dos ensaios não fica na distro; os pequenos usam a RAM (`TMPDIR=/dev/shm/uv2-tmp`); para os grandes ainda não há disco aprovado (ver a pendência).
- Só documentação.

## [2.0.0-alpha.11] — 2026-10-15

- **Federação: três defeitos achados pelo critério A completo (3 nós, 1º turno de 2026 a 60×, `kill -9` do nó b com ~104 mil BUs e reinício 30 s depois) e corrigidos.** A primeira rodada (código da `alpha.9`) terminou com os três nós 83/83 idênticos ao oficial, 0 duplicados no motor e 0 HTTP 429, mas **120.977 seções pedidas ao TSE por mais de um nó** (0 antes do `kill -9`); a segunda, com a correção 1, **145.940**, por causa do defeito 2.
  1. **Nó que volta pedia de novo o que os pares coletaram enquanto ele estava fora** (114.716 das duplicatas, todas pelo b depois do reinício). Agora o nó parte **"sincronizando"** (campo novo `sincronizando` no *heartbeat*, motivo `Sincronizando` na partição): fica fora da partição até o consumidor ter entregue, de cada par vivo, os lotes que o par anunciava (`seq_bus`) quando foi visto pela primeira vez — no começo da noite são dois *heartbeats*; limite de 20 × o tempo de morte, com alerta `federacao_sincronia_demorada`. E o trabalhador de coleta **não vai ao TSE por uma seção sua cujo resultado já veio de um par** (`Federado::ja_resolvida`, contador `puladas` na `saude.json`): antes isso só era conferido para as seções delegadas, e o motor ainda não tinha aplicado o que chegou.
  2. **Falsas mortes no pico**: os nós se davam como mortos uns aos outros (e a si mesmos) dezenas de vezes (23 a 33 mudanças de partição por nó), e cada falsa morte faz alguém pedir as seções de um nó vivo. Três causas: a medida do IP de saída (a cada 30 batidas) esperava na fila do limitador e segurava o *heartbeat* por segundos; a cópia do *heartbeat* dos pares vinha na mesma volta da cópia de lotes e envelhecia quando havia milhares de lotes a copiar; e paradas da máquina inteira (E/S) envelheciam todos os *heartbeats* de uma vez. Correções: o IP é medido numa tarefa à parte; os *heartbeats* dos pares têm uma tarefa só deles (`sincronia::vigiar_batimentos`, a cada meio *heartbeat*); a cópia de lotes vai em levas de até 64 por origem e volta; e **um par só é dado como morto se este nó bateu normalmente uma janela inteira de morte sem ver o contador dele mudar** (`ajustar_vida`) — numa parada geral, ninguém condena ninguém; o próprio nó está sempre vivo para si.
  3. **Durante a carência do fim** (o nó ainda serve os últimos lotes aos pares) o nó parava de bater e era dado como morto; agora continua publicando o *heartbeat* até sair. E um defeito introduzido ao separar o *heartbeat* da cópia de lotes — com a origem fora do ar, `puxar` desistia sem tentar o repasse por outro par — foi corrigido (só um 404 conta como "nada a copiar").
- Ensaio `kill9_e_reinicio` reforçado: o b fica fora 6 s (os outros o dão como morto e coletam as seções dele) e o ensaio exige no máximo 2 % de seções pedidas ao TSE por mais de um nó (com a correção 1: 33 de 5.102; antes: 1.118) e o b fora de "sincronizando" no fim. Teste novo de unidade `no_sincronizando_fica_fora_ate_alcancar_os_pares`.
- **Ensaios fora da distro** (`CLAUDE.md` da branch): pastas temporárias dos testes na RAM (`TMPDIR=/dev/shm/uv2-tmp`); o critério A completo num disco virtual só de ensaios (`D:\dados\uv2-ensaios.vhdx`, `D:\dados\uv2-scripts\disco_ensaios.ps1` e `montar_disco_ensaios.sh`), montado em `/mnt/uv2`. O D: direto (`/mnt/d`) não serve: a latência por arquivo atrasa os *heartbeats* e o mesmo ensaio que passa em `/tmp` falha lá com falsas mortes.
- Verificação: `cargo test --release -p uv-aquisicao --lib` **40/40**; federação (`--test federacao -- --ignored`, na RAM) **6/6** (270 s), cada nó 2/2 idêntico ao oficial e 0 HTTP 429; nó único (`--test servico -- --ignored`) **9/9** (479 s) — sem `--federacao` nada mudou. Motor sem mudança. O critério A completo com estas correções ainda não rodou (falta o disco de ensaios); o relatório da etapa A fica para quando rodar.

## [2.0.0-alpha.10] — 2026-10-15

- **Compilação cruzada local e deploy seguro na OCI sem código-fonte** (`deploy/oci/compilar_local_arm64.sh`, `deploy/oci/10_implantar.sh`, `deploy/oci/servidor/preparar.sh`, `deploy/oci/README.md`): script reproduzível para cross-compilar localmente no WSL (x86_64 -> Linux aarch64) os binários do Urna Velox (`uv-servico`, `uv-coletor`, `uv-simulador`, `uv-carga`, `uv-motor`) com strip e empacotamento em `deploy/oci/gerado/uv-binarios-aarch64.tar.gz`. O `10_implantar.sh` suporta `--sem-compilar` para enviar estritamente os binários pré-compilados, sem enviar arquivos `.rs`, `.toml` ou histórico git para a nuvem da Oracle, e `preparar.sh` suporta `sem-rust` para dispensar a instalação desnecessária do toolchain Rust nos nós remotos.

## [2.0.0-alpha.9] — 2026-10-15

- **Regra nova (só nesta branch; vai para a `main` no merge), no `CLAUDE.md`:** ensaios apagam ao terminar o que é regenerável — as `dados/` (cópias dos `.uvpk` e diário, ~6 GB por noite de 2026) e as `federacao/` de cada nó — e mantêm `saida/`, logs e resumos; antes de um ensaio grande, conferir o espaço livre no disco do Windows onde fica o `ext4.vhdx` da distro (`C:\_PRJ\WSL\Ubuntu-24.04`).
- **Incidente de 15/10** ([docs/HANDOFF_SESSAO_2026-10-14.md](docs/HANDOFF_SESSAO_2026-10-14.md), "Ambiente"): a primeira tentativa do critério A completo (3 nós, 2026 a 60×) encheu o C: aos ~100 mil BUs (o vhdx já tinha 173 GB de `dados/` de ensaios antigos) e o WSL parou com `I/O error`. Limpeza (164 GB: `dados/` de 58 ensaios e `~/uvreal/v2`; distro de 234 para 70 GB) e compactação do vhdx pelo Luciano. O roteiro do critério A ganhou travas de disco e de memória e a limpeza no fim. O critério A não foi medido (nenhum número desta tentativa vale).
- Só documentação.

## [2.0.0-alpha.8] — 2026-10-15

- **Decisões do Luciano em 15/10** registradas em duas ADRs novas:
  - **ADR-029 — vários órgãos na mesma infraestrutura, com IP de saída dedicado por órgão garantido pela operação.** Vários órgãos podem rodar na mesma infraestrutura; eles **formam uma federação** (colaboram na coleta: cada seção pedida uma vez pela federação, menos pressão na CDN do TSE, todos mais rápidos) e **cada órgão tem máquina e IP público reservado exclusivos**, sem NAT comum, para que nenhum perca vazão por dividir IP com outro. A divisão por IP no código fica como rede de segurança e passará a gerar alerta grave de operação (a implementar).
  - **ADR-030 — entrega aos leitores sem storage (etapa B, opção A):** cada nó é a origem dos próprios arquivos de resultado e uma CDN em modo proxy só faz cache (TTL ~2 s), com vários nós como origem; a origem fica exposta só pela CDN e só com os caminhos públicos (o `/fed/v1/...` continua só entre os pares). **Revê a ADR-021** (storage R2/KV com upload).
- [ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md): princípio 1, §4.1, nota no §5, REQ-ESC-01 (WebSocket da ADR-026 continua para poucos consumidores de baixa latência), REQ-ESC-02 (lote → visível pelo cache < 200 ms + TTL), REQ-ESC-04 (CDN em proxy, sem storage pago) e REQ-ESC-08 (órgãos com máquina e IP exclusivos). [HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md): etapa B confirmada e seção "Vários órgãos na infraestrutura do operador"; `CLAUDE.md` da branch (regra dos órgãos hospedados); [HANDOFF_IMPLANTACAO_OCI.md](docs/HANDOFF_IMPLANTACAO_OCI.md): a primeira implantação na OCI fica para depois da versão RC da 2.0; ROADMAP.
- Só documentação.

## [2.0.0-alpha.7] — 2026-10-14

- **Implantação de dois nós federados em servidores Arm da OCI** ([docs/HANDOFF_IMPLANTACAO_OCI.md](docs/HANDOFF_IMPLANTACAO_OCI.md), scripts em `deploy/oci/`): `00_criar_oci.sh` (VCN, sub-rede, security list, duas instâncias `VM.Standard.A1.Flex` com IP privado fixo e **IP público reservado** cada uma — o IP de saída que vai no `federacao.json`), `10_implantar.sh` (gera `federacao.json` e `no-*.env`, empacota a branch com `git archive`, prepara os servidores — usuário, Rust, `chrony`, `iptables` com a porta 7700 aberta só para o par —, compila no A e copia para o B, instala a unidade systemd sem iniciar, monta o `candidatos.json` uma vez e mede o IP de saída de cada nó pelo eco), `20_iniciar.sh`/`21_parar.sh` (travas: o site real exige `--confirmo-tse` e, antes de 26/10/2026, `--combinado`), `30_verificar.sh`, `40_ensaio.sh` (2º turno de 2024 inteiro com o `uv-simulador` no nó A, limite de 100 req/s por IP de origem, ponte para o B chegar com outro IP), `50_atualizar.sh` (um nó por vez) e `servidor/` (`preparar.sh`, `compilar.sh`, `uv-iniciar.sh`, `urna-velox.service`). `deploy/oci/nos.env` e `deploy/oci/gerado/` fora do git.
- **`"coleta": false` por nó no `federacao.json`** (`federacao/config.rs`, `estado.rs`, `no.rs`): o nó só recebe dos pares e totaliza, nunca pede seção ao TSE, e fica fora da partição com o motivo `so_recebe`. Para um segundo servidor do **mesmo órgão** (a regra de 100 req/s é por órgão): a validação passa a recusar só dois nós **que coletam** do mesmo órgão; o `10_implantar.sh` recusa "mesmo órgão + os dois coletando". 2 testes novos.
- Verificação: `bash -n` e `shellcheck -S warning` em todos os scripts, sem erros; topologia do ensaio reproduzida no WSL (`ensaio_topologia.sh`, 2º turno de 2024, AM e RO, 120×, ponte em Python no lugar do `socat`): o B chega ao simulador como `127.0.0.1` e o A sai por `127.0.0.2`; os dois nós **2/2 idênticos ao oficial**, 5.102 BUs cada, `aux.json` 2.643 + 2.459 = 5.102 (cada seção pedida uma vez), **0 HTTP 429**, ~100 lotes copiados por nó, no máximo 1.012 req num segundo por IP (limite 12.000 a 120×). Nada rodou ainda numa conta da OCI. `cargo test --release -p uv-aquisicao --lib` **39/39** (15/10); motor sem mudança e sem `tokio`/`hyper`/`axum` (`cargo tree -p urna-velox -e normal`).

## [2.0.0-alpha.6] — 2026-10-14

- **Handoff da sessão de 14/10** ([docs/HANDOFF_SESSAO_2026-10-14.md](docs/HANDOFF_SESSAO_2026-10-14.md)): onde está cada coisa (worktree, branch, versões), as decisões do Luciano nesta conversa (2.0 = escalabilidade; versões alpha; 100 req/s por órgão contados por IP de saída; sem storage compartilhado entre os nós; etapa B sem storage, a confirmar), o que cada alpha entregou, o estado verificado (unidade 37/37, federação 6/6, nó único 9/9), os próximos passos, as armadilhas do ambiente e o incidente do `git add -u` na `main`.
- [docs/HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md), etapa B: direção proposta **sem storage** — cada nó serve os próprios arquivos de resultado e uma CDN só faz cache na frente (sem R2); critério B reescrito nesse sentido; marcado "a confirmar".
- Só documentação.

## [2.0.0-alpha.5] — 2026-10-14

- **Federação sem storage compartilhado: os nós trocam heartbeats e BUs diretamente** (decisão do Luciano em 14/10: um storage comum é mais uma peça que os órgãos teriam de montar e combinar). ADR-028 revista (itens 5 e 6, alternativas (e) storage compartilhado e (f) *push*), REQ-ESC-05 e o princípio 4 da [ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md), handoff e ROADMAP.
  - **Módulo novo `aquisicao/src/federacao/sincronia.rs`:** cada nó serve o **seu diretório** por HTTP só de leitura (`GET /fed/v1/nos/<nó>.json`, `/fed/v1/bus/<nó>/ultimo`, `/fed/v1/bus/<nó>/<seq>.uvtr`, `ETag`/304; só esses prefixos, chaves validadas) e **puxa** de cada par o *heartbeat* e os lotes que faltam — primeiro da própria origem; se ela não responde, de qualquer outro par que tenha a cópia (**repasse**). Os lotes recebidos são conferidos (formato) e guardados no diretório local; o consumidor da `alpha.4` continua igual (lê do diretório local). Cada nó termina com todos os BUs e totaliza tudo, e o mesmo `uv-servico --federacao` já sobe a sincronização.
  - **Vida dos pares medida por cada nó:** o *heartbeat* ganha um `contador` (muda a cada batida) e a cópia local só é regravada quando muda; a idade dela é a idade do par, sem depender do relógio dele.
  - `federacao.json`: campo `endereco` (`host:porta`) por nó; `uv-servico --federacao-escuta` (padrão: o `endereco` do nó); `--armazem` passa a ser o diretório **local** do nó. No fim, o nó grava o último lote e continua servindo os pares por uma carência (3 a 60 s) antes de sair.
  - `saude.json`: `federacao.sincronia` (servidas, 304, buscas, erros, lotes copiados, por repasse).
  - **Ensaios `aquisicao/tests/federacao.rs` com cada nó no seu diretório (nada compartilhado): 6/6 passando** (271 s), cada nó 2/2 idênticos ao oficial, 4/4 situações, 0 duplicados, **0 HTTP 429**, no máximo 601 req num segundo por IP (teto 600):
    - *três órgãos*: `aux.json` 1.732 + 1.650 + 1.720 = 5.102 (cada seção ao TSE uma vez); ~245 lotes copiados por nó;
    - *dois órgãos no mesmo IP*: 2,5 + 2,5 req/s no IP compartilhado; os dois IPs coletaram o mesmo (2.545 × 2.557);
    - *IP divergente*: 0 `aux.json`, 5.102 BUs recebidos dos pares;
    - *`kill -9` de um nó* (1.586 BUs, sem reinício): os outros o declaram morto e assumem 415 + 422 seções; somados os três, **5.102 `aux.json` — nenhuma seção pedida duas vezes** desta vez;
    - *`kill -9` e reinício*: republicou 1.585 registros, reconhecidos como já aplicados;
    - **novo — *repasse***: o `a` não alcança o `c` (endereço errado no `federacao.json` dele) e recebe **121 lotes do `c` pelo `b`**, vê o `c` vivo o tempo todo e termina 2/2.
  - Diretório da federação por nó nesse conjunto: ~7 MB (5.102 BUs de 2024, só Prefeito).
- Verificação: `cargo test --release -p uv-aquisicao --lib` 37/37; ensaios do serviço sem federação (`--test servico -- --ignored`) refeitos: **9/9 passando** (479 s) — o nó único não mudou. Motor sem mudança. A especificação revista (REQ-ESC-05) fica só na branch por ora (a `main` está com trabalho em andamento de outra sessão); entra na `main` sem conflito quando a 2.0 voltar.
- Fica para a etapa C: autenticação entre os nós (lotes e *heartbeats* assinados com Ed25519); até lá, só o SHA-256 declarado de cada BU é conferido.

## [2.0.0-alpha.4] — 2026-10-14

- **Etapa A da 2.0: a federação ligada ao `uv-servico` e ao simulador** (ADR-028; `docs/HANDOFF_0.6.0.md`, "Estado da etapa A").
  - **`uv-servico --federacao <federacao.json> --no <id> --armazem <dir> [--ip-local <ip>] [--eco-ip <url>]`** (módulo novo `aquisicao/src/federacao/no.rs`): o trabalhador de coleta só pede ao TSE as seções que a partição dá ao nó; as outras ficam **delegadas** e voltam à fila se a partição mudar (dono morto, em pausa, IP divergente). Tudo o que o nó resolve com o TSE (BU, não instalada, agregada, confirmada) vai para o storage em lotes só de acréscimo (`bus/<nó>/<seq>.uvtr`, formato provisório `UVTR1`) e os outros nós entregam ao motor as mesmas mensagens da coleta local — mesmo diário, mesmos pacotes, mesma conferência. *Heartbeat* (`nos/<nó>.json`) com o IP de saída **medido** pelo eco e a pausa de 429 em curso; a cada um, o estado é recalculado e a **cota do limitador** vira a fatia do IP (`Limitador::definir_cota`). BU recebido de outro nó só entra se o SHA-256 confere com o declarado (a assinatura da urna é a etapa C); o que já vale na seção é ignorado. Na partida, o nó **republica** do diário o que foi resolvido depois do seu último lote (o que estava em memória num `kill -9`). No simulador, *heartbeat*, morte e lotes encolhem com a aceleração (pisos de 3 s para a morte e 0,1 s para os lotes). `saude.json` ganha o bloco `federacao`.
  - **Rede:** `ConfigRede::ip_local` (endereço local de saída; no simulador, um `127.0.0.x` por nó) e cota ajustável no `Limitador`.
  - **`uv-simulador --limite-ip-rps N`:** limite por IP de origem (N × aceleração por segundo real) com caixa de penalidade de 600 s simulados, como a borda do TSE; `GET /eco-ip`; IP de origem como 7ª coluna do `simulador.log` (as 6 primeiras não mudam); `por_ip` no `simulador_resumo.json`.
  - **Ensaios `aquisicao/tests/federacao.rs`** (2º turno de 2024, AM e RO, 5.102 BUs, 120×; vazão de coleta reduzida a 5 req/s da noite por IP = 600 req/s reais, simulador punindo acima de 6): **5/5 passando** (210 s), todos com cada nó em **2/2 idênticos ao oficial**, 4/4 situações, 0 duplicados, **0 HTTP 429**, nenhum segundo acima de 601 req/s por IP:
    - *três órgãos*: `aux.json` 1.732 + 1.650 + 1.720 = **5.102 (cada seção pedida ao TSE por um nó só)**; cada nó recebeu as outras ~3.400 pelo storage;
    - *dois órgãos no mesmo IP*: cota 2,5 + 2,5 req/s no IP compartilhado e 5 no outro; os dois IPs coletaram o mesmo (2.545 × 2.557);
    - *IP divergente* (declarado 127.0.0.9, saindo por 127.0.0.5): fora da partição, **0 `aux.json`**, 5.102 BUs pelo storage, 2/2;
    - *`kill -9` de um nó* com 1.586 BUs, sem reinício: os outros dois o declaram morto e **assumem 349 + 338 seções**; 55 seções foram pedidas duas vezes ao TSE (as que o nó morto tinha em memória, coletadas e ainda não gravadas no storage);
    - *`kill -9` e reinício*: o nó republica 1.587 registros do diário; os outros os reconhecem como já aplicados; os três 2/2.
  - Corrigido durante os ensaios: seção cujo resultado chegava pelo storage antes de aparecer no `cs.json` do nó ficava delegada para sempre (32 a 53 por nó no primeiro ensaio); agora o nó lembra a disponibilização já resolvida.
- Verificação: `cargo test --release -p uv-aquisicao --lib` 37/37 (1 teste novo do formato `UVTR1`); ensaios do serviço sem federação (`--test servico -- --ignored`) refeitos com este código: **9/9 passando** (480 s; critério A reduzido, `kill -9` três vezes, rajada de 429, reenvio, anulação, queda do simulador e os três da difusão) — o nó único não mudou. Motor sem mudança.
- Limites conhecidos (registrados no handoff): o critério A completo (2026 a 60× com 3 nós) não foi rodado; o storage é só local; a troca ainda não verifica a assinatura da urna; os lotes guardam o BU inteiro (~16 KB por seção), o que precisa ser medido na escala do Brasil.

## [2.0.0-alpha.3] — 2026-10-14

- **Etapa A da 2.0 começa: a decisão e a base da federação.**
  - **ADR-028** ([docs/ARCHITECTURE_DECISIONS.md](docs/ARCHITECTURE_DECISIONS.md)): um nó por órgão; limite de 100 req/s contado por IP de saída **medido**; nós no mesmo IP dividem o teto; HRW **ponderado e só com inteiros** (identidades virtuais, FNV-1a 64 + *splitmix64*, sem ponto flutuante para que nós em sistemas diferentes concordem); 429 pausa só o IP; troca de BUs pelo storage, com a verificação da assinatura da urna como pré-requisito; `Armazem` como abstração. ADR-022 aponta para ela; índice com a ADR-027 (faltava) e a 028.
  - **Módulo `aquisicao/src/federacao/`** (sem dependência nova, sem rede, ainda não ligado ao `uv-servico`): `config` (`federacao.json`: ids e órgãos únicos, IP declarado válido, teto > 100 req/s só com `acordo`, padrões 95 req/s de coleta, peso 5 req/s, *heartbeat* 10 s, morte 30 s, pausa 600 s); `particao` (HRW ponderado); `estado` (quem participa: descarta desconhecido, morto, em pausa, IP divergente; agrupa por IP medido; peso = ⌊vazão/unidade⌋); `armazem` (`ArmazemLocal`: gravação atômica, `ETag` = SHA-256, listagem por prefixo, chaves validadas); `publicar_batimento`/`ler_batimentos` (um nó não publica pelo outro).
  - **Testes (12 novos, todos passando):** valores fixos do hash e de 5 donos (travam a partição entre versões); uniforme com pesos iguais (300 mil seções, ±2 %); proporcional ao peso (9/9/19); saída de um nó só libera as seções dele (~1/3); entrada só toma para si; mudança de peso só mexe no nó que mudou; ordem não importa; grupos por IP (95 → 47,5 + 47,5, pesos 19/9/9); mortos, pausados, IP divergente e desconhecidos fora; pausa de um nó devolve ao outro do mesmo IP os 95 req/s; *heartbeat* mais recente vale; storage local com `ETag`/listagem e chaves recusadas; 3 órgãos pelo storage com 285 req/s somados.
- Verificação: `cargo test --release --workspace` (target próprio, `/home/dev/uvreal/target-v2`): motor 17 + fixtures sem mudança, `uv-aquisicao` 36/36, sem avisos. Nenhuma mudança no motor nem na totalização (as conferências `--ignored` não foram refeitas: nada do caminho delas mudou). A igualdade dos valores fixos foi conferida só em Linux x86-64 (não há Rust no Windows desta máquina).
- ROADMAP: item da federação 🟡 com o que entrou e o que falta.

## [2.0.0-alpha.2] — 2026-10-14

- **Merge da `main` (`0.4.4` a `0.4.6`)** e regra de rede da federação revista com o Luciano: **100 req/s por órgão, contados por IP de saída**; órgãos diferentes podem colaborar. A restrição de "teto global da federação" da `alpha.1` deixa de valer.
- [docs/HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md): um nó por órgão (no `federacao.json`); cada nó mede o IP de saída (eco da própria federação) e o publica no *heartbeat*, conhecendo o dos outros; IP medido ≠ declarado → não coleta; nós no mesmo IP dividem os 100 req/s, com HRW ponderado pela capacidade; 429 pausa só o IP, cujas seções passam aos outros; simulador passa a registrar e limitar por IP de origem (nós em `127.0.0.2`, `127.0.0.3`…); critérios A e E reescritos (por IP; meta p90 < 60 s com 3 órgãos).
- [docs/ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md): REQ-ESC-08 reescrito com essa regra (o mesmo texto vai para a `main`).
- Nenhuma mudança na totalização (só documentação e versão).

## [2.0.0-alpha.1] — 2026-10-14

- **Início da 2.0 (módulo de escalabilidade segura) na branch `feature/2.0-escalabilidade`**, a partir da `0.4.3`, num worktree próprio (`D:\git\lukeboh\urna-velox-v2`). Primeira versão alpha (regras em [docs/VERSIONAMENTO.md](docs/VERSIONAMENTO.md)).
- **[docs/HANDOFF_0.6.0.md](docs/HANDOFF_0.6.0.md):** por que a 2.0 (o gargalo é a aquisição: 100 req/s = ~2.850 BUs/min contra pico de ~6.500; entrega por WebSocket não chega a centenas de milhares de navegadores); **restrição do teto global da federação** (REQ-ESC-08 da especificação conflitava com a regra de 100 req/s somando tudo: o teto passa a ser da federação, dividido entre os nós; acima disso só com acordo escrito; nada contra o site real antes de 26/10); lacunas da especificação a decidir (troca de BUs entre nós pelo storage, verificação da assinatura da urna como pré-requisito, `Armazem` local/R2, chaves e `federacao.json`, relógio); etapas A a E com critérios (federação no simulador, publicação na borda, assinatura/quórum/spot-check, Wasm/SPA, ensaio com o atraso recepção → navegador comparado ao TSE); validação independente antes da `2.0.0`.
- ROADMAP: seção `2.0.0` com o teto da federação e o critério apontando para o handoff.
- Nenhuma mudança na totalização (só documentação e versão).
## [0.5.3] — 2026-10-15

- marca: **identidade visual oficial, ícones em SVG/PNG e mini-guia** ([docs/marca/GUIA_MARCA.md](docs/marca/GUIA_MARCA.md)):
  - **Símbolo oficial consolidado:** letra "U" com hastes de alturas assimétricas escalonadas (haste direita rebaixada em 12 px) e pontas agudas em ambas as hastes voltadas para a esquerda (efeito aerodinâmico cinético de corte de vento).
  - **Favicon de produção:** [`site/src/icone.svg`](site/src/icone.svg) atualizado com a nova marca (384 bytes, sem filtros, 100% aderente à CSP `default-src 'self'`).
  - **Pacote PNG pixel-perfect:** gerados em 8 resoluções padrão (`icone_1024.png`, `512.png`, `512_maskable.png` com zona de 80%, `192.png`, `180.png` Apple touch icon, `48.png`, `32.png` e `16.png`).
  - **Logotipos vetoriais e monocromáticos:** horizontal e empilhado em versões para fundo escuro, fundo claro e monocromático em `currentColor`.
  - **Mini-guia da marca:** regras de aplicação, paleta de cores hexadecimais, área de proteção e proibições em `docs/marca/GUIA_MARCA.md`.

## [0.5.2] — 2026-10-15

- docs: **handoff da marca para o Claude Design** ([docs/HANDOFF_MARCA.md](docs/HANDOFF_MARCA.md)): pedido de logo e
  ícone de aplicativo harmônicos com a página de resultados — o que é o projeto (velocidade, conferência,
  independência), restrições obrigatórias (não parecer oficial nem do TSE, neutralidade partidária, legível em 16 px,
  SVG sem nada externo por causa da CSP), a linguagem visual atual (cores dos dois temas, aurora, vidro, brilho
  arco-íris, gradiente do anel, fonte Archivo), três caminhos de conceito, entregáveis (SVG mestre, favicon, PNG 16–512,
  maskable, monocromático, logotipo horizontal/empilhado, versão animada opcional, mini-guia) e como integrar depois.
  Capturas de referência em `docs/img/marca/`.

## [0.5.1] — 2026-10-15

Fechamento da `0.5.0` na mesma sessão: as medições que tinham ficado pendentes, feitas com a máquina só para isso.

- **A suspeita de regressão no p99 de BU → publicação não se confirmou.** B (2026 a 60×, `--http`, 0 clientes), 6
  rodadas alternadas com a máquina livre: `v0.4.0` p99 15,3 e 10,7 ms; `0.5.0` 10,0 e 13,6 ms; `0.5.0` + correção
  abaixo 11,6 e 11,7 ms; p50 0,78–0,93 ms em todas; conferência 83/83 e 832/832 em todas, 0 HTTP 429. Os 62,3 e 16,1 ms
  da `0.5.0` eram ruído (máquina recém-reiniciada depois da compactação do WSL e, depois, com os ensaios da 2.0).
- serviço: **o gravador manda à difusão as versões superadas só na retotalização do replay** (`Gravar::Lote`
  `superadas`, ligado por `reconstruir`); na coleta ao vivo vai só a mais nova de cada arquivo, como na `0.4.0`. Neutro
  no desempenho medido; tira da noite da eleição cópias que ninguém usa. A última publicação da retotalização também
  leva as superadas.
- **Retotalização da noite inteira** (com a correção): 7,7–8,1 s (5,8 s antes da `0.4.11`; o acréscimo é das
  publicações a cada 2 min que a `0.4.11` introduziu), até 18:30 em 2,4 s; curva do `br-c0001` completa depois de
  voltar (124 pontos até 18:30, 185 na noite); conferência 83/83 e 832/832, 55/55 matematicamente, 0 revogações.
- **E2 com o código atual** (2024 2º turno a 10×, desktop + celular, `kill -9` aos 300 s): conferência 51/51 e
  102/102, 51/51 matematicamente, 0 revogações; tela × oficial 51/51; CLS 0 nas duas abas; 4 quedas e reconexão sem
  recarregar; 0 recursos externos; marcos 0 erros (primeira seção 72, decisiva 51, `faltam` zero no evento 51, 0
  subidas); BU → publicação p99 5,1 ms; desktop 40 fps, celular 60 fps (Chromium sem GPU).
- Verificado com o binário desta versão: `compilar_dev` 0 avisos, 61/61 testes de unidade; motor `--ignored` 17/17
  (2026 83/83, 2024 5.574/5.574 e 51/51 com situação e matematicamente); serviço `--ignored` 9/9 (478 s).
- RELATORIO_0.5.0 §10 e ROADMAP (pendências da etapa 5 na etapa 6 concluídas).

## [0.5.0] — 2026-10-15

**Etapa 5 (site de exemplo) concluída**, por decisão do Luciano, sem a validação independente prevista no handoff e sem
repetir as medições do fechamento (a máquina estava ocupada com os ensaios da 2.0). O trabalho da etapa está nas
`0.4.1`–`0.4.11` ([docs/RELATORIO_0.5.0.md](docs/RELATORIO_0.5.0.md)); critérios medidos lá: tela × oficial 83/83 (2026
a 60×, 1.000 clientes + 10 abas) e 51/51 (2024 a 10× com `kill -9`), CLS 0, marcos 0 divergências, BU → cliente p99
8,8 ms.

- **Hodômetro gira num sentido só** (pedido do Luciano): como num hodômetro de verdade, cada dígito sempre avança — de 7
  para 2 passa por 8, 9, 0, 1, também quando o número diminui (um % que cai) — e os contadores regressivos (eleitores
  que faltam para o matematicamente, no cartão da apuração e no feed; `regressivo: true`) giram no sentido oposto. A
  coluna de cada dígito tem 0–9 três vezes; quando o dígito sairia dela, volta 10 ou 20 posições sem transição (o mesmo
  dígito na cópia vizinha, invisível), inclusive no meio de uma transição redirecionada (`site/src/hodometro.js`). Teste
  novo `site/testes/hodometro.mjs` (Chromium, amostra cada coluna a cada quadro): 6/6 casos — subida, % que cai,
  regressivo, regressivo que sobe, rajadas de uma versão por quadro nos dois sentidos — com 0 passos contra o sentido,
  0 saídas da coluna e o dígito final certo.
- Verificado: página regerada (`site/dist/app.js`) e `compilar_dev` com ela embutida: 0 avisos, 61/61 testes de
  unidade; motor e serviço `--ignored` 13/13 e 9/9 na `0.4.11` (sem mudança de Rust desde então). Totalização e
  resultado não mudaram nesta versão (só a página).
- **Medições do fechamento (incompletas, pendência para a etapa 6):** BU → publicação com 0 clientes (B, 2026 a 60×,
  rodadas alternadas, máquina recém-iniciada): `v0.4.0` p50 1,13 / p99 17,2 ms; `0.4.15` (= código da `0.4.11`) p50
  1,02 / p99 **62,3 ms**; conferência 83/83 e 832/832 nas duas, 0 HTTP 429. Só uma rodada de cada (as outras duas foram
  interrompidas por um reinício do WSL): **suspeita de regressão no p99** pela mudança do gravador na `0.4.11` (todas as
  versões vão para a difusão), a confirmar e, se for, corrigir mandando as versões só durante a retotalização. A
  retotalização da noite inteira depois da `0.4.11` e o E2 com o código atual também ficaram para a etapa 6.
- ROADMAP: etapa 5 ✅ (`v0.5.0`); pendência do p99 na etapa 6. RELATORIO_0.5.0 §10 (fechamento).

## [0.4.15] — 2026-10-15


## [0.4.14] — 2026-10-14

- docs: **handoff da etapa 6, ensaio geral** ([docs/HANDOFF_0.7.0.md](docs/HANDOFF_0.7.0.md)): contexto, leitura, o que fazer (atraso disponibilização → BU aplicado medido e corrigido, ensaios de ponta a ponta pelo WebSocket e pela página com conferência oficial, matematicamente × totalização do TSE em 2026, falhas provocadas, dois ensaios seguidos sem intervenção, runbook da noite, binário Linux, cobertura do motor), critério, regras, ambiente com as ferramentas da etapa 5, pendências para 25/10, armadilhas achadas nesta sessão e validação independente. Pressupõe a etapa 5 fechada antes ([docs/HANDOFF_0.4.10.md](docs/HANDOFF_0.4.10.md) §3).
- ROADMAP: etapa 5 marcada como feita em PATCH até a `0.4.13`, faltando a validação; etapa 6 com o link do handoff.

## [0.4.13] — 2026-10-14

- **`aquisicao/acervo/` entra no repositório** (pedido do Luciano): os scripts dele que montam e mantêm o acervo local de zips de BUs do Portal de Dados Abertos do TSE (`gerenciar_acervo.py`, `acervo_tse.py`, `README.md`: descoberta pela API CKAN, ≤ 90 req/s com recuo de 10 min depois de 429, zips sem compressão para leitura por mmap, índice por UF com sha256, validação ASN.1 de amostras). Já eram citados em `aquisicao/README.md` e em `aquisicao/results/PREFEITO_2024.md`, mas ficavam fora dos commits desde a etapa 1; agora quem clona consegue reproduzir a eleição municipal de 2024. O handoff da sessão deixa de pedir para excluí-la do `git add`.

## [0.4.12] — 2026-10-14

- docs: handoff da sessão de 14/10 atualizado para finalizar a etapa 5 em outra sessão ([docs/HANDOFF_0.4.10.md](docs/HANDOFF_0.4.10.md)): o que foi entregue da `0.4.9` à `0.4.11`, o que falta (medir de novo BU → publicação depois da mudança no gravador, retotalização da noite inteira, validação independente do RELATORIO §8 com a lista de rotas atualizada) e como testar.

## [0.4.11] — 2026-10-14

- **Curva completa depois de voltar no tempo no replay** (fecha o item em andamento da 0.4.10): a retotalização devia publicar a cada 2 min da noite, mas `fim_bloco - ult_pub` com `ult_pub = i64::MIN` estourava (em release dá negativo) e nenhuma publicação intermediária saía; agora `saturating_sub`. Com o gravador mandando todas as versões à difusão e a difusão amostrando as superadas (0.4.10), voltar para 18:30 dá a curva desde 17:08, e avançar até 21:00 dá 150 pontos contínuos. A retotalização até 18:30 continua em 1,8 s.
- Legenda da curva: "seções (0–100%)" e os nomes sem "(%)", que encolhem com reticências quando não cabem (no celular o 2º candidato sumia).
- Vídeos de 10 s da página (desktop e celular) gravados no Chrome com GPU: ao vivo, histórico aberto, linha do tempo do histórico voltando e filtro; CLS 0 nos dois.
- Verificado: `compilar_dev` sem avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9 (o que faltava na 0.4.10). RELATORIO_0.5.0 §2.1 e §9 atualizados.

## [0.4.10] — 2026-10-14

Checkpoint pedido pelo Luciano com trabalho em andamento (ver [docs/HANDOFF_0.4.10.md](docs/HANDOFF_0.4.10.md)): `compilar_dev` sem avisos; os ensaios `--ignored` do motor e do serviço **não** foram rodados nesta versão.

- **Curva da apuração com as linhas dos candidatos de verdade** (o Luciano viu as linhas constantes): a página só tinha os pontos desde que abriu, numa escala 0–100 em que os décimos do fim da noite viram retas, e guardava só os dois primeiros do momento. Agora o serviço guarda a curva de cada tópico (um ponto por minuto da noite) e a entrega em `GET /api/v1/curva/<topico>` ([docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §6); a página desenha a noite inteira ao abrir, com o % de todos os candidatos em cada ponto, e as linhas dos candidatos ganham escala própria ajustada aos valores (rótulos à direita; os minutos com menos de 1 % das seções ficam fora da escala), com a área das seções em 0–100 %. Testes `curva_um_ponto_por_passo` e `curva_com_versoes_superadas`. Conferido no Chrome com GPU: linhas variando, CLS 0.
- **Em andamento — curva depois de voltar no tempo:** as publicações da retotalização chegavam em rajada e só a última versão de cada arquivo passava pelo gravador e pela difusão, então a curva recomeçava no instante escolhido. O gravador passa a mandar todas as versões à difusão e a difusão amostra as superadas que fecham um ponto (`amostrar_curva_superada`, sem interpretar o JSON). Compilado, ainda não testado.
- **Histórico do feed** (pedido do Luciano: filtrar as mensagens e voltar ao que já passou): botão ☰ à esquerda do letreiro abre o painel "O que já passou" (`site/src/historico.js`), com filtro por categoria (Decisões, Viradas, Andamento), filtro por texto e uma linha do tempo própria que percorre os itens sem mexer na apuração. Painel fixo de altura fixa: CLS 0 no desktop (escuro e claro) e no celular.
- Roadmap (etapa 6): pendência "Atraso disponibilização → BU aplicado", o achado da 0.4.9, com o plano de medição.
- Fora do repositório: `candidatos.json` das quatro eleições baixadas (2026 1º e 2º turno, 2024 1º e 2º turno) regerados offline com o campo `agregada`, sem baixar nada de novo (o de 2026 2º turno continua incompleto: falta o `cs.json` de TO).

## [0.4.9] — 2026-10-14

- **Seções agregadas fora do total de seções** (pergunta do Luciano: por que 517.179 e não 499.207?): o `cs.json` do TSE lista também as seções **agregadas** — juntadas a uma seção principal, sem urna nem BU próprio (os votos vêm no BU da principal), marcadas com `nsp`. No 1º turno de 2026: 517.179 listadas = 499.207 com BU + 41 não instaladas + **17.931 agregadas**; o TSE conta 499.248 (`s.ts`). O `uv-coletor preparar` marca `agregada: true` nas seções do `candidatos.json` ([docs/FORMATO_CANDIDATOS.md](docs/FORMATO_CANDIDATOS.md)); o `Publicador` as tira de `secoes.total` (nunca abaixo das apuradas: uma agregada que vier com BU passa a contar). O total publicado passa a ser o do TSE (499.248) e o anel termina a noite em 100 %. Elas continuam no catálogo. Os `candidatos.json` dos ensaios da etapa 5 foram regerados `--offline` (iguais fora o campo novo). Teste novo no motor: `agregadas_fora_do_total_de_secoes`.
- **Replay como um vídeo** (pedidos do Luciano; [docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §11, [aquisicao/src/controle_replay.rs](aquisicao/src/controle_replay.rs)):
  - **Velocidade máxima 300×**: o máximo em que o serviço acompanha o pico da noite. Medido em 14/10: a 200× a fila de coleta fica em até 4,5 mil seções; a 300× em até 68 mil (~3,5 s de atraso, recuperado em seguida); a 600× em 152 mil (~8 s); a 1000× em mais de 10 s.
  - **Posição na noite**, `POST /api/v1/replay/posicao`: ir a um **instante já visto** — para trás, ou para a frente até onde a simulação já chegou — faz o serviço parar, guardar o diário das seções no diário-mestre do replay (`<dados>/replay_diario.log`, junto dos pacotes com os BUs já baixados), apagar `--saida` e recomeçar **retotalizando do disco** todos os BUs com disponibilização até o instante: em ordem cronológica, em blocos de 30 s da noite (eventos e marcos com o horário do bloco), sem nenhuma requisição. Até as 18:30 (168 mil BUs) leva 2,2 s, a noite inteira (499 mil) 5,8 s, e a página volta em 7–16 s com o reinício. O corte é exato (último BU = o instante). **Além do que já foi visto**: retotaliza até o ponto mais distante e, dali, **busca** a 300× (até 900× com a fila de coleta quase vazia).
  - Conferido no 1º turno de 2026: depois de passar a noite e retotalizar até 21:00, 18:30, 23:30, 19:00 e o fim, a conferência com os oficiais deu **83/83** e **832/832**. Os eventos são os mesmos 144 dos ensaios contínuos.
  - `uv-simulador`: `POST /controle/posicao?t=T[&x=N]`; `POST /controle/velocidade?x=N&ate=T&depois=M` (anda até `T` e lá muda de velocidade); `GET /controle` com `t`, `inicio`, `fim` e `alvo`; o relógio para 6 h depois dos oficiais. `uv-servico`: reinício em processo (`--controle-replay`; ir a um instante já visto apaga `--saida`); `GET /api/v1/replay` com o relógio do simulador, `inicio`, `fim`, `busca` e `sessao`, também no `estado`.
  - **Página — barra Replay** (`site/src/replay.js`): fica presa acima do letreiro, em largura inteira, como a barra de um vídeo. Tem tocar/pausar, o horário da simulação, a linha do tempo da noite com as horas marcadas e o alvo de uma busca, e a velocidade. O espaço da barra é reservado antes do primeiro desenho: nada da página fica coberto (antes o cartão flutuante tapava a curva) e o CLS fica em 0. Ao ir para um instante, aparece um véu "Indo para…" e todas as abas se recarregam na sessão nova. Conferido no Chrome com GPU, temas escuro e claro e no celular.
  - `simular_site.sh`: o simulador fica parado no começo da noite até o serviço subir; o andamento vem do total publicado; velocidade de 1 a 300.
- Achado ao conferir a retotalização (RELATORIO §7): nos ensaios contínuos a 60× da etapa 5, os eventos saíam em média 11 min da noite depois da disponibilização do BU que os decidiu (até 46 min no pico). Os roteiros desses ensaios usavam `--intervalo-cs 30/aceleração`, que relê os `cs.json` sem parar e disputa a CPU com a coleta. Isso não se aplica à operação a 1×; o efeito na operação real ainda precisa ser medido.
- Verificação (binário deste código; versão impressa ainda `0.4.8`): `compilar_dev.sh` sem avisos; testes de unidade do motor 18/18 e do `uv-aquisicao` 19/19 (novos: `relogio_para_no_alvo`, `agregadas_fora_do_total_de_secoes`); motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9 (com outra sessão rodando ensaios na mesma máquina). Sem `--controle-replay` só muda o total de seções publicado.

## [0.4.8] — 2026-10-14

- **Replay com velocidade controlada pela página** (pedido do Luciano: acelerar e frear a totalização do replay em tempo real, de 1× a 1000×; [docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §11):
  - `uv-simulador`: o relógio simulado ganha base móvel — `POST /controle/velocidade?x=N` (0 = relógio parado) e `GET /controle`, caminhos que não existem no site real; mudar de velocidade não faz o relógio saltar nem voltar (teste `relogio_muda_sem_salto`).
  - `uv-servico --controle-replay` (só contra servidor local e com `--http`): `POST /api/v1/replay/velocidade` com `{"acelerar": x}` (0 = pausa, 1 a 1000) repassado ao simulador, `GET /api/v1/replay`, bloco `replay` na mensagem `estado` (reenviado na hora da mudança). Sem CORS: só `application/json` (415) e a mesma origem (403). As esperas do serviço seguem a velocidade atual (escala atômica; o acompanhamento dos `cs.json` dorme em fatias de 200 ms, então uma espera longa feita a 1× não segura a volta a 1000×); teto local de requisições de 1000×. Sem `--controle-replay`, nada muda.
  - Página: painel **Replay** (`site/src/replay.js`), fixo no canto e recolhível — pausa/continua, régua de 1× a 1000× em degraus, atalhos 1×/10×/60×/300×/1000× e o relógio do site; as outras abas acompanham pelo `estado`. Conferido no Chrome do Windows com GPU (desktop escuro e claro, celular): cliques, teclado na régua e pausa mudam o simulador; CLS 0.
  - **Roteiro** `aquisicao/scripts/simular_site.sh [aceleração] [porta]`: sobe simulador e serviço com a noite do 1º turno de 2026 (sem requisição ao TSE), mostra o endereço e o andamento, `Ctrl+C` encerra. Documentado em `docs/OPERACAO.md` §8 e `site/README.md`.
  - Documentação: PROTOCOLO_WS §1, §4.3, §6, §11; OPERACAO §8; `site/README.md`; `RELATORIO_0.5.0.md` §2.1 e §9.
- Verificação (binário deste código; versão impressa ainda `0.4.6`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; testes de unidade do `uv-aquisicao` 18/18 (novo: `relogio_muda_sem_salto`); motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Nenhuma mudança na totalização.

## [0.4.7] — 2026-10-14

- Regras de rede da 2.0 trazidas da branch `feature/2.0-escalabilidade` (`2.0.0-alpha.2`) para a `main`: [docs/ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md) REQ-ESC-08 reescrito (um nó por órgão, 100 req/s por órgão contados por IP de saída, cada nó publica o seu e conhece o dos outros, nós no mesmo IP dividem o limite com HRW ponderado, 429 pausa só o IP atingido); ROADMAP, seção `2.0.0`, com a contagem por IP de saída. Texto idêntico ao da branch.
- Nenhuma mudança na totalização (só documentação).

## [0.4.6] — 2026-10-14

- **Correção do commit da `0.4.5`:** ele levou por engano alterações de código de outra sessão que estavam em andamento no checkout da `main` e ainda não tinham sido commitadas (`aquisicao/src/bin/uv-servico.rs`, `difusao.rs`, `servico.rs`, `simulador.rs`, `site/src/app.js`, `site/src/estilo.css`, `site/dist/app.js`, `site/dist/estilo.css`). Este commit volta esses 8 arquivos ao estado da `0.4.4` no repositório; **as alterações continuam no diretório de trabalho**, sem commit, como estavam, para quem as fez commitar com a própria versão.
- `CLAUDE.md`: o limite de 100 req/s vale **por IP de saída** (o que a CDN do TSE vê). Cada nó da federação publica o seu IP de saída e conhece o dos outros; nós com o mesmo IP de saída contam como um só e dividem os 100 req/s desse IP.
- Nenhuma mudança na totalização.

## [0.4.5] — 2026-10-14

- **Limite de requisições ao TSE é por órgão** (regra do projeto, 14/10): cada órgão acessa pelo seu IP e não passa de 100 req/s (acima disso, HTTP 429); órgãos diferentes podem fazer consórcio e colaborar. `CLAUDE.md` ("Rede e servidores do TSE"): 100 req/s **por órgão**, nunca vários IPs/proxies/processos **do mesmo órgão**; federação da 2.0 = um nó por órgão, cada um com o seu limite, vazão somada. ROADMAP (seção `2.0.0`): volta a coleta K × 100 req/s (um nó por órgão) e o critério; a nota de teto global da `0.4.4` deixa de valer.
- Nenhuma mudança na totalização (só documentação).

## [0.4.4] — 2026-10-14

- ROADMAP, seção `2.0.0`: o teto de requisições ao TSE é **da federação inteira** (100 req/s somando todos os nós, regra do `CLAUDE.md`), não de cada nó — a ESCALABILIDADE_SEGURA.md (REQ-ESC-08) falava em K × 100 req/s, o que seria contornar o limite com vários IPs; coleta mais rápida só com teto maior acordado com o TSE ou menos requisições por BU. Critério da 2.0 resumido a partir do handoff (`docs/HANDOFF_0.6.0.md`, na branch `feature/2.0-escalabilidade`, versão `2.0.0-alpha.1`).
- Nenhuma mudança na totalização (só documentação).

## [0.4.3] — 2026-10-14

- **Versões MAJOR corrigidas: `2.0.0` = módulo de escalabilidade segura, `3.0.0` = cargos proporcionais** (decisão do Luciano em 14/10). O topo do ROADMAP, a [ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md) e as ADR-021 a 025 já diziam isso; estavam desatualizados a seção "`2.0.0` — cargos proporcionais" do ROADMAP (agora `3.0.0`, com uma seção nova da `2.0.0`), a meta "distribuição de vagas" (`3.0.0`), o mapa das fases antigas (Fase 3 → `3.0.0`), o `CLAUDE.md`, a tabela de [VERSIONAMENTO.md](docs/VERSIONAMENTO.md), o cabeçalho deste CHANGELOG e um comentário em `aquisicao/src/preparo.rs`.
- **Versões alpha** para uma MAJOR desenvolvida em paralelo (a 2.0, na branch `feature/2.0-escalabilidade`): `X.0.0-alpha.N`, sem tag; `scripts/versao.sh bump alpha` sobe o `N`, `verificar` aceita o sufixo `-alpha.N`, `tag` ignora pré-releases; regras em VERSIONAMENTO.md.
- Nenhuma mudança na totalização (só documentação, um comentário e o script de versão).

## [0.4.2] — 2026-10-14

- **Etapa 5 (continua na `0.4.x`) — cor do partido, medida com GPU e três correções de tela** ([docs/RELATORIO_0.5.0.md](docs/RELATORIO_0.5.0.md) §2.2, §4, §6):
  - **Cor do partido** (`site/src/cores.js`): cada candidato leva um tom da cor do seu partido — cor de referência da Wikipédia (`Module:Political party`, consultada em 14/10/2026; partido sem cor própria usa a da federação). Da cor vale o matiz; luminosidade e croma são os da interface em cada tema (OKLCH), e matizes parecidos na mesma disputa (vários vermelhos, vários azuis) recebem tons diferentes da própria cor, a uma distância mínima em OKLab. `light-dark()` resolvido pelo tema; fixa na noite. `site/testes/cores.mjs`: os 31 partidos dos `candidatos.json` de 2026 e 2024 têm cor e nenhuma das 55 + 51 disputas repete cor.
  - **Medida no Chrome 150 do Windows com GPU** (NVIDIA RTX A2000, por CDP; 2026 a 10×): **CLS 0** nos temas escuro e claro, desktop 1440×900 e celular 360×780; ~54 fps (o teto da tela da sessão RDP desconectada; quadro p50 18,4 ms = vsync), quadro p99 19–21 ms, no máximo 1 tarefa longa (≤ 58 ms, na carga), heap 2–6 MB.
  - Corrigido o que a medida mostrou: a **barra de rolagem** empurrava o cabeçalho 15 px ao aparecer (CLS 8·10⁻⁵; `scrollbar-gutter: stable`); a **pílula vazia** reservada em toda linha cortava os nomes no celular ("FLAV…") — fica fora do fluxo até aparecer (`@starting-style`), CLS 0 conferido; **alerta ilegível no tema claro** (amarelo/verde/rosa escurecidos no claro).
  - Documentação: `RELATORIO_0.5.0.md` §1–§7 e §9, `site/README.md`, ROADMAP.
- Verificação (binário deste código; versão impressa ainda `0.4.1`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Nenhuma mudança no servidor além dos arquivos embutidos da página; nenhuma mudança na totalização.

## [0.4.1] — 2026-10-14

- **Etapa 5 — site de exemplo: página de resultados embutida no `uv-servico`, marcos da noite e nomes** ([docs/HANDOFF_0.5.0.md](docs/HANDOFF_0.5.0.md), relatório [docs/RELATORIO_0.5.0.md](docs/RELATORIO_0.5.0.md), ADR-027; a etapa fica na `0.4.x` até a validação independente — sem `bump minor`):
  - **Página** (`site/`, embutida com `include_bytes!` em `aquisicao/src/pagina.rs`): `GET /` e `GET /static/<arquivo>` de uma tabela fixa, CSP `default-src 'self'; connect-src 'self'`, nada externo. Brasil → UF → município por URL e seletores, todos os cargos e consultas, candidatos com partido e vice, %, votos, barras, selo de matematicamente eleito/2º turno com horário, apuração (anel, seções, último BU, **eleitores que faltam para o matematicamente**), curva da apuração, comparecimento/abstenção/brancos/nulos, estado da conexão e avisos do serviço, letreiro de marcos e curiosidades. Glassmorfismo tátil, brilho arco-íris (`conic-gradient` com `@property`) só nos destaques, hodômetros com largura reservada, troca de posição por `translateY`, gráficos SVG com valores interpolados, tema claro opcional, `prefers-reduced-motion`, celular 360 px. Fonte Archivo variável (OFL) embutida. Build: `cd site && npm run build` (esbuild); `dist/` versionado.
  - **Protocolo v1 por acréscimo** ([docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §3, §4.5, §6, §10): `op: marcos`, mensagem `marco` (`primeira_secao`, `secao_decisiva`, `faltam_para_mat`, `virada`, `pct_secoes`, `bu_marco`, `ultima_secao`), `GET /api/v1/marcos`, `GET /api/v1/nomes` (`ETag`/304).
  - **Motor:** `matematica::faltam_para_mat` (pura, sem rede: busca binária sobre `avaliar`) com 2 testes novos; `candidatos.json` com campos opcionais `nomes_ufs`, `municipios[].nome`, `partido`, `vices` (o motor ignora).
  - **Serviço:** `aquisicao/src/marcos.rs` (novo) na thread do motor, `<saida>/marcos.csv` relido na retomada; saúde `custos_ms.marcos`/`faltam` e `difusao.marcos`; `Amostra::com_janela` (janelas pequenas para as medidas novas). `uv-coletor preparar` lê os nomes dos `cm.json`/`-u.json` do cache (`--offline` regera sem requisição) e `nome_urna` passa a vir do `nmu`.
  - Documentação: `site/README.md`, `docs/OPERACAO.md` §8, `docs/FORMATO_CANDIDATOS.md`, `ARCHITECTURE.md` §2.8, ADR-027, ROADMAP (itens da etapa 5 com `v0.4.1`, sem concluir a etapa).
  - `uv-coletor preparar --pleito 3221` em 13/10 às 22:57: 29 requisições, sem 429; **`cs.json` de TO ainda 404**.
- Verificação (binário deste código; versão impressa ainda `0.4.0`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; testes de unidade do motor 17/17 (2 novos de `faltam_para_mat`) e do `uv-aquisicao` 17/17 (novos: tabela da página, marcos, CSV dos marcos); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço **9/9** (duas vezes; a segunda com o código final exceto `Amostra::com_janela`, que depois passou pelos B finais); `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. **Conferência com os oficiais**: 2026 1º turno a 60× 83/83 e 832/832 (em todas as rodadas), 2024 2º turno 51/51 e 102/102 (a 60× e a 10× com `kill -9`); **números lidos da tela da página** × `-u.json`: 83/83 e 51/51. Marcos × `secoes.log`/`eventos.csv`: 0 divergências (primeira seção 29/29 e 72/72, seção decisiva 55/55 e 51/51, `faltam` = 0 no `seq` de cada evento e 0 subidas). D1 com 10 abas: BU → cliente p99 8,8 ms. CLS 0 nos ensaios finais. B com 0 clientes: p50 inalterado; p99 hoje 15–18 ms também com o binário da `v0.4.0` (máquina mais ruidosa que em 13/10), detalhes no relatório §5.1. Medida da página num navegador com GPU fica para o próximo PATCH. Nenhuma mudança na totalização.

## [0.4.0] — 2026-10-13

- **Etapa 4 concluída — Publicação em tempo real** ([ROADMAP](ROADMAP.md), [docs/RELATORIO_0.4.0.md](docs/RELATORIO_0.4.0.md)): `uv-servico --http` com WebSocket e REST ([docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md), ADR-026), `uv-carga`, critérios D1–D4 e limites executados na `0.3.3`/`0.3.4`. Critério do ROADMAP: **BU → cliente p99 7,7 ms** com 1.000 clientes na noite de 2026 a 60× (meta < 50 ms); até 10.000 clientes, 35,6 ms.
- **Concluída por decisão do Luciano (13/10/2026), sem a validação independente** prevista no fim do [HANDOFF_0.4.0.md](docs/HANDOFF_0.4.0.md). O roteiro dessa validação continua no handoff e no §8 do relatório, para quem quiser fazê-la depois.
- Sem mudança de código desde a `0.3.4`: só versão, ROADMAP (etapa 4 ✅), status do relatório e cabeçalho do [HANDOFF_0.5.0.md](docs/HANDOFF_0.5.0.md) (a etapa 5 parte da `0.4.0`, em PATCH `0.4.x`). Verificação: a mesma da `0.3.4` (`compilar_dev.sh` sem avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9); `scripts/versao.sh verificar` coerente.

## [0.3.4] — 2026-10-13

- **Etapa 4: relatório e critérios executados** ([docs/RELATORIO_0.4.0.md](docs/RELATORIO_0.4.0.md); a etapa fica na `0.3.x` até a validação independente — sem `bump minor`):
  - **D1** (1º turno de 2026 a 60×, 1.000 clientes): **BU → cliente p99 7,7 ms** (meta < 50 ms), 0 conexões perdidas, 1.500/1.500 últimas mensagens idênticas ao arquivo, eventos = `eventos.csv` em 1.000/1.000 conexões, conferência 83/83 e 832/832; **D2** (+100 lentas): lentas juntadas e cortadas (`cliente_lento`), as outras com p99 8,0 ms, pico de RSS 432 MB; **D3** (2024 2º turno a 60×, 200 clientes): `kill -9` com reconexão (nenhum `instantaneo` com `seq` menor; publicação final = A da etapa 3, 51/51), rajada de 429 (`estado` com a pausa e a retomada 10 s depois), reenvio idêntico + anulação (alertas no `estado`, São Paulo corrigido e igual ao arquivo); **D4** (2024 2º turno a 10×, um cliente em todos os tópicos): 51/51 últimas mensagens = arquivo, `seq` só sobe, eventos 102/102, 102/102 situações; **escala**: 3.000 / 6.000 / 10.000 clientes com p99 11,7 / 21,3 / 35,6 ms (o limite de 50 ms não foi alcançado até 10.000), ~125 KB de RSS por conexão; **limites** (item 5): 6.000 conexões com `--max-clientes 5000` → 1.000 recusadas com 503; 1 MB → 1009; 1.000 msg/s → 1008 depois de 39 `erro taxa`; 219 tópicos → 200 aceitos e `limite_topicos`; *handshake* incompleto fechado em 10 s; caminhos do cliente no REST → 404; `POST` → 405; `ETag`/304; a apuração seguiu idêntica ao oficial durante os ataques (MA 219/219).
  - `uv-carga --limites`: o teste de tópicos vai em mensagens pequenas (com 300 tópicos numa só, caía no limite de tamanho, não no de tópicos) e o `ETag` é testado no primeiro tópico existente (antes, `br-c0001`, inexistente nos ensaios de 2024).
  - [docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §9: exemplos reais das mensagens (D4). ROADMAP: item "Carga" da etapa 4 marcado e o critério com o número medido (sem concluir a etapa).
  - **Handoff da etapa 5** ([docs/HANDOFF_0.5.0.md](docs/HANDOFF_0.5.0.md)) e ROADMAP da etapa 5 com os pedidos do Luciano: glassmorfismo tátil e glow arco-íris nos destaques, gráficos suaves, nenhum pulo na tela (só `transform`/`opacity`, CLS = 0), números em hodômetro, feed de marcos e curiosidades (primeira seção totalizada, seção que determinou o matematicamente, eleitores que faltam para o matematicamente, primeiro governador/senador/prefeito matematicamente eleito…); o que falta no servidor para isso (nomes de municípios e partidos no `candidatos.json`, mensagem `marco`, função `faltam_para_mat` no motor) por acréscimo ao protocolo.
- Verificação: `compilar_dev.sh` sem avisos (0 com `--all-targets`); testes de unidade do `uv-aquisicao` 14/14; motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Só o `uv-carga` mudou de código; nenhuma mudança na totalização nem no serviço.

## [0.3.3] — 2026-10-13

- **Difusão: correções guiadas pelas medidas do B e do D1** (1º turno de 2026 a 60×; [docs/RELATORIO_0.4.0.md](docs/RELATORIO_0.4.0.md) §1.2):
  - **Montagem preguiçosa da mensagem**: na `0.3.2` a thread `uv-difusao` montava a mensagem de toda versão de tópico (845 mil na noite, com *parse*, cópia e alocação) mesmo sem cliente, e disputava CPU e o alocador com o motor — **com `--http` e 0 clientes, BU → publicação p99 subia de 4,3 para 8,4 ms**. Agora ela só guarda o `Arc` dos bytes; a mensagem é montada uma vez, pela primeira conexão que a usa (`OnceLock`), nas threads HTTP; o REST serve os bytes do arquivo direto. Com 0 clientes: p50 0,805 / p99 4,80 / máx. 45 ms (sem `--http`, mesmo binário: 0,815 / 4,34 / 44).
  - **Aviso `seq_pulado` no máximo a cada 5 s por tópico e conexão** (com a soma das juntadas): com o `br-c0001` mudando ~100 vezes por segundo, um aviso por versão pulada era quase metade das mensagens (D1 da `0.3.2`: 21,9 M avisos em 50,9 M mensagens; agora 96,6 mil em 60,1 M). [docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) §4.4 atualizado.
  - **Threads `uv-http` com prioridade menor** (nice 10, `difusao::prioridade_baixa`): cedem a CPU ao motor, à gravação e à coleta (D1 da `0.3.2`: lote do motor p50 0,055 → 0,335 ms com 1.000 clientes; agora 0,205).
  - O bloco `difusao` da `saude.json` é calculado pela `uv-difusao` (o motor só copia o último); na parada, a saúde final recebe o bloco completo depois de fechadas as conexões.
  - `uv-carga`: `--amostras` (mensagens reais da conexão 0, para o protocolo) e, no relatório, as mudanças de `estado` e os avisos/eventos de apuração vistos pela conexão 0.
- Verificação: `compilar_dev.sh` sem avisos (0 com `--all-targets`); testes de unidade 14/14 do `uv-aquisicao`; motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Com o binário deste commit (versão impressa ainda `0.3.2`: compilado antes do *bump*), 1º turno de 2026 a 60×, simulador/serviço/carga em núcleos separados (`taskset`): **B sem `--http`** 83/83 e 832/832, BU → publicação p50 0,815 ms e p99 4,34 ms (o B da etapa 3, `0.2.6`, dava 0,675/4,3: a diferença é do ritmo ocioso dos `cs.json` da `0.2.8`, que faz 1,23 M requisições contra 1,03 M — não da difusão); **B com `--http` e 0 clientes** 83/83, p50 0,805 e p99 4,80 ms; **D1** (1.000 clientes) 83/83, **BU → cliente p50 3,2 ms, p99 7,7 ms**, máx. 47 ms, 1.500/1.500 últimas mensagens idênticas ao arquivo, eventos = `eventos.csv` em 1.000/1.000 conexões (3 de Presidente), 0 conexões perdidas, 0 regressões de `seq`; **D2** (+100 lentas) as 100 cortadas por `cliente_lento`, as 1.000 com p99 8,0 ms, pico de RSS do serviço 432 MB. Com 1.000 clientes o BU → publicação do serviço sobe para p50 2,9 / p99 10 ms (CPU e memória divididas com a carga e o simulador na mesma máquina; ver o relatório). Nenhuma mudança na totalização.

## [0.3.2] — 2026-10-13

- **Etapa 4 — difusão em tempo real no `uv-servico`** ([docs/HANDOFF_0.4.0.md](docs/HANDOFF_0.4.0.md); a etapa não está concluída: faltam D1–D4 completos, os limites do item 5 e a validação independente):
  - `--http <end:porta>` liga **WebSocket** (`/ws`) e **REST** (`/api/v1/estado`, `/topicos`, `/resultado/<tópico>`, `/eventos`); sem o parâmetro nada muda. Também `--max-clientes` (5.000) e `--http-threads` (4). Contrato em **[docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md)**: tópicos = abrangência × cargo (`br-c0001`…), `instantaneo` na inscrição, `atualizacao` a cada publicação, `evento` (matematicamente eleito/2º turno, revogação, apuração encerrada/reaberta), `estado`, `aviso` (`seq_pulado`, `desligamento`), `erro`. Toda mensagem de resultado leva `seq`, `secoes_apuradas`, `t_bu`, `t_envio` e **os bytes do arquivo publicado** como `resultado`, sem campo novo dentro dele.
  - Arquitetura (ADR-026, `aquisicao/src/difusao.rs`): a thread `uv-gravacao` converte cada JSON em `Bytes` e o entrega, **antes de gravar**, a uma thread nova `uv-difusao` (canal sem limite: a totalização nunca espera um cliente), que monta a mensagem do tópico uma vez e a põe num `tokio::sync::watch` (último valor por tópico); `axum` 0.8 num runtime próprio; cada conexão observa os `watch` dos seus tópicos e, lenta, recebe só a versão mais nova (aviso `seq_pulado`); 30 s sem conseguir escrever ou 4 MB de buffer derrubam a conexão. Limites: 200 tópicos, 20 msg/s (rajada 40), 4 KB por mensagem recebida, 10 s de *handshake*, `TCP_NODELAY`. Na partida a camada só escuta depois da publicação a frio; na parada limpa avisa e fecha as conexões depois da gravação final. `saude.json` ganha o bloco `difusao` (clientes, inscrições, mensagens/s, bytes/s, juntadas, desconexões por motivo, BU → envio e BU → difusão).
  - **O motor não mudou** (sem rede e sem `tokio`: `cargo tree -p urna-velox` sem `tokio`/`hyper`/`axum`); a única mudança no caminho da publicação é a `uv-gravacao` guardar `Bytes` em vez de `String` e entregar uma cópia de referência à difusão quando `--http` está ligado.
  - **`uv-carga`** (novo binário): N conexões em rampa, perfis de inscrição (`misto`, `br`, `todos`, lista), mede BU → cliente e servidor → cliente (histogramas), conta `seq` pulados, regressões e conexões perdidas, `--lentos`, `--reconectar`, `--limites`, e no fim confere a última mensagem de cada tópico **byte a byte** com o arquivo publicado e os eventos com o `eventos.csv`.
  - Documentação: [docs/PROTOCOLO_WS.md](docs/PROTOCOLO_WS.md) (novo), [docs/OPERACAO.md](docs/OPERACAO.md) §7 (ligar, `ulimit`, proxy reverso com TLS, saúde, clientes lentos), `ARCHITECTURE.md` §2.7, ADR-026, ROADMAP (itens da etapa 4 marcados com `v0.3.2`, sem concluir a etapa).
- Verificação: `compilar_dev.sh` sem avisos (0 também com `--all-targets`); testes de unidade novos `difusao::testes` (4: tópico do arquivo, mensagem com os bytes exatos, último valor na thread de difusão, e **protocolo de ponta a ponta num servidor real**: REST com `ETag`/304 e 404 para caminhos de fora da tabela, instantâneo/atualização byte a byte, limites de tópicos, taxa — 1008 —, tamanho — 1009 — e clientes — 503 —, eventos, estado, desligamento 1001); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço **9/9** — os 6 da etapa 3 sem `--http` (a publicação continua idêntica ao oficial: AM/RO 2/2, MA) e 3 novos: `difusao_d4_reduzido` (cliente WebSocket no teste, todos os tópicos: `seq` só sobe, última mensagem = arquivo, eventos = `eventos.csv`, 2/2), `difusao_cliente_lento` (50 conexões que não leem + 10 normais: serviço no mesmo tempo, BU → publicação p99 < 50 ms, normais 20/20 idênticas), `difusao_kill9_reconexao` (`kill -9` com 2.500 BUs e reinício: 20 clientes reconectam, nenhum `instantaneo` com `seq` menor, 40/40 idênticas, eventos 20/20). Fumaça AM+RO a 120× com 20 clientes: 40/40 idênticas, BU → cliente p99 7,3 ms. Nenhuma mudança na totalização: a conferência com os oficiais não muda. Dependências novas no `uv-aquisicao`: `axum` 0.8 (só `ws`, `tokio`, `http1`), `tokio-stream`, `tokio-tungstenite` 0.29 (a mesma versão que o `axum` usa: um `tungstenite` só na árvore).

## [0.3.1] — 2026-10-13

- **Handoff da etapa 4** ([docs/HANDOFF_0.4.0.md](docs/HANDOFF_0.4.0.md)): publicação em tempo real por WebSocket e REST mínimo dentro do `uv-servico` (`--http`), motor ainda sem rede e sem `tokio`, mesmos bytes do arquivo publicado para todos os clientes, último valor por tópico (cliente lento nunca atrasa a totalização), protocolo em `docs/PROTOCOLO_WS.md`, cliente de carga `uv-carga` e critérios D1–D4 com o simulador (D1: BU → cliente < 50 ms no p99, 1º turno de 2026 a 60×, 1.000 clientes).
- **Tags só nas versões "zeradas"** (`X.Y.0`, a pedido do Luciano): `scripts/versao.sh tag` (chamado pelo hook `post-commit`) não cria mais tag nos PATCH. As 43 tags de PATCH foram apagadas, no repositório local e no GitHub; ficam `v0.0.0`, `v0.1.0`, `v0.2.0` e `v0.3.0`. [docs/VERSIONAMENTO.md](docs/VERSIONAMENTO.md) e `CLAUDE.md` atualizados. Esta versão é a primeira sem tag.
- Verificação: sem mudança de código Rust; `compilar_dev.sh` sem avisos; `scripts/versao.sh verificar`; `scripts/versao.sh tag` na `0.3.1` responde "sem tag".

## [0.3.0] — 2026-10-13

- **Etapa 3 concluída — operação automatizada** ([ROADMAP](ROADMAP.md)): `uv-servico` roda a noite sozinho a partir do `candidatos.json` — acompanha os `cs.json` (dois ritmos: ocioso em rodízio e trabalhando), coleta `aux.json` e BUs dentro dos limites (≤ 100 req/s e pausa ≥ 10 min depois de qualquer 429 contra o TSE, impostos pelo host no código), totaliza em fluxo, calcula o "matematicamente", publica só o que mudou, retoma depois de `kill -9` pelo diário de seções, pede de novo sozinho as seções pendentes, trata reenvio e anulação depois de totalizada, confere com os oficiais no fim e mostra a apuração ao vivo com `--painel`; `uv-simulador` reproduz o site do TSE em `127.0.0.1` para os ensaios.
- Critérios ([docs/RELATORIO_0.3.0.md](docs/RELATORIO_0.3.0.md), binário `0.2.6`; repetidos depois nas `0.2.7`–`0.2.9` onde a mudança tocava o serviço): A — 2º turno de 2024 a 10×, 51/51 e 102/102, 0 HTTP 429, 0 duplicados, 0 revogações; B — 1º turno de 2026 a 60×, 83/83 e 832/832, 3 eventos de Presidente, BU → publicação p50 0,675 ms (meta < 1 ms); C — `kill -9` ×3, rajada de 429, reenvio, anulação depois de publicada, queda do site e rede ruim, sem perda nem duplicação; teste real no pleito 3221 por 31 min, só `cs.json`, 0 HTTP 429.
- **Concluída por decisão do Luciano sem a validação independente prevista no [handoff](docs/HANDOFF_0.3.0.md)** (outra sessão relendo o diff e repetindo A, B e C): os critérios foram executados e medidos pela mesma sessão que fez as correções da `0.2.6`.
- Pendente para 25/10 (relatório §7): `cs.json` de TO ainda 404 (o `candidatos.json` do 3221 segue `incompleto`); `s.ts` pré-apuração parcial em 9 UFs para Presidente (o fim automático pode não vir: usar `CONFERIR`); destinação presumida "Válido".
- Verificação: versão apenas (sem mudança de código desde a `0.2.9`); `compilar_dev.sh` sem avisos; `scripts/versao.sh verificar`.

## [0.2.9] — 2026-10-13

- **`--intervalo-cs-ocioso` passa a controlar o gasto**: na `0.2.8` cada ciclo ocioso lia `4 × intervalo` UFs, então o custo ficava em ~4 req/s qualquer que fosse o intervalo (aumentá-lo só juntava as mesmas leituras em lotes maiores) — contraintuitivo. Agora, contra o TSE, cada ciclo lê sempre 4 UFs em rodízio e o intervalo (mínimo 1 s) decide o gasto: 1 s = 4 req/s (padrão, inalterado; cada UF relida a cada ~7 s), 2 s = 2 req/s, 5 s = 0,8 req/s. Contra o simulador, todas as UFs a cada ciclo, como antes. [docs/OPERACAO.md](docs/OPERACAO.md) §2 atualizado.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaio `criterio_a_reduzido` OK; site real, pleito 3221, 60 s ocioso com `--intervalo-cs-ocioso 2`: 144 requisições, todas `cs.json` (28 do primeiro ciclo completo + 4 a cada 2 s), 2,4 req/s em média (com 1 s, na `0.2.8`: 4,4), 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar`.

## [0.2.8] — 2026-10-13

- **`uv-servico`: dois ritmos de releitura dos `cs.json`** (a pedido do Luciano):
  - **ocioso** (fila vazia e nada novo no último ciclo — antes da eleição e nos intervalos da noite): um ciclo por `--intervalo-cs-ocioso` (novo, padrão 1 s), lendo só algumas UFs em rodízio; contra o TSE no máximo 4 req/s (`OCIOSO_REQ_S`: 4 UFs por ciclo de 1 s, cada UF relida a cada ~7 s com 28 UFs). Percebe a primeira seção publicada em segundos, e não em até 30 s;
  - **trabalhando** (seções chegando ou na fila): todas as UFs a cada `--intervalo-cs` (padrão 30 s, como antes). Ao achar seção nova no rodízio, o ciclo seguinte é completo e imediato; quando um ciclo não traz nada e a fila esvazia, volta a ocioso. `saude.json` → `ritmo_cs`; `servico.log` → evento `ritmo_cs` a cada troca.
  - **Os dois intervalos agora são em hora do site**: com `--aceleracao-local` são divididos pela aceleração (antes o `--intervalo-cs` era em segundos reais e tinha de ser ajustado à mão: `--intervalo-cs 0.06` a 500×). Os ensaios `aquisicao/tests/servico.rs` e os exemplos de [docs/OPERACAO.md](docs/OPERACAO.md) não passam mais `--intervalo-cs`. **Quem passar `--intervalo-cs` ajustado à mão contra o simulador agora o terá dividido de novo** (o mínimo é 50 ms); contra o TSE nada muda nesse parâmetro.
  - Custo contra o TSE antes da eleição: ~4 req/s (~14 mil requisições por hora, quase todas 304), eram ~0,5 req/s; dentro do ≤ 5 req/s do teste pré-eleição. Com 5 UFs por ciclo, 60 s deram 5,38 req/s (o primeiro ciclo é completo): por isso 4.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 6/6 (agora sem `--intervalo-cs`); 1º turno de 2026 a 500× só com `--aceleracao-local 500` (sem `--intervalo-cs`): 83/83 e 832/832 em 75 s; site real, pleito 3221, 60 s ocioso: 264 requisições, todas `cs.json` (228 com 304), 4,4 req/s em média contando o primeiro ciclo completo, 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar`.

## [0.2.7] — 2026-10-13

- **`uv-servico --painel [prefixo]`**: painel no terminal com a totalização ao vivo de uma abrangência — por padrão `br-c0001` (Presidente no Brasil); `--painel sp-c0003` = Governador de SP (prefixo do arquivo em `publicacao/`). Mostra seções apuradas, BUs aplicados, último BU, comparecimento, válidos, brancos e nulos, cada candidato com nome (do `candidatos.json`), votos, %, barra, o "matematicamente" com a sequência e a hora em que aconteceu, e a situação final. Desenhado pela thread `uv-gravacao` a partir do JSON que ela acabou de gravar, no máximo 4 quadros por segundo e sempre na publicação encerrada — nada roda na thread do motor. Num terminal redesenha a tela; redirecionado, escreve uma linha de resumo por quadro. Exemplo em [docs/OPERACAO.md](docs/OPERACAO.md) §2. Teste de unidade `painel_desenha`.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaio `criterio_a_reduzido` OK (2/2 idênticos); 1º turno de 2026 a 500× com `--painel` contra o `uv-simulador`: 195 quadros em 76,6 s (de 0 % a "DEFINIDO (ENCERRADA)"), 83/83 arquivos e 832/832 situações idênticos — o painel não muda a publicação. Nenhuma mudança na totalização.

## [0.2.6] — 2026-10-13

- **Etapa 3 — verificação completa com o binário final e correções** ([docs/RELATORIO_0.3.0.md](docs/RELATORIO_0.3.0.md); a etapa não está concluída, falta a validação independente):
  - **Registro do que entrou na `0.2.4` sem constar do CHANGELOG** (o commit `14a0298`, "docs: relatório de auditoria", também trouxe código do serviço): thread `uv-gravacao` para gravar a publicação fora do laço do motor; cache de totais por (UF, cargo) no `Publicador` (`ativar_cache_uf`/`invalidar_uf`/`invalidar_tudo`, teste `publicador_cache_por_uf`); latências com histograma da execução inteira (`p50_total`, `p99_total`, `max_total`); `M_ARENA_MAX=2` e `malloc_trim` depois de cada ciclo de `cs.json` (RSS do B: pico 365 → 288 MB); 4 threads de rede; sondagem das não instaladas só no exterior (no B de 12/10 pedia 2.596 `aux.json` 404 em UFs que ainda iam receber BU); `Situ::Bu` com a impressão do hash; `docs/OPERACAO.md`, `docs/AQUISICAO.md` §5.1, `ARCHITECTURE.md` §2.6.
  - **Seções pendentes pedidas de novo sozinhas** (`servico.rs` `Pend`, `repedir_pendentes`, `Pedido::Recoletar`): uma seção publicada cujo `aux.json`/BU não veio depois das novas tentativas era pedida de novo só se a disponibilização mudasse ou numa retomada; com o site fora do ar por 2 min reais a 60× (ensaio C-queda, 20 UFs), 160 seções ficaram pendentes e a apuração nunca encerrou. Agora o motor as pede de novo a cada 2, 4, 8, 16, 32 min (hora do site), até 12 vezes (alerta `pendente_sem_solucao`), só as que têm disponibilização; `secoes.repedidas` na saúde.
  - **`sync_data` fora da thread do motor** (thread `uv-sincronia`, `Escritor::descarregar_e_copiar`): o `fsync` dos ~30 arquivos (pacotes e diário) a cada 2 s parava o motor e atrasava os BUs que chegavam nesse intervalo.
  - **`saude.json`**: custos novos (`lote`, `saude`, `verificacao`, `sincronia_disco`); uma ordenação por amostra; contagem por UF sem uma `String` por seção.
  - **Alerta `secoes_esperadas_incoerentes` mais completo**: compara Brasil × soma das UFs mesmo com UFs sem `s.ts` e acusa `s.ts` < 90 % da lista do `cs.json` (Presidente/Governador, fora o exterior). No 3221 em 13/10 ele passava calado ("1 UF sem s.ts") com SP em 33.403 de 91.326 seções e o Brasil 494.864 × 358.396 na soma das UFs; agora lista CE, ES, GO, MA, PA, PE, PI, RJ e SP.
  - **`uv-simulador` recusa reenvio/anulação marcados antes da disponibilização da seção** (viravam em silêncio "dois hashes desde o início"/"anulada desde o início", e o ensaio não provava o que dizia).
  - Teste `segundo_turno_2026_candidatos_pre_apuracao`: lê os `-u.json` pré-apuração de `fontes/u/` (onde o `preparar` os guarda) e exige pelo menos um; falhava desde a regeração do `candidatos.json` do 3221 (12/10 21:31). Nenhum número mudou.
  - `candidatos.json` do 3221 regerado em 13/10 (72 requisições, ≤ 5 req/s): o `cs.json` de SE já existe (500.111 seções, eram 494.044); o de TO continua em 404, e o arquivo segue `incompleto`.
  - Ensaio `queda_do_simulador`: queda de 60 s reais no pico de chegada de AM e RO (era 10 s às 17:40, quando já não havia seção em coleta), exigindo 0 pendentes.
- Verificação (binário 0.2.6, detalhes e comandos no [relatório](docs/RELATORIO_0.3.0.md) §3–5): `compilar_dev.sh` sem avisos, 46 testes de unidade; motor `--ignored` 13/13 (2026 83/83, 2024 5.574/5.574 e 51/51, situação e matematicamente, **sem mudança de número**); ensaios `--ignored` do serviço 6/6. Contra o `uv-simulador`: **A** (2024 2º turno, 10×) 51/51 e 102/102 pela conferência do próprio serviço e pelo `uv-motor conferir-publicacao`, 0 HTTP 429, 0 duplicados, 0 revogações, matematicamente com mediana de 1.562 s de antecedência (cenário "site" do `PREFEITO_2024_2T.md`: 1.561 s); **B** (2026 1º turno, 60×) 83/83 e 832/832, 3 eventos de Presidente, 0 revogações, BU → publicação p50 0,675 ms e p99 4,3 ms (0.2.5: 0,825 e 172,5 ms); **C** `kill -9` ×3 (publicação idêntica à do A), rajada de 429 (pausa de 10 s reais = 600 s simulados, 0 requisições nela), dois reenvios idênticos, anulação depois de publicada (só o arquivo do município diverge do oficial, como esperado), queda de 120 s e queda com `--timeout 2` (45 pendentes pedidas de novo e resolvidas), rede com 2 % de 503 e latência de 20–400 ms. **Teste real** no 3221 (31 min, binário 0.2.6): 868 requisições, todas `cs.json`, 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar` (43 arquivos, "apurando"); rede real da sessão ≈ 2.030 requisições, nenhum BU.

## [0.2.5] — 2026-10-13

- Ajustes nos gráficos do benchmark comparativo (não fazem parte desta cópia).

## [0.2.4] — 2026-10-13

- **Auditoria de controle do projeto:**
  - Levantamento completo de requisitos e afirmações que não conferem no código (parsing zero-copy vs alocações dinâmicas na heap, promessa de `rasn`/`rayon`/`seqlock`/`epoca` ausentes no código, falta de validação de assinaturas ECDSA da urna, badge de licença em desacordo com licença Source-Available).
  - Identificação de problemas e bugs invisíveis (vulnerabilidade de Path Traversal/Zip Slip em `zip.rs`, descompressão sem teto máximo/risco de Zip Bomb, divergência de percentuais por arredondamento flutuante em vez de truncamento oficial do TSE, e risco de concorrência na escrita desacoplada).
  - Mapeamento de recursos necessários pendentes para o 2º turno de 2026 (servidor Linux dedicado de produção com IP exclusivo para evitar penalidade de 429 da Akamai, pendência dos `cs.json` de SE e TO no pleito 3221 e inconsistência no `s.ts` de SP nos `-u.json` pré-apuração).

## [0.2.3] — 2026-10-13

- **Módulo de Escalabilidade Segura (arquitetura da v2.0.0)** ([docs/ESCALABILIDADE_SEGURA.md](docs/ESCALABILIDADE_SEGURA.md)):
  - Especificação completa da federação colaborativa de nós coletores sem coordenação central, balanceamento determinístico por *Rendezvous Hashing* (HRW) com tolerância a falhas e dinamicidade via *heartbeats* leves em storage público.
  - Distribuição massiva aos clientes invertida: eliminação do *push* ativo via conexões abertas permanentes no servidor único, adotando *Static-File Edge Broadcast* via CDN global com custo R$ 0,00 de tráfego de saída (Edge Cloudflare R2 / KV / Pages com SPA hospedada no GitHub Pages).
  - Segurança Zero-Trust e validação descentralizada por quórum criptográfico de operadores autorizados (chaves Ed25519) e integridade dos votos amarrada às assinaturas digitais originais da urna eletrônica do TSE (ECDSA / ICP-Brasil).
  - Adoção de SPA / PWA universal e motor em WebAssembly (Wasm) no navegador, eliminando a necessidade e burocracia de aplicativos nativos de lojas mobile (Android/iOS).
  - Novos registros de decisões de arquitetura em [docs/ARCHITECTURE_DECISIONS.md](docs/ARCHITECTURE_DECISIONS.md): ADR-021 (Static-File Edge Broadcast), ADR-022 (Rendezvous Hashing), ADR-023 (Zero-Trust e quórum Ed25519), ADR-024 (Wasm no navegador) e ADR-025 (SPA/PWA em vez de apps nativos).
  - Lista formal com 17 requisitos de engenharia rastreáveis (`REQ-ESC-01` a `REQ-ESC-17`).
  - Alinhamento de roadmap: `v1.0.0` (majoritários em nó único), `v2.0.0` (escalabilidade segura) e `v3.0.0` (cargos proporcionais).

## [0.2.2] — 2026-10-12

- **Etapa 3 — operação automatizada** ([docs/HANDOFF_0.3.0.md](docs/HANDOFF_0.3.0.md), itens 1 a 4; a etapa não está concluída):
  - **Motor — retirar e substituir um BU** (ADR-019): `Estado::retirar` (mesmo agregador, `fetch_sub` em votos, comparecimento, `aptos`, `secoes`, `bus` e contadores de conferência; limpa o bit de `vistos`), `substituir`, `aplicar_secao` (anulação dada pela situação do `aux.json`), `aplicada`. `Acompanhamento::bu_alterado` reavalia a circunscrição e guarda o motivo, que vai com qualquer revogação de "matematicamente" (`Evento::motivo`). Testes `motor/tests/retirada.rs` com BUs reais das fixtures: aplicar → retirar volta ao estado zero (todos os contadores e o checksum); substituir A por B = aplicar só B; anular depois = anulada desde o início; caso real MA 07315/0023/0354. `pacote::indice`/`ler_bu_em`/`Escritor::gravar_pos`/`descarregar`; `Publicador::nao_instalada`/`secoes`.
  - **`uv-simulador`** (novo binário, ADR-020): site de resultados em `127.0.0.1` com os caminhos do real (`cs.json` reconstruído no instante simulado com `ETag`/304, `aux.json` no formato real, `-bu.dat` dos `.uvpk`, `-u.json` oficiais depois do último BU), relógio acelerado, `--horario recebido` (2026: o `cs.json` foi coletado dias depois da noite), `--corte`, e falhas por parâmetro: rajadas de 429, latência e 5xx com semente fixa, reenvio idêntico, anulação depois de publicada, queda; `simulador.log` com cada requisição.
  - **`uv-servico`** (novo binário): `candidatos.json` → `cs.json` de todas as UFs com `If-None-Match` → `aux.json` → BU → motor numa thread nativa (o motor continua sem rede e sem `tokio`) → matematicamente a cada lote → publicação atômica só das abrangências que mudaram → eventos. Diário de seções só de acréscimo (`secoes.log`) e `.uvpk` só de acréscimo: retomada depois de `kill -9` sem perder nem duplicar BU, BUs órfãos do pacote reaproveitados; reenvio idêntico (alerta, totais inalterados), reenvio diferente e anulação depois de totalizada (substituição com alerta); não instaladas contadas em `s.st`; situação desconhecida = pendente com alerta. Fim da apuração pelo `s.ts` dos candidatos, com fila vazia e toda seção publicada resolvida, ou por `CONFERIR`/`--conferir-a-cada`; conferência automática (`conferencia.md`/`.json`, ≤ 5 req/s); publicação reaberta se chegar BU depois; contra o site real nunca sai sozinho. `saude.json` a cada 2 s, `servico.log` em linhas JSON, `alertas.csv`; parada limpa com `SIGTERM`.
  - **Rede** (`coletor.rs`): `limites`/`base_local`/`Rede::para` — contra qualquer host que não seja `127.0.0.1`/`localhost`/`::1`, teto de 100 req/s e pausa global ≥ 600 s depois de **qualquer** 429, qualquer que seja a configuração, e aceleração recusada; contra o simulador, teto e pausa escalados pela aceleração e a rede só aceita URLs dele. Testes de unidade `servidor_local` e `limites_nao_afrouxam_contra_o_tse`.
  - **Sondagem das não instaladas** (única exceção a "seção sem disponibilização não gera requisição"): só depois de 30 min (hora do site) sem BU novo, com toda seção publicada resolvida, em abrangências quase completas (≤ 5 % ou 50 seções), nunca a do Brasil, ≤ 1.500 por abrangência e 3.000 na noite, ≤ 2 req/s — milhares de 404 seguidos já provocaram 429 a ~70 req/s (docs/AQUISICAO.md §1.3).
  - `candidatos.json`: `abrangencias[].secoes` (`s.ts` pré-apuração), opcional ([docs/FORMATO_CANDIDATOS.md](docs/FORMATO_CANDIDATOS.md)); os quatro `candidatos.json` locais regerados offline (iguais aos anteriores fora esse campo). Alerta na partida quando o `s.ts` é incoerente — **o do 3221 baixado em 12/10 é** (Brasil 488.941 ≠ soma das UFs; SP 33.403 seções com 91.326 no `cs.json`); o eleitorado das circunscrições (Brasil e os 5 Governadores) está coerente.
  - Crate `uv-aquisicao` passa a ter biblioteca (`src/lib.rs`) usada pelos três binários; dependências `hyper` 1, `hyper-util`, `http-body-util`, `libc`.
- Verificação: `compilar_dev.sh` sem avisos (15 + 4 + 1 + 7 + 4 + 5 + 8 testes); `--ignored` do motor 13/13 (2026 83/83, 2024 5.574/5.574 e 51/51, situação e matematicamente, sem mudança de número); ensaios `--ignored` do serviço contra o simulador 6/6: critério A reduzido (AM e RO a 120×, 2/2 idênticos, 0 HTTP 429, 0 sondagem), `kill -9` três vezes (mesmo resultado da execução sem queda, 5.102 "aplicado" no diário), rajada de 429 (nenhuma requisição durante a pausa), reenvio idêntico, anulação depois de publicada (MA, 1º turno de 2024: 219/219 arquivos e 608/608 situações idênticos), queda do simulador.

## [0.2.1] — 2026-10-12

- [docs/HANDOFF_0.3.0.md](docs/HANDOFF_0.3.0.md): plano da etapa 3 (operação automatizada) para outra sessão executar: retirar/substituir BU no motor (reenvio e anulação depois de totalizada), site simulado local `uv-simulador` (os BUs de 2024 e 2026 não podem ser baixados de novo), serviço `uv-servico` (acompanha `cs.json`, coleta ≤ 95 req/s com pausa de 10 min após 429, totaliza em fluxo, matematicamente, publicação, diário de seções e retomada, saúde em arquivo, conferência automática no fim), critérios A/B/C no simulador e teste real sem BUs no pleito 3221. ROADMAP aponta para o handoff.

## [0.2.0] — 2026-10-12

- **Etapa 2 concluída — catálogo e candidatos pré-eleição** ([ROADMAP](ROADMAP.md)): `uv-coletor preparar` monta o `candidatos.json` só com arquivos publicados antes da apuração ([docs/FORMATO_CANDIDATOS.md](docs/FORMATO_CANDIDATOS.md)) e o `uv-motor` totaliza, publica e reproduz a noite com ele (`--candidatos`).
- Revalidação ([docs/VALIDACAO_0.2.0.md](docs/VALIDACAO_0.2.0.md) §6) dos seis itens da validação sobre a `v0.1.7`: votos idênticos ao oficial nos três conjuntos; com a destinação presumida "Válido", só as 33 situações (32 municípios) de candidatos cassados/indeferidos de 2024 divergem; noite de 2026 com 3 eventos de Presidente, 0 revogações; 3221 a frio com os 43 arquivos certos. Correções e revalidação feitas pela mesma sessão.
- Pendente para 25/10: `cs.json` de SE e TO do 3221 (404 em 12/10) — o `preparar` sai com código 2 até serem publicados.

## [0.1.7] — 2026-10-12

- **Correções da validação da etapa 2** ([docs/VALIDACAO_0.2.0.md](docs/VALIDACAO_0.2.0.md) §3; detalhes e evidências em [docs/RELATORIO_0.2.0.md](docs/RELATORIO_0.2.0.md) §8):
  - `uv-coletor preparar` reescrito (`aquisicao/src/preparo.rs`): baixa ele-c, `mun-e…-cm.json`, os `-u.json` de cada abrangência e os `cs.json`, com cache em `--fontes`, 1 conexão, ≤ 5 req/s e parada no 1º HTTP 429; `--offline`, `--atualizar`, `--cargos`, `--ignorar-destinacao`; fonte ausente → `incompleto` e código de saída 2.
  - Só a circunscrição do cargo vira circunscrição (Presidente: 1, no Brasil); as outras abrangências vão para `cargos[].abrangencias` com o eleitorado. Na noite de 2026: 3 eventos de Presidente (eram 64), `nao_confere` 0 (era 23).
  - Municípios de cada eleição a partir do `cm.json` (`eleicoes[].municipios`): o 3221 passa a ter 28 UFs e 5.757 municípios; cargo municipal só tem região nos municípios da eleição.
  - `Publicador`: abrangências do `candidatos.json` (consultas do 3221 por UF; a frio, 43 arquivos com os mesmos nomes dos `-u.json` pré-apuração, eram 721) e eleição por circunscrição (`Candidatos::eleicao_de`); `st` vazio enquanto o `estado` é "apurando".
  - `Catalogo::de_arquivo_candidatos` devolve `Result` (UF inválida é erro, não pânico).
  - `candidatos.json` ganha `destinacao_presumida` e `incompleto`; premissas (destinação "Válido", circunscrição "finalizada", contador de votáveis desconhecidos) em [docs/FORMATO_CANDIDATOS.md](docs/FORMATO_CANDIDATOS.md); `ARCHITECTURE.md` §3.1 com o slot `OFF_VNT`.
  - Testes: `*_pre_eleicao` falham sem o arquivo e, em 2024 1º turno, admitem só as 33 situações (32 municípios) com candidato não apto no oficial; `matematica::*_candidatos` (3 conjuntos, 0 revogações); `segundo_turno_2026_candidatos_pre_apuracao`; fixture e teste unitário cobrem abrangências, eleição por circunscrição e UF inválida. 13 testes `--ignored` passam.

## [0.1.6] — 2026-10-12

- [docs/VALIDACAO_0.2.0.md](docs/VALIDACAO_0.2.0.md): validação independente da etapa 2 como entregue na `v0.1.5` — **não aprovada para 0.2.0**: Presidente com 29 circunscrições (matematicamente "eleito no AC"), pleito 3221 sem SE e TO, consultas publicadas por município (683 arquivos), `st` "Eleito" com zero voto, testes "pré-eleição" montados com os arquivos oficiais já apurados; verificado e correto: `OFF_VNT`, desempenho, eleitorado ≥ Σ aptos nos três conjuntos, rede. As correções vêm nas versões seguintes (feitas por esta sessão, a pedido do Luciano).
- Publicação das pendências: `v0.1.4` (estava só local) e `v0.1.5` (trabalho sem commit). **A mensagem do commit da `v0.1.5` (`d0813df`) saiu errada** — repete a da `v0.1.3` ("docs: handoff da etapa 2 usa os arquivos…") por falha ao gravar o arquivo da mensagem; o conteúdo é o da entrada [0.1.5] abaixo. Não reescrevi o histórico já publicado.

## [0.1.5] — 2026-10-12

- **Catálogo e candidatos pré-eleição** ([docs/HANDOFF_0.2.0.md](docs/HANDOFF_0.2.0.md), passos 1, 2, 4 e 5):
  - Especificação e modelo de dados estruturado `candidatos.json` (`docs/FORMATO_CANDIDATOS.md`), contendo pleito, UFs, municípios, seções, eleições, cargos, vagas e candidatos homologados com número, nome, destinação e data de nascimento.
  - Implementado carregador `Candidatos::ler(caminho)` e construtor direto de catálogo `Catalogo::de_arquivo_candidatos(arq)` sem necessidade de processar BUs.
  - Subcomando `uv-coletor preparar --pleito <P> --saida <DIR>` implementado para consolidação automática de candidatos a partir dos metadados pré-eleição ou de fontes locais em cache.
  - Binário `uv-motor` (comandos `totalizar`, `conferir`, `noite`) passa a aceitar a opção `--candidatos <caminho>` para carregar o catálogo e os candidatos pré-eleição.
  - Preparado e verificado o caso de teste real do 2º turno de 2026 (pleito 3221): 26 UFs, 5.543 municípios, 494.044 seções, 11 eleições (Presidente com 2 candidatos, Governador nas 5 UFs com 2 candidatos, 6 suplementares de Prefeito e 3 consultas); totalização a frio sem BUs executa com sucesso gerando 721 arquivos de publicação com estado `"apurando"`.
  - Novos testes de integração automatizados (`--ignored`) executando a totalização e conferência com catálogo pré-eleição nos 3 conjuntos reais: 2026 1º turno (83/83 idênticos), 2024 1º turno (5.574/5.574 idênticos) e 2024 2º turno (51/51 idênticos).
  - Fixture versionada `motor/tests/fixtures/candidatos_exemplo.json` e testes unitários de carregamento adicionados.

## [0.1.4] — 2026-10-12

- **Votável desconhecido vira alerta e nulo técnico** ([docs/HANDOFF_0.2.0.md](docs/HANDOFF_0.2.0.md), passo 3):
  - Adicionado slot `OFF_VNT` no layout da região (`motor/src/catalogo.rs`), com `EXTRA = N_LEGENDA + 5` (105). Essa mudança no layout altera o checksum estrutural por região, sem alterar totais de votos nominais, brancos ou comparecimento.
  - Votos nominais em candidatos ausentes do catálogo pré-eleição são contabilizados em `OFF_VNT`, contam na soma da seção para validação de integridade de comparecimento (`soma == comp * vagas`) e são registrados como alertas consultáveis em `Estado::lista_alertas_vnt()` (com cargo, município, número votado e votos).
  - `Totais` em `motor/src/resultado.rs` ganha campo `vnt: u64` lido diretamente do vetor de tally e consolidado nas UFs e no Brasil.
  - `Publicador::valor` e `conferir_votos` unificam o cálculo de `vnt`: somam `t.vnt` às sobras nominais, garantindo compatibilidade total com o catálogo montado de BUs e com o catálogo pré-eleição.
  - Testes unitários e testes completos (`--ignored`: 83/83 em 2026, 5.574/5.574 e 51/51 em 2024) 100% aprovados.

## [0.1.3] — 2026-10-12

- [docs/HANDOFF_0.2.0.md](docs/HANDOFF_0.2.0.md): o TSE já gerou os arquivos de candidatos do 2º turno de 2026 (pleito 3221), segundo o Luciano. Eles passam a ser a fonte principal e o caso de teste real da etapa 2, usados desde o passo 1, e a validação vai conferir os candidatos do 3221 com esses arquivos. Nenhuma mudança de código.

## [0.1.2] — 2026-10-12

- **"Cadastro" passa a se chamar "candidatos"**, a pedido do Luciano, para não confundir com o cadastro de eleitores: módulo `motor/src/cadastro.rs` → `motor/src/candidatos.rs`, tipo `Cadastro` → `Candidatos` (`Candidatos::dos_oficiais`, `dos_arquivos_oficiais`), campo `Config::cadastro` do replay → `candidatos`; o contador de seções fora do catálogo passa de `fora_cadastro` a `fora_catalogo` ("fora do catálogo" nas mensagens). Documentação atual (ARCHITECTURE, ROADMAP, FORMATO_RESULTADO, `motor/README.md`) e [docs/HANDOFF_0.2.0.md](docs/HANDOFF_0.2.0.md) atualizados: o arquivo da etapa 2 será `candidatos.json` (`docs/FORMATO_CANDIDATOS.md`, opção `--candidatos`). As entradas antigas deste CHANGELOG mantêm o nome da época.
- Só renomeação, sem mudança de comportamento: testes normais (30) e completos (`--ignored`: 2026 83/83, 2024 5.574/5.574 e 51/51, com situação e matematicamente) passam iguais.

## [0.1.1] — 2026-10-12

- [docs/HANDOFF_0.2.0.md](docs/HANDOFF_0.2.0.md): plano da etapa 2 (catálogo e cadastro pré-eleição, alvo 16/10) para ser executado por outra sessão e validado de forma independente — sondagem das fontes publicadas antes da apuração (pleito 3221 como prova), `cadastro.json` local documentado, `uv-coletor preparar`, `uv-motor … --cadastro`, votável desconhecido como alerta e nulo técnico sem depender do catálogo dos BUs, testes com o cadastro pré-eleição nos três conjuntos, preparo do pleito 3221; quem executa para na `0.1.x`, e o `bump minor` para `0.2.0` vem depois da validação. Link no ROADMAP.
- Nenhuma mudança de código ou de resultado.

## [0.1.0] — 2026-10-12

**Etapa 1 do roadmap concluída: o motor** ([handoff](docs/HANDOFF_0.1.0.md), passos 0.0.23–0.0.27). O critério de pronto foi atingido:

- Crate **`urna-velox`** (`motor/`) no workspace, com o binário **`uv-motor`** (renomeado de `urna-velox` a pedido do Luciano: todos os executáveis do projeto começam com `uv-`); `uv-coletor` depende dele; versão e hooks coerentes (`scripts/versao.sh`, `.githooks/pre-commit`, um `Cargo.lock` na raiz).
- **Testes automatizados reproduzem as conferências da etapa 0** com a situação de cada candidato calculada pelo motor: 1º turno de 2026 **83/83** abrangências e **832/832** situações; Prefeito de 2024 **5.569/5.569** municípios (+ 5 consultas; **15.042/15.042** situações) e **51/51** no 2º turno (**102/102**) — pelo estado e pelas publicações JSON. Unidade + fixtures: `cargo test --release -p urna-velox` (30 testes, < 1 s); completos: `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (6 testes, ~15 s).
- **Matematicamente eleito / 2º turno** no motor: nas noites reproduzidas (2026, 2024 1º e 2º turnos) nunca revogado e sempre igual ao eleito oficial; 2º turno de 2024 com o motor: mediana de 26 min antes da totalização do TSE.
- **Formato JSON documentado** ([docs/FORMATO_RESULTADO.md](docs/FORMATO_RESULTADO.md)) e usado pelo replay e pelo `uv-motor totalizar`.
- **Desempenho** (i7-14700, WSL), antes (v0.0.22) → depois: totalização em lote numa thread, 2026 2,86 → **2,14 µs por BU** e 2024 1º turno 1,75 → 1,57 µs (mesma ordem; variação de cache); **com situação matemática recalculada por lote**, 3,1–3,3 µs (2026), 2,1–2,3 µs (2024 1º turno), 0,4 µs (2º turno) — critério < 10 µs; eleição geral inteira em memória com 28 threads **0,119 s** (critério < 1 s). Replay 60× de 2026 com publicação JSON e matematicamente: latência p50 5,1 ms, p99 12,8 ms.
- **Cobertura** (`scripts/cobertura.sh`, só testes sem `--ignored`): motor **84,3 %** das linhas e 81,9 % das funções (90 % das linhas sem o binário `main.rs`); `matematica.rs` e `situacao.rs` 100 %. `uv-aquisicao` 9,9 % (o núcleo saiu de lá). Novo `tests/publicacao.rs`: publicação em pasta e conferência dos arquivos, publicação sem cadastro, noite e antecedência sobre o TSE, retomada de pacote com registro incompleto.
- Documentação: [ARCHITECTURE.md](ARCHITECTURE.md) §2.5 (o crate, módulos e fluxo) e tabela do layout da região atualizada; [motor/README.md](motor/README.md); [README.md](README.md) com os comandos reais do binário (a seção antiga descrevia comandos que não existem); `CLAUDE.md`: a verificação contra os oficiais passa a ser pelos testes automatizados (os scripts Python ficam como conferência secundária); ROADMAP com a etapa 1 ✅ e a cobertura de cargos com a situação "no motor".
- Próxima etapa: **0.2.0, catálogo pré-eleição** (cadastro e catálogo montados antes do primeiro BU, a partir do site do TSE, para o pleito 3221).

## [0.0.27] — 2026-10-12

Etapa 1, passo 5: **formato JSON do resultado, documentado e usado pelo replay**, e o binário `urna-velox totalizar | conferir`.

- [docs/FORMATO_RESULTADO.md](docs/FORMATO_RESULTADO.md) (versão 1): um JSON por (eleição, cargo, abrangência), `<abr>-c<cargo>-e<eleição>.json` — `v`, `eleicao`, `turno`, `cargo`, `abr`, `circ` (circunscrição), `seq`, `ultimo_bu`, `encerrada`, `secoes {total, apuradas}`, `eleitorado`, `apurado`, `comparecimento`, `vb`, `vn`, `vnt`, `validos`, `legenda`, `estado` (`apurando`, `matematicamente_eleito`, `matematicamente_2t`, `definido`, `sem_eleito`) e `cand[] {n, v, pct, dvt, st, mat}`; a situação final (`st`) sai só na publicação encerrada; durante a apuração vale `mat` (estado, sequência e horário). É o contrato do WebSocket (etapa 4) e do site (etapa 5). Eventos do "matematicamente" em CSV (`replay_eventos.csv`).
- Motor: `resultado::Publicador` (monta e grava as publicações; totais de UF e Brasil pelas listas de municípios por UF), `Estado::secoes` (seções totalizadas por município), `Catalogo::secoes_por_municipio`, `Cadastro::eleitorado_abr` (eleitorado de cada abrangência publicada), `conferencia::conferir_publicacao(es)` e `conferir_pasta_publicacoes`.
- Binário `urna-velox`: **`totalizar`** (lote, 1 ou N threads, `--saida` com os JSON), **`conferir`** (votos e situação pelo estado e pelas publicações; código 1 se divergir), **`conferir-publicacao`** (confere uma pasta de JSON já gravados, ex.: a do replay), além de `noite` e `inspecionar`.
- **Replay** (`uv-coletor replay`) publica no formato novo (no lugar do JSON mínimo): Brasil, UFs e municípios tocados a cada ciclo; com **`--oficial`** (cadastro) recalcula o matematicamente a cada ciclo (`Acompanhamento`), grava `replay_eventos.csv` e falha se houver revogação; no fim, uma publicação encerrada de tudo. Sem `--oficial` continua funcionando (sem situação). O publicador passou a rodar em `thread::scope`.
- **Conferências pelo JSON publicado** (testes `--ignored` e `urna-velox conferir`): 2026 **83/83** e **832/832** situações; 2024 1º turno **5.574/5.574** e **15.042/15.042**; 2º turno **51/51** e **102/102** — idênticas às do estado. Fixtures: 10 + 1 arquivos e os campos do formato (versão, abrangência, circunscrição, seções, eleitorado, `pct` 51,75 = `pvap` oficial, `definido`/`sem_eleito`/`apurando`, `st` vazio fora da publicação encerrada, nome do arquivo).
- **Replay 60× com o formato novo e o matematicamente** (a publicação final conferida com `urna-velox conferir-publicacao`, idêntica ao oficial): 2026 (Presidente, Governador, Senador; 499.207 BUs, 13.993 publicações, 144 eventos) latência p50 **5,1 ms**, p99 **12,8 ms** (antes, com o JSON mínimo: 6,7 / 14,2 ms), ciclo de publicação p50 2,4 ms; 2024 2º turno (chegadas da simulação do site) p50 0,85 ms, p99 2,1 ms (antes 0,65 / 1,73 ms), 102 eventos.
- `urna-velox totalizar` no 1º turno de 2026 (catálogo do `cs.json`): 2,14 µs por BU numa thread, 0,119 s com 28 threads; 83 JSON gravados em 0,01 s.
- Regressão do `uv-coletor totalizar` (`conferir_tres.sh`): idêntica à v0.0.22.

## [0.0.26] — 2026-10-12

Etapa 1, passo 4: **matematicamente eleito / matematicamente 2º turno no motor**, com o teste de propriedade sobre as noites reais.

- Novo módulo `matematica` (`avaliar`): com `R` = eleitorado − aptos das seções já totalizadas (os aptos de cada BU passam a ser somados por região no estado, `Estado::aptos`), maioria simples com `k` vagas (o `p`-ésimo apto garantido se os `k − p + 1` adversários de menor custo não alcançam todos), maioria absoluta (`2·v_1 > V + R`), circunscrição matematicamente em 2º turno (`2·v_c + R ≤ V` para todo apto) e candidatos garantidos entre os 2 primeiros, guarda do art. 224 (líder apto, nenhum não apto alcança o candidato, `2·A + R ≤ N`). **Empate pela idade, como na lei** (custo `v_i − v_j` se o adversário vence o empate, `+1` se não; sem data, o caso desfavorável).
- Ajustes ao handoff, registrados no módulo: no **Senador com 2 vagas** cada eleitor dá até 2 votos (a candidatos diferentes) — o `p`-ésimo fica garantido se `c_max > R` ou `Σ c > 2·R`; com a **mesma data de nascimento** o empate vai para o menor número (como a ordem da situação final); seções **não instaladas** (sem BU) não entram no apurado, o que só atrasa a declaração.
- Novo módulo `acompanhamento`: reavalia, a cada lote, só as circunscrições que receberam BU, somando **só a diferença** dos contadores das regiões tocadas (custo proporcional aos BUs, não ao tamanho da circunscrição), e registra os **eventos** (sequência = nº de BUs aplicados, horário do BU que disparou; revogação seria um evento `revogado:`).
- Novo módulo `noite` (`reproduzir`): aplica os BUs em ordem de chegada numa thread, um lote por horário distinto; `antecedencias` compara com a hora de totalização dos oficiais (convertida da hora local da UF para a de Brasília).
- Binário `urna-velox` com `clap`: `inspecionar` e **`noite`** (`--dados --oficial --chegadas [--disponivel-antes] [--anuladas] [--vagas] [--dia] [--latencia-ms] --saida`; grava `eventos.csv`, `circunscricoes.csv`, `resumo.json`).
- **Teste de propriedade** (`tests/matematica.rs`), noites reais na ordem de chegada: **nenhuma revogação e nenhuma declaração diferente do oficial** — 2026 55/55 circunscrições decididas (Presidente, Governador e Senador; 89 candidatos declarados), 2024 1º turno 5.533 de 5.574 (as demais: ninguém eleito ou totalização não finalizada; 5.582 candidatos), 2024 2º turno 51/51. Mais um teste de força bruta (3.000 circunscrições aleatórias × todas as distribuições do eleitorado restante) e uma tabela de 21 casos (igualdade exata com o líder mais idoso/mais novo/sem data, `R = 0`, 2 vagas, absoluta, 2º turno, art. 224).
- **Simulação do 2º turno de 2024 refeita com o motor** (`urna-velox noite`, latência de 1,726 ms): antecedência do matematicamente eleito sobre a totalização do TSE em 49 municípios, **mediana de 1.561 s = 26 min** pelo site público (mín. 85 s, máx. 2 h 48 min) e 1.590 s recebendo os BUs junto com o TSE — os mesmos números da simulação Python da v0.0.19; apuração completa, mediana de 285 s e 349 s. Eleito matemático confere com o oficial nos 51.
- **Desempenho numa thread, com situação matemática recalculada por lote:** 2026 **3,06 µs por BU** (Presidente no Brasil, Governador e Senador nas UFs; 11.888 lotes), 2024 1º turno 2,12 µs, 2º turno 0,40 µs (critério: < 10 µs).
- Integração completa (votos e situação) e regressão do `uv-coletor` (`conferir_tres.sh`) seguem idênticas.

## [0.0.25] — 2026-10-12

Etapa 1, passo 3: **cadastro e situação final dos candidatos no motor** (antes só nos scripts Python de conferência).

- Novo módulo `cadastro`: o que a situação precisa e não está nos BUs — destinação (`Destinacao`: Válido, Válido sub judice, Anulado, Anulado sub judice, outra), nascimento (desempate), eleitorado, turno, vagas, totalização finalizada — por eleição, cargo e **circunscrição** (Brasil para Presidente, UF para Governador/Senador, município para Prefeito e consultas). Carregador desta etapa: `Cadastro::dos_oficiais` / `dos_arquivos_oficiais` (só os arquivos da própria circunscrição de cada cargo). Valores de `dvt` encontrados nos três conjuntos: "Válido", "Anulado", "Anulado sub judice".
- Ajuste ao desenho do handoff: o **sistema** (`MaioriaAbsoluta { base: Aptos | Nominal }` ou `MaioriaSimples { vagas }`) e o **art. 224** ficam na circunscrição, não no cargo, porque o do Prefeito depende do eleitorado de cada município (`Sistema::para`: Presidente/Governador maioria absoluta sobre os aptos; Prefeito > 200 mil maioria absoluta sobre a votação nominal, senão simples, art. 224 nos dois turnos; Senador simples com 1 ou 2 vagas; 2º turnos e consultas simples com 1 vaga).
- Novo módulo `situacao`: `situacao_final` com as regras que deram 100 % nos oficiais (ordem por votos, empate pelo mais idoso — sem data por último —, só aptos concorrem, art. 224, maioria absoluta/simples, `tf = n` → pendente). Os não aptos ficam "Não eleito", como na divulgação (inclusive os 38 "Anulado sub judice" de 2026, que `comparar_oficial.py` não conferia).
- `conferencia::conferir` recebe o cadastro e confere a situação de cada candidato de cada arquivo com a calculada pelo motor na circunscrição do cargo.
- Testes de unidade: tabela com 17 casos (maioria absoluta atingida, metade exata, 2º turno, base dos aptos × nominal, 1 e 2 vagas, empate pela idade, sem data, sem datas, não apto entre os mais votados, art. 224 com líder anulado, anulados > 50 % e = 50 %, sem aptos, sem votos), 200 mil exatos, `tf = n`, sistemas, destinações, datas e circunscrições. Fixtures: empate em MG 46191 (eleito o mais idoso), ninguém eleito em SP 62081, anulado com votos em RS 85766, pendente em SE 31313.
- **Integração completa com a situação calculada pelo motor (sem Python):** 2026 **83/83** abrangências e **832/832** situações (348 Presidente, 183 Governador, 301 Senador); 2024 1º turno **5.574/5.574** arquivos e **15.042/15.042** situações (15.032 de Prefeito + 10 opções de consultas); 2024 2º turno **51/51** e **102/102**. 5,4 s.
- Regressão do `uv-coletor` (`conferir_tres.sh`): idêntica à v0.0.22.

## [0.0.24] — 2026-10-12

Etapa 1, passo 2: **fixtures versionadas e testes de integração** do que o motor já faz (totais), e as conferências oficiais em Rust.

- Motor, novos módulos: `oficial` (lê os `-u.json` de divulgação: candidatos com `vap`/`dvt`/`st`/nascimento, consultas `perg`, `e.te`, `e.c`, `vb`, `vn`, `vnt`, `s.st`, turno, `tf`, vagas; abrangência pelo nome do arquivo), `resultado` (totais por município, UF e Brasil a partir do estado), `metricas` (seções anuladas, não instaladas e horários de recebimento das métricas da coleta; `segundos`/`hhmm` passaram do replay para cá) e `conferencia` (o mesmo que `comparar_oficial.py`/`comparar_municipal.py` para os votos: candidatos, brancos, nulos, nulos técnicos, comparecimento, seções totalizadas = BUs + não instaladas).
- `Catalogo::dos_pacotes` (catálogo só com os BUs, sem `cs.json`) e `catalogo::candidatos_dos_bus` (usado também pelo `uv-coletor catalogo`); `Catalogo::usar_anuladas`.
- `uv-coletor filtrar --secoes uf:município[/zona/seção],… [--soltos]`: recorta pacotes por município ou seção (gerador dos fixtures).
- **Fixtures** em `motor/tests/fixtures/` (2,3 MB, gerados por `gerar.sh`): 5 BUs avulsos (2026 `bu` e `busa`, 2024 com consulta, 2024 2º turno, seção anulada MA 07315/0023/0354) com o texto esperado do leitor (idêntico ao da v0.0.22); 8 municípios inteiros do 1º turno de 2024 (pequeno, 2 com consulta popular, seção anulada, empate no 1º lugar, ninguém eleito pelo art. 224, totalização não finalizada, anulado com votos) e BA 34134 do 2º turno, com os arquivos oficiais e as métricas.
- **Testes** (`cargo test -p urna-velox`, 14 no total): leitor × texto esperado, BU truncado em qualquer ponto sem pânico, seção anulada vira nulos; municípios de 2024 idênticos ao oficial (10 arquivos no 1º turno, 1 no 2º); sem a seção anulada só MA 07315 diverge; checksum igual com 1, 4 e 16 threads.
- **Integração completa** (`UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox --test integracao -- --ignored`), as conferências da etapa 0 agora em Rust, **5,6 s** no total: 2026 **83/83** (Presidente 29, Governador 27, Senador 27); 2024 1º turno **5.574/5.574** arquivos (Prefeito 5.569/5.569 e as 5 consultas; 9 não finalizados); 2024 2º turno **51/51**. A situação dos candidatos entra no passo 3.
- **Regressão** com os scripts Python (`conferir_tres.sh`): três conferências idênticas à v0.0.22, mesmos catálogos e totais byte a byte.

## [0.0.23] — 2026-10-12

Etapa 1 ([handoff](docs/HANDOFF_0.1.0.md)), passo 1: **workspace Cargo e crate `urna-velox`** (motor), sem mudança de comportamento.

- `Cargo.toml` na raiz com o workspace (`motor`, `aquisicao`; `bench/rust` fica de fora), `profile.release` único e **um `Cargo.lock` só**, na raiz (o de `aquisicao/` foi movido).
- Novo crate **`urna-velox`** em `motor/` (biblioteca `urna_velox` + binário `urna-velox`, que por ora só mostra a versão e inspeciona um BU): `bu.rs` e `pacote.rs` movidos sem mudança; `totaliza.rs` dividido em `catalogo.rs` (catálogo, `cargo_municipal`, `ColetaCandidatos`, layout das regiões) e `estado.rs` (estado denso, agregador, `totalizar_lote`, CSVs de totais); `chave(município, zona, seção)` passa a ser do motor (`tse::chave` a reexporta).
- `uv-aquisicao` (`uv-coletor`) depende do motor (`urna-velox = { path = "../motor" }`); coletor, `tse`, `zip`, `replay` e `main` ficam na aquisição.
- `scripts/versao.sh` (manifesto `motor/Cargo.toml`, pacotes `urna-velox` e `uv-aquisicao` no `Cargo.lock` da raiz), `.githooks/pre-commit`, `scripts/cobertura.sh` (inclui o motor) e `scripts/testar_versao.sh` atualizados; `compilar_dev.sh` compila e testa o workspace inteiro.
- Novo `aquisicao/scripts/conferir_tres.sh`: as três conferências (catálogo → totalização → comparação oficial) com um binário (`UV_BIN`), em 17 s.
- **Regressão** — binário da `v0.0.22` (compilado da tag) × binário novo, mesmos dados: **2026 83/83 abrangências** e 794 situações idênticas; **2024 1º turno 5.569/5.569** municípios (15.032 situações) e consultas 5/5; **2024 2º turno 51/51** (102 situações). Checksums do estado iguais (2026 `6122989966576576`, 2024-1t `410969429627600`, 2024-2t `143536550724`); `catalogo.txt`, `totais_uf.csv` e `totais_municipio.csv` **idênticos byte a byte** nos três conjuntos. Conferência do estado sem alarmes (0 cargos inválidos, desconhecidos, duplicados, divergentes, erros).
- Desempenho, uma thread (sem mudança esperada; ruído de cache): 2026 2,18 µs/BU (antes 2,86), 2024-1t 1,57 µs/BU (antes 1,75).

## [0.0.22] — 2026-10-12

- [docs/HANDOFF_0.1.0.md](docs/HANDOFF_0.1.0.md): decisões do Luciano registradas — crate em `motor/` (pacote e binário `urna-velox`), fixtures até 5 MB, e no "matematicamente" o empate é decidido **pela idade, como manda a lei** (custo de ultrapassagem `v_i − v_j` se o adversário é mais idoso, `+1` se não é; sem data, vale o caso do adversário mais idoso).
- Handoff reescrito com **caminhos absolutos** (repositório no Windows e no WSL, dados em `/home/dev/uvreal`); passos do plano renumerados para `0.0.23`–`0.0.27` antes da `0.1.0`.

## [0.0.21] — 2026-10-12

- Versão 0.0.20 ratificada: `VERSION`, manifestos, `Cargo.lock`, tag `v0.0.20` e `origin/main` coerentes (`scripts/versao.sh verificar`). Nenhuma mudança de código ou de resultado.
- [docs/HANDOFF_0.1.0.md](docs/HANDOFF_0.1.0.md): plano da etapa 1 (motor, `0.1.0`, alvo 14/10) — workspace com o crate `urna-velox`, cadastro (destinação, nascimento, eleitorado, vagas, turno), situação final e **matematicamente eleito / matematicamente 2º turno** com as fórmulas, formato JSON por abrangência, fixtures e testes de integração que reproduzem as conferências (83/83, 5.569/5.569, 51/51), passos com versões e critério de pronto. Link no ROADMAP.
- `CLAUDE.md`: `versao.sh bump patch` sem resumo na linha de comando (o resumo vai no CHANGELOG), como já era feito.

## [0.0.20] — 2026-10-12

- Roadmap reorganizado em etapas com datas: 0.1.0 motor (situação, matematicamente eleito/2º turno, formato JSON), 0.2.0 catálogo pré-eleição, 0.3.0 operação automatizada (`urna-velox servir`), 0.4.0 HTTP + WebSocket, 0.5.0 site de exemplo, 0.6.0 ensaio geral e runbook.
- 1.0.0 passa a ser o sistema completo para cargos majoritários (automatizado, resultado por WebSocket, site de exemplo), com meta 23/10/2026 para acompanhar o 2º turno de 2026; cargos proporcionais vão para a 2.0.0.
- Usar o sistema em eleição real não muda a versão; a regra vale também em `docs/VERSIONAMENTO.md` e `CLAUDE.md`.

## [0.0.19] — 2026-10-12

Fase 2: **2º turno de Prefeito de 2024**, coletado pela API, e simulação da noite município a município ([aquisicao/results/PREFEITO_2024_2T.md](aquisicao/results/PREFEITO_2024_2T.md)).

- **Coleta pela API** (pleito 453, 20 UFs): 97.392 BUs de 98.863 seções em 34 min 44 s, 94,2 req/s, 0 HTTP 429.
- **Conferência oficial:** **51 de 51 municípios idênticos** e 102 de 102 situações. 2 municípios retotalizados depois da eleição (SP 66192 e 66893), só contados.
- **Simulação da noite** (`scripts/simular_noite_municipal.py`): disponibilização real de cada seção no `cs.json`, releitura a cada 30 s, coleta a 95 req/s com as latências medidas na coleta, latência do Urna Velox medida no replay (p99 1,7 ms). Antecedência sobre a totalização do TSE (49 municípios): **matematicamente eleito, mediana de 26 min** (máx. 2 h 48 min, Santarém; São Paulo, 1 h 15 min); apuração completa, mediana de 4 min 44 s pelo site público e 5 min 49 s recebendo os BUs junto com o TSE. O eleito matemático confere com o oficial nos 51.
- Achados: a hora de totalização dos arquivos oficiais está na hora local da UF (os `aux.json` e `cs.json`, na de Brasília); em 25.839 seções o recebimento do `aux.json` é posterior à disponibilização (lotes recarregados), e a simulação usa a disponibilização.
- `comparar_municipal.py`: no 2º turno (`t` = 2) o mais votado é eleito, sem a regra dos 200 mil eleitores.

## [0.0.18] — 2026-10-12

Fase 2, marco dos cargos majoritários: **Prefeito e consultas populares do 1º turno de 2024** ([aquisicao/results/PREFEITO_2024.md](aquisicao/results/PREFEITO_2024.md)).

- **Conferência oficial, por município:** Prefeito idêntico nos **5.569 de 5.569 municípios** (candidatos, brancos, nulos, nulos técnicos, comparecimento, seções) e a situação dos **15.032 candidatos** idêntica (5.479 eleitos, 50 municípios com 2º turno, 31 sem eleito); as **5 consultas populares** idênticas, com a opção vencedora. Em 9 municípios a totalização oficial não está finalizada (`tf = n`, situação vazia): votos conferidos, situação não.
- **Aquisição** ([AQUISICAO §1.1.1](docs/AQUISICAO.md)): 478.026 BUs dos zips de dados abertos no acervo local (65,1 GB, 46,7 min a 23,3 MB/s, uma requisição por UF); `aux.json` de 500.341 seções em 1 h 33 min a 89,7 req/s, 0 HTTP 429, para o horário de recebimento.
- `uv-coletor baixar --so-aux`: só os `aux.json` (situação e recebimento nas métricas), sem BU.
- **Seção anulada pela Junta** (aux.json "Anulada", BU "Excluído"): `totalizar --anuladas` e `replay --anuladas` contam os votos dela como nulos, como o oficial (`vsan`). Uma no 1º turno de 2024 (MA 07315/0023/0354).
- `comparar_municipal.py`: regras observadas no oficial — ninguém eleito quando o mais votado foi anulado ou os anulados passam de 50 % da votação nominal (Código Eleitoral, art. 224); maioria absoluta sobre a votação nominal inteira; municípios com totalização não finalizada à parte.
- **Replay** publica os cargos municipais por município, só nos que receberam BU desde a publicação anterior; grava `totais_municipio.csv`.
- **Desempenho:** Prefeito, 478.026 BUs em 0,065 s (28 threads) ou 0,57 s (1 thread, 1,19 µs por BU), estado de 2,3 MB; todos os cargos (com Vereador) em 0,085 s, estado de 6,1 MB. Replay da noite 60×: latência p50 2,4 ms, p99 5,5 ms; totais do replay idênticos ao oficial.
- Scripts: `municipal_acervo.sh`, `desempenho_municipal.sh`, `chegadas_cs.py`.
- **Regressão de 2026:** Presidente, Governador e Senador seguem idênticos em 83 de 83 abrangências, situação de 794 candidatos idêntica, `totais_uf.csv` com as mesmas linhas.

## [0.0.17] — 2026-10-12

Transição para o modelo de licença Source-Available (Uso Cívico e Restrito).

- **Licença Source-Available ([`LICENSE`](LICENSE)):** Substituição da licença permissiva anterior por licença personalizada sob as Leis Federais nº 9.609/1998 e 9.610/1998. Uso gratuito garantido para pessoas físicas, auditoria cívica independente e pesquisa acadêmica; vedação expressa de uso comercial e corporativo desautorizado por veículos de imprensa, emissoras de rádio/TV, portais de notícias, consultorias e partidos políticos.
- **Guia de Licenciamento ([`docs/LICENCIAMENTO.md`](docs/LICENCIAMENTO.md)):** Dupla licença e canal oficial de contato.
- Atualização da seção de licença no [`README.md`](README.md) e apontamento `license-file = "../LICENSE"` nos manifestos.

## [0.0.16] — 2026-10-12

Eleições municipais de 2024, passos 1 e 2 do [handoff](docs/HANDOFF_2024.md): formato levantado e **cargos de circunscrição municipal** no totalizador.

- **Levantamento de 2024** ([AQUISICAO §1.1.1](docs/AQUISICAO.md)): ciclo `ele2024`, pleitos 452 (1º turno) e 453 (2º turno), eleição 619 (Prefeito e Vereador) e 5 consultas populares (633–637); `bu.asn1` de 2024 estruturalmente igual ao de 2026; resultado oficial por município (`<uf><mun>-c0011-e000619-u.json`). O leitor de BU não precisou mudar.
- **Catálogo por município** para Prefeito, Vereador e consultas (cargos 11, 13 e 25..99): o mesmo número é um candidato diferente em cada município. Linhas `K <município> <cargo> …` no `catalogo.txt`, busca binária na lista do município, consolidação por UF somando por número (`Estado::somar_na_uf`, usada também pelo replay). Catálogos antigos (sem `K`) continuam com circunscrição estadual. Teste de unidade novo (`cargo_municipal_por_municipio`).
- `uv-coletor totalizar --totais-municipio` (uf, município, cargo, tipo, código, votos).
- Scripts: `baixar_oficial_municipal.py` (arquivos oficiais por município, teto de 15–20 req/s, disjuntor no primeiro 429, retoma), `comparar_municipal.py` (votos, brancos, nulos, nulos técnicos, comparecimento, seções e situação de Prefeito — 2º turno só acima de 200 mil eleitores — e das consultas); `coleta_nacional.sh` aceita `UV_CICLO`, `UV_PLEITO`, `UV_UFS`, `UV_DADOS`, `UV_SAIDA`; `totalizar_comparar.sh` aceita `UV_BIN`.
- **Conferência — Prefeito, AC, 1º turno de 2024** (2.118 BUs do acervo local, sem rede): **22 de 22 municípios idênticos** ao oficial e a situação dos 60 candidatos idêntica. Estado de 6.670 contadores, contra 19.800 com as listas por UF.
- **Regressão de 2026:** Presidente, Governador e Senador seguem idênticos em **83 de 83 abrangências**, situação de 794 candidatos idêntica; `totais_uf.csv` com as mesmas linhas do da `v0.0.13`.

## [0.0.15] — 2026-10-12

- [`docs/HANDOFF_2024.md`](docs/HANDOFF_2024.md): handoff para a próxima etapa — totalizar Prefeito e consultas populares das eleições municipais de outubro de 2024 (marco dos cargos majoritários, Fase 2), com contexto, regras, ambiente, plano e critério de pronto.
- `ROADMAP.md`: próximo passo aponta para o handoff de 2024.

## [0.0.14] — 2026-10-12

- CHANGELOG: completa a entrada da 0.0.13, publicada só com a primeira palavra do resumo ("Senador") porque o PowerShell cortou o argumento do `versao.sh bump` no primeiro espaço. Daqui em diante, o resumo do bump é escrito direto no CHANGELOG.

## [0.0.13] — 2026-10-12

Fase 2, marco dos cargos majoritários: **Senador** ([aquisicao/results/SENADOR.md](aquisicao/results/SENADOR.md)).

- **Conferência oficial (1º turno de 2026):** Senador idêntico nas 27 UFs (candidatos, brancos, nulos, nulos técnicos, comparecimento, seções); **54 de 54 eleitos** e a situação dos 277 candidatos idênticas. Na mesma rodada, Presidente e Governador seguem idênticos: **83 de 83 abrangências**, situação de 794 candidatos idêntica.
- `comparar_oficial.py`: Senador (`--cargos 1,3,5`) e **situação de cada candidato** (eleito / 2º turno / não eleito) calculada com os votos do Urna Velox e conferida com o `st` oficial — Presidente pela circunscrição nacional, Governador e Senador pela UF.
- `uv-coletor totalizar --cargos` (catálogo restrito; os demais cargos do BU são pulados sem alarme) e `replay --cargos` (cargos publicados a cada ciclo; padrão passa a ser 1,3,5).
- **Desempenho do Senador**, 499.207 BUs em memória: 0,118 s com 28 threads (4,2 milhões de BUs/s), 1,17 s numa thread (2,35 µs por BU), estado de 2,5 MB. Replay da noite 60×, só Senador: latência p50 3,8 ms, p99 11,3 ms; com os três cargos: p50 6,7 ms, p99 14,2 ms.
- `baixar_oficial.sh` baixa Senador (`UV_CARGOS`) e pula arquivos já baixados; novos `conferir_senador.sh`, `desempenho_cargo.sh`, `replay_senador.sh`, `senador_eleitos.py`.
- `CLAUDE.md`: toda implementação é verificada contra os arquivos oficiais até haver testes automatizados.

## [0.0.12] — 2026-10-12

- **Roadmap:** a Fase 2 passa a ter o marco de **todos os cargos majoritários** (Presidente, Governador, Senador e Prefeito, 1º e 2º turnos, conferidos com o oficial e com a situação de cada candidato) — `0.2.0`; nova **Fase 3, todos os cargos proporcionais** (Deputado Federal, Estadual, Distrital e Vereador, com quocientes, sobras, eleitos e suplentes idênticos ao oficial) — `0.3.0`. Tempo real passa a ser a Fase 4 (`0.4.0`) e distribuição/auditoria a Fase 5 (`1.0.0`). Tabela de cobertura de cargos.
- **Cobertura de testes medida** (`scripts/cobertura.sh`, `cargo-llvm-cov`): `uv-coletor` 10,6 % das linhas, `bench/rust` 1,3 %; meta de ≥ 80 % no motor registrada no roadmap.
- `scripts/versao.sh`: a mensagem do bump mostrava a versão nova dos dois lados (variável sobrescrita); corrigido, com teste em `scripts/testar_versao.sh`.

## [0.0.11] — 2026-10-12

- **Versionamento:** arquivo `VERSION` como fonte única, sincronizado com `aquisicao/Cargo.toml`, `bench/rust/Cargo.toml`, `bench/js/package.json` e os locks (`scripts/versao.sh`); hooks versionados em `.githooks/` exigem bump a cada commit e criam a tag anotada (`scripts/instalar_hooks.sh`); tags `v0.0.0`–`v0.0.10` aplicadas ao histórico.
- **Roadmap revisado** com a situação de cada fase e de cada item, ligada às versões em que foi atingida.
- `CLAUDE.md` com as regras permanentes do projeto (limite de 100 req/s, versionamento, onde fica cada coisa).
- Os manifestos passam de `0.1.0`/`1.0.0` (avulsos) para a versão do projeto, `0.0.11`.

## [0.0.10] — 2026-10-12

Fase 0: **coleta nacional e totalização idênticas à divulgação oficial.**

- Coleta pela API de todas as UFs e do exterior: 499.207 BUs (450.269 em 2 h 41 min a 95 req/s, 0 HTTP 429).
- BU do Sistema de Apuração (`-busa.dat`, tipo `busa`) aceito pelo coletor e pelo leitor de zip (31 seções com votos).
- Disjuntor de HTTP 429 (`--max-429`) e retomada sem as seções sabidamente sem BU (`--pular-sem-bu`).
- Totalização de 499.207 BUs em 0,167 s (28 threads); Presidente e Governador idênticos ao oficial em 56/56 abrangências.
- `replay --corte` (2.922 BUs reenviados de 05/10 a 07/10); benchmark refeito com o Brasil inteiro (p50 4,9 ms).

## [0.0.9] — 2026-10-11

- Benchmark "noite da eleição" TSE × Urna Velox: `uv-coletor replay` (injeção no horário real de recebimento, totalização em fluxo, publicação contínua) e simulação da aquisição.

## [0.0.8] — 2026-10-11

- `progresso.py`: percentual da coleta por UF, sem rede.

## [0.0.7] — 2026-10-11

- Conferência com os arquivos oficiais de divulgação (Presidente e Governador): `baixar_oficial.sh`, `comparar_oficial.py` (com nulos técnicos), `totalizar_comparar.sh`, `totalizar_acervo.sh`, `totalizar_subconjunto.sh`.

## [0.0.6] — 2026-10-11


## [0.0.5] — 2026-10-11

Fase 0: **coletor de BUs (`uv-coletor`)** — critério de sucesso da Fase 0 atingido no DF.

- Mapeamento da API de resultados e do zip de dados abertos; coletor HTTP/2 com teto de 100 req/s, AIMD e métricas; pacotes `.uvpk`; leitor do BU de 2026 conferido com `asn1tools`; teste real reproduzível.

## [0.0.4] — 2026-10-11

- Glossário e benchmark sintético reproduzível em um comando (`bench/`).

## [0.0.3] — 2026-10-11

- Código do membench versionado (estava ignorado pelo `.gitignore`).

## [0.0.2] — 2026-10-11

- Arquitetura revisada com medições de memória e tempo em Rust, Java e JavaScript.

## [0.0.1] — 2026-10-11

- Arquitetura inicial, roadmap e ADRs.

## [0.0.0] — 2026-10-11

- Commit inicial.
