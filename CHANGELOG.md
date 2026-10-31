# Changelog

Todas as versões do Urna Velox. Regras em [docs/processo/VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md): cada commit sobe a versão; só as versões `X.Y.0` ganham tag (`vX.Y.0`; até a `0.3.0` todo commit ganhava a sua); o MINOR acompanha a última etapa concluída do [ROADMAP](ROADMAP.md) (etapas 1 a 9 = `0.1.0` a `0.9.0`, com a escalabilidade segura na etapa 6, a comparação com o TSE na 7, a previsão na 8 e o ensaio geral na 9 desde a renumeração de 16/10/2026; nas entradas anteriores à `0.6.7`, "etapa 7" / `0.7.0` = ensaio geral, hoje `0.9.0`; majoritários completos = `1.0.0`; proporcionais = `2.0.0`; até a 0.0.19 valia o esquema antigo de seis fases). As entradas `2.0.0-alpha.N`, abaixo da `0.6.0`, são o trabalho da etapa 6 feito na branch `feature/2.0-escalabilidade` antes da renumeração de 15/10/2026; nelas, "2.0" quer dizer a etapa 6 e "3.0", os proporcionais (hoje `2.0.0`).

## [1.1.6] — 2026-10-30

- **M-029: o `sonar-project.properties` passa a viver neste repositório** (Luciano, 30/10/2026 23h06: a configuração da
  análise tem de chegar ao civica pela sincronia, sem passo manual). É a configuração da 1.1.5 (chave
  `lukeboh_urna-velox-civica`; `site` avaliado; `**/testes.rs`, `uv-carga`, `site/testes/**`, `site/dist/**` fora). O
  roteiro da cópia pública deixa de tratá-lo como arquivo só do civica; a action do SonarCloud continua só lá, sem
  mudança. Não mexe em código, totalização nem resultado.

## [1.1.5] — 2026-10-30

- **M-029: o Sonar avalia a página e deixa de avaliar o código de teste.** Os 36 módulos de teste embutidos no fim dos
  arquivos de `motor/src` e `aquisicao/src` foram para `<módulo>/testes.rs` (`#[cfg(test)] mod testes;`), para o Sonar
  poder excluí-los pelo caminho (`**/testes.rs`); mesmos 204 testes antes e depois (nomes conferidos), todos verdes. Novo
  `sonar-project.properties` do civica (fora deste repositório): `site` entra em `sonar.sources`; saem os `testes.rs`, o
  `uv-carga`, `site/testes/**`, `site/dist/**` (gerado) e os testes de `motor/tests` e `aquisicao/tests`; a página fica
  fora da cobertura (não gera relatório). Regra nova no `CLAUDE.md`: teste de Rust no `testes.rs` do módulo. Detalhe em
  [docs/itens/M-029.md](docs/itens/M-029.md). Não mexe em totalização nem em resultado (`cargo test --release
  --workspace` verde, inclusive a conferência do motor com as fixtures oficiais).
- Correção das entradas 1.1.2 e 1.1.3: a suíte tem 204 testes, não 208 (erro de soma).

## [1.1.4] — 2026-10-30

- **M-028 feito (lote 3 de 3): achados do SonarCloud na página do painel** (`aquisicao/src/painel/pagina/app.js`).
  Comparador explícito na ordem dos nós (javascript:S2871; a mesma ordem de antes, por unidade de código);
  `.catch(falhou)` nas chamadas assíncronas soltas de `enviar`, `carregarConfig` e `carregarDetalhe` (S9383: a falha
  vira aviso no painel em vez de rejeição perdida); `semBarraFinal()` sem regex no lugar de `replace(/\/+$/, '')`
  (S8786, retrocesso super-linear). Prova: `node --check`, equivalência dos dois auxiliares com o código antigo e
  `site/testes/painel_celular.mjs` tudo ok. Com os três lotes, os 26 achados da análise de 30/10 estão tratados; falta a
  reanálise do Sonar na cópia pública. Não mexe em totalização nem em resultado.

## [1.1.3] — 2026-10-30

- **M-028 (lote 2 de 3): achados do SonarCloud nas ferramentas e nos testes.** `std::fs` dentro de `async` trocado por
  `tokio::fs` no `uv-carga` (arquivo da porta, amostras), no `uv-coletor secoes --csv`, na gravação dos `cs.json` em
  cache do coletor e nos testes do painel (rust:S7493, 11 achados); no teste `rest_pleitos_relido_quando_muda` da
  difusão, `tokio::time::sleep` no lugar de `std::thread::sleep` (rust:S7488). Prova: `cargo test --release --workspace`
  (204 testes, 0 falhas). Não mexe em totalização nem em resultado.

## [1.1.2] — 2026-10-30

- **M-028 (lote 1 de 3): achados do SonarCloud no `uv-servico` e no painel dos nós.** O SonarCloud da cópia pública
  (`lukeboh_urna-velox-civica`, sobre a 1.1.1) reprovou o quality gate por confiabilidade C no código novo: 26 achados
  (24 bugs, 2 code smells), nenhum no motor. Este lote tira o `std::fs` de dentro do código assíncrono que roda no ar
  (rust:S7493, 11 achados): o download dos oficiais no `uv-servico` (`servico.rs`, dentro de `tokio::spawn`) e os handlers
  do painel (`painel/mod.rs`: ler o `no.env`, gravar e apagar o pendente, apagar as rotas) passam a usar `tokio::fs`; o
  `gravar_atomico` do painel, que também bloqueava, virou `async`. Item registrado em
  [docs/itens/M-028.md](docs/itens/M-028.md); sem teste antes (decisão do Luciano: o Sonar é o vigia). Prova:
  `cargo test --release --workspace` (204 testes, 0 falhas). Os testes `--ignored` do serviço (dados reais) não rodaram
  neste ambiente. Não mexe em totalização nem em resultado.

## [1.1.1] — 2026-10-30

- **Revisão para a cópia pública:** o relatório da borda do B-067 sai de `docs/relatorios/` (o resumo da análise fica em
  [docs/itens/B-067.md](docs/itens/B-067.md) e na entrada da 1.0.14); textos do CHANGELOG, do ROADMAP e do
  `docs/handoffs/HANDOFF_1.1.0.md` sobre as parciais oficiais e a linha do tempo da noite ficam só com o que o sistema
  usa; faixas de IP saem do `docs/handoffs/HANDOFF_FS_COMPARTILHADO.md`; regra 7 do `docs/README.md` reescrita.
- Não mexe em totalização nem em resultado.

## [1.1.0] — 2026-10-30

- **Etapa `1.1` concluída: Multipleito** (decisão do Luciano em 30/10/2026: "considere que a versão 1.1 está completa").
  No ar nos dois nós: todo pleito do `ele-c.json` descoberto sozinho (agendados, em curso, encerrados), acervo
  automático, playlist e ciclo dinâmicos, o modo em curso, a consulta de 38 pleitos com o slider de cada visitante e o
  lado do TSE, o endereço por pleito e o cabeçalho com o seletor de pleito (M-025). Ficam para depois, fora do critério:
  a prova do modo em curso com um pleito real (suplementares de dezembro), as parciais do TSE vistas numa noite ao vivo
  no acervo de versões e o `60_simulado.sh` como "totalizar de novo um pleito encerrado". ROADMAP, `CLAUDE.md`,
  `VERSIONAMENTO.md` e `HANDOFF_1.1.0.md` atualizados.
- **B-089 corrigido: no celular, o letreiro ficava em "pausado" depois de tocar numa notícia.** Duas causas: o aviso
  "Os marcos da noite aparecem aqui…", invisível, continuava por cima do trilho e engolia o toque (a notícia não abria);
  e qualquer foco pausava o letreiro — no iPhone o item tocado guarda o foco, e fechar o detalhe pelo Esc ou por um toque
  fora não o tirava. Agora o aviso não recebe toques, só o foco do teclado pausa, e o detalhe, ao fechar por qualquer
  caminho, faz o letreiro retomar. Teste `site/testes/letreiro_celular.mjs` (Chromium e WebKit): antes 20 falhas
  (`data-pausado=1` para sempre), depois tudo ok.
- **M-026: a seta da escolha da eleição maior** (selo de 13–14 px; no celular, na 2ª linha, ao lado do turno e da data,
  sem cortar o nome). **M-027: o letreiro esmaece nas bordas** (`mask-image`), em vez de cortar o texto seco junto ao
  botão do menu. Testes `cabecalho.mjs` e `letreiro_celular.mjs`. Não mexe em totalização nem em resultado.

## [1.0.62] — 2026-10-30

- **M-025 feita (etapa `1.1`): o cabeçalho novo da página**, a proposta P2 escolhida pelo Luciano com os ajustes dele. O
  topo fica só com o que é geral (a marca e o tema). Logo abaixo, presa no alto ao rolar, a **barra da eleição**
  (`site/src/identificacao.js`, reescrito): o nome inteiro da eleição ("Eleições Gerais 2026", "Eleição suplementar ·
  Roraima", "Eleições Municipais 2024"), o turno e a **data do pleito**, num botão que abre a escolha da eleição; o selo do
  modo, menor; o local (seletor nativo: Brasil, estados, municípios do estado) e as pílulas de cargo, com as que não
  cabem no botão **"…"** e a da tela sempre à vista. Saem o cartão de identificação da M-001 (e o botão de recolher), a
  escolha da abrangência do topo, as pílulas soltas, o chip do pleito da faixa e o turno "1º 2º" que parecia botão; a
  faixa fica só com o que muda na noite. No celular o cabeçalho acaba em y = 188 (era 274) e o primeiro número sobe de
  y = 572 para 486; no computador, de 526 para 480. Teste novo `site/testes/cabecalho.mjs` (360 a 1920 px, dois temas, ao
  vivo/replay/consulta, Chromium e WebKit): antes 63 falhas ("sem a barra da eleição"), depois tudo ok, CLS 0. Ajustados
  `faixa_estavel`, `pleitos_pagina`, `consulta_pagina`, `topo_anuncie`, `visual_pagina`, `validacao_1_1` e
  `identificacao`; o `promo_letreiro` (que falhava desde o B-077, `1.0.50`, por supor a promo antes do ritmo) passa a
  conferir só que os dois chips não se sobrepõem. Suíte da página sem servidor: 28 testes verdes (Chromium; o
  `cabecalho` e o `consulta_celular` também no WebKit); `pagina::testes` 5/5. Não mexe em totalização nem em resultado.

## [1.0.61] — 2026-10-30

- **M-025 registrada (destino `1.1`): melhoria do cabeçalho da página.** Diagnóstico com medidas, sem mudar a página
  (`site/testes/diag_cabecalho.mjs`: 360/390/768/1366 px, dois temas, ao vivo/replay/consulta): no celular o cabeçalho
  vai até y = 274 de 844 (32 %) e o primeiro número fica em y = 572; pleito, turno, local e cargo aparecem duas a três
  vezes; o turno "1º 2º" parece botão e não é; rolada a página, a barra presa do celular não diz a eleição e as pílulas
  de cargo somem. Três propostas em protótipo (`site/testes/proto_cabecalho.mjs`, topo remontado no navegador): P1 barra
  única (cabeçalho até y = 188 no celular), P2 o topo vira a identificação (142), P3 trilha (130); capturas em
  `docs/itens/img/M-025-*.png`. Linha nova na etapa `1.1.0` do ROADMAP. Nada implantado.

## [1.0.60] — 2026-10-30

- Os três protótipos do topo da M-001 (18/10/2026: faixa fina fixa, topo fixo com a identificação, cartão à moda do
  aplicativo da JE — a escolhida) guardados em `docs/itens/img/M-001-prototipo-{a,b,c}-{390,1366}.png` e citados no
  `docs/itens/M-001.md` e no `PROMPT_CABECALHO_1.1.md` como ponto de partida da melhoria do cabeçalho.

## [1.0.59] — 2026-10-30

- `docs/handoffs/PROMPT_CABECALHO_1.1.md`: prompt para começar a melhoria do cabeçalho da página na etapa `1.1`
  (diagnóstico com medidas, duas ou três propostas de layout para o Luciano escolher, depois a implementação com testes).

## [1.0.58] — 2026-10-30

- `site/testes/auditar_nomes_pagina.mjs`: rola até o mapa antes de esperar por ele (o mapa só carrega à vista; no 3220 a
  1366 px ficava abaixo da dobra e a auditoria dava "não abriu"). Com a `1.0.57` no ar: ao vivo (3221), 3220, 3221, 3226,
  3237, 452 e 453 com todos os nomes (municípios do mapa, candidatos, nenhum código no texto); validação T1, T7–T9 ok.

## [1.0.57] — 2026-10-30

- **B-088 corrigido: nomes de municípios e candidatos do pleito anterior depois de a playlist trocar o pleito no ar.**
  A página carregava nomes, tópicos e mapa uma vez; quando a playlist trocava o pleito, mostrava municípios pelo código
  ("spNNNNN") e candidatos pelo número. Agora a página recomeça quando o pleito do `estado` muda. Teste novo
  `site/testes/troca_pleito.mjs` (falhava antes).
- **Auditoria dos nomes em todos os pleitos:** `aquisicao/scripts/auditar_nomes.py` (arquivos das 38 séries: abrangências,
  candidatos do resultado final, municípios dos marcos) e `site/testes/auditar_nomes_pagina.mjs` (a página no ar: mapa,
  candidatos, nenhum código no texto): **todos ok** — os dados estavam certos; o problema era o B-088.
- **B-087 corrigido:** o cartão de identificação dizia "Municipais" no suplementar de Governador de RR; agora o tipo vem da
  lista dos pleitos ("Suplementar", "Gerais", "Municipais") ou, sem ela, dos cargos. Teste em `consulta_pagina.mjs`
  (falhava antes).

## [1.0.56] — 2026-10-30

- **B-087 registrado** (só o sintoma, destino `1.0.x`): na consulta do suplementar de Governador de RR (3237) o cartão de
  identificação diz "Eleições 2026 · Municipais". Implantação da `1.0.55` conferida no ar: T1, T7, T8, T9 e T14 passam;
  a curva da apuração do 3220 sem saltos de relógio; os dois nós sincronizados.

## [1.0.55] — 2026-10-30

- **Bugs B-082 a B-086 corrigidos e M-024 feita** (Luciano, 30/10: "Pode atacar tudo", "Pode continuar com tudo");
  cada bug com o teste que falhava antes (detalhe em `docs/itens/`):
  - **B-082** (curva da apuração com dois relógios no eixo depois de um reinício): uma curva tem um relógio só, no
    serviço (`Hub::amostrar_curva`; teste `curva_b082_um_relogio_so`) e na página (`graficos.js`; `curva_relogio.mjs`).
  - **B-083** (marcas de hora do slider umas sobre as outras): passo de 1 a 24 h conforme a largura, "dd/mm" na virada
    do dia (`replay.js`).
  - **B-084** (selo "CONSUL…" cortado e "0 BUs/min" na consulta): o selo diz "CONSULTA", sem a seta da playlist; sem o
    chip do ritmo na consulta.
  - **B-085** (número de seções acesas saindo do cartão) e **B-086** (mapa todo aceso ao voltar ao início): o cartão sai
    e volta ao documento quando a consulta recomeça; o hodômetro e o mapa agora medem/redesenham no quadro seguinte.
  - Teste novo `site/testes/consulta_celular.mjs` (375 px, Chromium e **WebKit**, o motor do Safari; as bibliotecas do
    WebKit foram instaladas no WSL com `playwright install-deps webkit`).
  - **M-024** (escolha do pleito discreta demais): "Eleições 2026 · Gerais" no cartão de identificação vira o seletor,
    com o selo "trocar ▾"; o painel se chama "Escolha a eleição", com instrução, botão principal em cada eleição ("Ver
    esta eleição ›", "● Ver ao vivo"), item inteiro clicável, selo "na tela", "▶ Pedir na playlist" como secundária e,
    no celular, por cima do slider e do letreiro. Testes em `pleitos_pagina.mjs` e `consulta_pagina.mjs`.

## [1.0.54] — 2026-10-30

- **Bugs registrados (só o sintoma, destino `1.0.x`), vistos pelo Luciano no celular em 30/10:** B-082 (a curva da
  apuração mistura a hora da noite e a de agora no eixo, depois dos reinícios da manhã), B-083 (marcas de hora do
  slider da consulta umas sobre as outras), B-084 (selo "CONSUL…" cortado e "0 BUs/min" na consulta), B-085 (o número
  de seções acesas do mapa sai do cartão no celular).

## [1.0.53] — 2026-10-30

- **Validação da `1.1` na implantação: T14 passa na `1.0.52`.** Contra o site no ar, com a CPU 4× mais lenta a 360 px:
  452 com tarefas longas em 9 % do tempo (era 97 % e uma tarefa de 31,6 s na `1.0.50`), maior 363 ms; 3220 27 %, 3221
  4 %, 3237 2 %; T1, T7, T8 e T9 passam de novo. Relatório `docs/relatorios/VALIDACAO_1.1_IMPLANTACAO_2026-10-30.md`
  com as tabelas das três versões; `docs/itens/B-081.md` com o resultado no ar. Pendentes no relatório: T5 (observar um
  voto), T9 no Safari, T10–T12 (suplementares de dezembro), T13 com login, T14 num aparelho real.

## [1.0.52] — 2026-10-30

- **B-081, segunda causa: as ondas grandes do mapa.** Com a `1.0.51` no ar, o T14 no 452 ainda dava tarefas longas em
  83 % do tempo (a maior já em 808 ms). Rastro com a CPU 4× mais lenta (`traco_consulta.mjs`, agora com `UV_CPU` e
  `UV_DIST`, que usa a página desta cópia com os dados do site no ar): cada "primeira seção" de município pedia ao mapa
  uma onda grande (gradiente redesenhado a cada quadro por 2,2 s), sem limite. Correções: `mapa.js` no máximo 8 ondas
  grandes ao mesmo tempo; `feed.js` guarda menos itens numa tela estreita (16 no celular, 55 a 1366 px, até 70); `app.js`
  um lote de no máximo 8 itens novos a cada 250 ms no letreiro. `consulta_lote.mjs` põe os marcos a mais em seções que
  existem no mapa: sem o limite falha ("tarefas longas 84 %"), com ele 2 %. Página nova com os dados do site no ar: 452
  com tarefas longas em 12 % do tempo, maior 781 ms (era 97 % e 31,6 s na `1.0.50`).

## [1.0.51] — 2026-10-30

- **B-081 corrigido: a consulta de um geral travava a página no celular.** Achado pela validação da `1.1` (T14,
  `site/testes/consulta_desempenho.mjs` contra o site no ar: 360 px, CPU 4× mais lenta): tocando o 452 a 300×,
  tarefas longas em 97 % do tempo e uma de 31,6 s. Causa: cada marco entregue ia na hora para o letreiro, que mede a
  largura de cada item novo (leiaute forçado); tocando acelerado, centenas por tarefa, e a tarefa mais longa faz o
  passo seguinte cobrir mais noite (espiral). Correção (`app.js`): o histórico recebe todos na hora; o letreiro, em
  lote no fim da tarefa, só os últimos 40 novos (mais as atualizações e os fixos). Teste novo
  `site/testes/consulta_lote.mjs` (sem servidor; o 3237 com 24 mil marcos a mais, CPU 4×): antes "maior tarefa longa
  18579 ms", depois 464 ms (a do reposicionamento) e 7 % do tempo. Detalhe em `docs/itens/B-081.md`.
- **Validação da `1.1` (T1, T5–T14) com testes automatizados contra o site no ar** (os que dependiam de estar na tela
  na hora certa usam agora a consulta dos encerrados): `site/testes/validacao_1_1.mjs` (T1 seletor a 1366/360 px; T7
  suplementar municipal 3226; T8 B-070 no 3237 — ninguém "Eleito", selo "Sem eleito (art. 224)"; T9 o link do
  art. 224 no Chromium e no Firefox) e `consulta_desempenho.mjs` (T14). Ferramentas de diagnóstico:
  `perfil_consulta.mjs` (perfil de CPU), `traco_consulta.mjs` (rastro do Chromium), `sonda_consulta.mjs`. Relatório em
  `docs/relatorios/VALIDACAO_1.1_IMPLANTACAO_2026-10-30.md`; `HANDOFF_1.1.0.md` com §3.3–§3.6 atualizados.
- Implantação da `1.0.50` nos dois nós (30/10, de manhã): binários, scripts (com `uv-iniciar.sh` e `urna-velox.service`
  agora no pacote do `63_ciclo.sh --scripts`), Caddy (`--playlist`), `--reiniciar` para a página nova; `PLEITO`
  tirado do `no.env` dos dois nós (o `uv-iniciar.sh` sem ciclo sai com 78, conferido).

## [1.0.50] — 2026-10-30

- **Multipleito (etapa `1.1`): seletor de pleito no topo, "ao vivo" no pleito em curso, catálogo pré-eleição dos
  agendados e `no.env` sem `PLEITO`** (decisões tomadas sem o Luciano, que pediu "crie casos de teste, decida, corrija e
  implante o que falta"; revisão dele depois).
  - **Seletor de pleito** (`app.js`, `estilo.css`): o primeiro chip da faixa do topo mostra o pleito em exibição (no ar
    ou consultado) e abre a lista dos pleitos (agendados com data, cargos, onde e "faltam N dias"; em curso; encerrados
    com replay e consulta). 220 px no computador; no celular, só o ícone (40 px), para a contagem caber inteira.
    Teste novo `site/testes/faixa_estavel.mjs` (1366/390/360 px).
  - **B-077 corrigido** (o chip do ritmo pulava de lugar): a faixa punha o aviso "parado" e a promo, que aparecem e
    somem, antes do ritmo. Ordem nova: seletor, ritmo, http, parado, promo… `faixa_estavel.mjs` cenário `ritmo` falhava
    antes ("x = 36, 1035, 36") e passa depois; detalhe em `docs/itens/B-077.md`.
  - **"Ao vivo" no pleito em curso.** `uv-serie --em-curso`: série refeita durante a noite a partir do acervo de versões
    que o `uv-servico` grava, troca atômica da pasta (`.nova` → no lugar), marca `EM_CURSO` e `em_curso` no índice;
    `uv-servico` serve a consulta de um pleito em curso com `max-age=30`. O vigia (`uv-ciclo-vigia.py`, modo `aovivo`)
    refaz a série a cada `SERIE_INTERVALO` (120 s, `nice`) e o índice (`gerar_consultas.sh --indice`, que agora refaz as
    séries `EM_CURSO`). Página: a consulta de um pleito em curso relê o índice a cada minuto e estende o slider (parada no
    fim, acompanha a noite); botão "● ao vivo" na barra volta à página em tempo real; no painel, "⧗ Linha do tempo da
    noite (até agora)" no pleito em curso. Testes: `serie::` (em curso: marca, índice, sem sobras), `difusao::testes::
    consulta` (cache de 30 s), `deploy/oci/testes/test_vigia_serie.py` (aovivo refaz; replay não mexe) e o cenário
    "em curso" de `consulta_pagina.mjs` (slider estende, acompanha o fim, "ao vivo" leva a `#/rr/c0003`; encerrado sem o
    botão).
  - **Catálogo pré-eleição sozinho** (`uv-pleitos.sh`, passo 2b): para cada pleito agendado nos próximos
    `PREPARO_DIAS` (45) dias sem `candidatos.json` completo, `uv-coletor preparar` (≤ 5 req/s, para no 1º 429) com as
    fontes em `<acervo>/<pleito>/fontes-aovivo`, as mesmas do modo `aovivo`; sem `cs.json` publicado ou com fontes
    incompletas, tenta de novo em `PREPARO_H` (6) h; nunca na noite nem no dia seguinte de uma eleição. Na noite, se o
    preparo do `uv-ciclo.sh` falhar, o catálogo pré-eleição é a reserva. Testes novos em `test_uv_pleitos.py` (4).
  - **`no.env` sem `PLEITO`**: o `uv-iniciar.sh` usa o pleito do ciclo (`UV_PLEITO`); sem nenhum, sai com 78 dizendo por
    quê, e o `urna-velox.service` não reinicia nesse código (`RestartPreventExitStatus=78`); `CICLO` vazio = `ele2026`.
    No painel dos nós, `PLEITO` passa a opcional (vazio tira a linha). Testes: `test_uv_iniciar.sh` (21) e
    `painel::config_no`.
  - Suítes: `deploy/oci/testes` OK; página (`modo_pagina`, `consulta_pagina`, `pleitos_pagina`, `faixa_estavel` e as
    demais) OK; `cargo test` do workspace OK.

## [1.0.49] — 2026-10-30

- **Multipleito (etapa `1.1`): a consulta pela borda (Cloudflare).** O domínio principal não passa pela Cloudflare (sem
  `cf-ray`); o `dados.urnavelox.com.br` passa (o `/api/v1/estado` dá HIT), mas só liberava resultado e estado.
  - `Caddyfile.template`: o bloco `dados` serve também `/consulta/*`, com o `Cache-Control` do próprio `uv-servico`
    (5 min no índice, 1 h no resto), sem a troca para 30 s das rotas ao vivo. Teste novo
    `test_caddyfile.TestDados.test_dados_serve_a_consulta_com_o_cache_do_servico` (falhava antes da mudança).
  - Página: a consulta e a lista de pleitos com série buscam primeiro na base do `<meta name="uv-dados">` (a borda) e,
    se ela falhar, no próprio domínio (`basesConsulta()` em `serie.js`). `consulta_pagina.mjs` cobre os dois caminhos
    (borda no ar a 1366 px; fora do ar a 390 px); `pleitos_pagina.mjs` responde a borda localmente (sem rede).

## [1.0.48] — 2026-10-30

- **B-080 corrigido: um nó fora da rotação realinha sozinho.** O vigia de cada nó só trocava de execução quando a
  apuração em curso encerrava; o nó B, que começou o 3239 meia hora antes do A e recebe os BUs do par, nunca encerrou
  e ficou nele de 29/10 21h30 a 30/10 07h34. Agora, a cada minuto, o vigia compara a execução dele com a que a playlist
  (nó A) tem no ar; se a divergência durar `DIVERGENCIA_MIN` minutos (padrão 10), realinha com ela, com o mesmo
  carimbo. Teste novo que falhava antes (`test_b080_realinha_com_o_que_a_playlist_tem_no_ar`) e
  `test_b080_mesma_execucao_nao_mexe`; `deploy/oci/testes` 86 OK. Implantado nos dois nós (`63_ciclo.sh --scripts`).

## [1.0.47] — 2026-10-30

- **Multipleito (etapa `1.1`): consulta dos pleitos encerrados implantada nos dois nós da OCI** (30/10/2026, de manhã,
  depois de a outra sessão terminar; liberado pelo Luciano).
  - Binários da `1.0.46` (`compilar_local_arm64.sh`, `63_ciclo.sh --binarios`), scripts do ciclo com o passo 3 do
    `uv-pleitos.sh` (`63_ciclo.sh --scripts`) e as séries geradas localmente (452, 453, 3220, 3221, 3237) enviadas aos
    dois nós (ação nova `63_ciclo.sh --consultas [pasta]`).
  - O passo 3 gerou sozinho, nos dois nós, as séries dos suplementares com acervo: 33 por nó em poucos minutos, nenhum
    replay diferente do oficial; 3201 e 3207 ficam de fora (acervo nunca terminou de montar). **38 pleitos com série
    em cada nó.**
  - O `uv-servico` do ciclo passa a ter `--consulta` quando o ciclo inicia um pleito (o `--reiniciar` reaproveita os
    argumentos da unidade em curso: vale a partir da rotação seguinte).
  - **Conferido em `https://urnavelox.com.br`** (07h37): `/consulta/indice.json` 200, gzip, `Cache-Control: public,
    max-age=300`, 38 pleitos com série; a página (Chromium) abre `#/p/3220/br/c0001` e `#/p/3221/br/c0001` no resultado
    final (499.207 BUs) com o lado do TSE das parciais, e `#/p/452` num município, com o mapa de 478.026 seções; todas as
    respostas de `/consulta/*` 200 em gzip; CLS ≤ 0,0001; nenhum erro na página.
  - Achado na implantação (registrado como **B-080**, só o sintoma): o nó B estava parado na execução de 29/10 21h30
    (3239), fora da rotação, e respondia 404 na consulta; realinhado às 07h34 com `63_ciclo.sh --iniciar 3220 cadencia`.
  - `63_ciclo.sh --consultas [pasta]`: ação nova (as séries geradas fora dos nós para os dois nós).

## [1.0.46] — 2026-10-30

- **Multipleito (etapa `1.1`): ajustes da consulta e séries geradas; handoff da noite.**
  - Lado do TSE pelas parciais: só a noite (até 6 h depois do último BU do Urna Velox). As parciais trazem também as
    regenerações dos arquivos feitas dias depois (no 3220, 15/10 e 24/10: ordem dos candidatos, "Anulado" no lugar de
    "Anulado sub judice"), que não são da apuração e eram tomadas como a "última versão". Teste
    `serie::testes::tse_das_parciais_por_minuto` com o limite.
  - Página (consulta): no fim da noite, a coluna do TSE mostra a última versão do arquivo do TSE (que pode ter saído
    depois do último BU do Urna Velox: no 3221, a de 23:22:22, e não a de 23:09:21, com +263 votos de diferença);
    "Totalizado pelo Urna Velox às" mostra a hora da noite, não a do replay que gerou a série.
  - Ponta a ponta com o `uv-servico` real (`--sem-coleta --consulta`) e a página embutida: consulta do 3221 (Presidente,
    Brasil: 499.207 BUs, mapa com 499.207 seções, lado do TSE pelas parciais oficiais, CLS 0,00005) e do 453 (lado do
    TSE reconstruído); todas as respostas de `/consulta/*` em gzip.
  - Séries de consulta geradas nesta máquina (`D:\dados\urna-velox\consulta`, a árvore para os nós; nada implantado):
    - 452 (1º turno de 2024): 5574 tópicos, 275 instantes, 21.73 MB comprimidos, 5574/5574 idênticos à publicação final; lado do TSE reconstrucao: 5574 tópicos, 5574 idênticos.
    - 453 (2º turno de 2024): 51 tópicos, 173 instantes, 0.67 MB comprimidos, 51/51 idênticos à publicação final; lado do TSE reconstrucao: 51 tópicos, 51 idênticos.
    - 3220 (1º turno de 2026): 83 tópicos, 912 instantes, 6.37 MB comprimidos, 83/83 idênticos à publicação final; lado do TSE parciais: 83 tópicos, 79 idênticos, divergentes: am-c0003-e006259, am-c0005-e006259, rj-c0003-e006259, rj-c0005-e006259.
    - 3221 (2º turno de 2026): 45 tópicos, 207 instantes, 1.84 MB comprimidos, 45/45 idênticos à publicação final; lado do TSE parciais: 45 tópicos, 44 idênticos, divergentes: rr-c0001-e006258.
    - 3237 (suplementar de Governador de RR): 1 tópicos, 146 instantes, 0.03 MB comprimidos, 1/1 idênticos à publicação final; lado do TSE reconstrucao: 1 tópicos, 1 idênticos.
  - [HANDOFF_1.1.0.md](docs/handoffs/HANDOFF_1.1.0.md) §2b: o que foi feito na noite de 29 para 30/10, as decisões
    tomadas, o que falta (seletor no topo, "ao vivo" no pleito em curso, implantação, §3.6) e o roteiro de implantação.

## [1.0.45] — 2026-10-29

- **Multipleito (etapa `1.1`): consulta de qualquer pleito encerrado na página, com o slider de cada visitante**
  (§3.3 a §3.5 do handoff). Nada muda na totalização nem no modo ao vivo da página.
  - **Página** (`site/src/serie.js`): `#/p/<pleito>/<abrangência>/c<cargo>` abre a consulta do pleito. Em vez do
    WebSocket, a `ConexaoSerie` (a mesma interface da `Conexao`) lê a série estática de `/consulta/<pleito>/serie/` e
    entrega as mensagens do protocolo — resultado, `tse`, evento, marco, mapa, estado — no instante do slider: todos os
    componentes (contadores, cartões, curva, mapa das seções, letreiro, "matematicamente", comparação com o TSE) andam
    com ele. O slider é a barra do replay com o relógio local (tocar, pausar, ir a qualquer instante, até 300×); abre
    parado no fim da noite (o resultado final); voltar no tempo recomeça a página até o instante. O selo diz
    "CONSULTA", e o aviso explica a consulta e leva de volta ao que está no ar. No painel "Pleitos do TSE", os
    encerrados com série ganham "Consultar a noite (linha do tempo)" (lista em `/consulta/indice.json`).
  - **`uv-servico --consulta <pasta>`**: serve `/consulta/*` (só `.json` de nome simples; o `.json.gz` pré-comprimido
    para quem aceita gzip; cache de 5 min no índice e de 1 h no resto), para a borda guardar. A série também ganhou
    `nomes.json` e `topicos.json` (os mesmos corpos de `/api/v1/nomes` e `/api/v1/topicos`), e o lado do TSE passou a
    ser gravado como o `dados` da mensagem `tse` (PROTOCOLO_WS §4.6). Documentado no §6 do protocolo.
  - **`aquisicao/scripts/gerar_consultas.sh`**: a série de cada acervo de replay pronto (replay rápido + `uv-serie`,
    lado do TSE das parciais ou reconstruído), trocada só se conferir com a publicação final, e o índice geral.
  - **Nós (preparado, não implantado):** `uv-serie`, `replay_acervo.sh` e `gerar_consultas.sh` no pacote; o
    `uv-iniciar.sh` e o `uv-ciclo.sh` passam `--consulta /srv/urna-velox/consulta`; o `uv-pleitos.sh` ganhou o passo 3
    (gera as séries dos acervos prontos, com `nice`, nunca na noite de uma eleição nem no dia seguinte).
  - **Testes:** `site/testes/consulta_pagina.mjs` (sem servidor, com a série real do 3237 em
    `site/testes/fixtures/consulta/`: abre igual à publicação final, sem WebSocket; volta ao meio da noite com
    resultado, mapa, letreiro e lado do TSE daquele instante; toca a 300×; o painel oferece a consulta só aos pleitos com
    série; 1366 e 390 px, CLS 0,0001 e 0); `difusao::testes::consulta_estatica_com_gzip`. Os testes de página sem
    servidor seguem passando.

## [1.0.44] — 2026-10-29

- **Multipleito (etapa `1.1`): lado do TSE na série de consulta** (§3.2 do handoff). O `uv-serie` grava
  `tse/<tópico>.json` (a última versão de cada minuto do `-u.json` do TSE) de uma de duas fontes:
  - **parciais oficiais de divulgação** (`--tse-parciais <pasta>`: as versões de cada `-u.json` ao longo da noite, pela
    hora da geração; as de zona ficam de fora, o Urna Velox não publica zona);
  - **reconstrução pela hora de recebimento de cada BU no TSE** (`--tse-dados <.uvpk> --linha-do-tempo <csv>`): o TSE
    simulado do `uv-simulador` com 1 s de atraso; cada versão leva `"reconstrucao": true`, e a última é o oficial.
  A última versão de cada tópico é conferida com o `-u.json` oficial final (`--oficial`). Provas: **3221 com as
  parciais** — 45 tópicos, 44/45 idênticos ao oficial; o `rr-c0001` diverge porque o `-u.json` "oficial" do acervo de
  26/10 é de antes do fim (geração 22:45:56, `tf = n`) e a última parcial é a final (23:26:57, `tf = s`); série inteira
  com o lado do TSE 2,7 MB comprimidos, 40 s. **3237 reconstruído** — 143 versões, a última idêntica ao oficial. Teste
  novo `serie::testes::tse_das_parciais_por_minuto`.

## [1.0.43] — 2026-10-29

- **Multipleito (etapa `1.1`): série de consulta de cada pleito, a base do slider** (formato aprovado pelo Luciano em
  29/10/2026; [docs/propostas/PROPOSTA_SERIE_CONSULTA.md](docs/propostas/PROPOSTA_SERIE_CONSULTA.md)). Nada muda na
  totalização nem na página.
  - **Acervo de versões no `uv-servico`** (regra de guardar toda versão): a thread de gravação guarda, só por
    acréscimo, toda versão de todo arquivo publicado, com a hora da noite, e as seções que acenderam em cada lote, em
    `<saida>/serie/versoes-<partida>.jsonl.gz` (gzip com descarga a cada lote: legível até a última descarga numa
    queda). `--serie <dir>` muda a pasta; `--sem-serie` desliga.
  - **`uv-serie`** (binário novo): monta os estáticos de consulta do pleito — `indice.json`, `t/<tópico>.json` (a última
    versão de cada minuto da noite), `mapa-secoes.json` e `mapa.json` (minuto em que cada seção acendeu), `marcos.json`,
    `eventos.json`, `previsoes.json`, cada um também em `.json.gz` — e só grava se a última versão de cada tópico for
    idêntica, byte a byte, à publicação final (que o `uv-servico` confere com os oficiais).
  - **Provas** (replay rápido do acervo + `uv-serie`): 3237 (suplementar de RR) — replay idêntico ao oficial (1/1, 3/3
    situações), série 1/1 idêntica, 21 KB comprimidos; 3221 (2º turno de 2026) — série 45/45 idêntica à publicação,
    209 instantes, 1,4 MB comprimidos (mapa 466 KB, `br-c0001` 14 KB), acervo de versões 11,1 MB; a conferência do
    replay com o oficial dá 42/45 e situações 91/91, as mesmas três consultas populares de seções totalizadas do
    replay de 26/10 (B-066). Testes novos: `serie::testes::serie_por_minuto_com_volta_e_queda` e `serie::testes::mapa_da_serie`;
    `cargo test --release --workspace` passa inteiro.

## [1.0.42] — 2026-10-29

- **Bugs registrados na validação da 1.1 com o Luciano** (só o sintoma, regra de 18/10; destino `1.0.x`): **B-078** —
  no replay acelerado do 3237 às 21h06, o nó A parou em 1.443 de 1.483 seções (40 delegadas ao nó B não chegaram;
  aviso `federacao_sem_fonte`), sem encerrar, e a página dele ficou sem situação e sem o selo do art. 224 (o nó B
  encerrou, idêntico ao oficial); **B-079** — o cartão de identificação diz "Municipais" no suplementar de Governador de
  RR. Testes T8 e T9 da validação: falharam por causa do B-078.

## [1.0.41] — 2026-10-29

- **ROADMAP: etapa nova `1.5.0`, eleições anteriores** (pedido do Luciano em 29/10/2026), em dois grupos: **com o BU
  binário** (gerais 2022, municipais 2024 e gerais 2026 no slider, suplementares; o motor de hoje, falta baixar os dados)
  e **com o boletim de urna em texto** (`bweb`: municipais 2016 e 2020, gerais 2018, o suplementar do AM de 2017; leitor
  a desenvolver; 2014 só com o resultado final). A noite na ordem real vem da linha do tempo por seção. Só documentação.

## [1.0.40] — 2026-10-29

- **Multipleito (etapa `1.1`): a linha do tempo da noite por seção** (pedido do Luciano em 29/10/2026: só seção, hash
  do BU e horários), para os pleitos ordinários de 2016 a 2026 e o suplementar de Governador do AM de 2017. Com ela e
  os BUs, um replay de eleições antigas na ordem real da noite fica possível. Nada muda nos nós nem na página.

## [1.0.39] — 2026-10-29

- **Multipleito (etapa `1.1`): parciais oficiais de divulgação e a regra de guardar toda versão.** Só documentação;
  nada muda nos nós nem na página.
  - **Parciais de divulgação** (as versões de cada arquivo `-u.json` ao longo da noite), quando disponíveis, em
    `D:\dados\urna-velox\pleitos\<data-pleito>\tse-parciais\`. 2024 e os suplementares só têm a versão final: o lado
    do TSE deles no slider será a reconstrução pelo `aux.json`.
  - **Regra do Luciano (29/10/2026): o sistema guarda toda versão de todo arquivo que vê ou publica** (as parciais do TSE
    numa noite ao vivo e as publicações do Urna Velox), num acervo só de acréscimo; a série das consultas pode ser
    processada depois. Novo item na etapa `1.1` do ROADMAP.
  - `docs/handoffs/HANDOFF_1.1.0.md` §3.2 e ROADMAP (item "Lado do TSE no slider"): as parciais quando disponíveis.

## [1.0.38] — 2026-10-29

- **Multipleito (etapa `1.1`): a visão alinhada com o Luciano em 29/10/2026 e o handoff do que falta para a `1.1.0`.**
  Só documentação; nada muda no sistema.
  - **Visão:** um pleito por vez vale só para o motor (noite ao vivo e replay da playlist); para o visitante, todo
    pleito encerrado é consultável a qualquer momento, com toda a riqueza de uma eleição totalizada: fotografias
    consolidadas ao longo da noite (por minuto; por lote nos pleitos pequenos), servidas só por cache/CDN, e um slider
    de cada visitante comandando tudo na tela, inclusive a coluna do TSE (parciais oficiais quando disponíveis; senão,
    reconstruídas pela hora de recebimento de cada BU); no pleito em curso, até o agora, com o botão "ao vivo"; endereço
    por pleito e um seletor de pleito que absorve o painel "Pleitos do TSE".
  - **ROADMAP:** o texto da etapa `1.1` com a visão; três itens novos (consulta de qualquer pleito com o slider, lado do
    TSE no slider, seletor de pleito e endereço por pleito); o critério da etapa passa a exigir que todo encerrado seja
    consultável no slider, com o lado do TSE, idêntico ao oficial no fim.
  - **`docs/handoffs/HANDOFF_1.1.0.md`** (no índice): a visão, onde estamos, o que falta em ordem sugerida, a validação
    com o Luciano em andamento (T1 a T4 passaram; T5 a T9, T13 e T14 pendentes; T10 a T12 dependem dos suplementares de
    dezembro) e as pendências de decisão (a rota `/painel/qualidade` que entrou na `1.0.37`, o B-077, a proposta da
    série de fotografias).

## [1.0.37] — 2026-10-29

- **Multipleito (etapa `1.1`) implantado nos dois nós da OCI** (Fase 3 do prompt de 29/10/2026), com os binários da
  `1.0.36` (`compilar_local_arm64.sh`, `63_ciclo.sh --binarios`, `--scripts`, `--playlist`).
  - **No ar:** `uv-pleitos.timer` ativo nos dois nós; `pleitos.json` com os 43 pleitos (todos encerrados em 29/10;
    nenhum agendado no `ele-c.json`); `pleitos.log` sem erro e sem HTTP 429; os acervos dos suplementares sendo
    montados, 2 por rodada (3239, 3240, 3237 e 3236 nas primeiras); `GET https://urnavelox.com.br/api/v1/pleitos` com
    200; a página na `1.0.36`; a `/playlist` com os suplementares votáveis nos dois modos.
  - **Troca do que estava no ar** (pedido do Luciano às 15h42, para ver tudo já): o 2º turno de 2026 em tempo real
    (3221 cadência, desde as 12h) foi interrompido com `63_ciclo.sh --iniciar 3239 rapido` — o histórico o registra
    como "apuração não encerrou", de propósito. O 3239 (suplementar de Reginópolis/SP) totalizou no `uv-servico`
    1.0.36 com `--pleitos`, conferência **1/1 arquivo e 2/2 situações idênticos ao oficial**, e a rotação marcou o 3240
    para a meia hora redonda seguinte (16h00), os dois nós sincronizados.
  - **Configuração de produção conferida** (para o modo `aovivo`): `no.env` com a BASE do TSE e RPS 95 nos dois nós;
    `federacao.json` com dois órgãos e dois IPs de saída próprios (o B coleta no IP dele), teto 95.
  - ROADMAP: descoberta, acervo automático, playlist e ciclo dinâmicos e página ✅; "Em curso" 🟡 (implantado, falta a
    prova com um pleito em curso). Faltam na etapa: "Agendados", "Operação" (`no.env` sem `PLEITO` obrigatório;
    `60_simulado.sh`) e o critério com um pleito agendado e um em curso reproduzidos pelo simulador.

## [1.0.36] — 2026-10-29

- **Correção dos bugs vistos nos testes do merge da `1.0.35`** (ordem do Luciano em 29/10/2026: "corrija todos os bugs";
  em cada um, primeiro o teste que comprova, falhando, depois a correção; detalhe em `docs/itens/<id>.md`). Nenhuma
  mudança na totalização nem no resultado.
  - **B-071** — `pagina::testes::tabela_fixa` recusava o link do art. 224 para o Código Eleitoral no site do TSE
    ("/static/app.js cita recurso externo"). É navegação numa aba nova (regra de 29/10), não recurso carregado: o teste
    aceita `https://www.tse.jus.br/legislacao/`, só no `app.js`; qualquer outro endereço externo continua recusado.
    Antes: falha; depois: ok.
  - **B-072** — a conferência do CLAUDE.md (`UV_DADOS=… cargo test --release -p urna-velox -- --ignored`) caía em
    `situacao_igual_a_oficial_em_uma_pasta` sem `UV_OFICIAIS` (e o cargo parava ali). Sem `UV_OFICIAIS`, o teste usa as
    pastas de oficiais do `UV_DADOS` (2026 1º turno, Prefeito 2024 nos dois turnos); `UV_OFICIAIS` aceita várias pastas
    (`:`). Antes: "defina UV_OFICIAIS"; depois: **5.666 arquivos de circunscrição, 0 situações divergentes**.
  - **B-073** — `servico::nao_instaladas_exterior` falhava ("21 seções com 404 que nunca responderam 200", reproduzido):
    o teste lia o `simulador.log` com o simulador no ar, e o registro só é descarregado a cada segundo — faltavam as
    últimas requisições (a seção `zz 29980/1/863`, por exemplo, estava aplicada no serviço sem o 200 no registro). O
    serviço estava certo. `Simulador::log_completo` encerra o simulador com SIGTERM (que descarrega o registro) antes de
    ler, nos quatro testes de `servico.rs` que leem o registro. Depois: 5 de 5 rodadas ok e ok na suíte.
  - **B-074 e B-075** — `identificacao` (CLS 0,0183 a 0,0304) e `horarios_pagina` (0,0195) no computador: a linha dos
    termos de uso (no fluxo, depois do painel, desde a 0.9.19) aparecia no pé da tela durante o "Carregando…" (520 px)
    e era empurrada pelo primeiro resultado. Já acontecia na `main` antes do merge (1.0.26). O `.carregando` passa a ter
    `height: max(520px, 100vh)`: os termos começam fora da tela. Depois: `identificacao` ok (CLS 0,0001/0,0001/0 a
    1366 px, 0 a 390 px) e `horarios_pagina` ok. `site/dist/` regerado.
  - **B-076** (descartado, `fora`) — o replay do 3237 com 2/3 situações rodou com um `uv-servico` 1.0.25 (o padrão do
    `replay_acervo.sh`, `target/release` da pasta do repositório); as 132 seções sem BU são as agregadas (`nsp`). Com o
    binário da `1.0.35`: 3237 1/1 arquivo e 3/3 situações, 3080 1/1 e 2/2. O `replay_acervo.sh` agora diz na primeira
    linha qual `uv-servico` fez o replay.
  - **B-077 registrado** (só o sintoma, destino `1.0.x`): o chip do ritmo da faixa troca de lugar no computador durante
    a apuração (CLS pequeno, abaixo do limite dos testes), visto no diagnóstico do B-074.
- **Suíte inteira depois das correções** (WSL, dados reais): `cargo test --release --workspace` sem falha; a conferência `--ignored` do motor (2026 83/83, 2024 5.569 municípios e 51/51, situações e "matematicamente" idênticos, e a situação de 5.666 arquivos de circunscrição, 0 divergências); `situacao_oficial` com `UV_OFICIAIS`: 2026 69 arquivos, Prefeito 2024 5.560 e 51, 0 divergências; `federacao` 6/6 e `servico` 18/18; `deploy/oci/testes` 84 Python (3 pulados) e 18/18 do `uv-iniciar`; página sem servidor: os 17 testes tudo ok (`visual_pagina` 1.260 conferências); com servidor e dados reais: `cores`, `identificacao`, `horarios_pagina` ok e `comparacao` 4/4 abas idênticas ao oficial (CLS máximo 0,0096, antes 0,037); replays dos acervos 3080 (1/1, 2/2) e 3237 (1/1, 3/3).

## [1.0.35] — 2026-10-29

- **Merge do branch `claude/multipleito-1.1` na `main`: multipleito (etapa `1.1`: descoberta dos pleitos, acervo de
  replay automático, playlist/ciclo/vigia dinâmicos, painel "Pleitos do TSE", rotação a cada meia hora, pleito em curso
  ao vivo, artigos de lei como link) e a correção do B-070** (art. 224 em todo cargo majoritário). O branch saiu da
  `1.0.25` e numerou o trabalho como `1.0.26` a `1.0.34`, números que colidem com a `1.0.26` da `main` (M-023): **a
  numeração do branch foi refeita** e as nove entradas viraram esta, como no merge da sonda (`1.0.24`); todo "desde a /
  na `1.0.26`…`1.0.34`" do trabalho do branch na documentação e nos comentários passou a `1.0.35` (o `1.0.26` do M-023
  ficou como está). O conteúdo das entradas do branch segue abaixo, sem mudança, cada uma com o número que tinha.
- **Ajustes do merge.** `site/src/app.js`: as duas mudanças juntas (silêncio do TSE e checkup da sonda do M-023; pleitos
  e normas do branch); `site/dist/` regenerado (`npm ci && npm run build`); listas do ROADMAP
  regeneradas (`scripts/itens.py gerar`); VERSION e manifestos em
  `1.0.35`.
- **Testes do merge** (WSL, `~/uvreal`, 29/10/2026): `deploy/oci/testes` 84 testes Python (3 pulados) e 18/18 do
  `test_uv_iniciar.sh`; ensaios `federacao` 6/6 e `servico` 17/18; página sem servidor: os 17 testes (`pleitos_pagina`,
  `normas_links`, `modo_pagina`, `visual_pagina` 1.260 conferências ok, `topo_anuncie`, `sem_bus`, `telemetria`,
  `seletor_fonte`, `aviso_versao`, `antes_17h`, `termos`, `relogio`, `previsao_cartao`, `promo_letreiro`,
  `playlist_identidade`, `preposicoes`, `hodometro`) tudo ok; com servidor e dados reais: `cores` (55 disputas de 2026
  e 14 do 3221, nenhuma cor repetida) e `comparacao` (4 abas, 4/4 idênticas ao oficial) ok. **Falharam e foram
  registrados (B-071 a B-076), corrigidos na `1.0.36`:** `cargo test --workspace` em `pagina::testes::tabela_fixa`
  (B-071); a conferência `--ignored` sem `UV_OFICIAIS` (B-072); `servico::nao_instaladas_exterior` (B-073);
  `identificacao` e `horarios_pagina` pelo CLS (B-074, B-075, já na `main` antes do merge); o replay do 3237 (B-076,
  descartado: o replay rodou com um binário antigo). `playlist_pagina` roda contra os nós depois da implantação.
- **Verificação contra os oficiais do TSE (o B-070 mexe em resultado):** `UV_DADOS=/home/dev/uvreal cargo test --release
  -p urna-velox -- --ignored --test-threads 1`: **2026 83/83, 2024 5.569 municípios e 51/51**, situações e
  "matematicamente" idênticos (`integracao` 8/8, `matematica` 6/6); `situacao_oficial --ignored` com `UV_OFICIAIS`:
  **2026 (Presidente, Governador e Senador, dois turnos) 69 arquivos, Prefeito 2024 5.560 + 51, 0 divergências**;
  acervo automático montado de verdade (≤ 95 req/s, 0 HTTP 429): **3080 1/1 arquivo e 2/2 situações; 3237 1/1 e 3/3**
  (com o binário da `1.0.35`; ver B-076).
- **Entrada `1.0.34` do branch:**
  - **Artigos de lei na página viram link para a norma no site do TSE, em aba nova, com âncora no artigo** (regra do
    Luciano, 29/10/2026, agora no CLAUDE.md, "Convenções").
    - **`site/src/normas.js`:** tabela das normas (`NORMAS`: Código Eleitoral, página do TSE) e dos artigos citados
      (`ARTIGOS`: hoje o art. 224, com o início literal "Art. 224. Se a nulidade atingir a mais de metade"); `comLinks`
      troca cada artigo conhecido de um texto pelo link (`target="_blank"`, `rel="noopener noreferrer"`, `title` com a
      norma e o resumo). **Âncora:** fragmento de texto `#:~:text=` (Chrome/Edge 89+, Safari 16.1+, Firefox 131+: rola até o
      artigo e o destaca; sem suporte, abre a norma no topo), porque o TSE não publica âncoras por artigo e o portal recusa
      leitura automática (não deu para conferir `id`s na página).
    - **Página:** o selo "Sem eleito (art. 224)" (B-070) tem "art. 224" como link. O selo guarda o texto em
      `data-texto` e desenha um `span` só (o grid continua centralizando).
    - **Teste:** `site/testes/normas_links.mjs` (tabela sem navegador; na página, a 1366 e 360 px: texto, `href`, aba
      nova, a apuração fica na aba dela): tudo ok. Sem regressão: `seletor_fonte`, `visual_pagina` (1.260 ok),
      `modo_pagina`, `topo_anuncie`, `telemetria`, `pleitos_pagina`. `dist/` regerado.
    - Documentos: o art. 224 com o mesmo link em `docs/arquitetura/FORMATO_RESULTADO.md` e `docs/itens/B-070.md`;
      `site/README.md`.
- **Entrada `1.0.33` do branch:**
  - **Multipleito (etapa `1.1`), passo 5: pleito em curso totalizado ao vivo, sozinho** (autorizado pelo Luciano em
    29/10/2026: "autorizo qualquer pleito em curso totalizar sozinho"; registrado no CLAUDE.md, seção "Rede e servidores do
    TSE").
    - **Vigia:** a cada volta pergunta ao catálogo se há pleito em curso (do dia da eleição às 8 h do dia seguinte,
      calculado na hora; ordinária primeiro) e dispara na hora `uv-ciclo.sh avancar <pleito> aovivo`; nova tentativa a
      cada 10 min se a partida não pegar.
    - **`uv-ciclo.sh iniciar <pleito> aovivo`:** `candidatos.json` contra o TSE (`uv-coletor preparar`, ≤ 5 req/s; fontes
      incompletas aceitas) e o `uv-servico` pelo `uv-iniciar.sh` da produção (`no.env`/`federacao.json`: ≤ 95 req/s por IP
      de saída, 429, B só recebendo), com o pleito, o ciclo, as UFs do pleito e as pastas da execução; `--continuar
      --aceitar-incompleto`. Partida que falha devolve o vigia (para tentar de novo).
    - **`uv-iniciar.sh`:** `UV_PLEITO`, `UV_CICLO`, `UV_CANDIDATOS`, `UV_DADOS`, `UV_SAIDA`, `UV_ARMAZEM`, `UV_UFS` e
      `UV_EXTRA` por cima do `no.env` (sem elas, a linha de comando de sempre).
    - **`uv_pleitos.py`:** `em_curso()`, `ao_vivo_agora()`, `ufs()` e os comandos `aovivo` e `ufs`; **playlist:** modo
      "Ao vivo" no "no ar".
    - **Testes:** `test_pleitos_dinamicos.py::TestAoVivo` (7) e `test_uv_iniciar.sh` casos 12 a 18; `deploy/oci/testes`:
      84 testes Python + 18 do `uv-iniciar`, sem falha. Não mexe em totalização nem em resultado.
    - `docs/operacao/CICLO_DIARIO.md` (seção "pleito em curso totalizado ao vivo"), ROADMAP (item "Em curso" em 🟡),
      CLAUDE.md.
- **Entrada `1.0.32` do branch:**
  - **Rotação de replays a cada meia hora** (pedido do Luciano em 29/10/2026: "na próxima meia hora redonda pode
    trocar"): o vigia (`uv-ciclo-vigia.py`) dispara a troca na próxima meia hora redonda (hh:00 ou hh:30) depois que a
    apuração do replay encerra, em vez da hora fechada; a margem continua de 3 min (encerrou a menos de 3 min da virada →
    a seguinte). A agenda da playlist (`uv-playlist.py`, início previsto de cada replay) usa a mesma regra. `PASSO_MIN` no
    `ciclo.env` / `UV_PASSO_MIN` no ambiente da playlist trocam o passo (60 = como antes). Rotação: sem peso maior para as
    ordinárias (decisão do Luciano), do pleito mais antigo para o mais novo.
    - **Testes:** `test_playlist.py::test_agenda_em_meias_horas_redondas` (era `test_agenda_em_horas_fechadas`, que
      fixava a regra antiga: 15:00 → agora 14:30) e `test_proxima_virada`; `test_pleitos_dinamicos.py::
      test_virada_a_cada_meia_hora` (vigia, inclusive `UV_PASSO_MIN=60`). `deploy/oci/testes`: 77 testes, sem falha.
    - CHANGELOG: a entrada da `1.0.31` explica a versão que saiu junto com a `1.0.30`.
    - `docs/operacao/CICLO_DIARIO.md` (nota no início da rotação contínua) e comentários do `uv-ciclo.sh`.
- **Entrada `1.0.31` do branch:**
  - Sem mudança própria: a `1.0.31` saiu no mesmo commit da `1.0.30` (`4f0a489`, B-070) porque duas execuções do
    `scripts/versao.sh bump` rodaram ao mesmo tempo (um comando de conferência travou e o commit seguiu em segundo plano
    enquanto o bump era refeito). O conteúdo é o da `1.0.30`, abaixo; o histórico não foi reescrito (já estava no remoto).
- **Entrada `1.0.30` do branch:**
  - **B-070 corrigido: art. 224 do Código Eleitoral em todo cargo majoritário** (ordem do Luciano em 29/10/2026: "não
    podemos subir com bug grave desses"). No suplementar de Governador de RR (pleito 3237, 21/06/2026) o mais votado estava
    "Anulado sub judice" e os anulados passavam de 50 % da votação nominal: o TSE não proclamou eleito, e o motor dava o
    2º colocado como eleito. **Legislação** (detalhe e fontes em `docs/itens/B-070.md`): art. 224, *caput* (nulidade de
    mais da metade dos votos → nova eleição) e § 3º (indeferimento do registro ou cassação do eleito em pleito majoritário
    → novas eleições, independentemente do número de votos anulados; ADI 5.525 e Tema 986 do STF), para todo pleito
    majoritário; o TSE publica `esae = "s"` e os motivos em `mnae`.
    - **Causa:** `Sistema::para` ligava o art. 224 só para Prefeito (a regra tinha saído dos oficiais de 2024).
    - **Teste que comprova, antes:** `motor/tests/situacao_oficial.rs` (situação com os votos do `-u.json` × `st` do TSE,
      fixture `motor/tests/fixtures/oficial-majoritarios/`, 52 KB: o 3237 e quatro controles de 2026) falhava com
      `[("rr-c0003-e006278-u.json", 10, "Eleito", "Não eleito")]`. **Depois:** verde.
    - **Correção:** art. 224 para Presidente, Governador, Senador e Prefeito, nos dois turnos (consultas e o Conselheiro
      Distrital, não). `situacao_final`, o "matematicamente" e a página (`sem_eleito`, "Sem eleito (art. 224)") já
      tratavam o caso quando a circunscrição tinha a regra.
    - **Verificação contra os oficiais do TSE:** situação de todos os 62 arquivos de circunscrição de Presidente,
      Governador e Senador dos dois turnos de 2026 (teste `--ignored` novo, `UV_OFICIAIS=<pasta>`): **62/62, 0
      divergências**, antes e depois (nenhum resultado de 2026 muda); replays de ponta a ponta: **3237 1/1 arquivo e 3/3
      situações idênticos** (antes 2/3; "matematicamente" 0 diferente, antes 1), 3080 1/1 e 2/2, 3040 2/2 e 5/5.
      `cargo test -p urna-velox` sem falha. Pendente no WSL: as conferências `--ignored` com `UV_DADOS` (2024 e 2026
      completos).
    - Documentação: `docs/itens/B-070.md`, `docs/BUGS.md` (B-070 corrigido), `docs/arquitetura/FORMATO_RESULTADO.md`
      (`sem_eleito`), `ARCHITECTURE.md`, comentários de `situacao.rs`, `matematica.rs` e `candidatos.rs`.
- **Entrada `1.0.29` do branch:**
  - **Multipleito (etapa `1.1`), passo 4: a página mostra todos os pleitos do TSE** (decisão do Luciano em 29/10: "todo
    frontend deve ser capaz de exibir todos os pleitos disponíveis no JSON, com os detalhes e situações de cada um").
    - **`site/src/pleitos.js`** + `app.js`: a página lê `GET /api/v1/pleitos` na partida e a cada ~10 min; com a lista, o
      aviso do selo ganha a linha **"Pleitos"** ("Ver todos os pleitos · 1 em curso · 1 agendado · 43 encerrados"), que abre
      o painel **"Pleitos do TSE"**: grupos em curso, agendados (o mais próximo primeiro, com "faltam N dias") e
      encerrados (o mais recente primeiro); em cada um, título, dia da semana e data, lugar (Brasil com as suplementares e
      consultas do mesmo dia, a cidade com a UF), cargos (consultas populares contadas, sem o texto da pergunta) e o selo
      "no ar" no pleito que o serviço totaliza; nos encerrados, "▶ na playlist de replays" (com acervo) ou "replay em
      preparo". Filtro por texto, Esc fecha; painel fixo de altura fixa (nada se desloca), dentro da tela em 360 px. Sem
      `/api/v1/pleitos` (serviço sem `--pleitos`), nada muda. `dist/` regerado (`npm run build`).
    - **Teste:** `site/testes/pleitos_pagina.mjs` (sem servidor; 1366, 390 e 360 px): tudo ok. Antes de passar, ele pegou
      dois erros desta versão (o texto da pergunta das consultas populares no lugar do cargo; o Conselheiro Distrital de
      Fernando de Noronha, cargo 25 de uma eleição ordinária, tomado por consulta), corrigidos antes do commit. Testes de
      página sem servidor que já existiam: `modo_pagina`, `topo_anuncie`, `sem_bus`, `aviso_versao`, `antes_17h`,
      `termos`, `telemetria`, `relogio`, `previsao_cartao`, `seletor_fonte`, `promo_letreiro`, `playlist_identidade`,
      `preposicoes`, `hodometro` tudo ok; os que precisam de um `uv-servico` com dados reais (`identificacao`,
      `comparacao`, `horarios_pagina`, `playlist_pagina`, `cores`) não rodaram neste ambiente.
    - `docs/arquitetura/PROTOCOLO_WS.md` §10 e `site/README.md`.
- **Entrada `1.0.28` do branch:**
  - **Multipleito (etapa `1.1`), passo 3: playlist, ciclo e vigia montados a partir do `ele-c.json`** (decisão do Luciano
    em 29/10: os encerrados entram todos na playlist; a playlist é dinâmica, para os pleitos novos — já há suplementares
    previstos para dezembro de 2026 — entrarem sozinhos).
    - **`deploy/oci/servidor/uv_pleitos.py`**: o catálogo (o `pleitos.json`, relido quando muda, e o `acervo.json` de cada
      acervo) para a playlist, o vigia e o `uv-ciclo.sh`. Rotação = pleitos encerrados com acervo pronto, do mais antigo
      para o mais novo; sem o `pleitos.json`, a lista fixa de antes (452, 453, 3220, 3221).
    - **`uv-playlist.py`**: votáveis, descrições, rotação e duração estimada pelo catálogo; suspensão também pelas datas
      dos pleitos agendados e em curso (ordinária como antes; suplementar e consulta só do dia às 8 h do dia seguinte); a
      "próxima eleição ao vivo" inclui os suplementares agendados.
    - **`uv-ciclo-vigia.py`** e **`uv-ciclo.sh`**: rotação local, pleitos aceitos da playlist e dados de cada pleito (ciclo,
      nome, data, corte, cadência) pelo catálogo; o `uv-servico` do ciclo e da produção (`uv-iniciar.sh`) recebe
      `--pleitos` quando o arquivo existe e o binário conhece a opção.
    - **`uv-pleitos.sh`** + `uv-pleitos.service`/`.timer` (15 min, nos dois nós): `uv-coletor pleitos` e os acervos que
      faltam (`montar_acervo.sh`, no máximo 2 por rodada, do mais novo para o mais antigo), nunca na noite de uma eleição
      nem no dia seguinte (CLAUDE.md); falha espera 6 h; 429 encerra a rodada.
    - **`uv-coletor pleitos --acervo`** põe no `pleitos.json` o `acervo` (seções e duração da cadência do `acervo.json`).
    - **`63_ciclo.sh --scripts`/`--instalar`/`--playlist`** levam `uv_pleitos.py`, `uv-pleitos.sh`, `montar_acervo.sh` e as
      unidades, e ligam o timer. **Ainda não implantado nos nós.**
    - **Testes:** `deploy/oci/testes/test_pleitos_dinamicos.py` (14: catálogo, linha de comando, playlist com os
      suplementares votáveis e na rotação, suspensão curta e longa, vigia), `test_uv_pleitos.py` (5), `test_uv_iniciar.sh`
      (casos 9 a 11), `pleitos::testes::acervo_marca_o_replay_e_json` (o campo `acervo`). Toda a suíte de
      `deploy/oci/testes` passa (75 + 11), inclusive os testes de antes sem o `pleitos.json` (lista fixa);
      `cargo test -p uv-aquisicao --lib` sem falha. Não mexe em totalização nem em resultado.
    - `docs/operacao/CICLO_DIARIO.md`: seção "Multipleito: playlist, ciclo e vigia dinâmicos".
- **Entrada `1.0.27` do branch:**
  - **Multipleito (etapa `1.1`), passo 2: acervo de replay montado sozinho** para qualquer pleito encerrado do
    `ele-c.json` (decisão do Luciano em 29/10: todos os encerrados entram na playlist).
    - **`aquisicao/scripts/montar_acervo.sh <pleito> <destino> [pleitos.json]`**: `cs.json`, BUs (`uv-coletor baixar`,
      ≤ 95 req/s, retomável, sem pedir de novo as seções sem BU), linha do tempo (métricas com o `recebido_tse`, ordenadas),
      `candidatos.json` e `-u.json` finais (`uv-coletor preparar`, com a destinação final), `acervo.json` (nome, data, corte,
      duração da cadência) e `.completo`; só pleito encerrado; acima de 20.000 seções, só com `FORCAR=1`.
    - **`aquisicao/scripts/replay_acervo.sh <acervo>`**: repete a noite no `uv-simulador` com o `uv-servico` (500×, sem
      rede externa) e sai com 0 só com a conferência final idêntica ao oficial.
    - **Verificação contra os oficiais do TSE** (os replays totalizam: regra do CLAUDE.md), com os acervos montados de
      verdade em 29/10/2026 (0 HTTP 429): **3080** (suplementar de Paranhos/MS, 32 seções) 1/1 arquivo idêntico, 2/2
      situações; **3040** (suplementares de Neves Paulista e Eldorado/SP, duas eleições, 72 seções) 2/2 idênticos, 5/5
      situações; **3237** (suplementar de Governador de RR, 1.615 seções) **votos idênticos, 1 situação divergente de 3**:
      registrada como **B-070** (só o sintoma, regra de 18/10), destino `1.0.x`.
    - `docs/operacao/CICLO_DIARIO.md`: seção "Multipleito: acervo automático".
- **Entrada `1.0.26` do branch:**
  - **Multipleito (etapa `1.1`), passo 1: descoberta dos pleitos no `ele-c.json`** (pedido do Luciano em 16/10; escopo
    de 29/10/2026, no ROADMAP: motor com um pleito por vez, página com todos os pleitos, encerrados todos na playlist,
    playlist dinâmica). Trabalho na branch `claude/multipleito-1.1`.
    - **`uv_aquisicao::pleitos`** + **`uv-coletor pleitos`**: lê o `ele-c.json` (uma requisição, ou `--ele-c` com um
      arquivo local) e grava o `pleitos.json` (`--saida`, gravação atômica): cada pleito com data, ano, turno, nome curto
      ("1º turno de 2024", "Suplementar de Paranhos (MS)") e completo ("Eleições Municipais 2024 · 1º turno"), eleições
      (tipo ordinária/suplementar/consulta, lugar, cargos majoritários — proporcionais de fora —, UFs e municípios) e a
      **situação** na hora de Brasília: agendado (antes do dia), em curso (do dia às 8 h do dia seguinte;
      `--fim-apuracao-h`) ou encerrado; `--acervo` marca `replay` nos pleitos com `<acervo>/<pleito>/.completo`.
    - **`uv-servico --pleitos <arquivo>`** → **`GET /api/v1/pleitos`** (`docs/arquitetura/PROTOCOLO_WS.md` §6): o
      arquivo, com `ETag`/304, relido quando muda (o `uv-coletor pleitos` pode regravá-lo com o serviço no ar); 404 sem
      ele.
    - **Testes:** `pleitos::testes` (5) sobre o `ele-c.json` real de 29/10/2026, guardado em
      `aquisicao/tests/fixtures/ele-c-2026-10-29.json`: 43 pleitos (2024, 2026 e 39 suplementares de 2024 a 2026), nomes,
      cargos, classificação em 01/10 (3220 e 3221 agendados), na noite de 25/10 (3221 em curso) e em 29/10 (todos
      encerrados), acervo; `difusao::testes::rest_pleitos_relido_quando_muda`. `cargo test -p uv-aquisicao --lib`: 143
      passaram, 0 falhas. Não mexe em totalização nem em resultado (sem conferência contra os oficiais).
    - ROADMAP: texto da etapa `1.1` com as decisões de 29/10 e a descoberta em 🟡.

## [1.0.26] — 2026-10-29

- **Página pública: feedback contextual no silêncio do TSE e checkup em vez de milissegundos na sonda (M-023)** (ordem do Luciano).
  - **Aviso de silêncio do TSE com tempo decorrido (M-019/M-023):** além do horário de corte ("desde 17h11"), informa dinamicamente há quantos minutos o TSE não envia BUs (ex.: `TSE sem novos BUs há 18 min (desde 17h11) · retomamos automaticamente quando voltarem`; no celular `TSE sem novos BUs há 18 min`).
  - **Checkup executado na ociosidade (M-023):** no chip da faixa do topo, a mensagem técnica em milissegundos (`< 1 ms`) foi substituída por `✓ checkup ok · ativo` (no celular `✓ ativo`).
  - **Exibição contextual:** o chip só alterna para o checkup quando o sistema estiver ocioso / aguardando BUs (`ritmo == 0`). Enquanto a apuração flui com BUs/min ativos (`ritmo > 0`), a própria operação em andamento já indica o funcionamento normal e o chip foca no ritmo.
  - **Alerta de atraso ou falha:** caso a sonda não seja confirmada dentro do limite (100 s sem resposta), o chip exibe alerta explícito (`⚠ sem confirmação há X min`), com destaque visual em vermelho/âmbar em vez de status normal.
  - Testes `site/testes/telemetria.mjs` e `site/testes/sem_bus.mjs` atualizados e aprovados; toda a suíte de testes da página e do workspace passando com 100% de sucesso.

## [1.0.25] — 2026-10-29

- **Verificação da 1.0.24 contra os oficiais e com dados reais (WSL, `~/uvreal`), e dois testes ajustados** (ordem do
  Luciano, 29/10/2026). Nenhum código do serviço, do motor ou da página mudou: só dois testes que estavam
  desatualizados.
  - **Conferências do motor contra os oficiais: abrangências idênticas, 0 divergências.** `cargo test --release -p
    urna-velox -- --ignored --test-threads 1`: 8/8 de integração (2026: 83/83; 2024 1º turno: 5.569/5.569; 2024 2º
    turno: 51/51, com a situação de cada candidato) e 6/6 do "matematicamente".
  - **Ensaios do serviço com dados reais:** `tests/federacao.rs` 6/6 (inclui `tres_orgaos`, o eco das sondas entre os
    três nós) e `tests/servico.rs` 18/18 (inclui `sonda_nao_muda_a_publicacao`: 2º turno de 2024, AM e RO, publicação
    final idêntica com e sem sondas, mesmo checksum). O `nao_instaladas_exterior` falhava até esta versão (B-069, abaixo).
  - **Página:** `telemetria.mjs` tudo ok; `mapa_pagina` 36/36; `ritmo_pagina`, `ws_recusado`, `antes_17h`,
    `previsao_cartao`, `modo_pagina`, `painel_celular`, `relogio`, `termos` e `sem_bus` tudo ok; `visual_pagina` 1.260 ok
    e 0 falha (eram 56 do B-037, abaixo). Fumaça local: `uv-servico --sem-coleta --sonda-intervalo 2` com o `uv-sonda
    --duracao 30 --min-sondas 5` saiu com 0 (15 sondas, nenhuma pulada, atraso p99 até a ponta 0,435 ms).
  - **`visual_pagina` (B-037, 56 falhas):** o teste lia o fundo do link `.termos a`; desde a `0.9.19` (M-013) a pílula é a
    caixa `.termos-caixa`, e o link ficou transparente. O CSS estava certo (fundo a 94 %); o teste agora lê a caixa.
  - **`nao_instaladas_exterior` (B-069, `758 aux.json 404`):** o limite `aux_404 < 100` era anterior ao reforço da
    sondagem do exterior (B-056, B-057, B-060, B-065), que pede de propósito o `aux.json` das seções do ZZ ainda sem BU, e
    o 404 é a resposta esperada enquanto o BU não chega (759 pedidos com 404 de 409 seções distintas, espalhados pela
    noite). O teste agora exige o que importa: todo 404 é de seção que depois respondeu 200 (BU ou "Nao instalada") e
    nenhuma agregada é pedida. Antes: falhava; depois: `ok` (2.126 `aux.json`, 768 com 404, 816 sondagens). Detalhe em
    `docs/itens/B-069.md`.
  - `cargo test --release --workspace` sem falha; `scripts/itens.py verificar` e `scripts/docs.py verificar` em ordem.
  - **Implantação nos nós: ainda não feita** (ver o relatório do dia).

## [1.0.24] — 2026-10-29

- **Merge do branch `claude/sonda-test-message` na `main`: sondas (padrão *Test Message*) no caminho dos BUs e a
  telemetria na página** (pedido do Luciano em 24/10; o branch saiu da `0.9.3` e numerou o trabalho como `0.9.4` e
  `0.9.5`, números que na `main` já eram de outras versões: aqui as duas entradas viraram esta, e todo "desde a
  `0.9.4`/`0.9.5`" da sonda na documentação passou a "desde a `1.0.24`"). O conteúdo das duas entradas do branch segue
  abaixo, sem mudança, salvo os ajustes do merge.
- **Ajustes do merge.**
  - `docs/SONDA.md` → **`docs/arquitetura/SONDA.md`** (organização de `docs/` da `1.0.19`; entrou no índice
    `docs/README.md`); referências a `docs/PROTOCOLO_WS.md`, `PAINEL.md`, `OPERACAO.md` e ao ADR nas fontes da sonda
    apontam para as pastas novas.
  - **Protocolo: a mensagem `sonda` passou de §4.8 para §4.9**, porque a §4.8 da `main` já é o mapa das seções
    (`0.9.19`); o `op_desconhecida` lista `inscrever, cancelar, eventos, marcos, mapa, sonda`. No `Hub` da difusão
    convivem as sondas e o mapa (`Entrada::Sonda` e `Entrada::Secoes`, rotas `/api/v1/sonda` e `/api/v1/mapa/*`); no
    bloco `difusao` da saúde, `mapa`, `sondas_enviadas` e `latencias_ms.sonda_envio`; no `estado`, `trava` (M-020) e
    `latencia_bu_ms`. A página pede `op: sonda` e, quando há mapa, `op: mapa` ao conectar; o chip de BUs/min (botão da
    telemetria) fica na faixa nova da `main` (com o aviso de TSE parado do M-019 e o letreiro da próxima eleição).
    `site/dist/` regenerado (`npm run build`).
  - `TRAVAR_MOTOR` (falha provocada da sonda, só no simulado) e a trava do M-020 (`crate::trava`) são coisas
    diferentes e convivem no motor.
  - **B-068** (antigo B-055 do branch: CLS 0,00005 da barra de identificação com as fixtures): o B-055 da `main` é o
    eleitorado do 3221; renumerado e registrado com destino `1.0.x`.
- **Testes do merge** (contêiner Linux, sem os dados de `~/uvreal`): `cargo test --release --workspace` sem falha
  (`uv-aquisicao`: 137 de unidade; `tests/sonda.rs`: 4/4). Página, sem servidor: `telemetria.mjs` tudo ok (com o Chromium do sistema, `UV_CHROMIUM`); `mapa_pagina`, `ritmo_pagina`, `ws_recusado`, `antes_17h`, `previsao_cartao`, `hodometro`, `preposicoes`, `termos`, `modo_pagina`, `painel_celular`, `relogio`, `playlist_identidade`, `sem_bus`, `promo_letreiro`, `aviso_versao`, `seletor_fonte`, `topo_anuncie` tudo ok (Playwright global 1.63). `visual_pagina`: 1204 ok e 56 falhas do B-037 (pílula dos termos translúcida), **idênticas na `main` sem o merge**; `identificacao`, `horarios_pagina`, `previsao_pagina` e `playlist_pagina` param do mesmo jeito nos dois lados (precisam de servidor). `scripts/itens.py verificar` e `scripts/docs.py verificar` em ordem.
- **Verificação contra os oficiais: feita na 1.0.25** (ver a entrada dela: abrangências idênticas, 0 divergências) (rodar no WSL): `UV_DADOS=/home/dev/uvreal cargo test
  --release -p urna-velox -- --ignored --test-threads 1` e `UV_DADOS=/home/dev/uvreal cargo test --release -p
  uv-aquisicao -- --ignored --test-threads 1`, que inclui o `sonda_nao_muda_a_publicacao` (publicação final idêntica
  com e sem sondas) e o `tres_orgaos` com o eco das sondas. A sonda não muda a totalização (só passa pela thread do
  motor), mas a regra do `CLAUDE.md` vale.

### Do branch, como `0.9.4` e `0.9.5`

#### `0.9.5` do branch (24/10/2026): telemetria na página

- **Telemetria na página: a sonda à vista, discretamente** (pedido do Luciano em 24/10; branch
  `claude/sonda-test-message`). O chip de BUs/min alterna (3,5 s a cada 9 s, numa camada por cima da mesma caixa, só
  opacity/transform: nada se mexe) com a confirmação da sonda: **"✓ estável · 4 ms"** ("✓ 4 ms" no celular) — o
  atraso injeção → difusão da última sonda, no relógio do servidor, que é o que um BU recebido agora leva para ser
  processado e publicado. Sem sonda nova por mais de 2,5 × o intervalo (75 s com o padrão de 30 s; o intervalo é
  estimado das próprias sondas), o chip fica fixo em **"⚠ sem sonda há …"**. O chip virou botão: **tocar abre a
  janela de telemetria** com quatro velocímetros em SVG (ponteiros por transição CSS): ritmo (BUs/min, escala que
  cresce), atraso de processamento (sonda; com o caminho parado, "> idade" em vermelho), BU recebido → publicado (p50)
  e idade da última confirmação, com faixas verde/amarela/vermelha; abaixo, o atraso mediano, a sonda até a conexão,
  o p99 BU → publicado, o intervalo das sondas, quantas chegaram (e as que não) e os BUs aplicados. Abre logo abaixo do
  chip; Esc ou tocar fora fecha. Servidor sem sonda (versões antigas): o chip não alterna e a janela diz que o
  servidor não envia sondas.
  - Página: `site/src/sonda.js` (lógica, sem DOM) e `site/src/telemetria.js` (janela, velocímetros) novos;
    `conexao.js` pede `{"op":"sonda"}` ao conectar (uma mensagem de ~400 bytes a cada ~30 s por cliente); `app.js`
    (chip, alternância, janela); `estilo.css`. `site/dist/` regenerado (`npm run build`; com as fontes sem mudança o
    build reproduz o `dist/` versionado byte a byte).
  - Serviço: o `estado` (WebSocket e `GET /api/v1/estado`) ganha **`latencia_bu_ms`** = `{p50, p99, n}`, BU recebido →
    publicado (a medida `latencias_ms.bu_publicacao` que a `saude.json` já tinha), para o velocímetro.
  - Documentação: `docs/PROTOCOLO_WS.md` (acréscimos da `0.9.5`, §3, §4.3), `docs/SONDA.md` (a sonda na página),
    `site/README.md`.
- **Testes** (contêiner Linux, sem os dados de `~/uvreal`): **`site/testes/telemetria.mjs`** (novo, sem servidor;
  Playwright com WebSocket falso e relógio controlado): 15 conferências da lógica e, a 1366 px (escuro), a 390 px
  (claro) e a 390 px com servidor sem sonda, 52 conferências da página — tudo ok, CLS 0. Na primeira rodada falhou "a
  janela cabe na tela": a regra `.vidro` (mais abaixo no CSS) trocava a `position: fixed` da janela; corrigido movendo
  o bloco e ancorando a janela abaixo do chip na abertura. Regressão da página sem servidor, tudo ok: `ritmo_pagina`,
  `visual_pagina` (1260 conferências, 0 falha), `ws_recusado`, `antes_17h`, `previsao_cartao`, `hodometro`,
  `preposicoes`, `termos`, `modo_pagina`, `painel_celular`, `relogio`, `playlist_identidade` (rodados com o
  Playwright 1.63 instalado na máquina, porque o 1.55.1 do `site/` não tem Chromium aqui). Contra o `uv-servico` de
  verdade (`--sem-coleta`, sondas a cada 2 s): chip "✓ estável · <1 ms", janela "✓ Sistema estável — sonda nº 7
  confirmada há 1,8 s", intervalo estimado 2,0 s. `cargo test --release -p uv-aquisicao` sem falha (o
  `sondas_atravessam_o_servico` agora confere o `latencia_bu_ms` do `estado`).
- **B-055 registrado** (hoje **B-068**) (só o sintoma, regra de 18/10): com o `candidatos_exemplo.json` das fixtures, a barra de
  identificação mede CLS 0,00005 logo ao abrir; acontece igual sem as mudanças desta versão.
- Verificação contra os oficiais: continua **pendente** a da `0.9.4` (dados reais). Esta versão não mexe na
  totalização: só acrescenta um campo ao `estado` e muda a página.

#### `0.9.4` do branch (24/10/2026): sondas

- **Sondas: o padrão *Test Message* (Hohpe & Woolf) no caminho dos BUs** (pedido do Luciano em 24/10; branch
  `claude/sonda-test-message`; ADR-032, `docs/SONDA.md`). A saúde dizia que cada processo estava vivo, não que o
  caminho dos BUs andava; antes do primeiro BU e quando os BUs param, não dava para separar "o TSE parou" de "o Urna
  Velox parou". Agora: se as sondas chegam e os BUs não, parou o TSE; se as sondas também param, o problema é nosso.
  - **Injetor** (thread `uv-sonda` do `uv-servico`, `--sonda-intervalo`, padrão 30 s; 0 desliga): `Msg::Sonda` no
    **mesmo canal dos BUs**. O motor a passa adiante depois do lote e da publicação que estavam na frente
    (`Motor::passar_sondas`), a gravação depois dos arquivos do mesmo lote (`Gravar::Sonda`, `entregar_sondas`), e a
    difusão a entrega só a quem pede. A sonda **não toca** no estado, no diário, nos pacotes nem na publicação, e **não
    faz requisição nenhuma ao TSE**.
  - **Verificador no serviço** (`sonda::Vigia`): sonda pendente além de `--sonda-limite` (padrão: o dobro do intervalo,
    mínimo 5 s) → alerta **`sonda_atrasada`**, uma vez por episódio, gravado pela thread do injetor (que segue viva com
    o motor parado); no fim, `sonda_normalizada` no `servico.log`. Bloco `sonda` na `saude.json` (injetadas,
    concluídas, pendentes, perdidas, atraso, última, latências motor/gravação/total, `remotas`).
  - **Protocolo** (`docs/PROTOCOLO_WS.md`, acréscimos da `0.9.4`): `{"op":"sonda"}` (e `"ligar":false`), mensagem
    `sonda` (§4.8: `id`, `sessao`, `origem`, carimbos `t_injecao` → `t_motor` → `t_gravacao` → `t_difusao` → `t_envio`,
    `seq`, `ms`), `GET /api/v1/sonda` e, no bloco `difusao`, `sondas_enviadas` e `latencias_ms.sonda_envio` (injeção →
    socket). A página não pede sonda: nada muda para ela.
  - **Federação**: a sonda vai no lote do nó como registro `sonda` (sem seção, sem bytes, fora de `publicados`); o par a
    passa pelo próprio motor (eco) e a mostra em `sonda.remotas.<nó>` (pulos, repetidas, reinícios, `federacao_ms`
    entre os relógios, `atrasada`); contadores `sondas_publicadas`/`sondas_recebidas`. Todos os nós precisam da `0.9.4`
    (um nó antigo conta o registro em `rejeitados` e não aplica nada).
  - **`uv-sonda`** (binário novo): verificador de fora, cliente WebSocket de um ou mais serviços (ou da borda): situação
    `ok`/`esperando`/`atrasada`/`desconectado` por alvo, pulos, repetidas, reinícios, quedas, latência no servidor e de
    ponta; relatório em `--saida` e, com `--duracao`/`--min-sondas`, código de saída 0/1 para roteiros.
  - **Painel dos nós**: cartão "Sondas (caminho dos BUs)" (com o relatório do `uv-sonda` em `<saida>/sonda.json`, se
    houver), gráfico "Sonda: injeção → gravação", colunas `sonda_ms`/`sonda_atraso_s` e a situação do nó em vermelho com
    o caminho parado (o painel continua só lendo arquivos).
  - **Falha provocada para os ensaios**: `<saida>/TRAVAR_MOTOR` faz a thread do motor dormir até o arquivo sumir, **só
    contra o site simulado local** (ignorado contra qualquer outro servidor).
  - Documentação: `docs/SONDA.md` (novo), ADR-032, `docs/PROTOCOLO_WS.md` (§1, §3, §4.4, §4.8, §6), `docs/OPERACAO.md`
    (alerta `sonda_atrasada`), `docs/PAINEL.md`.
- **Testes** (nesta sessão, num contêiner Linux sem os dados de `~/uvreal`): `cargo test --release --workspace` sem
  falha. Novos: `sonda::testes` (9), `difusao::testes::{sondas_guardadas_e_rest, sonda_pelo_websocket}`,
  `federacao::no::testes::sonda_no_lote_e_no_par`, `painel::amostras::testes::verificador_externo_no_resumo` (e as
  colunas novas em `taxas_entre_amostras`) e **`aquisicao/tests/sonda.rs`, 4 testes de integração sem dados reais**
  (`uv-servico --sem-coleta` com o `candidatos_exemplo.json` das fixtures; rodam no `cargo test` comum):
  `sondas_atravessam_o_servico` (carimbos em ordem, nada na publicação nem no diário, `rede.requisicoes` = 0,
  `uv-sonda` com código 0), `sem_sondas_o_verificador_acusa` (código 1, `atrasada`/`desconectado`),
  `motor_travado_dispara_o_alerta` (`TRAVAR_MOTOR`: um `sonda_atrasada`, `uv-sonda` `atrasada`; destravado:
  `sonda_normalizada`, 0 perdidas) e `eco_entre_dois_nos` (federação de dois nós: eco nas `remotas` e no WebSocket do
  par). `uv-aquisicao`: 114 testes de unidade (eram 101 + os 13 novos). Cobertura (`cargo llvm-cov`, unidade +
  integração sem `--ignored`): `sonda.rs` 100 % das linhas e das funções, `bin/uv-sonda.rs` 91,9 %, `painel/amostras.rs`
  93,6 %; as funções novas de `servico.rs` e `difusao.rs` executadas por inteiro.
- **Verificação contra os oficiais: PENDENTE (rodar no WSL antes do merge).** A sonda passa pela thread do motor, então
  vale a regra do `CLAUDE.md`, mas os dados reais não estavam disponíveis nesta sessão. Para o Luciano rodar:
  `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (conferências do motor,
  que não muda nesta versão) e `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao -- --ignored
  --test-threads 1` — inclui o novo **`sonda_nao_muda_a_publicacao`** (2º turno de 2024, AM e RO a 120×: publicação
  final sem sondas × com uma a cada 50 ms, idênticas, mesmo `checksum` do motor, conferência 2/2) e o `tres_orgaos`
  agora com o eco das sondas entre os três nós.

## [1.0.23] — 2026-10-27

- Ajustes internos.

## [1.0.22] — 2026-10-27

- Ajustes internos.

## [1.0.21] — 2026-10-27

- Ajustes internos.

## [1.0.20] — 2026-10-27

- **Roadmap e versionamento** (regra do Luciano, 27/10/2026): depois da `1.0`, cada evolução nova tem a sua própria
  versão, sem ser juntada a uma versão já prevista, e as grandes sobem a MAJOR (`docs/processo/VERSIONAMENTO.md`,
  `ROADMAP.md`). O `ROADMAP.md` é o roadmap público; evolução nova só entra nele se o Luciano disser que é pública
  (`CLAUDE.md`).
- **Limite de requisições descrito por IP de saída** no `CLAUDE.md`: no máximo 100 req/s por IP (95 nas coletas
  longas, pausa de 10 min depois de um 429); a federação soma os limites dos IPs participantes.
- `ROADMAP.md` (`1.2.0`): folga por IP com "cada nó no seu IP"; linha do tempo com a `1.2.0`.
- pre-commit: confere também o roadmap conjunto quando o script dele existe no repositório.

## [1.0.19] — 2026-10-27

- **`docs/` organizado em pastas, com regras** (Luciano, 27/10, 09h18). Os 63 documentos soltos foram para
  `arquitetura/` (8: ADRs, aquisição, escalabilidade, protocolo WS, formatos, glossário, estimativas), `operacao/` (6:
  operação, ciclo diário, painel, borda), `relatorios/` (17: `RELATORIO_*`, `ANALISE_*`, `VALIDACAO_*`), `handoffs/` (23:
  `HANDOFF_*`, `PROMPT_*`), `propostas/` (1), `processo/` (versionamento) e `legal/` (licenciamento e privacidade); as
  imagens de `docs/bugs/` para `docs/itens/img/`. Na raiz de `docs/` ficam só o índice novo **`docs/README.md`** (uma
  linha por documento, por pasta, e as regras) e o `BUGS.md`. Movidos com `git mv` (histórico preservado); **142
  arquivos** com links e caminhos reescritos (Markdown de todo o repositório, comentários e roteiros com `docs/X.md`,
  `site/documentos.mjs` com os caminhos de `docs/legal/` e os novos relativos `../../LICENSE`; `dist` regerado: só o
  comentário com os caminhos mudou em `termos.html`/`privacidade.html`). **Regras** (`docs/README.md`, "Regras", e
  `CLAUDE.md`): nada solto na raiz; nome pelo tipo (prefixo por pasta, MAIÚSCULAS_COM_SUBLINHADO, datas AAAA-MM-DD);
  todo documento no índice no mesmo commit; imagens só em `img/`, `itens/img/` ou `marca/`; links sempre válidos;
  pasta nova só com o Luciano. **`scripts/docs.py verificar` no pre-commit** confere pastas, nomes, índice e links
  relativos dos `.md` de `docs/` e da raiz (`docs.py links --todos` mostra os do repositório inteiro). **Testes:** `scripts/testes/test_docs.py` 4/4 (em ordem; as oito recusas; links em código
  e externos ignorados; o próprio repositório) e `test_itens.py` 4/4.

## [1.0.18] — 2026-10-27

- **Bugs, melhorias e roadmap integrados** (Luciano, 27/10, 09h04: "a cada bug e melhoria cadastrados, isso precisa ser
  harmonizado com o roadmap"). (1) `docs/BUGS.md` ganha a coluna **roadmap** (destino de cada item: etapa do ROADMAP,
  `1.0.x` = manutenção da 1.0, série de um item já fechado, ou `fora`) e a coluna **detalhe**; os 85 itens classificados
  (os 9 abertos: B-010, B-017, B-027, B-028, B-046, B-066 e M-010 em `1.0.x`; B-067 e M-002 na etapa nova `1.2`).
  (2) A tabela fica curta (uma frase por item): o texto longo de 24 itens e as 30 seções "Testes que comprovam" foram
  para `docs/itens/<id>.md` (48 arquivos; links relativos ajustados) — `BUGS.md` de 141 KB para 43 KB, nada
  perdido. (3) `ROADMAP.md`: seção nova **🔧 Manutenção da 1.0 (`1.0.x`)**, etapa nova **🛡️ `1.2.0` — Coleta robusta
  na borda** (as correções do B-067; a ordem com a `1.1` é do Luciano), e em cada etapa a lista dos seus itens, gerada
  entre marcadores `<!-- itens:… -->` por `scripts/itens.py gerar`; histórico por série (`0.8.x`, `0.9.x`). (4)
  `scripts/itens.py verificar` no **pre-commit** (com Python 3; sem ele, só aviso): recusa item sem destino, destino de
  item aberto que não é etapa do ROADMAP, item fechado sem versão, descartado fora de `fora`, link de detalhe quebrado,
  etapa ✅ com item aberto e lista do ROADMAP desatualizada. (5) Regra nova no `CLAUDE.md` (seção "Bugs"): todo item
  entra já com destino, no mesmo commit. Estados: registrado → triado → na fila → teste vermelho → corrigido/feito ·
  descartado. **Testes:** `scripts/testes/test_itens.py` (4/4: coerência e idempotência, as sete recusas, lista
  desatualizada, o próprio repositório); `itens.py verificar`: 85 itens, 9 abertos, coerentes. M-012 fechado (revertido
  pelo M-022) e B-065 marcado como implantado (1.0.15).

## [1.0.17] — 2026-10-27

- **README e ROADMAP** (alterações do Luciano, sem commit desde 26/10): o README troca a seção "Para quem acompanha a
  eleição" e a dos replays por "Funcionalidades e Inovações" e "Playlist Interativa e Replays 24/7" (mapa das seções,
  previsão, comparação, player do replay), e o selo de estado passa a "production ready | 2.0 in dev"; ajustes no
  ROADMAP.

## [1.0.16] — 2026-10-26

- **Playlist: a suspensão da eleição oficial também no `uv-playlist`.** O `63_ciclo.sh --playlist` gravava
  `ELEICOES_OFICIAIS` só no `ciclo.env` (lido pelo vigia), mas o `uv-playlist` lê `UV_ELEICOES_OFICIAIS` do
  `/etc/urna-velox/playlist.env`: na volta ao ciclo (1.0.15) a página da playlist continuou "suspensa até 29/10" com o
  ciclo já rodando. Agora o `--playlist` grava as duas; no nó A foi aplicado à mão (`playlist.env` com 2026-10-04 e
  reinício do `uv-playlist`): `/api/v1/playlist` com `suspensao: null` e o 3221 rápido no ar.
- **`site/testes/playlist_pagina.mjs` acompanha a playlist de hoje:** o número de opções votáveis vem da
  `/api/v1/playlist` (oito desde a 1.0.13, eram seis fixas), a contagem da próxima eleição ao vivo só é conferida quando
  há uma, e o "replay no ar não pode ser votado" usa o item no ar da API (era o 2º turno de 2024 em tempo real, fixo).
  Contra o site no ar (https://urnavelox.com.br/, 3221 rápido): antes, 7 falhas (6 opções, contagem e o item no ar
  fixos); depois, tudo certo — selo "REPLAY ▶ Playlist" à vista nas três larguras, o selo abre o aviso, o link leva à
  playlist, voto registrado e retirado.

## [1.0.15] — 2026-10-26

- **M-022 — o selo "EM BREVE" / Playlist volta ao topo da página** (Luciano, 26/10, 20h16: "pode voltar o chip de
  playlist para o código"), com o link "▶ Playlist" no aviso "Próxima eleição AO VIVO" — o contrário do M-012 (escondidos
  em 24/10 até o fim da totalização do 2º turno). `site/src/index.html`: sai o atributo `hidden` do `#conexao` (o
  `app.js` já mostra o link quando o selo está à vista). **Teste:** `site/testes/topo_anuncie.mjs` passa a exigir o selo
  à vista e o link da playlist nos dois avisos — **antes** (dist da 1.0.14): 14 falhas ("selo … à vista" nas 10
  combinações de largura e tema, "o aviso … leva à playlist" nas 4); **depois**: tudo ok. Também `modo_pagina`,
  `promo_letreiro` e `antes_17h` ok (as conferências do selo à vista, puladas com o selo escondido, voltam a valer); o
  `playlist_pagina` precisa do site no ar e roda contra os nós depois da implantação.
- **Volta ao ciclo de replays com a playlist** (Luciano, 26/10, 20h16), agora com o 3221 na rotação (M-021, 1.0.13): os
  nós passam da produção do 3221 para o ciclo; a suspensão da playlist (de 24/10 a 29/10, em volta da eleição) foi
  encerrada antes, a pedido, com `ELEICOES_OFICIAIS` só com a do 1º turno (04/10) no `ciclo.env` dos nós.

## [1.0.14] — 2026-10-26

- **B-067 — análise dos 429 de 25/10** ([B-067](docs/itens/B-067.md)). (1) **T1:** a noite do 3221 repetida em tempo
  real no simulador local com a configuração de produção (2 nós federados com IP local próprio, 95 req/s, `cs.json` a
  cada 10 s, sondagem; `aquisicao/scripts/ensaio_b067.sh`), sem 429: atraso p50 17 s, p90 33 s, máximo ~50 s
  (produção: 389 s e 717 s), fila máxima 4.120 BUs — **o atraso da noite veio das pausas de 429**. (2) **T2:**
  `aquisicao/scripts/b067_regras.py` testa 7 janelas × 5 tetos × 3 pesos de 404 × 3 atrasos de rede sobre o registro
  do T1: nenhuma regra reproduz os horários; a chegada passa de 100 req/s em segundos isolados (até 110) a cada ~3 min
  de plena carga, com a saída espaçada a 1/95 s. (3) **T3, borda real dentro do limite:** 95 e 100 req/s por IP, dois
  IPs da mesma faixa a 95 cada e ~13 % de 404, sem 429. **Conclusão:** 404 e faixa de IP descartados; na noite a borda
  foi mais rígida que num dia comum; a pausa de 10 min fica; propostas: nó pausado fora da partição, ≤ 80 req/s por nó
  com mais nós, registro por segundo. `docs/BUGS.md` B-067 atualizado.
- **Simulador: regras do limite por IP** (`--limite-ip-regra segundo | fixa:W | deslizante[:W]`,
  `--limite-ip-peso-404 P`; padrão igual ao de antes, segundos fechados). Testes novos
  `simulador::testes::regras_do_limite_por_ip` e `regra_do_limite_de_texto` (6/6 do módulo `simulador`); o resumo traz
  a regra, o peso e o maior peso numa janela por IP.
- **Handoff para revisão externa** do uso de sistema de arquivos compartilhado com N instâncias
  ([docs/handoffs/HANDOFF_FS_COMPARTILHADO.md](docs/handoffs/HANDOFF_FS_COMPARTILHADO.md)); a revisão e o Luciano concluíram: manter a
  federação HTTP e aumentar o número de nós.

## [1.0.13] — 2026-10-26

- **M-021 — o 3221 vira replay no ciclo diário** (Luciano, 26/10, 18h36: "pode manter as 2h em silêncio do TSE, pois o
  objetivo do replay é repetir a realidade, sempre"). `uv-ciclo.sh`: o 3221 sobe com o `uv-simulador` e o acervo
  `/srv/urna-velox/acervo/3221` (1.0.12), como os outros, alternando rápido (500×) e cadência (1×); `CORTE` de
  26/10/2026 02:00; em cadência, acelera o fim depois de 4h15 (`TEMPO_CADENCIA_S=15300`, das 17:00 às 21:15: as 2 h sem
  BU entram como foram). O modo `espera` (vitrine sem coleta) continua aceito por `uv-ciclo.sh iniciar 3221 espera`,
  mas sai da rotação. `uv-ciclo-vigia.py`: o próximo modo do 3221 alterna como os outros; a vitrine vale para
  `modo == espera`, não mais para o pleito 3221. `uv-playlist.py`: oito opções votáveis (3221 nos dois modos),
  durações padrão do 3221 (rápido 600 s, cadência 15.900 s), a rotação sem votos não devolve mais `espera`.
  **Testes:** `deploy/oci/testes` 24/24 ok (`test_playlist`, `test_vigia_playlist`, `test_ciclo_resumo`); os testes
  novos com o código da 1.0.12 falham como esperado (`test_3221_e_replay_na_rotacao`: `'espera' != 'rapido'`).
  `bash -n` do `uv-ciclo.sh`. **Implantado** nos dois nós (`63_ciclo.sh --scripts`; `uv-playlist.py` no A com reinício
  do `uv-playlist`, que já lista as oito opções). O ciclo não foi iniciado: a produção do 3221 segue no ar e a playlist
  está suspensa até 29/10 00:00 (três dias depois da eleição).
- **Acervo completo também no nó B:** os pacotes `.uvpk`, `cs` e oficiais dos quatro pleitos (até aqui só no A, onde
  roda o simulador) copiados do A para o B pelo bucket, para os dois nós terem os mesmos dados (pergunta do Luciano:
  "não era para os dois ter todos os arquivos espelhados?"). SHA-256 do pacote de cada pleito igual nos dois lados;
  `acervo/` com 6,0 GB (3220), 601 MB (3221), 3,5 GB (452) e 116 MB (453) nos dois nós, 17 GB usados em cada um.
- **Análise do atraso, revisão** (`docs/relatorios/ANALISE_ATRASO_2026-10-25.md`): (1) o motivo das 2 h sem BU — a votação foi
  prorrogada até as 19h por decisão do ministro Cristiano Zanin (STF), segundo a imprensa (link para O Povo no
  documento; informação do Luciano, 26/10, 19h13): a suspeita 1 se confirma, mas pela mudança de horário, não por
  lentidão do TSE; (2) quadro explicando o "espelho" (instância de validação lendo uma cópia dos arquivos públicos fora
  da borda, sem o teto de 100 req/s; versão 1.0.8, a mesma da produção). Gráfico da chegada com o rótulo novo.

## [1.0.12] — 2026-10-26

- **Acervo do 2º turno de 2026 (3221) para os replays acelerados** (pedido do Luciano, 26/10: "precisamos ter os dados
  prontos para os replays acelerados"), montado só com o que já tínhamos da noite real, sem nenhuma requisição ao TSE:
  pacotes `.uvpk` do nó A (499.207 BUs), `cs.json` de 24/10, os 45 `-u.json` oficiais baixados na conferência do fim
  da apuração, `candidatos.json` com a destinação final (`uv-coletor preparar --offline` com esses oficiais no cache,
  como o B-005: 2 candidatos "Anulado sub judice", `destinacao_presumida` = false) e `linha-do-tempo.csv`. Novo
  `aquisicao/scripts/linha_do_tempo_servico.py`: a linha do tempo no formato das métricas da coleta, gerada do
  `secoes.log` do `uv-servico` (499.248 seções: 499.207 com recebimento, 41 não instaladas). Enviado aos nós em
  `/srv/urna-velox/acervo/3221` (A completo, B com candidatos e linha do tempo, `.completo` nos dois) e copiado para
  `D:\dados\urna-velox\pleitos\2026-10-25-3221\acervo`. **Verificação:** replay local a 500× (`uv-simulador` +
  `uv-servico` 1.0.8, como o modo rápido do ciclo): 499.207 BUs em 65 s; conferência 42 de 45 arquivos idênticos ao
  oficial, Presidente 29/29, situações 91/91; as 3 divergências que restam são as de seções totalizadas do B-066
  (cargos 25, 26 e 27). Evidências em `D:\dados\urna-velox\ensaios\20261026-1814-replay-3221`. **O ciclo diário ainda
  não usa esse acervo:** no `uv-ciclo.sh`, no vigia e na playlist o 3221 continua no modo "espera" (vitrine sem coleta);
  trocar por replay depende de decisão do Luciano.
- **Nós:** apagados, com o aval do Luciano (26/10, 18h05), os dados só de teste (`ciclo/historico`,
  `simulado_local_20261016_184634`, amostras) e as execuções do 3220 em `/srv/urna-velox/anteriores`, todos guardados
  no backup de 26/10 (`D:\dados\urna-velox\backup-nos\2026-10-26`). Disco usado: nó A de 44 para 16 GB, nó B de 21
  para 6,3 GB. O `acervo/` e a saída da noite de 25/10 ficam.

## [1.0.11] — 2026-10-26

- **Análise do atraso do 2º turno** ([docs/relatorios/ANALISE_ATRASO_2026-10-25.md](docs/relatorios/ANALISE_ATRASO_2026-10-25.md), pedido do
  Luciano em 26/10: "por que o Urna Velox ficou atrás do TSE?"). Com o backup completo dos dois nós, a instância de
  validação no espelho e a linha do tempo do 1º turno: (1) o TSE não registrou nenhum recebimento de BU de 17:09 a
  19:08 e depois recebeu 99 % das seções em ~2 h, a ~91 BUs/s sustentados por 1 h (1º turno: 75 BUs/s); (2) isso é
  96 % da capacidade da produção (2 nós × 95 req/s ÷ 2 requisições por BU); (3) três HTTP 429 (B 19:47, A 19:53 e
  21:00) pararam um nó por 10 min cada e a fila chegou a 63 mil BUs (21:12). Produção: p50 389 s / p90 717 s depois do
  recebimento e "matematicamente" às 21:23 (TSE ~21:09); espelho sem teto: p50 42 s e 21:12. Implantações, trava M-020,
  `cs.json` e CPU descartados como causa. Seis evoluções propostas (folga ≥ 1,5×: 1 requisição por BU ou 3º nó; nó em
  pausa por 429 sai da partição; causa dos 429; série de req/s na noite; replay de 25/10 no ciclo; nada de implantação
  na apuração), nenhuma feita. Gráficos em `docs/img/analise_2026-10-25_*.png`; scripts em
  `aquisicao/scripts/analise_noite/` (`curvas.py`, `fila.py`, `turnos.py`, `graficos.py`). Só lê dados: sem mudança em
  totalização, sem conferência contra os oficiais.
- **Bugs registrados (só o sintoma):** B-066 (conferência do fim da apuração com 5 de 45 arquivos divergentes, igual
  nos dois nós; Presidente 29/29 idêntico) e B-067 (três HTTP 429 no pico com o teto de 95 req/s).
- **Dados de cada pleito no D: em `pleitos/<AAAA-MM-DD-código>`** (pedido do Luciano, 26/10: a data no nome diz de
  qual eleição se trata): `2024-10-06-452`, `2024-10-27-453`, `2026-10-04-3220`, `2026-10-25-3221`, montadas a partir
  do backup completo dos nós de 26/10 (`backup-nos/2026-10-26`, 56,5 GB em pedaços de 1 GiB, SHA-256 conferido). As
  linhas do tempo (`linha-do-tempo/<código>.csv`) e os insumos do simulado (`simulado-3220/`) mudaram para dentro da
  pasta do pleito. Código ajustado: `pasta_pleito <código>` no `deploy/oci/lib.sh` (`UV_PLEITOS` troca a raiz);
  `60_simulado.sh` (insumos padrão = `<pasta do pleito>/simulado`, também para `--pleito` diferente de 3220),
  `61_relatorio.sh`, `aquisicao/scripts/ensaio_carga_pagina.sh` e os três testes `--ignored` de
  `aquisicao/tests/servico.rs` que leem a linha do tempo do 3220 (`UV_LINHA_3220` continua valendo). Docs:
  `CLAUDE.md`, `docs/operacao/CICLO_DIARIO.md`, `deploy/oci/README.md`, `docs/relatorios/RELATORIO_M002_VAZAO.md`,
  `docs/handoffs/HANDOFF_DEPLOY_SEM_SSH.md`. Verificação: `bash -n` nos quatro roteiros; os testes `--ignored` não foram rodados
  (só o caminho padrão mudou).

## [1.0.10] — 2026-10-26

- **Revisão para a cópia pública:** capturas de tela do celular em `docs/bugs/` (B-030, B-031, B-043, B-045, B-052)
  recortadas sem a barra de status do aparelho e a barra do navegador, e gravadas sem metadados (EXIF/ICC); as
  análises avulsas dos logs de acesso do Caddy saem do `deploy/oci/`; `docs/handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md`,
  `docs/handoffs/HANDOFF_CARGA_E_ARQUITETURA.md` e a entrada da 0.8.43 descrevem o WebSocket autenticado só pelo lado técnico; a citação do M-020 passa a "controle anti-exposição do TSE" (CHANGELOG, `docs/BUGS.md` e relatório
  de 25/10).
- Não mexe em totalização nem em resultado.

## [1.0.9] — 2026-10-26

- relatorio de acessos e operacao da eleicao de 25/10/2026

## [1.0.8] — 2026-10-25

- **M-020 — trava coberta por testes unitários** (Luciano, 18h34: "tente usar apenas testes unitários, na maior
  cobertura possível dos pontos ajustados"). A fila dos retidos sai do motor para `trava::Fila` (ordem de chegada,
  uma entrada por seção, solta até o limite; desligada, solta tudo) e a condição de valer, para `trava::vale`. Testes
  novos: `trava::testes::fila_retem_na_ordem_e_solta_ate_o_limite` (ordem, limite 0, seção repetida, soltar a cada
  segundo sem lote novo, desligar com retidos, BU substituto), `trava_e_fila_juntas_no_cenario_do_tse_soltando_tudo`
  (500 BUs de uma vez com o TSE em 20 %: para em 30 %; TSE a 45 %: solta até 55 %; 0 = nada sai; 100/desligada = sai
  tudo), `so_vale_contra_o_tse_com_a_comparacao` (simulador, replay, sem comparação),
  `tse_resultados::testes::secoes_do_ultimo_arquivo_para_a_trava` (a % do TSE vem do último arquivo lido) e, em
  `painel::config_no::testes::valida_cada_campo`, `TRAVA_PP` de 0 a 100 (101, -1 e texto recusados; não pede
  CONFIRMO). Lib do `uv-aquisicao`: 122 ok; `test_painel_acao.sh`: 43 ok. Testes amplos (18h32): workspace sem falhas;
  motor `--ignored` com dados reais 8 + 6 ok; página (mapa, aviso do TSE parado, 17h, modo) ok; ensaios do serviço em
  andamento. **Compilada e não implantada** (aguarda o aval do Luciano).

## [1.0.7] — 2026-10-25

- **M-020 — escala da trava de 0 a 100** (Luciano, 18h26): `TRAVA_PP=0` = **nunca à frente do TSE**, `100` = sem
  limite; vazio ou ausente = desligada (na 1.0.6, 0 desligava). O painel aceita de 0 a 100. Novo
  `deploy/oci/trava.sh <0-100|desligar>` (sem argumento, mostra o valor): muda nos dois nós sem reiniciar o serviço.
  `trava::testes` cobrem 0, 10, 100 e ausente. **1.0.6 e 1.0.7 compiladas e não implantadas** (Luciano, 18h26: "Não
  implante sem meu aval"); a implantação da 1.0.6 que tinha começado foi interrompida antes da troca de binários (os
  dois nós seguem na 1.0.5; `TRAVA_PP=10` já está no `no.env` dos dois, sem efeito na 1.0.5).

## [1.0.6] — 2026-10-25

- **M-020 — trava "no máximo X p.p. à frente do TSE"** (Luciano, 18h03: "controle anti-exposição do TSE"; decisões
  das 18h10 e 18h18). Com `TRAVA_PP` > 0 no `no.env` (editável no painel dos nós, relido a cada 5 s, **sem
  reiniciar**; vazio/0 = desligada), o `uv-servico` só **aplica** BU enquanto a % de seções totalizadas do Brasil
  (Presidente) for menor que a % do `br-c0001-e006258-u.json` do TSE + `TRAVA_PP`. A coleta segue igual (BU baixado,
  validado e gravado no pacote); os que passam do teto ficam **retidos** na ordem de chegada e entram a cada segundo,
  até o teto, quando o TSE anda. Sem dado do TSE vale o último lido (0 % antes do primeiro): nunca solta por falta de
  dado. A trava só vale contra o site do TSE (não no replay), depois da retomada (o diário entra inteiro) e com a
  comparação ligada (é dela a busca); o fim da apuração espera os retidos; a sondagem não pede seção retida.
  **Rede (combinado com o Luciano em 25/10, 18h10):** o `br-c0001` do TSE passa a ser buscado sempre, por uma
  inscrição permanente na comparação (uma busca por `--tse-intervalo`, 30 s, por nó, com `If-None-Match`, prioridade
  baixa, regra do 429). Nada na página (18h18: "sem precisar mostrar nada para o usuário"); o `estado` e o
  `saude.json` trazem `trava` (`pp`, `ligada`, `tse_pct`, `retidos`, `soltos`). Testes: `trava::testes`
  (`teto_em_pontos_percentuais`, `le_o_no_env_e_mantem_o_ultimo_tse`); lib do `uv-aquisicao` 118 ok; workspace sem
  falhas. Não muda o resultado final (só o momento em que cada BU entra): conferência contra os oficiais no fim da
  apuração. `uv-painel-acao`: `TRAVA_PP` entre as chaves editáveis.

## [1.0.5] — 2026-10-25

- **B-058 — `ultimo_bu` vazio depois de um reinício até o próximo BU** (Luciano, 17h58: "Ataque"). Achado de novo na
  implantação da 1.0.4 (17h56), com o TSE sem BU novo desde 17:11:45: o `estado` vinha com `ultimo_bu.site` nulo e o
  aviso M-019 não tinha o horário (e um BU atrasado do exterior, como os de Londres das 15h47, o faria dizer "desde
  15h47"). Agora a retomada tira o horário do diário: a maior disponibilização das seções com BU
  (`ultimo_bu_do_diario`). Teste: `servico::testes::ultimo_bu_vem_do_diario_na_retomada` — **antes**: `B-058: o último
  BU do diário na retomada — left: None, right: Some(1792948305)`; **depois**: ok (lib do `uv-aquisicao`: 116 ok;
  workspace sem falhas). Só o horário exibido: sem conferência contra os oficiais.

## [1.0.4] — 2026-10-25

- **M-019 — aviso no topo quando o TSE não publica BUs** (Luciano, 25/10, 17h42; o último BU publicado era das
  17:11:45). Chip âmbar no começo da faixa: **"TSE sem novos BUs desde 17h11 · retomamos automaticamente quando
  voltarem"** (no celular, só "TSE sem novos BUs desde 17h11", no lugar do ritmo, que fica em 0). Aparece na noite ao
  vivo (não no replay), depois das 17h, com a apuração aberta, quando o último BU no TSE que a página conhece
  (`ultimo_bu.site` do `estado`, que chega no máximo a cada 10 s, ou o `ultimo_bu` do resultado na tela) tem mais de
  10 min. Reavaliado a cada segundo: **some sozinho** no primeiro BU novo. Sem horário do último BU (ex.: logo depois
  de um reinício, B-058), não aparece. Teste novo `site/testes/sem_bus.mjs` (1366 e 360 px: à vista com "desde 17h11",
  nada da faixa fora da tela, some em até 3 s com um BU novo, nada com 4 min parado nem antes das 17h); `antes_17h.mjs`
  continua ok. Só a página: sem conferência contra os oficiais.
- Vai junto na implantação o B-065 da 1.0.3 (exterior sondado com o `cs.json` atrasado).

## [1.0.3] — 2026-10-25

- **B-065 — o exterior ficava preso ao ritmo do `cs.json` depois que ele passava a listar seções** (Luciano, 16h38:
  "nosso concorrente está com mais seções"). Às 16h40: concorrente com **776 de 1.351** urnas do exterior, Urna Velox
  com **708** (o mesmo das 15h46). A diferença, cidade a cidade: **Londres 0 × 48** e **Lisboa 83 × 103**. O `aux.json`
  de Londres 29971/0001/0985 estava no ar desde **15:47:02** (BU "Recebido"), mas o `zz-p003221-cs.json` seguia com as
  708 seções das 15h46 (`cs_alterados` 0). Causa: a sondagem antecipada do exterior (B-056) só valia enquanto o
  `cs.json` do ZZ não listasse nenhuma seção; ele passou a listar às 14h e, daí em diante, a sondagem só voltava
  faltando 5 % ou 50 seções — o último evento `sondagem` nos dois nós é das 14h01 (`sondadas` 0 desde o reinício das
  16h10). Agora, no domingo, o exterior segue sondado (mesmas regras: fora as agregadas e as que já estão na fila do
  `cs.json`, calendário do 1º turno, município com BU a cada 20 s, reforço só com a fila do Brasil abaixo de 20, 429
  desliga o reforço) enquanto faltar seção. Brasil: nada muda. Teste: `servico::testes::exterior_sondado_com_o_cs_json_atrasado`
  — **antes**: `B-065: o exterior segue sondado com o cs.json listando seções — left: Nao, right: Antecipada`;
  **depois**: ok (lib do `uv-aquisicao`: 115 ok; workspace sem falhas). `exterior_sondado_antes_do_cs_json` ajustado à
  regra nova (o caso "cs.json listando, domingo" passa de `Nao` a `Antecipada`; fora do domingo continua a de 16/10).
  Não mexe em totalização: sem conferência contra os oficiais. **Pronto e não implantado** (Luciano: só subir agora se
  for muito problemático).

## [1.0.2] — 2026-10-25

- **B-064 — o "de N" do mapa das seções não batia com o total de seções da página** (Luciano, 15h36 do 2º turno de
  2026, suspeitando das agregadas). Em produção (1.0.0): mapa com **517.179** seções (`/api/v1/mapa/estado`), página
  com **499.248** (`secoes.total` do `br-c0001`). O mapa numerava todas as seções do catálogo, inclusive as
  **agregadas** (sem urna nem BU próprio), que o `Publicador` tira do total desde a 0.9.x (B-051). Agora:
  `mapa::chaves_do_catalogo` deixa fora as agregadas do `candidatos.json`; as que se revelam agregadas na noite (`nsp`
  no `cs.json` ou sondadas sem `aux.json`) saem do total do mapa junto com o da página (`Motor::agregada` chama
  `Mapa::retirar`); a fotografia leva os índices retirados em `ret` e o `total` já descontado, as mensagens `mapa`
  levam o `total`, e a página relê a fotografia quando ele muda (o total da UF e o do exterior também descontam).
  Agregada que vem com BU volta ao total, como na página. Testes: `mapa::testes::b064_total_do_mapa_igual_ao_da_pagina`
  — **antes**: `lista do mapa sem a agregada do candidatos.json — left: Some(2), right: Some(1)`; **depois**: ok (lib
  do `uv-aquisicao`: 114 ok); `site/testes/mapa_pagina.mjs` (cenário B-064) — **antes** (dist da 1.0.0): `FALHA B-064
  agregada da noite 4796 ≠ 4794`, `FALHA B-064 total da UF`; **depois**: 36 conferências ok.
- **Mapa: o botão "Globo" passa a se chamar "Exterior"** (pedido do Luciano, 25/10, 15h47; o valor guardado em
  `uv-mapa` continua `globo`).
- Votos e totais da apuração não mudam (só o cartão do mapa): conferência = o total do mapa igual ao `secoes.total` da
  página em produção depois da implantação.

## [1.0.1] — 2026-10-25

- **B-063 — `30_verificar.sh` (e qualquer comando com script de ~2.300 a 3.500 bytes) recusado pela OCI** (achado ao
  conferir a implantação da 1.0.0): `content.source.text size must be between 1 and 4096`. O `uv_oci.py` mandava o script
  em linha (base64, que cresce 4/3, dentro do wrapper) se ele tivesse até 3.500 bytes (o repositório tinha 2.000; o 3.500,
  e a saída em linha até 600 bytes, eram mudanças locais do Luciano, mantidas); o `30_verificar.sh` (~2.500 bytes) virava
  um comando de ~4.650 caracteres. Agora o comando é montado (`montar_wrapper`) e vai em linha só se couber em
  `LIMITE_TEXTO` = 4.096 caracteres; senão o script vai pelo bucket. Teste novo `deploy/oci/testes/test_uv_oci.py`
  (`LimiteDoTexto`: scripts de 100 a 6.000 bytes) — **antes**: `4652 not less than or equal to 4096 : script de 2600
  bytes`; **depois**: 6 ok. Conferido de verdade: `./30_verificar.sh` contra os dois nós da 1.0.0 (antes: o traceback da
  OCI no nó A; depois: os dois nós, acompanhando, `--rps 95 --intervalo-cs 10`, 0 HTTP 429, federação com `no-a` e `no-b`).
  Só o script local de operação muda: nada a implantar nos nós (seguem na 1.0.0).
- Nada mexe em totalização ou resultado: sem conferência contra os oficiais.

## [1.0.0] — 2026-10-25

- **1.0.0 — sistema completo para os cargos majoritários** (pedido do Luciano, 24/10, 22h22: "quero subir … com a
  versão ganhando um 1.0.0"): automatizado, federação, WebSocket, site com comparação, previsão e, agora, o mapa.
- Branch `claude/mapa-secoes` rebaseada sobre o `b056-ci` **0.9.27** (`main` 0.9.24 com B-056 a B-059, o seletor de
  fonte M-015 da 0.9.25, o aviso de versão nova M-016 da 0.9.26 e a cobertura da sondagem B-060 da 0.9.27; ficam de
  fora, por decisão do Luciano, `claude/sonda-test-message` e `merge/ws-autenticado`); o mapa passa a ser o **M-017** (o
  M-013 e o M-014 são os termos e o letreiro; o M-015 e o M-016, o seletor e o aviso). Escrito antes como 0.9.19 e
  0.9.20 na branch.
- **M-017 — Mapa das seções** (Luciano, 24/10, 20h33; mockup aprovada no chat: "Só Brasil escuro. E com opção de
  mapa mundi"): novo cartão na coluna larga, abaixo da corrida. Cada BU totalizado "explode" (anel e clarão) e deixa um
  ponto aceso; a luz é aditiva (onde há muitas seções juntas, fica branco), e ao fim da noite o Brasil inteiro aparece
  iluminado pelas ~500 mil seções. Só a silhueta do Brasil é escura, com um halo da aurora; o cartão é o vidro do
  portal. Opções guardadas no navegador (`uv-mapa`): vista **Brasil** ou **Globo** (esfera translúcida que se gira com
  o mouse ou o dedo, continentes em malha de pontos; o exterior acende em rosa e cada BU de fora viaja num arco até
  Brasília) e cor **Luz** ou **Horário** (faixa de 6 min do BU, ciano → violeta → rosa). Os marcos da noite (1ª seção
  do Brasil e de cada UF, BU nº 100 mil…) soltam uma onda maior e um selo no mapa. Só busca e desenha quando o cartão
  está na tela; modo HTTP (B-026): relê a fotografia a cada consulta.
- **Servidor** (`aquisicao/src/mapa.rs`, protocolo §4.8): o servidor não sabe de geografia; numera as seções do
  catálogo (lista ordenada pela chave município/zona/seção), acende a seção na primeira aplicação do BU (do lote, da
  federação e, na retomada, do diário com a faixa da disponibilização) e entrega por `GET /api/v1/mapa/secoes` (lista
  fixa por processo, `ETag`, `max-age=300`), `GET /api/v1/mapa/estado` (fotografia: um byte por seção em base64, refeita
  no máximo 1×/s, `max-age=5`) e `{"op":"mapa","desde":n,"epoca":e}` no WebSocket (o que acendeu depois de `n`, no
  máximo uma mensagem a cada 250 ms por conexão, até 8.192 pares por mensagem; outra época → `reiniciar`). Saúde:
  bloco `difusao.mapa`. (Escrito numa sessão sem as dependências do Rust; compilou de primeira em 24/10 e os testes
  passaram — abaixo.)
- **Geografia** (`site/geo/gerar_mapa.py` → `site/src/mapa.json` → `/static/mapa.json`): contorno das UFs, centro dos
  5.570 municípios pelo código do TSE, malha dos continentes, cidades do exterior e os **locais de votação de 2026**.
  Fontes e licenças em `site/geo/README.md`.
  - Locais: base oficial "Eleitorado — local de votação" de **2026** (gerada pelo TSE em 09/10/2026), lida do cache
    que o mcp-eleitoral já guardava na estação (`locais-votacao-2026.v1.json`; nada baixado do TSE): 95.570 locais,
    517.179 seções. Novo `gerar_mapa.py --so-locais --locais <base>` troca só os locais (e a tabela do exterior) de um
    `mapa.json` já gerado. Formato `locais_v: 2`: por zona, só as fronteiras onde a posição muda (as seções de locais
    diferentes vêm intercaladas na numeração: 282.576 fronteiras, 1.195 sem coordenada → em volta do município),
    posição em 1/5 da unidade do plano (~0,9 km, abaixo de um pixel); a página lê o formato antigo também. No ensaio,
    512.557 seções ficam no seu local de votação; as demais são do exterior ou de locais sem coordenada.
  - Exterior: os locais do exterior vêm sem coordenada na base; continua pelo nome da cidade. A tabela EXTERIOR ganhou
    as 20 grafias do TSE que faltavam (RAMALLAH, BANGKOK, KINGSTON-JAMAICA, ISLAMABADE, KUAITE, RIADE, …) e agora
    cobre as **186 cidades da base** (`--exigir-exterior` falha se faltar alguma).
  - Tamanho: 3,6 MB (era 199 KB sem os locais).
- **Estáticos em gzip** (`aquisicao/src/pagina.rs`, `difusao.rs`): nem o Caddy (sem `encode`) nem o serviço
  comprimiam — até o `app.js` ia cru. Agora, com `Accept-Encoding: gzip` (sem `q=0`), o `uv-servico` manda o corpo em
  gzip (calculado uma vez por processo, nível 6; a fonte `woff2`, já comprimida, não), com `Vary: Accept-Encoding`. O
  `mapa.json` trafega com **786 KB** e o `app.js` com ~50 KB. Teste novo `difusao::testes::estaticos_em_gzip` —
  **antes**: `assertion failed: /static/mapa.json sem gzip`; **depois**: ok (descomprime byte a byte no original; sem o
  cabeçalho e com `gzip;q=0`, corpo cru; `woff2` nunca).
- **B-061 — raio negativo nas explosões do mapa** (achado no ensaio a 300×): o quadro é desenhado com o horário do
  `requestAnimationFrame`, que pode ser anterior ao início de uma explosão criada nele; a fase negativa dava raio
  negativo e `IndexSizeError` no `arc`. A explosão que ainda não começou espera o quadro seguinte. Teste em
  `site/testes/mapa_pagina.mjs` ("B-061 explosão adiantada", Brasil e globo) — **antes**: 4 falhas (raio −22,05 e
  −12,86); **depois**: 32 ok, 0 falha.
- **M-018 — sem o "Anuncie Aqui"** (Luciano, 25/10, 13h59: "Pode remover o 'anuncie aqui'. Não vou mais querer
  mostrá-la"): sai o chip âmbar do M-011 (`index.html`, estilos) e o link `mailto:`; com o selo escondido (M-012) a grade
  do topo passa a `auto 1fr auto` (celular: `1fr auto`, a navegação na 2ª linha), com o tema no fim da linha.
  `site/testes/topo_anuncie.mjs` reescrito para o pedido — **antes** (dist da 0.9.27): "sem o chip", "nenhum Anuncie" e
  "nenhum mailto" falham a 1920, 1366, 390, 360 e 320 px nos dois temas; **depois**: tudo ok (o tema no fim da linha, sem
  sobrepor a marca nem a navegação, sem rolagem lateral; M-010 e M-012 continuam).
- **Todo cartão do painel se maximiza e se reorganiza** (`site/src/cartoes.js`): no canto de cada cartão, a alça
  (arrastar para outra posição ou para a outra coluna; no teclado, setas com a alça em foco) e o botão de maximizar
  (Esc ou o véu fecham). A ordem fica no navegador (`uv-cartoes`); `window.__uv.cartoes.restaurarLayout()` volta ao
  padrão. Tudo por `translate`/`scale` (a `transform` continua da inclinação tátil); a vaga segura o lugar do cartão.
  `docs/legal/PRIVACIDADE.md` (e `/privacidade`) citam as duas preferências novas.
- Testes: `site/testes/mapa_pagina.mjs` (novo, sem servidor: carga, fotografia, mensagens ao vivo, mensagem velha,
  outra época, exterior e cidade sem posição, selo do marco, palco fixo ao trocar de vista, preferência guardada,
  globo que gira, maximizar/restaurar com a mesma caixa, véu, ordem pelo teclado guardada depois de recarregar, layout
  padrão, CLS 0 depois da carga, nada fora da página a 390 e 1366 px nos dois temas, e o B-061): 32 ok, 0 falha.
  Rust: `mapa::testes` (faixas, lista
  e índices, mensagens, fotografia, histórico em várias mensagens) e `difusao::testes::mapa_das_secoes_ponta_a_ponta`
  (REST com ETag/304, fotografia, `op: mapa` com histórico e ao vivo, seção fora do catálogo, outra época, servidor sem
  mapa) e `estaticos_em_gzip`: sobre a 0.9.27: `cargo test --release -p uv-aquisicao` **113 ok** (com o teste do B-060
  da 0.9.27), `-p urna-velox` ok, `versao.sh verificar` ok. Página (sem servidor): `mapa_pagina` 32 ok, `aviso_versao`,
  `seletor_fonte`, `termos`, `promo_letreiro`, `topo_anuncie` (M-018), `modo_pagina`, `antes_17h`, `painel_celular`,
  `relogio`, `ws_recusado`, `ritmo_pagina`, `previsao_cartao` ok; `visual_pagina` só as 56 falhas do B-037 de antes.
- **B-062 — próxima eleição anunciada no letreiro antes do primeiro estado** (achado na suíte da 1.0.0): com a playlist
  chegando antes do `estado`, o letreiro anunciava "Próxima eleição AO VIVO" na madrugada da eleição, já com BUs do
  exterior (o item, à vista, só sai ao passar da borda esquerda). Sem `estado` ainda, não se anuncia nada. Teste em
  `site/testes/antes_17h.mjs` ("madrugada, estado depois da playlist") — **antes**: 3 falhas (1366, 390 e 360 px);
  **depois**: tudo certo, duas vezes seguidas.
- **Ensaios com o simulador do 1º turno** (`site/testes/ensaio_mapa.mjs`, novo: compara o cartão com a saúde do serviço
  a cada passo; `carga_pagina.mjs --mapa`, novo: carga com o cartão à vista; nenhuma requisição ao TSE):
  - ao vivo a 60×: o cartão carrega em ~0,5 s (`mapa.json` em gzip); com a apuração parada, **cartão = mapa do serviço
    = seções aplicadas** (413.949 = 413.949 = 413.949; no fim, 499.207 nos três);
  - recarregar a página no meio da noite: o cartão volta em 472 ms e segue;
  - reiniciar o `uv-servico` (retomada de 440.957 BUs do diário em 13,1 s): o cartão troca de época e relê a
    fotografia. Com o serviço fora por ~12 s, a página entra no **modo HTTP** (B-026: 3 falhas seguidas do WebSocket) e
    fica até ~2 min só com as consultas de 60 s antes de voltar ao WebSocket — comportamento de antes, não do mapa, mas
    vale para cada reinício de nó com o site aberto. Reinício curto (2 s): segue ao vivo sem modo HTTP;
  - mensagens `mapa`: ~10,6 bytes por seção (~5,5 MB por cliente na noite inteira, espalhados pelas horas);
  - 500× não existe no simulador (o controle aceita de 1 a 300×); ensaiado a 300×;
  - com a barra de replay (`--controle-replay`) a página recarrega sozinha quando a sessão do replay muda (de antes;
    só no simulador).
- Para depois (não bloqueia): o domínio de leitura `dados.urnavelox.com.br` (Caddy) só repassa `/api/v1/resultado/*` e
  `/api/v1/estado`; as leituras do mapa caem no 404 dele e a página repete no próprio domínio (funciona, uma ida a
  mais). Sem `--controle-replay`, a 60× o serviço publicou ~200 atualizações/s do `br-c0001` (com ele, ~10/s): efeito
  do simulador acelerado, mesmo código da 0.9.26.
- Nada mexe em totalização ou resultado (o mapa só lê as seções aplicadas): sem conferência contra os oficiais.

## [0.9.27] — 2026-10-25

- **`uv-servico`: a sondagem do exterior cobre todas as seções do nó (B-060; Luciano, 25/10: "já temos mais de 237 BUs recebidos no painel do TSE, só 199 publicados: deixamos algum BU do exterior de lado?").** Nós e o site concorrente tinham as mesmas 199 seções, município a município; o painel do TSE conta BUs ainda não publicados. Mas o `sondadas` do evento `sondagem` estava parado em 1.170 de 1.351 sondáveis: ~181 seções do exterior nunca consultadas em 48 min de serviço (faixa quente de 140 seções a cada 20 s + 971 frias, contra 8 seleções por rodada, metade delas do outro nó e descartadas sem requisição). Agora: só as candidatas da partição do nó disputam a rodada (`so_do_no`); 1/4 da rodada é cota das sondadas há mais tempo, nunca sondadas primeiro, em qualquer faixa (`escolher_sondas`); o reforço do exterior passa de 8 para 12 por rodada, todas requisições reais do nó (a fila do Brasil vazia e o 429 desligando o reforço continuam); o evento `sondagem` traz `candidatas` e `nunca`. Teste: `servico::testes::sondagem_cobre_todas_as_secoes_do_no` (antes: falha "B-060: a rodada devia disputar só as seções deste nó", `20` × `10`; depois: ok no CI; simulação de 7 min com 2.000 seções sem nenhuma fria sem consulta e a faixa quente sempre a menos de 60 s). Não mexe em totalização: sem conferência contra os oficiais. **Só testes unitários; não implantado** (testes completos e o merge ficam para a próxima sessão do Luciano).

## [0.9.26] — 2026-10-25

- **Página: aviso de versão nova para quem deixou a aba aberta (M-016; Luciano, 25/10: "quando troca de versão, o usuário tem que fazer um refresh? perde a conexão?").** Antes: ao trocar a versão do serviço o WebSocket cai por ~60 s e a página **reconecta sozinha** (espera crescente de ~1 s a 30 s, balanceada para o outro nó pelo Caddy; depois de 3 falhas, modo HTTP a cada ~60 s), sem perder dados (o instantâneo da reconexão restaura o resultado e o número nunca volta); mas **não percebia a versão nova**: quem deixou a aba aberta seguia com o JavaScript antigo até recarregar (o `index.html` é `no-cache` e cita `?v=<versão>`; os estáticos são `immutable`). Hoje isso não machuca (só o front mudou, o protocolo é o mesmo); se uma versão mudar o protocolo ou a API, a página antiga ficaria incompatível. Agora: a mensagem `estado` traz `versao`; quando ela é **maior** que a da página (`?v=` do `app.js`), aparece um aviso fixo, fora do fluxo (nada se move), acima da barra do feed: **"Nova versão disponível · Atualizar · ✕"**. "Atualizar" recarrega; ✕ dispensa aquela versão (uma ainda maior traz o aviso de volta). Só maior, nunca diferente: durante uma troca de nós um já está na versão nova e o outro na antiga, e a página pode cair no antigo. Versão ilegível (desenvolvimento) nunca avisa. Medição (POST vazio em `/api/v1/uso/`, como no M-015): `versao-aviso`, `versao-atualizar`, `versao-dispensar`. Teste novo `site/testes/aviso_versao.mjs` (sem servidor; 106 verificações de 320 a 1440 px: igual/menor/ilegível sem aviso, maior com aviso inteiro e sem corte, nada se mexe, CLS +0,0000, ✕ e versão maior, "Atualizar" recarrega, medição); `seletor_fonte.mjs` continua com 212 verificações ok. Só front (`dist/` refeito); não implantado: sobe junto com a próxima versão.

## [0.9.25] — 2026-10-25

- **Página: seletor de fonte dos números que o usuário percebe e entende (M-015; Luciano, 25/10: "o pessoal não está percebendo que tem um botão para selecionar a origem dos dados").** Só no front (`site/src/`, `site/dist/` refeito), pensando no celular e **sem nada pular**: a linha das fontes tem altura fixa (117 px empilhada até 1199 px; 79 px com o texto ao lado do controle de 1200 px para cima) e cada texto cabe numa linha, sem reticência, de 320 a 1440 px. (1) Rótulo **"De onde vêm os números"** com o botão ⓘ. (2) Nomes **Urna Velox · Comparar · TSE oficial** (o do meio era "Ambos"; a chave interna e o `?comparar=ambos` continuam). (3) Frase de uma linha que muda com o modo: "Boletins somados assim que o TSE os publica", "Lado a lado: veja quem está à frente", "Só o arquivo oficial do TSE (≈ 1 min)". (4) **TSE ainda sem totais**: em vez do "arquivo do TSE das 19:19 · há 10 h" com zeros (que parecia defeito), "O TSE ainda não começou a totalizar" e a frase "Sem totais no TSE ainda; só o Urna Velox". (5) O ⓘ abre a folha com a explicação das três fontes e das 17h e **um botão por fonte** (escolhe e fecha). (6) **Dica da primeira visita**: sombra no controle e a frase "Toque em Comparar para conferir com o TSE" por 6 s, uma vez por navegador (`localStorage`); só no modo padrão. (7) **Medição de uso** sem backend: `navigator.sendBeacon` de um POST vazio em `/api/v1/uso/fonte-uv|fonte-ambos|fonte-tse|fonte-info|fonte-dica` (o servidor responde 404; vale a linha no log de acesso do Caddy: `grep '"uri":"/api/v1/uso/' /var/log/caddy/urna-velox-acesso.log`), sem IP gravado além do que o Caddy já registra. Rótulo com contraste 6,4:1 no tema claro (era 3,2:1 com a cor mais fraca). Teste novo `site/testes/seletor_fonte.mjs` (sem servidor; 212 verificações: textos, estado sem totais, folha, dica, medição, nada se mexe ao trocar de modo, sem corte, sem rolagem horizontal, CLS depois do carregamento 0,0000, contraste no claro). **O CLS do carregamento nas telas largas (0,013 a 0,034, no topo e no rodapé) já existia na `main`** (medido com o mesmo teste: 0,0319 / 0,0139 / 0,0132); no celular é 0,0000. Outros testes do site: `antes_17h`, `modo_pagina`, `previsao_cartao`, `promo_letreiro`, `ritmo_pagina` e `topo_anuncie` passam; `visual_pagina` (56 falhas do B-037) e `playlist_pagina` (erro) falham **igual na `main`**. Sem mudança de código do motor, da coleta ou do servidor; a página é embutida no binário (o `dist/` vai no commit).

## [0.9.24] — 2026-10-25

- **`uv-servico`: município do exterior com BU é sondado inteiro a cada 20 s; janela do calendário de 30 min (B-059; Luciano, 25/10, 05h46: "estamos atrás do site concorrente").** Comparado município a município: Camberra, Sydney, Wellington e Seul iguais; **Nagoya 3 × 13**. Os BUs de Nagoya chegaram ~20 min antes do horário do 1º turno, fora da janela do B-057 (5 min), e ficaram a 300 s por seção. Agora, para **qualquer** município do exterior: (1) se já tem BU e ainda faltam seções, todas as dele são sondadas a cada 20 s, independente do calendário; (2) a janela "devida" abre 30 min antes do horário esperado. Sem mudar o teto (reforço de 8 req/s por nó, fila do Brasil vazia, 429 desliga o reforço). Teste: `servico::testes::exterior_municipio_quente_sondado_a_cada_20s` (antes: falha "B-059: município com BU não é sondado a cada 20 s", `(2, 300)` × `(0, 20)`; depois: ok no CI). Não mexe em totalização: sem conferência contra os oficiais. Implantação nos dois nós, um de cada vez.

## [0.9.23] — 2026-10-25

- **Registro do B-058** (Luciano, 25/10, 05h35): depois de um reinício do serviço a hora "Totalizado pelo Urna Velox às" fica zerada até um BU novo ser aplicado (resultados publicados com `ultimo_bu` e `totalizado_em` vazios, mesmo com 32 seções apuradas). Só o sintoma, em [docs/BUGS.md](docs/BUGS.md); a ordem de ataque é do Luciano. Sem mudança de código.

## [0.9.22] — 2026-10-25

- **`uv-servico`: exterior sondado pelo calendário do turno anterior, com reforço e contingência de 429 (B-057; Luciano, 25/10, 05h: site de terceiros pegava BUs do exterior antes).** A sondagem antecipada do B-056 pedia 2 req/s, cada seção a cada 300 s, na ordem do catálogo (volta de ~11 min). Agora o `cs.json` do 1º turno (`fontes/cs/zz-p<pleito-1>-cs.json`, ao lado do `candidatos.json`; sem ele, nada muda) diz quando cada seção costuma chegar: as "devidas" (de 5 min à frente a 2 h atrás) saem primeiro e a cada 20 s, as atrasadas (2–12 h) a cada 60 s, as futuras a cada 300 s. Reforço de 8 req/s por nó (cada órgão com o seu teto de 100 req/s) só na sondagem antecipada e com a fila de pedidos abaixo de 20: o exterior não toma requisições do Brasil às 17h. **Contingência:** depois de qualquer HTTP 429 o reforço acaba até reiniciar o serviço (volta a 2 req/s e 300 s) e sai o alerta `reforco_exterior_desligado`. No Brasil nada muda: com o `cs.json` listando as seções, elas são pedidas direto (a sondagem só cobre as que nunca ganham disponibilização). Teste: `servico::testes::exterior_sondado_pelo_calendario` (antes: falha "B-057: seção devida agora", `(2, 300)` × `(0, 20)`; depois: ok no CI). Não mexe em totalização: sem conferência contra os oficiais. Implantação nos nós um de cada vez, com o acordo do Luciano.

## [0.9.21] — 2026-10-25

- **`uv-servico`: o exterior (ZZ) é sondado antes do `cs.json` listar as seções (B-056; Luciano, 25/10, 03h: "no primeiro turno a zz já divulgava às 1h").** Em 25/10 o `aux.json` e o BU de Wellington estavam no ar desde 01:51 e o `zz-p003221-cs.json` ainda era o de 09/10 sem nenhuma disponibilização; o serviço não via nenhum BU do exterior. Agora, num domingo e enquanto o `cs.json` do ZZ (com `nsp`) não listar nenhuma seção, as seções do ZZ sem BU são sondadas já (até 2 req/s, cada seção a cada 300 s, primeiro as dos municípios com BU), sem esperar o primeiro BU; depois volta a regra de 16/10. UFs do Brasil não mudam. Teste: `servico::testes::exterior_sondado_antes_do_cs_json` (antes: falha "B-056: o exterior não é sondado antes do cs.json"; depois: ok). Suíte no CI do GitHub: urna-velox 28+4+1+7+4+6 e uv-aquisicao 105 ok. Não mexe em totalização: sem conferência contra os oficiais. Comentário do nome do BU em `tse.rs` corrigido. Implantação nos nós só com o acordo do Luciano.

## [0.9.20] — 2026-10-24

- **`--intervalo-cs` configurável pelo `no.env` e em 10 s na produção** (Luciano, 24/10, 22h22: "Baixe o
  --intervalo-cs do uv-servico de 30 s para 10 s em produção (pleito 3221), antes da 01h"). O
  `deploy/oci/servidor/uv-iniciar.sh` passa `--intervalo-cs "${INTERVALO_CS:-30}"` (padrão 30 s, o mesmo do binário);
  o `10_implantar.sh` grava `INTERVALO_CS` no `no.env` (do `nos.env`, padrão 30; exemplo no `nos.env.exemplo`) e o
  `30_verificar.sh` mostra os `--intervalo-cs`, `--intervalo-cs-ocioso`, `--rps` e `--tse-intervalo` da linha de
  comando de cada nó. Nos dois nós, `INTERVALO_CS=10` no `/etc/urna-velox/no.env`. **Sem mudança** no
  `--intervalo-cs-ocioso` (1 s), no `--rps` (95) nem no `--tse-intervalo` (padrões do binário). Os 100 req/s por
  órgão continuam garantidos pelo código (o `--intervalo-cs` só encurta a espera entre ciclos de releitura com seções
  chegando; o teto de requisições é o mesmo). O painel não edita a chave (fica fora de `EDITAVEIS`, como antes).
- Teste: `deploy/oci/testes/test_uv_iniciar.sh` (novo; cópia do `uv-iniciar.sh` com os caminhos numa pasta temporária e
  um `uv-servico` falso que imprime os argumentos) — **antes**: 4 falhas de 8 (sem `--intervalo-cs` na linha de
  comando); **depois**: 8 ok (sem a chave → 30; `INTERVALO_CS=10` → 10, um só; vazia → 30; `EXTRA` continua por
  último; sem `--intervalo-cs-ocioso` nem `--tse-intervalo`; `--rps` do `no.env`).
- Implantação com os binários da 0.9.19 (`50_atualizar.sh --sem-compilar`: só os scripts mudaram; o
  `/opt/urna-velox/VERSION` dos nós segue 0.9.19).
- Nada mexe em totalização ou resultado: sem conferência contra os oficiais.

## [0.9.19] — 2026-10-24

- **Antes de começar:** o mesmo pedido tinha sido passado, sem querer, a outra IA. Ela deixou alterações **sem commit,
  sem build e sem implantar** em `site/src/estilo.css`, `site/src/feed.js` e `site/src/index.html` (a produção seguia
  na 0.9.18). Estavam incompletas: os termos iam para o fim da página, mas sem teste; e o "letreiro" só deixava o texto
  do item da próxima eleição vazar (`overflow: visible`), sem rodar e sem o aviso das 01h. Descartadas
  (`git checkout -- site/src`) e refeitas abaixo com testes; da ideia dela ficou a largura do item do letreiro
  recalculada quando o texto muda (`feed.js`), agora também empurrando os itens que vêm depois.
- **M-013 — termos de uso e privacidade no fim da página, centralizados** (Luciano, 24/10, 21h44: "Pode ficar sempre ao
  fundo, não precisa ficar sempre visível. Atrapalha a visualização. Deixe para ser visível só quando o usuário rolar até
  lá embaixo. E aí pode estar centralizado"). Já tinha subido para o celular na branch `merge/ws-autenticado` (M-009,
  0.9.13); aqui vale para todas as larguras. A linha sai do `position: fixed` (à direita, acima do letreiro) e vai para
  o fluxo, depois do painel, centralizada (`site/src/estilo.css`); o `padding-bottom` do body continua reservando o
  letreiro e o player, então, rolada até o fim, a linha fica acima deles.
- **M-014 — a próxima eleição ao vivo avisa dos BUs do exterior e roda como um letreiro quando não cabe** (Luciano,
  24/10, 21h44: "quero que o texto da próxima eleição fique rodando igual um letreiro se necessário, mas é que quero
  avisar que a partir da 01h da madruga já podem chegar BUs do exterior"):
  - chip da faixa: "· BUs do exterior a partir da 01h" até a 01h do dia da eleição e "· BUs do exterior já podem
    chegar" depois (até o primeiro BU, quando a noite passa a "ao vivo" e o chip dá lugar ao "Oficial às 17h"); só para
    eleições gerais (`textoExterior` em `site/src/app.js`);
  - o chip encolhe até o espaço que sobra na faixa (o BUs/min não encolhe; os avisos ainda ocultos cedem primeiro;
    mínimo de 116 px, com o selo AO VIVO sempre inteiro e parado); se o texto não cabe, ele roda dentro de uma janela,
    como um letreiro (cópia emendada atrás, ~40 px/s, só `transform`, pontas esmaecidas, algarismos de largura fixa para
    a contagem não mexer no passo); se cabe, fica parado; com "reduzir movimento" no sistema, não anda;
  - o mesmo aviso no item do letreiro do rodapé e no título do chip; o aviso que o chip abre ganha a linha "Exterior"
    (a partir da 01h o TSE já pode publicar os BUs do exterior, o Urna Velox totaliza cada um assim que sai, a oficial
    começa às 17h).
- Testes:
  - `site/testes/termos.mjs` reescrito para o pedido (com o conteúdo esticado para a página rolar: no alto, os termos
    fora da tela; rolada até o fim, à vista, centralizados na área útil, sem cobrir o letreiro nem o player) —
    **antes**: falha nos dois pontos novos a 1366 px (y 674 de 768 no alto; centro em 1019, à direita); **depois**: 0
    falhas (114 ok). O centro é o da área útil: o `html` tem `scrollbar-gutter: stable`, então a 1366 px o centro é 676.
  - `site/testes/promo_letreiro.mjs` (novo; 1920, 1366, 1024, 390, 360 e 320 px, antes e depois da 01h, e com "reduzir
    movimento") — **antes**, no `dist` da 0.9.18: 48 falhas (sem o aviso no chip, no letreiro e no aviso aberto, e sem
    janela de letreiro); no meio do caminho, 6 falhas no celular (o chip de pausa, invisível mas com 260 px guardados,
    deixou de ceder espaço e o chip da eleição ficou com 13 px), corrigidas com os avisos ocultos cedendo primeiro e o
    mínimo de 116 px; **depois**: 0 falhas — parado a 1920 e 1366, rodando a 1024, 390, 360 e 320.
  - Suíte da página (sem servidor): `topo_anuncie`, `modo_pagina`, `antes_17h`, `painel_celular`, `relogio`,
    `ws_recusado`, `ritmo_pagina`, `previsao_cartao` verdes; `visual_pagina` com as mesmas 56 falhas do B-037 de
    antes, nenhuma nova.
- Nada mexe em totalização ou resultado: sem conferência contra os oficiais.

## [0.9.18] — 2026-10-24

- **M-011 — "Anuncie Aqui" ocupa todo o espaço horizontal que sobra no topo** (Luciano, 24/10, 20h59). Com o selo
  escondido (`.topo:has(> #conexao[hidden])`), a grade do topo passa a `auto minmax(0, max-content) minmax(140px, 1fr)
  auto` no computador (a navegação fica do tamanho do conteúdo e o chip estica até o botão do tema) e `auto
  minmax(0, 1fr) auto` no celular (da marca ao tema; a navegação continua na 2ª linha). Com o selo de volta, vale a
  grade de sempre. Medido no teste: 1.638 px a 1920, 1.084 px a 1366, 177 px a 390, 147 px a 360 e 128 px a 320, com
  só o espaçamento da grade (14 px; 10 px a 320) dos dois lados.
- **M-012 — o aviso "Próxima eleição AO VIVO" sem o link "▶ Playlist"** enquanto o selo estiver escondido (Luciano,
  24/10, 21h00: "SIM"): vale para o aviso que abre sozinho na primeira visita do dia, para o toque na contagem da faixa
  e para o item do letreiro (`detalheAviso` em `site/src/app.js` olha o `hidden` do `#conexao`: tirar o `hidden` traz o
  selo e o link de volta juntos).
- Testes: `site/testes/topo_anuncie.mjs` ganha "ocupa o espaço que sobra no topo" (folga até 16 px dos dois lados; o
  fim da navegação medido pelo conteúdo) e o aviso sem a playlist (sozinho na primeira visita e pelo toque na contagem,
  1366 e 390 px) — **antes**, no `dist` da 0.9.17: 14 falhas (10 do espaço — o chip com 99 a 135 px e até 1.517 px de
  folga — e 4 da playlist); **depois**: 0. `modo_pagina`: com o selo escondido, confere que o aviso **não** leva à
  playlist. Suíte da página (sem servidor): `modo_pagina`, `antes_17h`, `termos`, `painel_celular`, `relogio`,
  `ws_recusado`, `ritmo_pagina`, `previsao_cartao` verdes; `visual_pagina` com as mesmas 56 falhas do B-037 de antes,
  nenhuma nova (as 12 larguras de 360 a 1920 px cobrem o topo novo).
- Nada mexe em totalização ou resultado: sem conferência contra os oficiais.

## [0.9.17] — 2026-10-24

- **Numeração:** feita na `main` (0.9.7) e numerada 0.9.17 de propósito (`versao.sh definir`), porque a branch
  `merge/ws-autenticado` já usou da 0.9.8 à 0.9.16 (o VIP que não foi para a produção e os registros de operação de
  24/10). Prioridade da noite, nas palavras do Luciano (24/10, 20h24): "O objetivo agora é priorizar a totalização".
- **M-010 — aviso "destinação dos votos presumida 'Válido'" fora da página** (Luciano, 24/10, 20h24: "vamos remover,
  temporariamente esse chip. Coloca como melhoria a gente pensar numa forma de apresentar ele e deixar poucas
  dúvidas"). Só a página (`site/src/app.js`): o alerta `destinacao_presumida` continua no `/api/v1/estado`, no
  `alertas.csv` e no `30_verificar.sh`; os outros avisos da faixa ("seções esperadas incoerentes", "BU reenviado ou
  seção anulada") continuam. A forma de apresentá-lo fica registrada como melhoria em `docs/BUGS.md`.
- **M-011 — chip "Anuncie Aqui" no alto da página**, em âmbar (o mesmo do selo REPLAY), na coluna que era do selo, com
  link só para `mailto:urna.velox@proton.me` (`site/src/index.html`, `site/src/estilo.css`).
- **M-012 — selo "EM BREVE" / Playlist escondido** até o fim da totalização do 2º turno (atributo `hidden` no
  `#conexao`; o `app.js` continua atualizando o selo, só não aparece). Para voltar: tirar o `hidden`.
- **Cache do `dados.urnavelox.com.br` de volta a 30 s** (Luciano, 24/10, 20h30: "podemos voltar as configurações do
  cloudflare para 30 s de cache"): `header_down Cache-Control "public, max-age=30, s-maxage=30"` no
  `Caddyfile.template` (era 120/150 desde a 0.8.48); a Cache Rule `dados-api` da Cloudflare respeita a origem, então
  nada muda lá. Implantado direto no `/etc/caddy/Caddyfile` dos dois nós (só essa linha, sem regerar o resto: o Caddy
  instalado ainda tem as rotas do VIP da 0.9.12), B e depois A, com cópia em `/etc/caddy/Caddyfile.20261024-cache150`.
  O Caddy dos nós roda sem a API de admin: `systemctl reload caddy` falha ("connection refused" na 2019) e deixa o
  processo com a config antiga; só o `restart` aplica (como o `instalar_caddy.sh`). Conferido: `caddy validate` ok,
  Caddy ativo, `https://urnavelox.com.br/api/v1/estado` 200 em cada nó e a Cloudflare devolvendo
  `public, max-age=30, s-maxage=30`.
- Testes: `site/testes/topo_anuncie.mjs` (novo; 1920, 1366, 390, 360 e 320 px, temas claro e escuro: chip à vista,
  inteiro, na primeira linha do topo, entre a marca e o tema, âmbar, link só com o e-mail; selo escondido; sem o aviso
  da destinação; o aviso de seção anulada continua; sem rolagem horizontal) — **antes**, no `dist` da 0.9.7: falha
  (`#anuncie` não existe); **depois**: 0 falhas. `deploy/oci/testes/test_caddyfile.py`
  (`test_dados_so_le_resultado_e_estado_com_cache_de_30s`): falha antes (o template tinha 120/150), passa depois;
  `deploy/oci/testes` 53 ok. Suíte da página (sem servidor): \modo_pagina\ (com as conferências do selo à vista e do toque no selo puladas enquanto o \#conexao\ tiver \hidden\, M-012; antes do ajuste, 12 falhas só nelas), \ntes_17h\, \	ermos\, \painel_celular\, \
elogio\, \hodometro\, \ws_recusado\, \
itmo_pagina\, \previsao_cartao\, \preposicoes\ verdes; \isual_pagina\ com as mesmas 56 falhas do B-037 (pílula dos termos) que a 0.9.7 já tinha, nenhuma nova (\identificacao\ e \playlist_pagina\ pedem servidor real e não rodaram).
- Nada mexe em totalização ou resultado (só a página e o cache da borda): sem conferência contra os oficiais.

## [0.9.7] — 2026-10-24

- **B-055: eleitorado do 3221 desatualizado no `candidatos.json`** (visto pelo Luciano em 24/10, 13h17: "eleitores que
  faltam" 157.563.461 no Urna Velox × eleitorado apto 158.745.502 no TSE, Brasil, Presidente). O TSE publicou os
  `-u.json` pré-apuração do 2º turno aos poucos (o TO entrou só em 15/10, `te` 1.182.041), e o `uv-coletor preparar
  --atualizar` só baixava de novo o `ele-c.json` e os `cs.json`: o `candidatos.json` de 18/10 nos nós tinha os
  `-u.json` de 13/10. Também estavam desatualizados o eleitorado e as seções de 11 abrangências de Presidente por UF, a
  circunscrição de Governador do ES (2.538.949 × 2.987.603) e os candidatos da suplementar de TO 95338.
  - **Correção operacional** (antes do código): preparo regerado com os `-u.json` baixados de novo
    (`~/uvreal/2026-2t/preparo-20261024`, 74 requisições a ≤ 5 req/s, código 0, 517.179 seções), Brasil 158.745.502 e
    TO 1.182.041 conferidos com o TSE, arquivo enviado aos nós A e B (`/srv/urna-velox/preparo/candidatos.json`; o
    anterior em `candidatos.json.20261018`) e 3221 reiniciado com `63_ciclo.sh --reiniciar` (13h40). Pelo domínio,
    `/api/v1/resultado/br-c0001` com `"eleitorado":158745502`.
  - `preparo.rs`: `--atualizar` baixa de novo todas as fontes (`ele-c.json`, `cm.json`, `-u.json`, `cs.json`).
  - Alerta novo `eleitorado_divergente` (`tse_resultados.rs`, `servico.rs`): o `e.te` do `-u.json` que a comparação com
    o TSE já baixou é conferido com o eleitorado do `candidatos.json` do tópico; um alerta por tópico, sem requisição a
    mais.
  - Testes: `preparo::testes::atualizar_baixa_de_novo_os_u_json` (vermelho na 0.9.6: `left: (1000, 0)`, `right: (1700,
    700)`; verde na 0.9.7) e `tse_resultados::testes::eleitorado_divergente_alerta_uma_vez_por_topico`. Suíte do
    workspace (`cargo test --release --workspace`) toda verde (uv-aquisicao 104, motor 28 + integrações). Não rodados
    os `--ignored` nem as conferências com os oficiais: a mudança não mexe em totalização nem em resultado (catálogo de
    entrada e alerta). Binários dos nós continuam na 0.9.5; o alerta só vale depois de implantar a 0.9.7.

## [0.9.6] — 2026-10-24

- **0.9.5 implantada nos nós, um de cada vez** (pedido do Luciano, 24/10, com a vitrine do 3221 no ar e a playlist
  na suspensão da eleição oficial): `compilar_local_arm64.sh` e, nó a nó, a troca dos binários (o passo do
  `63_ciclo.sh --binarios`, só no nó da vez) seguida de `63_ciclo.sh --reiniciar` do mesmo nó: B às 13:17:51 e,
  conferido o B, A às 13:21:05 (retomada do 3221 em 0,2 s, 0 BUs, 0 perdidos, 0 órfãos; só o alerta já conhecido
  `destinacao_presumida`). Nos dois, `uv-ciclo-no`, `uv-ciclo-tel`, `uv-ciclo-vigia` e `caddy` ativos, sem reinício
  extra; no `uv-servico` local (8080) `/`, `/api/v1/estado` (`versao` 0.9.5), `/privacidade`, `/termos`, `/robots.txt`
  e `/sitemap.xml` com HTTP 200 e o link "Privacidade" na página; a playlist do nó A (8090) com 200. Pelo domínio,
  `urnavelox.com.br` `/`, `/privacidade` e `/termos` com 200 (o `/privacidade` dava 404 enquanto só o B estava na
  0.9.5) e `dados.urnavelox.com.br/api/v1/estado` com 200 e `versao` 0.9.5. Nada mexe em totalização.

## [0.9.5] — 2026-10-24

- **Merge do branch `claude/politica-privacidade` (0.9.4 a 0.9.6 do branch, saídos da 0.9.3) na `main`.** Entram a
  página de privacidade, acesso e retenção de dados (`/privacidade`), os termos de uso e a licença no próprio site
  (`/termos`), as duas geradas no `npm run build` a partir de `docs/legal/PRIVACIDADE.md`, `docs/legal/LICENCIAMENTO.md`,
  `LICENCA.pt-BR.md` e `LICENSE` (`site/documentos.mjs`, molde `site/src/documento.html`), o link "Privacidade" no
  rodapé junto da licença e as rotas `GET /privacidade` e `GET /termos` no `uv-servico`. Só a versão conflitou:
  `VERSION`, manifestos e lockfiles ficam com a da `main`; as entradas do branch ficam abaixo como histórico (a `main`
  já tem outra 0.9.4 e a 0.9.5 é esta). `site/dist/` regenerado com `npm run build`.
- Testes fumaça depois do merge (WSL do Luciano, 24/10, pedido dele: sem a suíte exaustiva): `cargo build --release
  --workspace` sem erro nem aviso; `cargo test --release -p uv-aquisicao pagina` 4/4 (`termos_e_privacidade_no_site`,
  `tabela_fixa` e os demais); `site/testes/termos.mjs` tudo certo. Não rodados: o resto da suíte, os `--ignored` e as
  conferências com os oficiais (o merge não mexe em totalização nem em resultado).

#### Histórico do branch `claude/politica-privacidade`

### 0.9.6 do branch `claude/politica-privacidade` (2026-10-24)

- **Termos de uso e licença no próprio site** (`/termos`), sem depender do GitHub para o visitante conhecer as regras.
  A página traz o `docs/legal/LICENCIAMENTO.md`, a tradução `LICENCA.pt-BR.md` e o texto oficial do `LICENSE` (com as linhas
  `Required Notice:` como estão nos arquivos). O link "Termos de uso" do rodapé e o `<link rel="license">` da página da
  apuração apontam para `/termos` (antes, para a cópia pública no GitHub).
- **Fonte única:** `/termos` e `/privacidade` são geradas no `npm run build` a partir dos arquivos do repositório
  (`site/documentos.mjs`, conversor de Markdown mínimo, e o molde `site/src/documento.html`); a `privacidade.html`
  escrita à mão saiu, e `privacidade.{css,js}` viraram `documento.{css,js}`. Links entre os documentos viram links
  internos (`/termos#licenca-pt`, âncoras da licença com prefixo); endereços externos ficam só como texto.
- `docs/legal/PRIVACIDADE.md`: contato de dúvidas e pedidos passa a ser o e-mail do titular (o mesmo da licença) em vez dos
  chamados no GitHub. Rota `GET /termos` no `uv-servico` (`aquisicao/src/difusao.rs`, `aquisicao/src/pagina.rs`);
  `/termos` no `sitemap.xml` e no `robots.txt`. Ajuste de quebra de linha nas páginas de texto (a URL da PolyForm
  alargava a página a 390 px).
- Testes: `pagina::testes::termos_e_privacidade_no_site` (substitui o `pagina_de_privacidade`: molde preenchido,
  nenhum link nem recurso externo, licença integral nas duas línguas com as âncoras internas, mesmo texto dos arquivos
  do repositório, sitemap e robots) e `tabela_fixa` (a página da apuração não cita mais o GitHub) ok, 4 testes de
  `pagina`; `site/testes/termos.mjs` 90 conferências, tudo certo; `/termos` e `/privacidade` sem rolagem horizontal a
  390 e 1366 px. Não mexe em totalização nem em resultado.

### 0.9.5 do branch `claude/politica-privacidade` (2026-10-24)

- **Privacidade no rodapé, discreta como a licença, e documento irmão no repositório.** O link "Privacidade" deixa de
  ser uma caixa própria e entra na mesma linha e na mesma caixa da licença (`© Urna Velox · … · Termos de uso ·
  Privacidade`; `.termos-caixa` em `site/src/estilo.css`, `site/src/index.html`). A política ganha o texto no
  repositório em `docs/legal/PRIVACIDADE.md`, ao lado do `docs/legal/LICENCIAMENTO.md` (que agora cita o irmão), citada no
  `README.md` junto da licença e na própria página `/privacidade`.
- Testes: `pagina::testes::pagina_de_privacidade` confere também que o `docs/legal/PRIVACIDADE.md` traz os mesmos
  compromissos, preferências e data da página (4 testes de `pagina` ok); `site/testes/termos.mjs` 90 conferências, tudo
  certo (desktop e celular, dois temas, com e sem replay). Não mexe em totalização nem em resultado.

### 0.9.4 do branch `claude/politica-privacidade` (2026-10-24)

- **Página de privacidade, acesso e retenção de dados** (`/privacidade`): a parte
  burocrática do site numa página só, servida pelo `uv-servico` (`site/src/privacidade.{html,css,js}`, embutidos em
  `aquisicao/src/pagina.rs`; rota em `aquisicao/src/difusao.rs`). Diz que o Urna Velox faz o mínimo: sem cadastro,
  cookies, anúncios ou rastreadores; não vende, não aluga e não compartilha informação pessoal dos visitantes; o que é
  registrado serve só para a experiência do usuário e para melhorar a plataforma. Tabela do que é registrado, para quê
  e por quanto tempo: preferências no `localStorage` (`uv-tema`, `uv-comparar`, `uv-ident`, `uv-aviso-dia`; nunca saem
  do navegador), log de acesso do Caddy (rotação por tamanho), voto da playlist (IP só como hash com sal, apagado quando
  o replay entra no ar ou o voto é retirado) e números agregados de operação. Política de acesso, compartilhamento
  (provedores de hospedagem e da CDN do subdomínio de dados só transportam o tráfego), direitos pela LGPD, segurança e
  mudanças. Link "Privacidade" ao lado dos termos de uso, na página da apuração; `/privacidade` no `sitemap.xml` e no
  `robots.txt`.
- Testes: `pagina::testes::pagina_de_privacidade` (novo: versão preenchida, estáticos na tabela, compromissos e
  preferências descritos, nada externo além dos links para a cópia pública, link na apuração, sitemap e robots) e os
  demais de `pagina` ok; `site/testes/termos.mjs` passa a conferir também o link "Privacidade" (90 conferências, tudo
  certo, Chromium do ambiente). Não mexe em totalização nem em resultado (sem conferência com os oficiais).

## [0.9.4] — 2026-10-24

- **0.9.3 implantada nos nós, um de cada vez** (pedido do Luciano, 23/10, durante o replay do 452 em tempo real):
  `compilar_local_arm64.sh` e, nó a nó, a troca dos binários (o passo do `63_ciclo.sh --binarios`) seguida de
  `63_ciclo.sh --reiniciar` do mesmo nó: B às 20:24 (de volta às 20:25:57; retomada de 471.791 BUs do diário em 96 s,
  0 perdidos, 0 órfãos) e, conferido o B (`uv-servico 0.9.3`, `/` e `/api/v1/estado` com HTTP 200, a página servida já
  com o "por volta de" e a regra única do relógio), A às 20:28:56 (retomada de 472.797 BUs em 56 s, 0 perdidos). Os dois
  terminaram o 452 com os 478.026 BUs e às 23:01 passaram, pela rotação, à vitrine do 2º turno (3221, espera). A
  conexão local com a API da OCI caiu durante o acompanhamento do reinício do A; o reinício não foi repetido.
- **Saúde dos nós em 24/10, 12h** (só leitura): `63_ciclo.sh --estado` com os nós sincronizados em
  `20261023-2300-3221-espera`; nos dois, `uv-servico 0.9.3`, `uv-ciclo-no`, `uv-ciclo-tel`, `uv-ciclo-vigia` e `caddy`
  ativos, sem reinício do serviço desde as 23:01, fila 0, RSS ~48 MB, carga ~0, disco livre 54 GB (A) e 77 GB (B); nas
  últimas 24 h só o alerta já conhecido `destinacao_presumida` (1, na partida do 3221). Pelo endereço público de cada
  nó, `/`, `/api/v1/estado` (`versao` 0.9.3), `/playlist`, `/api/v1/playlist` e `/static/app.js?v=0.9.3` com HTTP 200;
  `urnavelox.com.br` e `dados.urnavelox.com.br` com 200. A playlist está na suspensão da eleição oficial (24/10 00:00
  a 29/10 00:00): nenhum simulado novo, a vitrine do 3221 fica no ar; o vigia registra a cada 5 min "hora fechada
  atingida … consultando a playlist" e mantém a suspensão, como previsto. Nada mexe em totalização.

## [0.9.3] — 2026-10-23

- **Merge do branch `claude/bugs-previsao-ritmo` (0.8.48 e 0.8.49 do branch, saídos da 0.8.47) na `main`; bugs
  renumerados.** O branch registrou e corrigiu B-029, B-030 e B-031, números que na `main` já eram dos bugs visuais da
  revisão de 22/10; aqui viraram **B-052** (antigo B-029: previsão sempre "em horário ainda incerto"; `eta`, o horário
  pelo ritmo atual, em `motor/src/previsao.rs` e `aquisicao/src/previsao.rs`, e "por volta de hh:mm" em
  `site/src/previsao.js`), **B-053** (antigo B-030: chip de ritmo com 200–300 mil BUs/min; ritmo medido no relógio do
  serviço, `site/src/app.js`) e **B-054** (antigo B-031: relógio "00:00:0" cortado no cartão da previsão), em
  `docs/BUGS.md` (tabela, ordem de ataque e seções), nos comentários do código e dos testes, em `docs/arquitetura/PROTOCOLO_WS.md`,
  no `site/README.md` e nas capturas (`docs/bugs/B-052-previsao-{1,2}.jpg`). Os B-029 a B-031 da `main` não mudam. As
  entradas 0.8.48 e 0.8.49 do branch ficam abaixo como histórico (a `main` já tem outras 0.8.48 e 0.8.49), com os
  números novos. Versões e manifestos ficam com a da `main`; `site/README.md` junta os testes dos dois lados.
- **Cartão da previsão, uma regra só para o relógio "mais provável"** (`site/src/estilo.css`): o B-044 da `main`
  (largura do molde, `min-width: 88px`) e o antigo B-031 do branch (`.pv-hod.seg` com `width: 100px`) resolviam o
  mesmo "hh:mm:ss" passando da borda. Fica a largura do B-044; `.seg` só reduz a fonte para 21 px com segundos e
  `.vazio` esconde o relógio sem horário (B-054). `site/dist/` regenerado com `npm run build`.
- Testes depois do merge (WSL do Luciano, 23/10): `aquisicao/scripts/compilar_dev.sh` (build e `cargo test --release
  --workspace`) sem falha, com o `uv-aquisicao` em 101 testes (os 100 da `main` e o
  `previsao::testes::previsao_sempre_com_horario`); `--ignored` com dados reais `previsao_acompanha_servico_atrasado`
  (1.114 previsões de ac-c0003) e `horarios_do_sistema_no_simulado` ok. Site sem servidor, com o `dist/` regenerado:
  `previsao_cartao` (B-052/B-054) e `ritmo_pagina` (B-053) tudo ok; `visual_pagina` 1260 conferências, 0 falha (B-044
  com a regra única do relógio); `hodometro`, `relogio`, `termos`, `preposicoes`, `ws_recusado`, `modo_pagina`,
  `painel_celular`, `playlist_identidade`, `cores` e `cores_distancia` ok; `antes_17h` com 1 falha na primeira rodada
  (390 px, "o letreiro não anuncia a próxima eleição AO VIVO", rodando junto do `visual_pagina`) e tudo certo nas 3
  rodadas seguintes, sozinho. Com `uv-servico` e `uv-simulador` locais (replay do 3220 a 500×, `ensaio_carga_pagina.sh`;
  nada do TSE): `previsao_pagina` ok (153 mensagens `previsao`, 40 redesenhos, maior trecho parado 1 s; o cartão mostra
  "sai por volta de 19:12:37" e depois "entre 19:12:35 e 19:12:41") e `horarios_pagina` ok (CLS ≤ 0,006); evidências em
  `D:\dados\urna-velox\ensaios\20261023-1914-integra-{previsao,horarios}`. Não rodados: `curva_sem_salto_no_modo_rapido`
  (interrompido por tempo; a curva não muda aqui) e as conferências com os oficiais (o merge não mexe em totalização nem
  em resultado: só previsão, ritmo e página).

#### Histórico do branch (renumerado: B-052 = antigo B-029, B-053 = antigo B-030, B-054 = antigo B-031)

### 0.8.49 do branch `claude/bugs-previsao-ritmo` (2026-10-22)

- **Bugs B-053, B-052 e B-054 corrigidos** (ordem de 22/10 aprovada pelo Luciano; testes, saídas antes/depois e
  causas em `docs/BUGS.md`):
  - **B-053 — chip de ritmo com 200–300 mil BUs/min:** o ritmo era medido no relógio da página; depois de uma pausa
    sem mensagens (celular em segundo plano, WebSocket reconectando) o salto do `seq` era dividido por poucos segundos.
    Agora vem do `estado`: BUs aplicados e relógio do serviço da mesma mensagem, janela de 60 s no relógio do serviço,
    série refeita numa sessão nova do replay (`site/src/app.js`). Teste `site/testes/ritmo_pagina.mjs`: antes 152.234
    BUs/min depois de 90 s de pausa (serviço a 42.000) e o chip preso em 0 num replay que recomeça; depois 42.000.
  - **B-052 — previsão sempre "em horário ainda incerto":** toda faixa da previsão ganha o `eta`, o horário **pelo ritmo
    atual** (o que falta ÷ a velocidade média móvel, como o ETA de um navegador de trânsito), recalculado a cada
    rodada e presente sempre que há ritmo, também no modo rápido (`motor/src/previsao.rs`, mensagem `previsao` em
    `aquisicao/src/previsao.rs`, `docs/arquitetura/PROTOCOLO_WS.md` §4.7). O cartão mostra "por volta de 10:12:30" e esse horário
    no relógio "mais provável" quando a faixa P10–P90 é larga demais; com a faixa estreita, "entre … e …" como antes.
    Testes `aquisicao/src/previsao.rs::previsao_sempre_com_horario` e `site/testes/previsao_cartao.mjs`.
  - **B-054 — relógio "00:00:0" cortado:** sem horário nenhum o relógio "mais provável" some (nada de 00:00:00
    esmaecido); com segundos (simulado acelerado) ele cabe no cartão (21 px em 100 px). Teste
    `site/testes/previsao_cartao.mjs` (dígitos dentro da área útil do cartão a 390 e 1366 px).
- Testes: `cargo test --release --workspace` verde; da página, `ritmo_pagina`, `previsao_cartao`, `modo_pagina`,
  `ws_recusado`, `hodometro` e `relogio` ok; clippy sem aviso novo. **Não rodados** (sessão sem `UV_DADOS` nem
  servidor): os `--ignored` com dados reais e os testes da página que precisam do `uv-servico`. Não mexe em
  totalização nem em resultado (só a previsão e a página): sem conferência com os oficiais.

### 0.8.48 do branch `claude/bugs-previsao-ritmo` (2026-10-22)

- **Bugs registrados (só o sintoma, sem análise até a ordem de ataque):**
  - **B-052** (Luciano): no card "Previsão" (replay do 1º turno de 2026, celular), o horário do matematicamente e o
    do fim da apuração ficam constantemente "em horário ainda incerto". Pedido junto: sempre dar uma previsão, ainda
    que imprecisa, reajustada continuamente (como o ETA do Waze), inclusive no modo muito rápido. Capturas em
    `docs/bugs/B-052-previsao-{1,2}.jpg`.
  - **B-053** (Luciano): o chip de ritmo chega a 200–300 mil BUs por minuto, claramente irreal.
  - **B-054** (Claude, nas capturas do B-052): relógio "00:00:0" com "mais provável" ao lado de "em horário ainda
    incerto", cortado na borda direita do card no iPhone.
- Não mexe em código, totalização nem resultado.

## [0.9.2] — 2026-10-23

- **B-051 corrigido: o fim da apuração não depende mais do `s.ts` pré-apuração** (pedido do Luciano: automático, sem
  regerar o `candidatos.json`). O `s.ts` do 3221 nos nós está desatualizado (Brasil 494.864 e SP 33.403, contra 499.248
  e 103.904 no catálogo), e o fim usava esse número: a apuração podia encerrar cedo e reabrir a cada BU. Agora o esperado
  de cada abrangência é o total de seções do catálogo (listadas no `cs.json` menos as agregadas, o mesmo número da
  página), com o `s.ts` só como piso; as agregadas descobertas na noite (no `cs.json` lido durante a apuração ou
  sondadas sem `aux.json`) saem do total. Base: esse total é igual ao `s.ts` oficial final em todas as abrangências de
  2026 1º turno (83/83) e 2024 (51/51 e 5.574/5.574). O alerta `secoes_esperadas_incoerentes` fica só para `s.ts` acima
  do catálogo; `s.ts` abaixo vira o registro `sts_desatualizado`. Teste novo
  `servico::sts_desatualizado_nao_encerra_cedo` (2º turno de 2024, AM e RO, `s.ts` cortado para 30 %): antes,
  primeiro encerramento com 1.807 de 5.102 BUs, 10 reaberturas e parada com 5.086; depois, um encerramento com os
  5.102 e conferência 2/2. Detalhes em `docs/BUGS.md`; `docs/operacao/OPERACAO.md` (alertas) e `ARCHITECTURE.md` atualizados.
  Verificação no WSL do Luciano (mexe no fim da apuração; código idêntico ao do commit): `cargo test --release
  --workspace` ok; conferências do motor contra os oficiais (`--ignored`) 14/14; ensaios do serviço (`--test servico
  --ignored`) 17/17, com o novo; federação 6/6. Replays completos a 300×, sem filtro de UF: 1º turno de 2026
  (3220) com um encerramento só, nos 499.207 BUs (+ 41 não instaladas), sem reabertura, conferência 83/83; 1º turno de
  2024 (452, como no ciclo diário: recebimento no TSE e corte às 2h) com um encerramento nos 478.026 BUs, sem
  reabertura, 5.569/5.569 abrangências completas, conferência 5.542/5.574 com os votos idênticos e as mesmas 33
  situações de sempre (32 municípios com candidatos cassados, destinação presumida).

## [0.9.1] — 2026-10-23

- **0.9.0 implantada nos nós** (pedido do Luciano, 23/10, durante a vitrine do 3221, antes do 452 das 17h):
  `compilar_local_arm64.sh` e `63_ciclo.sh --binarios` nos dois; `--reiniciar B` (16:30) e, com o B de volta,
  `--reiniciar A` (16:31); `63_ciclo.sh --playlist` (playlist com a identidade da apuração, B-048; no A o Caddy recusou
  o *reload* e foi reiniciado). Conferido: os dois nós com o `uv-servico 0.9.0`, a página pública na 0.9.0, `/`,
  `/api/v1/estado`, `/playlist` e `/api/v1/playlist` com HTTP 200, nós sincronizados. Nada mexe em totalização.
- **Sobras do teste de estresse da borda (21/10) fora do clone:** `scripts/restaurar_cloudflare.ps1` (uso único, com o
  ID da regra e o caminho do cliente particular) e `work/` (corpos das chamadas à API) arquivados em
  `D:\dados\urna-velox\ensaios\2026-10-21-estresse-borda`. Conferido na Cloudflare (só GET): `security_level` em
  `medium` desde 22/10 15:16 e nenhuma regra de acesso por IP na zona. A entrada 0.8.48, que listava o script como
  adicionado, foi corrigida.
- Alerta já conhecido e anterior a esta versão, visto nos reinícios: `secoes_esperadas_incoerentes` no
  `candidatos.json` do 3221 (`s.ts` pré-apuração parcial em várias UFs); regerar o `candidatos.json` perto da eleição.

## [0.9.0] — 2026-10-23

- **Etapa 9 (ensaio geral) fechada como `0.9.0` por decisão do Luciano (23/10)**, depois da revisão do replay do 1º
  turno de 2026 a 1× com a 0.8.59 (bugs visuais B-029 a B-045, B-047 e B-048, e o piso das 17h nos replays B-049/B-050).
  O conteúdo é o da 0.8.59, sem mudança de código. O critério do ROADMAP ("dois ensaios completos seguidos sem
  intervenção") não foi medido nesta versão, e continuam abertos os itens ⬜ da etapa 9 (critério A da federação,
  falhas provocadas, runbook, cobertura, atraso disponibilização → BU aplicado). Os testes `--ignored` com dados reais
  (conferência com os oficiais) não rodaram sobre esta versão; nada mexe em totalização desde a última conferência.

## [0.8.59] — 2026-10-23

- **Merge do `main` (0.8.48, cache da borda em 120–150 s e 7.500 clientes/nó) no branch `claude/bugs-revisao-visual`.**
  As duas linhas usavam a 0.8.48: as entradas do branch foram renumeradas para 0.8.49–0.8.58 (aqui e nas referências
  de `docs/BUGS.md` e `docs/operacao/OPERACAO.md`); a 0.8.48 do `main` fica como está. Código sem conflito (só versões e
  CHANGELOG). Testes depois do merge: `cargo test --release -p urna-velox` ok (50; 14 `--ignored` não rodados, sem
  `~/uvreal` nesta máquina); `cargo test -p uv-aquisicao --lib` 100 ok (inclui `simulador::testes::piso_das_17h` e
  `pagina`); `npm run build` com `site/dist/` idêntico ao commitado; site sem servidor: `hodometro`, `relogio`,
  `termos`, `preposicoes`, `ws_recusado`, `modo_pagina`, `painel_celular`, `playlist_identidade`, `antes_17h` tudo
  certo e `visual_pagina` 1260 conferências, 0 falha; `deploy/oci/testes` 53 ok (3 pulados). Pendentes, no WSL do
  Luciano: os `--ignored` (conferência com os oficiais) e o replay curto do 3220 com o piso das 17h.

## [0.8.58] — 2026-10-23

- **Replays: os BUs que saíram antes das 17h entram juntos às 17h** (B-049; decisão do Luciano: a madrugada em tempo
  real seria monótona). `uv-simulador --piso` (padrão `17:00:00` no dia da eleição; `""` desliga) e
  `simulador::com_piso`: seção publicada (`cs.json`) ou com horário antes do piso entra no piso. No 3220, os 781 BUs do
  exterior publicados antes das 17h entram às 17:00:00 (antes, às 17:08, pelo recebimento agrupado do `aux.json`).
  Teste novo `simulador::testes::piso_das_17h` (vermelho antes, verde depois); `cargo test -p uv-aquisicao --lib` ok.
  A noite real não muda.

## [0.8.57] — 2026-10-23

- **Site: totalização antes das 17h explicada; B-050 corrigido** (decisão do Luciano: mostrar e explicar). No Brasil a
  totalização começa oficialmente às 17h, mas o Urna Velox totaliza cada BU assim que o TSE o publica, e os do
  exterior saem a partir de ~1h da madrugada de domingo (1º turno de 2026: o primeiro às 01:49). Enquanto a
  totalização está no ar antes das 17h do dia da eleição, a faixa mostra o chip "Oficial às 17h · por enquanto, só BUs
  do exterior" e o letreiro um item fixo; os dois abrem a explicação. A contagem "Próxima eleição AO VIVO · faltam"
  some com o selo AO VIVO (B-050). Teste novo `site/testes/antes_17h.mjs`: antes 23 falhas, depois tudo certo; suíte
  do site sem servidor ok (`visual_pagina` 1260 conferências). `docs/operacao/OPERACAO.md`: no dia da eleição, o serviço no ar
  antes da 1h.

## [0.8.56] — 2026-10-23

- **BUs do exterior antes das 17h (análise, sem mudança de código).** Conferido nos arquivos públicos do 1º turno de
  2026 (1 `cs.json` e 9 `aux.json` do exterior, em 23/10, fora de qualquer apuração): o TSE publicou 781 das 1.310
  seções do exterior antes das 17h de Brasília, a primeira às 01:49:24 (Wellington). Horários do `cs.json` em hora de
  Brasília; o recebimento do `aux.json` dessas seções vem agrupado às 17:08. Registrados B-049 (os replays usam o
  recebimento e nunca reproduziram a madrugada) e B-050 (selo "AO VIVO" e contagem "faltam 15h" juntos).

## [0.8.55] — 2026-10-22

- **B-048: a página `/playlist` com a identidade visual do Urna Velox** (pedido do Luciano). Teste novo
  `site/testes/playlist_identidade.mjs` (compara, no navegador, a playlist com a página da apuração: tema padrão,
  tokens do tema, fonte, aurora, marca, vidro, títulos, botão de tema; 1366, 390 e 360 px): **antes 100 falhas, depois
  tudo certo**. `uv-playlist.html` com os tokens e estilos do `site/src/estilo.css`: tema escuro por padrão como o app
  (antes seguia o do sistema), botão ◐, marca "Urna Velox" com "Velox" leve (antes em degradê), títulos na Archivo
  larga, cartões de vidro iguais. `test_playlist.py` (14) ok. Vale nos nós depois do `63_ciclo.sh`.

## [0.8.54] — 2026-10-22

- **Site: segundo lote de bugs visuais, com testes antes da correção** (`site/testes/visual_pagina.mjs` ampliado:
  antes 188 falhas, depois 1260 conferências ok, 0 falha; detalhes em `docs/BUGS.md`).
  - B-033: o ☰ do letreiro desenhado em SVG, centrado no botão. B-037: pílula "Termos de uso" opaca. B-038: pausado,
    o letreiro para antes do rótulo "pausado".
  - B-039: no celular o selo do "matematicamente" ocupa o lugar do subtítulo, sem a faixa vazia reservada embaixo.
  - B-040: com a barra de identificação presa no alto, a faixa acima dela (e a do relógio do iPhone) não mostra mais o
    conteúdo que rola por baixo.
  - B-041: "Seções apuradas" alinhado ao rótulo e "BUs/min" centrado no chip (o hodômetro desliza a linha inteira).
  - B-046 (faixa da previsão serrilhada) continua registrado: precisa da série real de previsões para reproduzir.

## [0.8.53] — 2026-10-22

- **Site: bugs visuais corrigidos, cada um com o teste que o comprova antes da correção** (ordem deixada com o Claude
  pelo Luciano em 22/10; detalhes, saídas antes/depois em `docs/BUGS.md`). Teste novo `site/testes/visual_pagina.mjs`
  (sem servidor real: 2 cenários, 12 larguras de 360 a 1920 px, dois temas, modos "Urna Velox" e "Ambos"; mede caixas
  no navegador): **antes 526 falhas, depois 1004 conferências ok, 0 falha**. E `site/testes/preposicoes.mjs` (B-042):
  antes 35 falhas, depois tudo certo.
  - B-029/B-030: "." de milhar, "," decimal e ":" das horas na linha de base dos dígitos (`.hod .vis` em flex).
  - B-031: o "+"/"−" da diferença colado no número (opção `prefixo` do hodômetro). B-032/B-036/B-043: no celular, em
    "Ambos", o nome na largura inteira em cima e diferença, Urna Velox e TSE em colunas próprias embaixo (o selo da
    posição não cobre mais os votos); a pílula "eleito às …" encurta para "eleito" quando a linha é estreita e o nome
    fica à vista.
  - B-045/B-034/B-035/B-044: cartões e textos dentro das margens (coluna da direita com `min-width: 0`; uma coluna só
    abaixo de 1200 px; linha das fontes em duas linhas de 1200 a 1365 px; caixa dos eleitores que faltam inteira no
    cartão a 360 px; horário "mais provável" da previsão com a largura do molde).
  - B-047 (novo, visto nas capturas): sem valor, o hodômetro dos eleitores que faltam mostrava zeros-fantasma fora da
    caixa.
  - B-042: "Governador na Bahia", "Primeira seção totalizada no Rio de Janeiro", "se decide no Brasil".
  - Suíte do site sem servidor ok (`hodometro`, `relogio`, `termos`, `ws_recusado`, `modo_pagina`, `painel_celular`);
    `cargo test -p uv-aquisicao --lib pagina` ok. Os ensaios com `uv-servico` e dados reais não rodaram nesta sessão.
    Nada mexe em totalização ou resultado.

## [0.8.52] — 2026-10-22

- **Registros (só o sintoma):** B-045 (cartões "Apuração" e "Previsão" passam da borda direita no celular)
  confirmado pelo Luciano, com captura em `docs/itens/img/B-045-cartoes-borda.jpg`; a mesma captura mostra o B-034 no
  iPhone (número dos eleitores que faltam cortado embaixo pela caixa).

## [0.8.51] — 2026-10-22

- **Registros (só o sintoma):** B-030 ganhou a captura do Luciano (":" das horas bem deslocado no iPhone,
  `docs/itens/img/B-030-dois-pontos-hora.jpg`). Na mesma captura, registrados B-044 (horário "mais provável" da previsão
  mostra "00:00:0", cortado, com a previsão incerta e a chegada de BUs parada), B-045 (cartões "Apuração" e
  "Previsão" passam da borda direita no celular) e B-046 (faixa da previsão serrilhada).

## [0.8.50] — 2026-10-22

- **B-043 registrado (só o sintoma):** na comparação "Ambos" no celular, a linha do candidato eleito mostra só a
  pílula "eleito às hh:mm:ss" e não o nome (visto pelo Luciano no iPhone, Prefeito de Bujari/AC, replay do 1º turno de
  2024; captura em `docs/itens/img/B-043-nome-some-eleito.jpg`).

## [0.8.49] — 2026-10-22

- **Revisão visual do site (só registro, sem mudança de código):** 14 bugs visuais registrados em `docs/BUGS.md`
  (B-029 a B-042), só com o sintoma, aguardando a ordem de ataque. B-029 a B-033 vistos pelo Luciano no iPhone
  ("." de milhar e ":" das horas mais baixos; na comparação "Ambos", "+" da diferença deslocado e selo da posição
  cobrindo um dígito dos votos; ícone ☰ pequeno e descentralizado; captura em `docs/itens/img/B-031-ambos-celular.jpg`).
  B-034 a B-042 vistos pelo Claude em capturas com dados falsos, de 360 a 1920 px, nos dois temas (cartão "Apuração"
  transbordando de 961 a ~1150 px e a 360 px, título truncado, "Termos de uso" e "pausado" por cima do conteúdo,
  espaço vazio no cabeçalho da corrida no celular, "Governador em Bahia", entre outros). Sem rolagem horizontal em
  nenhuma largura; CLS ≈ 0.

## [0.8.48] — 2026-10-22

- **Diferencial de cache da CDN (`dados.urnavelox.com.br`) ampliado para 120–150 s e implantado nos nós:**
  - `deploy/oci/servidor/Caddyfile.template` ajustado para `header_down Cache-Control "public, max-age=120, s-maxage=150"` (antes `30` s): a borda da Cloudflare (`s-maxage=150`) retém os JSONs de `/api/v1/resultado/*` e `/api/v1/estado` por 2,5 minutos e o navegador (`max-age=120`) por 2 minutos, estabelecendo defasagem de 2 a 3 minutos no modo HTTP em relação ao WebSocket em tempo real (~6 ms).
  - Implantado imediatamente nos nós A e B da OCI e conferido na borda da Cloudflare (`cf-cache-status: HIT`, `Cache-Control: public, max-age=120, s-maxage=150`).
  - `deploy/oci/servidor/instalar_caddy.sh`: `chown -R caddy:caddy /var/log/caddy` também após o `caddy validate` (que como `root` cria os arquivos de log com dono `root`, causa do B-027) antes do `systemctl restart caddy`.
- **Dimensionamento WebSocket e relatório de estresse de borda (`docs/relatorios/RELATORIO_ESTRESSE_E_CAPACIDADE_BORDA.md`):**
  - Limite padrão `--max-clientes` (`aquisicao/src/bin/uv-servico.rs` e `aquisicao/src/difusao.rs`) ajustado de `5000` para `7500` por nó (`15000` conexões WebSocket no cluster de 2 nós), calibrado após ensaio de saturação individual e combinada (16.000 conexões WebSocket na OCI com 0 falhas e p99 de 7,55 ms + 100.000 sessões virtuais na CDN Cloudflare com 100% de Cache HIT e p50 de 15,8 ms).
  - Scripts de ensaio (`scripts/teste_carga_cdn.mjs`, `scripts/teste_combinado.mjs`) e relatório consolidado adicionados (o `restaurar_cloudflare.ps1`, de uso único, ficou fora do repositório; ver 0.9.1).
- Testes: `deploy/oci/testes` 53 ok (`test_dados_so_le_resultado_e_estado_com_cache_de_120_150s`).

## [0.8.47] — 2026-10-21

- **Passo 3 do plano de borda, implantação (sem mudança de código):** subdomínio `dados.urnavelox.com.br` no ar
  atrás da Cloudflare. Zona `urnavelox.com.br` delegada à Cloudflare (Free; NS `paris`/`zod`; o DS do Registro.br saiu
  na troca, DNSSEC a religar depois do 2º turno); SSL/TLS Full (strict); Cache Rule `dados-api` (cache do JSON,
  respeitando o `Cache-Control` da origem); nuvem **laranja só nos dois registros `dados`**, domínio principal e
  `www` cinza (direto nos nós, como antes). Feito pelo token restrito à zona e pelo cliente auditado da skill
  `cloudflare` (home-env).
  - **Conferido:** nos dois nós (Run Command), `/api/v1/estado` e `/api/v1/resultado/<tópico>` 200 com
    `Cache-Control: public, max-age=30, s-maxage=30`, `/painel` e `/ws` 404, versão 0.8.46; pela borda,
    `Server: cloudflare` e o mesmo resultado MISS → HIT com o mesmo `ETag`.
  - **Incidente (B-027, registrado):** ao aplicar o Caddy com o site `dados` (15h18 no A, 15h20 no B), o Caddy não
    subiu nos dois nós (`/var/log/caddy/urna-velox-dados.log` com dono `root`): **site principal fora do ar ~1h20**,
    até 16h37/16h38, restabelecido à mão (`chown -R caddy:caddy /var/log/caddy`, `systemctl restart caddy`).
    Até a correção, não reaplicar o `02_configurar_borda.sh` sem conferir esse arquivo.
  - **B-028 (registrado):** com `dados` apontando para os dois IPs, nenhum nó obtinha o certificado (o desafio da
    Let's Encrypt caía no outro nó). Obtido renomeando por alguns minutos um registro de cada vez (16h38–16h45).
  - Observação para o passo 4 (nó canônico): direto nos nós, o `seq` (ETag) do mesmo tópico difere entre A e B, então
    o `If-None-Match` pode voltar 200 quando o Caddy alterna de nó.
  - `docs/operacao/CHECKPOINT_BORDA.md` e `docs/BUGS.md` atualizados.
- **Da sessão anterior, sem commit até agora (revisado):**
  - `docs/propostas/PROPOSTA_COMPARACAO_DIRETA_CDN_TSE.md` e item no `ROADMAP.md` (`1.x`): proposta do Luciano de 21/10 — com a
    comparação ligada numa eleição ao vivo, o navegador lê o `-u.json` direto de `resultados.tse.jus.br` (no
    carregamento e a cada 30 s, com `ETag`/304) em vez da inscrição `tse:<tópico>` no `uv-servico`; replay continua
    pelo servidor (`tse_simulado`). **Só documento, nada implementado.** Pendências citadas lá: CORS da CDN do TSE e
    CSP. Correção a fazer no documento: o arquivo da CSP é `aquisicao/src/pagina.rs` (não `aquisicao/src/servico/`).
- Testes: `deploy/oci/testes` 53 ok. Não mexe em totalização nem em
  resultado.

## [0.8.46] — 2026-10-20

- **Passo 3 do plano de borda, parte do código: subdomínio `dados` para o modo HTTP (B-026)** — ordem trocada pelo
  Luciano (20/10, 22h23): Cloudflare antes da CPU paga (passo 2), para medir antes de pagar. **Nada implantado**: o
  site `dados` só entra com `DOMINIO_DADOS` no `nos.env`, depois de o nome estar no DNS.
  - **Caddy:** `gerar_caddyfile.sh` ganhou o 6º argumento (o nome do subdomínio; vazio = sem o bloco, e o Caddy não
    tenta o certificado de um nome fora do DNS). O bloco `DADOS` do `Caddyfile.template` só atende `GET`/`HEAD` de
    `/api/v1/resultado/*` e `/api/v1/estado`, com `Cache-Control: public, max-age=30, s-maxage=30` (`header_down`,
    no lugar do `max-age=1`/`no-cache` da origem); o resto 404 em JSON; erro do serviço 503 em JSON com `no-store`
    (não fica no cache da borda) e `Access-Control-Allow-Origin: *`; log próprio (`urna-velox-dados.log`, para
    medir o que chega aos nós). Origens como hoje (o próprio nó e o par, `round_robin`): a ordem A, B fica para o
    passo 4 (nó canônico), que muda todas as rotas. `instalar_caddy.sh` (5º argumento), `02_configurar_borda.sh` e
    `63_ciclo.sh --playlist` passam `DOMINIO_DADOS`; `nos.env.exemplo` explica quando preencher.
  - **Página:** `<meta name="uv-dados" content="https://dados.urnavelox.com.br">`; enquanto o nome não existe, a
    consulta falha e a página usa o próprio domínio (com pausa de 10 min na base), como já testado na 0.8.44.
  - **CSP** (`aquisicao/src/pagina.rs`): `connect-src 'self' https://dados.urnavelox.com.br` (antes só `'self'`: o
    navegador bloquearia a base); o `tabela_fixa` aceita o meta.
  - **Testes que comprovam:** `deploy/oci/testes/test_caddyfile.py` `TestDados` (5 novos: sem o argumento nada muda;
    com ele, o site principal idêntico e o `dados` só com a leitura e o cache de 30 s, nos dois papéis; nomes
    inválidos e `dados` = domínio principal recusados; roteiros passam `DOMINIO_DADOS`; `caddy adapt` 2.11.7 com as
    duas origens e `GET`/`HEAD`). Antes: 4 falhas (sem rota para o `dados`, nome inválido aceito, sem o bloco,
    roteiros sem `DOMINIO_DADOS`); depois: `deploy/oci/testes` 53 ok. `pagina::testes::csp_libera_a_base_das_consultas`
    (novo): antes falhava ("a CSP não libera a base das consultas"); depois ok. `site/testes/ws_recusado.mjs` ganhou o
    cenário "base no ar" (segundo servidor em outra origem): resultado e estado lidos na base, 0 `OPTIONS`
    (sem preflight), nenhuma consulta ao próprio domínio — tudo certo. Suíte do site (`modo_pagina`, `relogio`,
    `hodometro`, `termos`, `painel_celular`) ok; `cargo test -p uv-aquisicao --lib` 99 ok. Não mexe em totalização
    nem resultado.
  - `docs/operacao/CHECKLIST_CLOUDFLARE.md` (novo): o que o Luciano faz na Cloudflare e no Registro.br, na ordem, e onde eu
    entro (implantação e conferências com o ok dele). `docs/operacao/CHECKPOINT_BORDA.md` atualizado.

## [0.8.45] — 2026-10-20

- **`?modo=http` na URL — conferir o modo HTTP (B-026) do celular** (pedido do Luciano, 20/10): com o parâmetro,
  o próprio navegador age como se o `/ws` fosse recusado (sem abrir conexão; a espera e a entrada no modo HTTP são as
  de sempre, ~3 s) e consulta o servidor de verdade nos intervalos de verdade (60 s; a nova tentativa do WebSocket
  também é "recusada", então a página fica no modo HTTP). Nada muda no servidor; afeta só quem usa o parâmetro. Ex.:
  `https://urnavelox.com.br/?modo=http#/br/c0001`. Teste: `site/testes/ws_recusado.mjs` ganhou o cenário (antes:
  4 falhas — a página abria o WebSocket; depois: tudo certo). `modo_pagina`, `termos` e `cargo test -p uv-aquisicao
  --lib` (98) ok. Não mexe em totalização nem resultado.
- `site/testes/modo_http_producao.mjs` (novo): a mesma conferência contra o site real por um Chromium do Playwright
  com o `/ws` bloqueado só nele; depois de `--liberar` s libera o WebSocket e confere a volta ao tempo real (uma
  consulta por minuto de um navegador só).
- **0.8.44 implantada nos nós** (ok do Luciano, 19h47): `63_ciclo.sh --binarios` nos dois; B reiniciado
  (`--reiniciar B`, retomou com 477.215 BUs). A 0.8.45 substitui a 0.8.44 antes do reinício do A.

## [0.8.44] — 2026-10-20

- **B-026 — acima do `--max-clientes`, a página não mostrava a apuração** (passo 1 do
  `docs/handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md`): com o `/ws` respondendo 503 "limite de clientes", o
  `site/src/conexao.js` só reconectava. Agora, depois de **3 falhas seguidas** do WebSocket (recusa ou queda; uma
  conexão que durou 30 s zera a conta), a página entra no **modo HTTP**: a cada **60 s (±20 %)** consulta só o
  resultado dos tópicos da tela (`GET /api/v1/resultado/<tópico>`, com `If-None-Match` → 304 sem corpo) e o estado
  da apuração (`GET /api/v1/estado`), entregues ao mesmo `receber` do WebSocket; um tópico que entra na tela é
  consultado na hora. **Nova tentativa do WebSocket a cada ~2 min (±20 %)**, silenciosa; quando abre, volta ao tempo
  real e as consultas param. Aviso discreto na faixa, no lugar do BUs/min: "atualizando a cada minuto (muitos
  acessos)"; o selo mantém o modo (AO VIVO/REPLAY) e o título dele explica o que fica parado (letreiro de eventos e
  marcos, "faltam", comparação com o TSE e previsão: só pelo WebSocket). Nenhuma rota nova no servidor.
  - **Base das consultas configurável**: `<meta name="uv-dados" content="">` no `index.html` (vazia = o próprio
    domínio; no passo 3, `https://dados.urnavelox.com.br`). Na base de outro domínio a página não põe cabeçalho
    próprio (o `If-None-Match` pediria um preflight CORS, que o site `dados` não atenderá): o navegador revalida sozinho
    com o ETag (`cache: 'no-cache'`). Erro de rede ou 5xx na base → o próprio domínio, e a base fica em pausa por 10 min.
  - **Teste que comprova:** `site/testes/ws_recusado.mjs` (novo; servidor `node:http` falso com o `/ws` recusando
    com 503, a 1366 e a 390 px, e a base fora do ar). Antes: **22 falhas** (a página em "conexão caiu · volta em Ns",
    nenhuma consulta). Depois: **tudo certo** (resultado na tela e atualizado; só `br-c0001` e o estado consultados;
    3–4 respostas 304; menor intervalo entre consultas ≥ 80 %; WebSocket tentado de novo a cada ~5 s no teste; volta
    ao tempo real e as consultas param).
  - **Suíte do site:** `ws_recusado`, `modo_pagina`, `relogio`, `hodometro`, `termos` e `painel_celular` ok;
    `cargo test -p uv-aquisicao --lib` 98 ok (a página é embutida no `uv-servico`; o `tabela_fixa` pegou um
    endereço externo num comentário do `index.html`, tirado). Não mexe em totalização nem resultado: sem conferência
    nova contra os oficiais.
  - **Para o passo 3:** a CSP da página (`connect-src 'self'`, `aquisicao/src/pagina.rs`) bloquearia as consultas a
    `dados.urnavelox.com.br` (a página cairia no próprio domínio, sem o cache da borda): incluir a origem ali e a
    exceção no `tabela_fixa` junto com o `<meta name="uv-dados">` preenchido. `PROTOCOLO_WS.md` §10 e
    `site/README.md` citam o modo HTTP e o teste novo.
  - **Implantar** (não feito; só com o ok do Luciano): binários (`63_ciclo.sh --binarios`: a página está embutida no
    `uv-servico`).
  - `docs/operacao/CHECKPOINT_BORDA.md` (novo): onde está o plano de borda, para retomar de outra sessão.

## [0.8.43] — 2026-10-20

- **Metas do 2º turno no plano de borda** (decisão do Luciano, 20/10, 19h20): 10.000 clientes WebSocket em tempo real
  (por ordem de chegada; autenticação depois do 2º turno) e 100.000 por HTTP com até 2–3 min de atraso.
  `docs/handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md` reescrito com as metas (§0) e o roteiro na ordem: (1) modo HTTP na
  página a cada 60 s (B-026), (2) 4 OCPUs nos dois nós (pago, um de cada vez, o B primeiro), (3) Cloudflare **só no
  subdomínio `dados.urnavelox.com.br`**, cache de 30 s, domínio principal sem mudança, (4) nó canônico, (5) teste de
  carga contra as metas e `max_clientes`, (6) congelamento na sexta e no sábado. `HANDOFF_CARGA_E_ARQUITETURA.md`
  §7.4 cita as metas. Só documentação; não mexe em totalização nem resultado.

## [0.8.42] — 2026-10-20

- **Teste de carga e revisão do plano para o 2º turno (20/10):**
  - `uv-carga` com TLS/WSS (feature `rustls-tls-webpki-roots` do `tokio-tungstenite`; `urls(end)` escolhe
    `http`/`ws` ou `https`/`wss`) e fim das conexões contando a rampa (`fim_em = t0 + duracao + tempo_rampa`);
    automação em `scripts/teste_carga.py` / `teste_carga.sh` (`--local` com o simulador a 60×, `--producao`).
    Feito em outra sessão; rodado contra produção (2 × `max_clientes 5000`: 10.543 conexões, 1.457 recusadas com 503).
  - `docs/handoffs/HANDOFF_CARGA_E_ARQUITETURA.md`: relatório do teste, revisto (§7). A "capacidade máxima" de 10 mil é o
    limite configurado, não uma medida; as latências de produção têm de 0 a 37 amostras; o limite local dos 12 mil era
    o `ulimit` do gerador; a pasta de evidências citada era de outra rodada (corrigida). Painel dos nós: ~130 KB por
    conexão (memória com folga) e CPU de 40–50 % dos 2 OCPUs com ~5.000 clientes por nó: **a CPU é o limite**.
  - `docs/handoffs/HANDOFF_MODELO_BORDA_E_CONSISTENCIA.md` reescrito em passos (um por vez): decisões do Luciano, consulta
    HTTP quando o WebSocket é recusado, teste de carga que mede a CPU, nó canônico (mesma ordem A, B nos dois Caddys;
    a versão anterior faria o B preferir a si mesmo), mais CPU e cache na borda (opcionais), congelamento na sexta.
  - **B-026 registrado** (`docs/BUGS.md`, só o sintoma): acima do `max_clientes`, a página não mostra a apuração (só
    tenta reconectar o WebSocket).
  - Não mexe em totalização nem resultado; o `uv-carga` é só o gerador de carga.

## [0.8.41] — 2026-10-20

- **B-025 — a porta 8080 (o `uv-servico` direto, sem o Caddy) abria de qualquer lugar da internet:** a Security List
  tinha `0.0.0.0/0 → 8080` (criada à mão) e, por ela "cobrir" a sub-rede, a regra da sub-rede nunca foi criada; os
  firewalls dos nós tinham uma regra antiga sem origem. `uv_oci.py fechar-portas` (novo; só regras exatas, regra
  ampla falha alto), `liberar-portas sub-rede` sem contar regra pública, `02_configurar_borda.sh` fecha para a
  internet 8080/8090/8095/7700/7701 depois de liberar a sub-rede, `instalar_caddy.sh` tira a regra antiga. Teste
  `deploy/oci/testes/test_liberar_portas.py` (5 novos): antes 2 falhas + 3 erros, depois 10 ok; `deploy/oci/testes`
  48 ok. **Aplicado em produção** na mesma ordem (sub-rede → tira a pública → firewall): de fora a 8080 não responde
  mais; site, playlist, painel, WebSocket e o balanceamento entre os nós conferidos pelos dois nós. Sem reinício; não
  mexe em totalização nem resultado.

## [0.8.40] — 2026-10-20

- **B-024 — a contagem "Próxima eleição AO VIVO" e o "▶" do selo sumiam para quem caía no nó B:** o domínio aponta
  para os dois nós e o Caddy do B mandava a playlist para `127.0.0.1:8090`, que só existe no A (503). Agora o
  `gerar_caddyfile.sh` recebe o papel da playlist: no B, `remoto:<IP público do A>` (proxy para o HTTPS do A); no A,
  `local:<IPs do B>` (`trusted_proxies`; `X-Real-IP` = `{client_ip}`, um voto por visitante). O `02_configurar_borda.sh`
  e o `63_ciclo.sh --playlist` passam o papel de cada nó. A página, sem a playlist, tenta de novo em 25–35 s (antes,
  15 min). Testes que comprovam: `deploy/oci/testes/test_caddyfile.py` (4 novos) e `site/testes/modo_pagina.mjs`
  (playlist com 503 na partida); antes 3 falhas + 1, depois tudo certo; `deploy/oci/testes` 43 ok, `cargo test -p
  uv-aquisicao --lib` 98 ok. Não mexe em totalização nem resultado.

## [0.8.39] — 2026-10-20

(Feita em paralelo com a 0.8.38 do servidor espelho, 4faac0b; reaplicada em cima dela.)

- **M-004 — "replay" no lugar de "simulado", um selo só e a próxima eleição ao vivo sempre à vista** (pedido do
  Luciano, 20/10): os dados são reais, então o que passa fora da noite oficial é um **replay**, como a reprise de um
  jogo.
  - O selo do topo virou um botão que diz o modo — **REPLAY** em âmbar, **AO VIVO** em vermelho, **EM BREVE** antes
    de a apuração começar — e é também o acesso à playlist (abre o aviso com o link "▶ Playlist"); o chip "Dados
    reais do TSE" e o botão "▶ Playlist" da faixa saíram. O ponto do selo continua mostrando a conexão.
  - **Próxima eleição AO VIVO** com contagem regressiva, sempre à vista (também durante um replay): chip em destaque
    na faixa ("Próxima eleição AO VIVO · 2º turno · Eleições Gerais 2026 · domingo (25/10), 17h · faltam 4d 02h
    13min"; no celular "AO VIVO (25/10), 17h 4d 02h", com o BUs/min inteiro ao lado a 360 px — B-018) e item fixo no
    letreiro. Some quando a noite oficial está na tela. A data vem do campo novo `proxima_ao_vivo` do `uv-playlist`
    (17h de Brasília de cada `ELEICOES_OFICIAIS`, `UV_INICIO_APURACAO` troca a hora); com o `uv-playlist` antigo, da
    suspensão + 17h. A página busca a playlist na partida e a cada ~15 min.
  - Aviso na primeira entrada do dia (num replay ou com a próxima eleição anunciada; nunca com a noite oficial no
    ar), com a próxima eleição ao vivo no alto.
  - Textos para o público: página da apuração, `/playlist` ("Playlist de Replays das Eleições"), mensagens do
    `uv-playlist`, painel, README, `docs/operacao/CICLO_DIARIO.md` e glossário. Nomes internos (`uv-simulador`,
    `--simulacao`, campo `simulacao` do protocolo) ficam como estão.
- **B-022 — a vitrine do 2º turno (`3221-espera`) não mostrava a playlist e dizia "ao vivo"**: o acesso à playlist só
  existia com `simulacao.rotulo`, e o selo dizia "ao vivo" com o WebSocket conectado. Agora o selo diz EM BREVE e
  leva à playlist em qualquer modo. **B-023 — o aviso "uma vez por dia" contava o dia em UTC** e reabria depois das
  21h de Brasília; agora é o dia do visitante. Teste que comprova: `site/testes/modo_pagina.mjs` (novo, sem
  servidor: página de `dist/`, WebSocket e playlist falsos, a 1366, 390 e 360 px). Antes: 52 falhas (as do B-022 e do
  B-023 em `docs/BUGS.md`); depois: tudo certo. `deploy/oci/testes/test_playlist.py` ganhou `TestProximaAoVivo`.
- **M-005 — oficialização do endereço `urnavelox.com.br`:** o Caddy do nó A responde **301** (Moved Permanently) de
  tudo o que chegar por `urnavelox.duckdns.org` para o mesmo caminho no domínio oficial (inclusive `/robots.txt` e
  `/sitemap.xml`); o `index.html` tem `<link rel="canonical" href="https://urnavelox.com.br/">`. O Caddyfile de cada
  nó sai agora do `deploy/oci/servidor/gerar_caddyfile.sh` (o mesmo no `02_configurar_borda.sh` e no
  `63_ciclo.sh --playlist`): o bloco do endereço antigo só entra no A, para onde o DuckDNS aponta (`DOMINIO_ANTIGO` no
  `nos.env` troca; vazio desliga); o `instalar_caddy.sh` valida o arquivo novo antes de trocar. Teste novo
  `deploy/oci/testes/test_caddyfile.py` (A com o 301, B sem o bloco, nomes inválidos recusados; com `UV_CADDY`, o
  `caddy adapt` 2.11.7 confere o `status_code` 301 e o `Location` `https://urnavelox.com.br{http.request.uri}`);
  `pagina::testes::tabela_fixa` exige a tag canônica.
- **Testes:** workspace `cargo test --release` 148 ok (36 ignorados: os `--ignored` com dados reais, não necessários aqui); `deploy/oci/testes` ok (com o `caddy` 2.11.7); página: `modo_pagina`, `relogio`, `hodometro`, `termos` e `painel_celular` ok. Não mexe em totalização nem resultado: sem conferência nova contra os oficiais.
- **Implantar** (não feito nesta versão): binários (`63_ciclo.sh --binarios`, a página está embutida no
  `uv-servico`) e `63_ciclo.sh --playlist` (o `uv-playlist` com `proxima_ao_vivo` e o Caddy com o 301).
- **Selo "EM BREVE"** no lugar de "AGUARDANDO" (vitrine do 3221 e noite oficial antes das 17h): no celular
  (136 px) o "AGUARDANDO" aparecia cortado ("AGUAR…"); no celular o selo do modo também perde o espaçamento largo
  entre letras. `site/testes/modo_pagina.mjs` passou a exigir o nome do modo inteiro no selo (replay e espera, a
  1366, 390 e 360 px): antes "AGUAR…", depois tudo certo.

## [0.8.38] — 2026-10-20

- **Modo `interno` descrito como servidor espelho:** nos textos (`CLAUDE.md`, `docs/operacao/PAINEL.md`, ajuda do
  `uv-servico`, mensagens do coletor e do painel, rótulos do painel), o modo `ACESSO=interno` passa a ser "servidor
  espelho dos arquivos públicos, fora da borda pública"; os testes usam hosts `*.exemplo.org`. Nomes de opção e de
  chave (`--acesso interno`, `ACESSO`, `TETO_RPS_INTERNO`) e o comportamento não mudam.
- **Capturas de `docs/bugs/` sem metadados** (EXIF, perfil de cor e bloco do editor removidos).
- Os scripts de análise dos logs de acesso saem de `deploy/oci/`; o B-007 no `docs/BUGS.md` e no CHANGELOG fica sem
  os nomes deles.
- **Hook de pre-commit portátil:** a verificação de segurança roda pela versão PowerShell no Windows e pela versão
  Python fora dele (antes, sem `powershell.exe`, todo commit era recusado); sem nenhuma das duas, o commit é recusado.
- Verificação: `cargo test --release` 148 passam, 0 falham, 36 `--ignored`; `deploy/oci/testes` 33/33. Nenhuma
  mudança em totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.8.37] — 2026-10-20

- **Atualização global de links e SEO para `urnavelox.com.br`:**
  - Auto link adicionado no título principal do projeto no `README.md` (`# [⚡ Urna Velox](https://urnavelox.com.br)`).
  - Atualização dos arquivos canônicos de rastreamento e indexação (`robots.txt` e `sitemap.xml`) em `site/src/` e `site/dist/`.
  - Testes automatizados de validação de metadados em `aquisicao/src/pagina.rs` atualizados para o domínio canônico `urnavelox.com.br`.
  - Scripts operacionais e guias atualizados com o novo domínio (`deploy/oci/02_configurar_borda.sh`, `deploy/oci/63_ciclo.sh`, `docs/operacao/CICLO_DIARIO.md`, `docs/operacao/PAINEL.md`, `docs/operacao/GUIA_URL_E_BALANCEAMENTO.md`).
  - Implantação da versão atualizada nos nós da Oracle Cloud Infrastructure.

## [0.8.36] — 2026-10-20

- **Domínio próprio `urnavelox.com.br` implantado e ativo na OCI:**
  - Configuração do domínio próprio oficial registrado no Registro.br, substituindo o DuckDNS para eliminar restrições de redes corporativas.
  - Terminação TLS/HTTPS automática com certificado Let's Encrypt obtido com sucesso pelo Caddy nos nós da OCI.
  - Suporte a `urnavelox.com.br` e `www.urnavelox.com.br` no `deploy/oci/servidor/Caddyfile.template`.
  - Documentação e referências de ambiente atualizadas no `README.md`, `deploy/oci/README.md` e `deploy/oci/nos.env.exemplo`.
  - Ajuste no script de verificação de pendências para permitir o e-mail de autor git.

## [0.8.35] — 2026-10-19

- **B-019 — o painel mostrava só um nó** ("painel fora do alcance", `error sending request for url
  (http://10.0.1.11:8095/…)`): a porta 8095 nunca foi aberta na Security List da sub-rede. O `02_configurar_borda.sh`
  procurava a lista pelo nome com a OCI CLI e, sem achar, seguia em silêncio. Agora `uv_oci.py liberar-portas` chega à
  lista pelo caminho da instância (VNIC → sub-rede → Security List), só acrescenta as regras que faltam (nunca apaga) e
  falha alto; o `02` para com erro se não conseguir; o `80_painel.sh` confere no fim, de cada nó, o painel do par. O
  painel mostra a causa do erro (conexão recusada × tempo esgotado). Testes: `deploy/oci/testes/test_liberar_portas.py`
  (vermelho antes: `AttributeError … liberar_portas`, 5 erros; depois 5 ok) e
  `painel::testes::par_fora_do_ar_aparece_como_inalcancavel` (vermelho antes: "erro sem a causa: error sending request
  for url (http://127.0.0.1:9/…)"; depois ok).
- **B-020 — configuração larga no celular:** abaixo de 900 px a grade tinha coluna `1fr`, que cresce até o conteúdo
  mais largo (JSON da auditoria): a página ia a 1.236 px e o Safari reduzia tudo. Colunas `minmax(0, 1fr)`, células que
  quebram o texto, campos com 16 px no celular (o iPhone não aproxima ao tocar). Teste `site/testes/painel_celular.mjs`
  (360/390/430 px e desktop, dois temas, três abas): 8 falhas antes, tudo ok depois.
- **B-021 — aviso do ciclo de simulados** na configuração: com o `uv-ciclo-no` ativo e o `urna-velox` parado, a tela
  diz que a configuração mostrada é a do serviço normal, não a do simulado em andamento (mesmo teste).
- **Testes:** workspace inteiro ok (painel 32, lib 98), `deploy/oci/testes` 33 ok, ajudante 43 ok, termos ok. Não mexe
  em totalização nem resultado: sem conferência nova contra os oficiais.

## [0.8.34] — 2026-10-19

- **Painel: caminhos do site do TSE** (pedido do Luciano, 19/10) — a URL de resultados e os moldes dos caminhos
  (`{base}`, `{base_arquivos}`, `{amb}`, `{ciclo}`, `{pleito}`, `{uf}`, `{mun05}`, `{zona04}`, `{secao04}`, `{hash}`,
  `{nome}`…) ficam em `/etc/urna-velox/rotas_tse.json` (`uv-servico --rotas`); o painel **descobre** onde estão os BUs
  (no máximo 20 pedidos, 350 ms entre eles, para no primeiro 429) e permite buscar os arquivos de outro endereço
  (`base_arquivos`). O ajudante só instala um arquivo validado (`uv-painel validar-rotas`).
- **Borda × servidor espelho** (modo `interno`; regra do Luciano, 19/10; `CLAUDE.md` atualizado) — `ACESSO=interno` e `TETO_RPS_INTERNO`
  no `no.env` (0 = sem teto), pelo painel com `CONFIRMO`. O código recusa o modo interno para a borda pública
  (`resultados.tse.jus.br`, `cdn.tse.jus.br`): ali os 100 req/s valem sempre; a pausa de 10 min depois de um 429 vale
  nos dois modos; no interno a cota da federação não se aplica.
- **Controle protegido pelo login** — o Caddy manda todo `POST/PUT/PATCH/DELETE` em `/api/*` pelo `forward_auth` do
  painel (`/painel/auth/verificar`, CSRF + origem); sem sessão, 401. O voto da Playlist de Simulados continua público.
- **Termos de uso** — link discreto no rodapé do site para a licença no GitHub (teste `site/testes/termos.mjs`).
- **Identidade visual** — o painel usa a do Urna Velox (aurora, vidro, Archivo, cores das séries), modo escuro e claro.
- **Produção** — painel na porta 8095 (a 8090 é da playlist no nó A), lê a `saude.json` do ciclo diário
  (`/srv/urna-velox/ciclo/atual/saida`), mostra também `uv-ciclo-no`, `uv-playlist` e o atualizador; o ajudante recusa
  iniciar/reiniciar o `urna-velox` enquanto o `uv-ciclo-no` está ativo. Implantado nos dois nós da OCI com o cliente
  OAuth do Google.
- **Testes:** painel 32 (e2e com Google e site do TSE falsos), ajudante 43, `coletor`/`tse` (acesso, rotas), termos;
  Caddyfile validado com a playlist e o controle (voto sem login passa, replay sem login 401). Suíte do workspace:
  tudo passa. Não mexe em totalização nem resultado: sem conferência nova contra os oficiais.

## [0.8.33] — 2026-10-19

- **Painel dos nós (`uv-painel`, novo binário; `docs/operacao/PAINEL.md`)** — pedido do Luciano em 19/10: monitorar e ajustar
  os nós por uma página, com login do Google e acesso só para os e-mails autorizados (`UV_PAINEL_EMAILS`).
  - **Visão geral** (5 s), por nó: situação, BUs aplicados/s, seções apuradas, BUs baixados/s, req/s × teto, 429, fila,
    último BU, CPU/memória do serviço e da máquina, disco, clientes WebSocket; federação: BUs recebidos e publicados
    entre os nós por segundo, lotes copiados, nós ativos/fora, cota. Oito gráficos com os dois nós sobrepostos (15 min,
    1 h ou 2 h; histórico de 2 h na memória do painel). **Configuração:** campos do `no.env` (identidade do nó e `EXTRA`
    só leitura), iniciar/parar/reiniciar o `urna-velox`, travar/destravar o atualizador, `federacao.json` e auditoria.
    **Detalhe:** a `saude.json` inteira.
  - **Separado do `uv-servico`:** um painel por nó, que só lê arquivos (`saude.json`, `/proc`, `no.env`); o Caddy manda
    `/painel*` ao painel do próprio nó e ele busca o do par pela rede privada (porta 8090, só para o IP do par; pedidos
    assinados com HMAC-SHA256). O `urna-velox.service` não muda.
  - **Segurança:** OpenID Connect (code flow; `state` e `nonce` em cookie assinado; `id_token` conferido: emissor,
    audiência, validade, nonce, e-mail verificado); sessão em cookie assinado de 12 h, sem nada gravado no servidor;
    CSRF + origem em tudo o que muda algo; CSP sem inline. Mudanças como root só pelo `uv-painel-acao` (sudo para um
    programa só, com ações nomeadas); o `no.env` (lido pelo bash) passa por duas validações (no painel e no ajudante:
    chaves editáveis, caracteres seguros, nenhuma chave não editável entra ou sai, `bash -n`, cópia do anterior em
    `/etc/urna-velox/historico/`). Confirmação por frase: `CONFIRMO` (pleito, ciclo, ambiente, servidor, req/s,
    parar/reiniciar) e `CONFIRMO TSE` quando o servidor é o TSE; req/s acima de 100 é recusado.
  - **Implantação:** `deploy/oci/80_painel.sh` (usuário `uvpainel`, sudo restrito, porta 8090 para o par,
    `uv-painel.service`, rota `/painel` no Caddy pelo `02_configurar_borda.sh`, que ganha a regra 8090 na Security
    List); `compilar_local_arm64.sh` passa a empacotar o `uv-painel`; `nos.env.exemplo` com `PAINEL_*` e `URL_DOMINIO`.
    **Ainda não implantado** (falta o cliente OAuth do Google).
- **Testes:** 28 novos em `painel::` (HMAC com os vetores da RFC 4231; sessão, login, CSRF; `id_token`; `no.env` com
  tentativas de injeção; amostras e taxas; ponta a ponta com um Google falso e dois painéis que se enxergam — login
  aceito/recusado, `state` errado, sessão do A valendo no B, mudança no B pelo A com CSRF, origem, confirmação, teto
  de 100 req/s e injeção, rotas internas sem assinatura, par fora do ar) e `deploy/oci/testes/test_painel_acao.sh`
  (28 casos do ajudante, sem root). Suíte do workspace inteira: tudo passa. Página vista no Chromium (desktop claro e
  escuro, 390 px sem rolagem lateral). Não mexe em totalização nem resultado: sem conferência nova contra os oficiais.

## [0.8.32] — 2026-10-19

- **README:** seção "Simulados com dados reais e a Playlist de Simulados" (dados reais, o que se repete é a
  totalização, auditoria contra o oficial; os dois modos; a playlist com agenda, voto por IP e o mais votado como
  próximo; pausa nas eleições oficiais); item "Dois relógios" (horário do Urna Velox × horário do TSE, M-003); a
  comparação com o TSE explicada no mesmo relógio e com o ajuste de dia e velocidade nos simulados; o letreiro sem
  os itens que saíram na 0.8.11 (disputa mais apertada/folgada, comparecimento por UF — B-014) e com o chip de ritmo;
  `docs/operacao/CICLO_DIARIO.md` na lista da documentação. Só documentação.

## [0.8.31] — 2026-10-19

- **B-018: o chip de BUs/min voltou no celular.** A 0.8.29 o escondia (até 640 px) para caber o botão "▶ Playlist".
  Agora o chip da simulação mostra só "Dados reais" no celular (o resto no `title` e na explicação) e o ritmo fica,
  sem a largura fixa de 150 px: os três chips cabem inteiros, com margem, a 360 px.
- **Teste que comprova:** `site/testes/playlist_pagina.mjs` (faixa a 1366, 390 e 360 px). Antes: 2 falhas (ritmo
  oculto a 390 e 360 px). Depois: tudo certo. Só a página muda: sem conferência nova contra os oficiais.

## [0.8.30] — 2026-10-19

- **`63_ciclo.sh --playlist`: o Caddy não pegava a rota nova.** Na implantação de 19/10 (05:10) o `systemctl reload
  caddy` falhou nos dois nós: o Caddyfile tem `admin off`, que desliga a API usada pelo reload, e o Caddy seguiu com a
  configuração antiga (`/playlist` em 404; o site seguiu no ar). Agora, se o reload for recusado, o instalador
  reinicia o Caddy (~1 s). Corrigido em produção às 09:15 com `systemctl restart caddy` nos dois nós.
- **Implantado em 19/10:** playlist (serviço no nó A, rota no Caddy, vigias dos dois nós consultando-a; o B pela URL
  pública) e binários 0.8.29 (página com o aviso de dados reais e o link da playlist; valem a partir do simulado das
  10:00). Conferência feita de dentro do nó B, pela internet: `/playlist` 200, agenda 10:00 452 · 11:00 453 · 12:00
  3220 (rotação, sem votos), suspensão de 24 a 28/10.

## [0.8.29] — 2026-10-18

- **Ciclo: os dados do simulado são os reais, dito com clareza** (pedido do Luciano em 18/10: visitantes entravam e
  ficavam na dúvida). Página: o chip da faixa passa a ser um botão "Dados reais do TSE · retotalização · …" que abre a
  explicação (os BUs são os publicados pelo TSE; o que se repete é a totalização, do zero, na ordem de chegada à
  Justiça Eleitoral; no fim, a auditoria contra o oficial). A explicação abre sozinha na primeira visita do dia a um
  simulado. A folha de detalhes rola quando o texto é maior que a tela (no celular saía pelo alto). Rótulo do
  `uv-ciclo.sh`: eleição com a data em que os BUs foram coletados e o modo em palavras ("1º turno de 2024 (eleição de
  06/10/2024) · mesma ordem de chegada dos BUs, acelerado" / "… · em tempo real, no ritmo em que os BUs chegaram ao
  TSE").
- **Playlist de Simulados das Eleições** (`deploy/oci/servidor/uv-playlist.py` + `uv-playlist.html`, só no nó A,
  atrás do Caddy em `/playlist` e `/api/v1/playlist`): no ar agora, agenda prevista (início na hora fechada, previsão
  de fim, eleição e data da coleta, modo "tempo real" ou "mesma ordem, acelerado") e as seis opções votáveis. Um voto
  por IP (IPv6 por /64; IP guardado só como hash com sal), trocável; o mais votado é o próximo (empate: quem chegou
  antes à contagem); os votos do escolhido são consumidos; sem votos, a rotação de sempre. Link "▶ Playlist" na faixa
  da página.
- **Vigia:** na virada da hora, pergunta à playlist o próximo (`POST /api/v1/playlist/decisao`, com segredo comum); a
  decisão é tomada uma vez por execução e os dois nós recebem a mesma (o B pela URL pública). Sem playlist por ~2 min,
  segue a rotação calculada localmente (como antes).
- **Suspensão** (pedido do Luciano em 18/10): de um dia antes até três dias depois de cada eleição oficial
  (`ELEICOES_OFICIAIS`, padrão 2026-10-25: 24 a 28/10), a playlist não aceita votos, não decide nada e a página diz
  quando volta; o vigia não dispara nenhum simulado nesse período.
- **Instalação:** `./63_ciclo.sh --playlist [domínio]` (segredo em `gerado/playlist.segredo`, fora do git; não para a
  apuração no ar; o vigia é reiniciado e retoma a decisão salva). Os binários com a página nova entram com
  `./compilar_local_arm64.sh && ./63_ciclo.sh --binarios` (valem no próximo simulado).
- **Testes:** `deploy/oci/testes/test_playlist.py` (11: voto por IP e troca, /64, recusas, mais votado e consumo,
  decisão única com voto chegando entre A e B, empate, rotação, duração medida, agenda em horas fechadas, janela de
  suspensão, HTTP com segredo e sem JSON) e `test_vigia_playlist.py` (3: o vigia real sobe o mais votado, segue a
  rotação sem playlist, não sobe nada na suspensão) — 28/28 da pasta. Ponta a ponta local com o Caddy do template:
  voto pelo `X-Real-IP` do Caddy (cabeçalho forjado ignorado), decisão sem segredo 403. Página:
  `site/testes/playlist_pagina.mjs` (23 conferências, desktop claro e celular escuro). `cargo test --release
  --workspace` sem falhas. Totalização e resultado não mudam (só a página e scripts do ciclo): sem conferência nova
  contra os oficiais.

## [0.8.28] — 2026-10-18

- **M-003: os horários do Urna Velox são os do relógio do sistema, separados dos do TSE** (pedido do Luciano em
  18/10). Antes, o "horário do último BU", a curva, a previsão e o feed usavam o relógio do TSE (no simulado, a noite
  da eleição original). Agora:
  - publicação: `totalizado_em` (relógio do sistema, ISO com ms) logo depois do `ultimo_bu`, que continua sendo o do
    TSE; eventos e marcos ganham `sistema`; o `estado` traz `relogio_sistema` — a âncora entre a noite do TSE e o
    sistema (`aquisicao/src/relogio.rs`): um instante `t` do TSE equivale a `sistema0 + (t − noite0) / acelerar`, o
    ajuste de data (e, no modo rápido, de velocidade) para comparar as duas apurações num simulado de outro dia;
  - previsão: a conta corre na escala da noite pelo **relógio do sistema** (não mais o do site) e todos os horários
    saem no relógio do sistema (`relogio: "sistema"`); `previsoes.csv` continua na escala da noite;
  - curva (`GET /api/v1/curva`): eixo no relógio do sistema, um ponto por minuto da noite (0,12 s a 500×);
  - página (`site/src/relogio.js`): "Totalizado pelo Urna Velox às …" com "último BU no TSE: … de dd/mm" embaixo;
    selo, pílulas e feed no relógio do sistema (o horário do TSE no detalhe); "TSE confirmou" e o cabeçalho da
    comparação medem a distância no relógio do sistema, com o TSE deslocado pela âncora; horários com segundos no
    simulado acelerado.
- **Teste que comprova:** `horarios_do_sistema_no_simulado` (`aquisicao/tests/servico.rs`, `--ignored`). Antes: 5
  falhas (`totalizado_em` ausente, sem âncora, 136/136 pontos da curva e a previsão em 04/10/2026, 6/6 eventos sem
  horário do sistema). Depois: ok (811 pontos da curva no relógio do sistema, maior passo 0,4 p.p.). Página:
  `node site/testes/relogio.mjs` (unidade) e `site/testes/horarios_pagina.mjs` (tela a 1366 e 390 px, CLS < 0,01).
- **Conferência com os oficiais:** 2026 83/83, 2024 5.574/5.574 e 51/51, situações e "matematicamente" idênticos
  (motor `--ignored`); votos e situações não mudam (só campos novos). Serviço `--ignored` 16/16 (o
  `kill9_tres_vezes` passou a descartar o `totalizado_em` na comparação com a execução sem queda, como já fazia com o
  `ultimo_bu`).

## [0.8.27] — 2026-10-18

- **B-001 medido e fechado como resolvido pelo B-011** (só documentação). Com o simulador acompanhando os nós
  (0.8.13+) e a medida de ordem sem os BUs tardios (B-002), o atraso da ordem no modo rápido do 3220 caiu de
  4,3/2,8 min da noite na mediana (0.8.10) para 6/3 s (0.8.17); o p99 restante (8–28 min da noite) é 1–3,4 s de tempo
  real a 500×, a cadência da troca de lotes. Nenhuma mudança de código. Detalhes em `docs/BUGS.md`.

## [0.8.26] — 2026-10-18

- **B-006 corrigido: os contadores da federação atravessam a retomada.** Gravados no armazém local do nó
  (`estado/<nó>/contadores.json`, não servido aos pares) a cada lote e a cada ~1 s, e recuperados na partida; o
  `lotes_gravados` vem do `bus/<nó>/ultimo`. Antes, depois de um reinício, `publicados` e companhia recomeçavam do zero.
- **Teste que comprova:** `federacao::no::contadores_sobrevivem_a_retomada` (antes: `(0, 0)` em vez de `(700, 2)`;
  depois: ok) e `tests/federacao.rs::kill9_e_reinicio` com as duas verificações novas (ok).

## [0.8.25] — 2026-10-18

- **B-003 corrigido: reinício intencional não reprova mais o `RESUMO.json`.** O `63_ciclo.sh --reiniciar` anota a hora
  em `atual/reinicios_intencionais` (guardado nas evidências pelo `arquivar`); o `uv-ciclo-resumo.py` desconta a troca
  de pid vista até 60 s depois da anotação e registra a observação. Queda sem anotação continua reprovando.
- **B-002 corrigido: a medida de ordem do `RESUMO.json` não é mais dominada pelos BUs recebidos dias depois.** Os que
  entram no corte (`disponivel` antes do `recebido`) ficam fora da medida e são contados em `recebidos_depois_do_corte`.
  Atraso p99 com as evidências de 18/10: 452 de 12 dias para 5,4 h; 3220 de 2,9 dias para 7,5 h.
- **Teste que comprova:** `deploy/oci/testes/test_ciclo_resumo.py`. Antes: 2 de 5 falhavam (p99 de 543.097 s com
  20 BUs tardios; reinício intencional reprovado). Depois: 5/5 (`deploy/oci/testes` 13/13).

## [0.8.24] — 2026-10-18

- **Ciclo: a virada automática não derruba mais os nós** (revisão da 0.8.22/0.8.23, [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md), problema 3):
  - `uv-ciclo-vigia.py` dispara o `uv-ciclo.sh avancar` com `systemd-run --collect` numa unidade própria e sai. Antes, o avancar era filho do vigia e morria no `systemctl stop uv-ciclo-vigia` do `parar_unidades` (`KillMode=control-group`): com o vigia como root (0.8.22), a próxima virada pararia tudo nos dois nós sem subir a eleição seguinte. Se o disparo falhar, nova tentativa a cada minuto.
  - ID da execução igual nos dois nós: `CICLO_CARIMBO` (hora do disparo, vinda do vigia, ou minuto do comando no `63_ciclo.sh --iniciar/--avancar`) no lugar do relógio de cada nó; as evidências caem no mesmo `ciclo/<ID>/` do bucket.
  - `rotacao.json` mostra a execução em curso desde o início; o vigia retoma a decisão salva (próximo pleito, modo, horário) se reiniciar, em vez de recalcular.
  - Modo padrão sem histórico unificado em `rapido` (vigia e `uv-ciclo.sh`).
  - `63_ciclo.sh --estado` compara os IDs das execuções e acusa nós dessincronizados.
  - Documentação corrigida: disparo até 10 s depois da hora fechada; o diagnóstico do problema 2 (só falta de root) era parcial. Nota: a 0.8.22 foi publicada dentro do commit `76814ac` ("0.8.21: M-002").

## [0.8.23] — 2026-10-18

- **Documentação completa do ciclo rotativo contínuo** ([docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md)):
  - Especificação da rotação contínua (`452` → `453` → `3220` → `3221` → `452...`) e disparos na hora fechada (`HH:00:00`) com margem de segurança de 3 min.
  - Alternância automática de modos por eleição (`rapido` 500× $\leftrightarrow$ `cadencia` 1×) e tratamento do 2º turno de 2026 (`3221`) em vitrine de espera (1h).
  - Arquitetura do orquestrador `uv-ciclo-vigia.py`, integração com `uv-ciclo.sh avancar` e persistência em `rotacao.json`.
  - Diagnóstico e documentação dos erros identificados em produção (permissão em `/srv/urna-velox/ciclo` e execução não-root do vigia) e as correções aplicadas.
  - Atualização dos comandos operacionais (`--scripts`, `--avancar`, `--estado`) no `deploy/oci/63_ciclo.sh`.

## [0.8.22] — 2026-10-18

- vigia executa como root com logging de avancar

## [0.8.21] — 2026-10-18

- **M-002 medida: vazão de ponta a ponta e opções de topologia** ([docs/relatorios/RELATORIO_M002_VAZAO.md](docs/relatorios/RELATORIO_M002_VAZAO.md)).
  Na noite real o limite é o teto de 100 req/s por IP: com os 2 nós, atraso máximo < 1 min no pico (1 nó: ~40 min);
  falta folga se um nó cair no pico. Nas simulações o limite é a CPU (~1.100 BU/s para o par). Nada mudou nos nós.
- Novos `aquisicao/scripts/medir_vazao.py` (fases, vazão, CPU, rede, federação e custos de uma execução arquivada do
  ciclo) e `aquisicao/scripts/fila_noite.py` (fila da noite real por número de nós e requisições por BU), com
  `aquisicao/scripts/testes/test_m002.py` (4 testes).

## [0.8.20] — 2026-10-18

- ajuste de permissao do ciclo e rotacao

## [0.8.19] — 2026-10-18

- ciclo rotativo continuo por hora fechada

## [0.8.18] — 2026-10-18

- **B-007 resolvido:** destino dos scripts de análise dos acessos decidido pelo Luciano.
- **B-015 corrigido: `02_configurar_borda.sh` instala de fato o Caddy.** O `em "$N" <<CMD` sem `"bash -s"` chegava
  vazio ao nó (Run Command) e o roteiro dizia "configurado"; agora passa `"bash -s"` e falha se a instalação falhar.
  O mesmo defeito estava num dos scripts do B-007.
- **Teste que comprova:** `deploy/oci/testes/test_scripts_em.py` (heredoc do `em` sem `"bash -s"`, `bash -n` e
  compilação dos `.py` de `deploy/oci`). Antes: 2 achados (`02_configurar_borda.sh:49` e o do B-007).
  Depois: `deploy/oci/testes` 8/8.

## [0.8.17] — 2026-10-18

- **B-005 corrigido: as simulações usam a destinação final dos candidatos.** O `candidatos.json` do acervo do ciclo
  diário vinha do `preparar --ignorar-destinacao` (todos "Válido"); no 1º turno de 2024, 33 situações divergiam do
  oficial (candidatos depois anulados). Novo `deploy/oci/candidatos_simulacao.sh`: gera os de 452, 453 e 3220 com o
  `dvt` dos `-u.json` finais (só do cache, sem requisição) e, com `--enviar`, troca-os no acervo dos nós (pendente do
  OK do Luciano). A noite real não muda (sem `dvt`, destinação presumida e alerta). `docs/operacao/CICLO_DIARIO.md` atualizado.
- **Teste que comprova:** `motor/tests/integracao.rs::primeiro_turno_2024_simulacao_destinacao_final`. Antes: 33
  situações divergentes. Depois: 5.574/5.574 arquivos e 15.042/15.042 situações idênticas.
- `docs/BUGS.md`: linha da M-001 marcada "feito · 0.8.16" (ficou de fora do commit anterior).

## [0.8.16] — 2026-10-18

- **M-001 feita: a eleição que está na tela fica sempre à vista.** Logo abaixo do topo, um cartão à moda do aplicativo
  da Justiça Eleitoral (variação C, escolhida pelo Luciano entre três protótipos): "Eleições 2026 · Gerais", o turno
  marcado (1º/2º) e, numa faixa colorida, o local e o cargo. Ao rolar, a mesma identificação fica presa no alto numa
  linha só (a marca Urna Velox rola junto com o topo). O botão ⌃ recolhe tudo até um botão pequeno, e a escolha fica
  lembrada no navegador. O título da aba passa a dizer cargo, local e turno. Nada pula: o lugar do cartão é reservado
  e a barra fixa entra por transform/opacidade (`site/src/identificacao.js`, `index.html`, `estilo.css`).
- **Teste que comprova:** `site/testes/identificacao.mjs` (via `ensaio_carga_pagina.sh`, 1366 e 390 px). Antes: não há
  cartão (erro); a primeira versão, com o cartão virando `fixed`, dava CLS 0,23 ao rolar. Depois: identificação
  completa, presa a 8 px do alto ao rolar, recolher lembrado após recarregar, CLS ≤ 0,008. `site/dist` regerado.

## [0.8.15] — 2026-10-18

- **B-004 corrigido: o `para` não acusa mais erro quando o arquivo chegou.** O Run Command da OCI às vezes termina
  "SUCCEEDED" com o texto da saída ainda vazio (o tar de 3,7 GB de 17/10); o `_oci_op` desistia na primeira consulta.
  Agora consulta de novo (até 10 vezes) e, se a saída não vier, o `para` confere o tamanho do destino no nó e dá 0 se
  bater com o local (com aviso). O Python do `_oci_op` saiu do heredoc do `lib.sh` para `deploy/oci/uv_oci.py` (mesma
  lógica, agora com teste).
- **Teste que comprova:** `deploy/oci/testes/test_uv_oci.py` (clientes da OCI falsos). Antes: 2 de 5 falhavam
  (`para` dava 1 com o arquivo inteiro no nó; texto atrasado virava erro). Depois: 5/5. Conferido contra a OCI de
  verdade no nó A (`em`, `para`, `em bash -s`, `de`: código 0). Saídas em [docs/BUGS.md](docs/BUGS.md).

## [0.8.14] — 2026-10-18

- **B-012 corrigido: candidatos da mesma disputa com cores bem distintas.** `site/src/cores.js` escolhia o primeiro tom
  alternativo a 0,075 (OKLab, só no tema escuro) dos já escolhidos; partidos da mesma família de cor (MDB e PRTB em
  São Paulo; PT, PSOL, PSTU, PCB…) ficavam a 0,05. Agora cada candidato tem 25 tons da própria cor (luminosidade
  ±0,07/±0,14, matiz ±12°/±24°) e a disputa é resolvida em rodadas, maximizando a menor distância nos dois temas até
  0,14 (com folga, fica o tom mais perto da cor do partido). A cor continua fixa durante a noite.
- **Teste que comprova:** `site/testes/cores_distancia.mjs` (todas as disputas dos `candidatos.json` de 2024 1º/2º
  turnos e 2026 1º turno; ≥ 0,12 até 8 candidatos, ≥ 0,075 nas maiores). Antes: 2.650 de 5.446 disputas abaixo,
  pior par 0,047. Depois: nenhuma; pior 0,080 (pior até 8 candidatos 0,122). `cores.mjs` segue sem cor repetida.
  Captura a 1366 px conferida. `site/dist` regerado.
- B-010 marcado como não reproduzido (motor e situações conferem com o oficial; o Luciano avisa se vir de novo).

## [0.8.13] — 2026-10-18

- **B-011 corrigido: a curva da apuração não pula mais no modo rápido.** Causa: a mesma raiz do B-016 — no modo rápido
  o relógio do simulador corria a 500× sem esperar os nós; a curva (eixo = horário do site) subia devagar a noite toda
  e os BUs que faltavam entravam todos quando o relógio parava no fim (o salto vertical das capturas). Novo
  `uv-simulador --acompanhar-servico <pct>`: o relógio não passa do instante em que as seções disponibilizadas e ainda
  sem BU servido seriam mais que essa % das seções com BU; anda de novo, na mesma aceleração, quando o serviço busca os
  BUs (sem BU novo por 10 s, anda solto). `uv-ciclo.sh` passa `--acompanhar-servico 1` no modo rápido (vale com
  `63_ciclo.sh --instalar`); `simulador_resumo.json` ganhou `acompanhar`. Cadência real e TSE de verdade: nada muda.
- **Teste que comprova:** `curva_sem_salto_no_modo_rapido` (`aquisicao/tests/servico.rs`, `--ignored`; AC do 1º turno
  de 2026 a 500×, simulador lento como os nós). Antes: salto de 51,5 p.p. num minuto (48,5 % → 100 % às 02:53).
  Depois: maior passo 4,3 p.p., curva até 20:59. Saídas em [docs/BUGS.md](docs/BUGS.md). Suíte verde. Não implantado.

## [0.8.12] — 2026-10-18

- **B-013 e B-016 corrigidos: a previsão acompanha a apuração até o fim também no modo rápido.** Causa (vista no nó A
  no simulado das 13h de 18/10): a thread `uv-previsao` anda pelo relógio do site; no modo rápido o relógio do
  simulador chega ao fim da noite antes de os nós ARM aplicarem os BUs, e a previsão parava — `previsoes.csv` parado
  com 21 % das seções e `"definidas": 0` enquanto o serviço chegava a 499.207 seções; o cartão e o gráfico não andavam
  mais. Agora `previsao::relogio`: com o relógio do site parado há mais de 3 s e BUs ainda entrando, o relógio da
  previsão anda sozinho na velocidade do site (na noite real o relógio do site anda sempre; nada muda).
- **Cartão mais claro (B-013):** "Matematicamente 2º turno **sai** entre 01:57 e 02:30" (ou "sai em horário ainda
  incerto"), "**Fim da apuração:** 99 % … · 100 % …", título "Previsão (estimativa de horário)".
- **Testes que comprovam:** `previsao_acompanha_servico_atrasado` (`aquisicao/tests/servico.rs`, `--ignored`: AC do 1º
  turno de 2026 a 500× com o simulador respondendo em 200–300 ms). Antes: a última previsão de `ac-c0003` com 48,8 %
  das seções. Depois: segue até 100 % ("definida"). Na página, `site/testes/previsao_pagina.mjs` (replay do 3220 a
  500×, Presidente): 34 redesenhos do gráfico, nenhum trecho parado, "saiu às 22:37" dentro da faixa. Teste de unidade
  `relogio_anda_quando_o_site_para_e_os_bus_entram`. `aquisicao/scripts/ensaio_carga_pagina.sh` ganhou `UV_PLEITO`
  (452 ou 3220) e `UV_TESTE_PAGINA`. Suíte verde. Saídas em [docs/BUGS.md](docs/BUGS.md). B-016 registrado (pedido do
  Luciano às 13:41). Não implantado nos nós.

## [0.8.11] — 2026-10-18

- **B-014 corrigido: a página recebe só o que a tela usa** (celular travando e esquentando no modo rápido). Medida
  antes (replay do 452 a 500×, página como celular em Prefeito de São Paulo): 61.326 mensagens e 16 MB em 92 s (pico
  de 11,5 MB/min), resultado de 191 tópicos, 13 % da thread principal em tarefas longas; o grosso eram 47 mil marcos
  `faltam_para_mat` de todas as circunscrições e 11 mil eventos de todos os prefeitos.
  - Página: resultado inteiro só da tela e da circunscrição dela (`atualizarTopicos`); ritmo da faixa também pelo
    `ultimo_bu.seq` do `estado`; saíram do letreiro "disputa mais apertada / mais folgada" e "comparecimento por UF"
    (precisavam de todos os tópicos; podem voltar como marcos do servidor).
  - Protocolo (compatível: sem os campos novos, nada muda): `op: marcos` com `faltam: [tópicos]` limita o
    `faltam_para_mat` aos tópicos pedidos (a página pede o da tela e o de Presidente); `eventos` e `marcos` com
    `escopo: "uf"` mandam, de fora dos tópicos inscritos, só os do Brasil e das UFs. `docs/arquitetura/PROTOCOLO_WS.md` §3.
- **Teste que comprova:** `aquisicao/scripts/ensaio_carga_pagina.sh` (+ `site/testes/carga_pagina.mjs`; tetos: 3
  tópicos de resultado, 1.500 KB/min, 25 % em tarefas longas). Antes: 191 tópicos e 11.500 KB/min, falhou. Depois: 1
  tópico, 466–536 mensagens e ~480 KB no replay inteiro, 4–5 % em tarefas longas. Capturas a 390 e 1366 px no fim do
  ensaio. Suíte verde (`-p urna-velox`, `-p uv-aquisicao --lib`, `difusao_d4_reduzido`, hodômetro). `site/dist`
  regerado. Saídas em [docs/BUGS.md](docs/BUGS.md). Não implantado nos nós (pedido do Luciano: simulado das 13h).

## [0.8.10] — 2026-10-18

- **B-009 corrigido: o contador de eleitores que faltam para o "matematicamente" não fica mais parado.** Causa: a
  conta do `faltam_para_mat` rodava na verificação de 1 s da thread do motor com orçamento de 0,5 ms por rodada para
  todas as circunscrições; no replay do 452 a 500× (o modo rápido do ciclo) Prefeito de São Paulo teve só 3 valores na
  noite inteira. Agora a conta roda na thread `uv-faltam` (`marcos::ContaFaltam`, prioridade baixa): a verificação só
  manda a fotografia (votos e restante) das circunscrições em disputa que receberam BU e recolhe as respostas da rodada
  anterior; resposta mais velha que o último valor mandado (o 0 do evento) é descartada. O protocolo não muda.
- **Teste que comprova:** `faltam_acompanha_a_apuracao` (`aquisicao/tests/servico.rs`, `--ignored`, SP do 1º turno de
  2024 a 120×). Antes: 3 valores para São Paulo antes do "matematicamente", parado por 29,7 % dos BUs. Depois: 52
  valores, maior trecho sem valor novo 3,9 % (limite 5 %). Saídas em [docs/BUGS.md](docs/BUGS.md). Suíte verde
  (`-p urna-velox`, `-p uv-aquisicao --lib`, `difusao_d4_reduzido`, hodômetro); votos e situação não mudam (só o
  marco `faltam_para_mat`).
- `docs/BUGS.md`: B-015 registrado (o `02_configurar_borda.sh` manda o comando do Caddy vazio ao nó).

## [0.8.9] — 2026-10-18

- **B-008 corrigido: a página não vira mais download quando o serviço está fora do ar** (troca de eleição do ciclo,
  partida da noite). Causa: das 07:00:02 às 07:02:25 de 18/10 os dois nós ficaram sem `uv-servico` (o `uv-ciclo.sh`
  para o serviço, espera ~1 min e sobe o novo) e o Caddy respondia `502`/`503` vazio, sem `Content-Type` e com
  `X-Content-Type-Options: nosniff` — o navegador oferecia salvar o arquivo. Agora o `Caddyfile.template` tem
  `handle_errors 502 503 504`: página HTML curta ("O serviço está sendo atualizado e volta em instantes", recarrega a
  cada 10 s) para a página e JSON (`servico_indisponivel`) para `/api/*` e `/ws`, ambos `503` com `Retry-After: 10`.
- **Teste que comprova:** `deploy/oci/testes/pagina_indisponivel.sh` (Caddy v2.11.7, a versão dos nós, com o
  `Caddyfile.template` real; `site/testes/indisponivel.mjs` abre a página no Chromium e no Firefox do Playwright).
  Antes: 3 de 4 casos falhavam (`503` sem `Content-Type`; API sem JSON). Depois: 4/4. Saídas em
  [docs/BUGS.md](docs/BUGS.md). Suíte: `cargo test --release -p urna-velox` e `-p uv-aquisicao --lib` verdes (nada
  de totalização ou resultado mudou). Nos nós, o Caddyfile novo entra pelo `02_configurar_borda.sh` (reinicia o Caddy).

## [0.8.8] — 2026-10-18

- **Lista de bugs e melhorias fechada e ordenada** ([docs/BUGS.md](docs/BUGS.md)): além dos 7 da `0.8.7`, os vistos
  pelo Luciano no ciclo de 18/10 — B-008 (página vira download na troca das 07h), B-009 (eleitores que faltam para o
  "matematicamente" travado em SP), B-010 (sempre 2º turno no 1º turno: regra dos 50 % dos válidos para presidente,
  governador e prefeito e do limiar de eleitorado dos municípios), B-011 (curva da apuração pula), B-012 (cores
  parecidas na curva), B-013 (previsão não apareceu / pouco clara), B-014 (celular trava; a página assina ~190 tópicos
  e recebe resultados de toda a totalização) — e as melhorias M-001 (identificação permanente da eleição, abrangência
  e turno no topo) e M-002 (medir a vazão de ponta a ponta e avaliar uma topologia que escale). Capturas em
  `docs/bugs/`.
- **Handoff com a ordem de ataque ponderada** ([docs/handoffs/HANDOFF_BUGS_2026-10-18.md](docs/handoffs/HANDOFF_BUGS_2026-10-18.md)):
  peso = 3·gravidade + 2·público + 2·apontado pelo Luciano + esforço + 2·relevância para o 2º turno de 25/10; ordem
  B-008, B-009, B-014, B-013, B-011, B-010, B-012, M-001, B-004, B-005, M-002, B-003, B-002, B-006, B-001, B-007; por
  item, por onde começar e o teste que comprova; como implantar sem quebrar o ciclo diário; prompt para a sessão que
  vai executar. Prazo: o que muda a página ou o serviço no ar até 22/10.

## [0.8.7] — 2026-10-18

- **Regra nova no `CLAUDE.md` (Luciano, 18/10/2026): bug se corrige com o teste primeiro.** Todo bug entra em
  [docs/BUGS.md](docs/BUGS.md) só com o sintoma; a ordem de ataque é do Luciano; para corrigir, primeiro o caso de teste
  automatizado que comprova o bug (rodado e visto falhando, saída registrada), só depois a correção, com o teste e a
  suíte verdes no commit.
- **`docs/BUGS.md` criado** com os 7 itens vistos no ciclo diário (B-001 a B-007), estado "registrado", sem análise.

## [0.8.6] — 2026-10-18

- **`63_ciclo.sh --binarios` e `--reiniciar [A|B]`**: trocar só os executáveis nos dois nós sem parar nada, e
  reiniciar o `uv-servico` de um nó retomando de onde parou (o simulador não é reiniciado). Roteiro de bug no meio da
  totalização em [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md).
- **Primeiro horário do ciclo (18/10, 07h, 1º turno de 2024, rápido)** disparou sozinho nos dois nós; o 3221 da noite
  foi arquivado. **Retomada provada no meio da apuração:** `--reiniciar B` às 07:06 com 265.838 BUs aplicados →
  retomada de 265.838 BUs do diário em 9,2 s (0 perdidos, 0 órfãos); os dois nós encerraram com 478.026 BUs,
  conferência igual à de ontem (5.542/5.574, nenhuma divergência de votos; 33 situações por destinação presumida),
  0 duplicados, 0 HTTP 429.
- `urna-velox.service` **desabilitado** nos dois nós durante o ciclo (com o `no.env` atual, um reinício do nó voltaria a
  coletar o 3220 do TSE e brigaria com o ciclo); o roteiro de 24/10 inclui religá-lo. Lista dos roteiros de implantação
  que não devem ser usados durante o ciclo em `docs/operacao/CICLO_DIARIO.md`.

## [0.8.5] — 2026-10-18

- **Ciclo diário provado na OCI com as quatro eleições** (binário `0.8.4` nos dois nós; tabela em
  [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md)): 2º turno de 2024 rápido e em cadência (com a aceleração do fim),
  51/51 e 102/102; 1º turno de 2024 rápido em 7,8 min, 478.026 BUs, **nenhuma divergência de votos** (5.542/5.574
  arquivos idênticos: 33 situações diferentes por destinação presumida de candidatos depois indeferidos/cassados e 9
  oficiais não finalizados); 1º turno de 2026 rápido em 9,8 min, 83/83 e 832/832; nos dois nós, 0 revogações,
  0 duplicados, 0 HTTP 429. O site ficou no 3221 (espera) até o primeiro horário, 07h de 18/10.
- `uv-ciclo-resumo.py`: o veredito separa divergência de votos/totais (falha) de situação divergente pela destinação
  presumida e de oficiais não finalizados (observações); o resumo mostra o atraso de recebimento p50/p99 em segundos da
  noite.
- `uv-ciclo.sh`: o 3221 sem coleta roda com `--aceitar-incompleto` (só exibição; o alerta `candidatos_incompleto`
  continua). O `candidatos.json` do 3221 nos dois nós (`/srv/urna-velox/preparo/`, o anterior guardado como
  `candidatos.json.20261016`) foi **regerado em 18/10 00:11**: completo, 28 UFs, 517.179 seções (o de 16/10 não tinha o
  `cs.json` de TO); 32 requisições ao TSE pelo `uv-coletor preparar` (≤ 5 req/s, fora da apuração).
- Ordem no modo rápido documentada: cada nó segue a ordem do TSE, mas os BUs do par chegam em lotes (p50 ~10 min da
  noite no 1º turno de 2026, com a noite passando a 500×); na cadência real a ordem é a do TSE.

## [0.8.4] — 2026-10-17

- **Selo de simulação no celular:** o chip "Simulação · dados reais do TSE · …" da faixa não encolhe mais até virar
  "Simula…" (390 px: ~200 px com reticência, texto inteiro no `title`); conferido com captura da página a 1366 e a
  390 px. `site/dist` regerado.
- **Primeira execução do ciclo na OCI** (`./63_ciclo.sh --iniciar 453 rapido`, binário `0.8.3`): os dois nós coletaram
  do simulador do A (51.655 e 45.763 BUs publicados na federação, cada um recebeu os do outro), 97.392 BUs aplicados,
  conferência 51/51 `-u.json` idênticos e 102/102 situações nos dois nós, 0 revogações, 0 duplicados, 0 HTTP 429.

## [0.8.3] — 2026-10-17

- **Ciclo diário de simulações com dados reais** (18 a 23/10/2026, pedido do Luciano em 17/10;
  [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md)). Nos dois nós da OCI, todo dia: 07h 1º turno de 2024, 11h 2º turno de
  2024, 13h 1º turno de 2026 (1 h antes do pedido, para a cadência real caber) e 17h 2º turno de 2026 (só os
  candidatos, sem coleta, até 25/10). Modo alternado por dia: rápido (simulador a 500×, totalização o mais rápido
  possível) nos dias 18, 20 e 22; cadência real do TSE (1×, com o TSE simulado progressivo e o resto da noite sem
  intervalo 15 min antes da troca) nos dias 19, 21 e 23.
  - `deploy/oci/servidor/uv-ciclo.sh` (troca de eleição, arquivamento, pontes, federação do ciclo),
    `uv-ciclo.timer`/`.service` (`OnCalendar` em `America/Sao_Paulo`), `uv-telemetria.py` (uma linha JSON a cada 15 s:
    a `saude.json` inteira, sistema, processos, conexões na 8080, relógio do simulador), `uv-ciclo-resumo.py`
    (`RESUMO.json` com veredito, tempo até encerrar, ordem × recebimento e picos; evidências no bucket) e
    `deploy/oci/63_ciclo.sh` (`--instalar`, `--iniciar`, `--estado`, `--telemetria`, `--remover`).
  - Acervo persistente `/srv/urna-velox/acervo/<pleito>` no nó A (BUs já baixados do TSE, `cs.json`, oficiais,
    candidatos e linha do tempo; o B recebe candidatos e linha do tempo): **nenhuma requisição ao TSE nas simulações**,
    nada baixado de novo; cada eleição começa do zero numa pasta própria, apagada na troca seguinte.
  - Os dois nós coletam ao mesmo tempo do simulador do A (ponte `socat` na rede privada, como no `40_ensaio.sh`).
- **`uv-servico --simulacao "<rótulo>"`**: `simulacao.rotulo` na mensagem `estado`, no `GET /api/v1/estado` e na
  `saude.json`. A página acrescenta "SIMULAÇÃO" ao selo de situação do topo (explicação no `title`) e um chip na faixa
  ("Simulação · dados reais do TSE · <eleição> · <modo>"); sem o rótulo, nada muda. `site/dist` regerado.
- **O limite de 100 req/s é só para buscar arquivos no TSE** (regra do Luciano, 17/10/2026): contra um servidor local
  (`127.0.0.1`), `coletor::limites` aceita `--rps` acima de 100 (teto = `max(100, rps) × aceleração`). Contra o TSE
  nada mudou (teste `limites_nao_afrouxam_contra_o_tse` ganhou o caso local de 5.000 req/s).
- **Linha do tempo das eleições** em `D:\dados\urna-velox\linha-do-tempo\<pleito>.csv` (formato das métricas, ordenada
  pelo recebimento no TSE, `dr`/`hr` do `aux.json`): 452 com 478.026 seções, 453 com 97.392, 3220 com 499.207 (6.969
  completadas pelo `ordem_3220.csv`). **Sondagem de 200 `aux.json` do 3220** (5 req/s, combinada): o `dg`/`hg` é a hora
  da última regeração do arquivo (05/10 12:53–12:56 em 126 de 200; outras de 06 a 16/10), não a da totalização da
  seção; o `aux.json` não traz a hora de totalização, e a extração completa não foi feita.
- **Verificação:** o motor e a totalização não mudaram. Teste local do ciclo com o 2º turno de 2024, dois nós
  federados contra o simulador, nos dois modos: modo rápido 51/51 `-u.json` idênticos ao oficial e 102/102 situações
  nos dois nós, 0 revogações, encerrou em 1,2 min, ordem × recebimento com atraso p50 4 s e p99 148 s da noite;
  cadência (30× com o TSE simulado e, no meio da noite, 500× pelo `/controle/velocidade`, como a aceleração do fim)
  também 51/51 e 102/102 nos dois nós, 0 revogações, sem reinício; o atraso p99 (459 s da noite) vem da rajada da
  aceleração.

## [0.8.2] — 2026-10-17

- **Republicação da federação na partida sem perda e com memória limitada** (`aquisicao/src/federacao/no.rs`,
  `republicar`). Revê a `0.8.1`, que limitava a memória descartando registros. Ela usava um teto de 1.000 registros,
  lia só os últimos 2 MB do diário e saía em silêncio quando o diário estava mais de 2 min à frente do último lote.
  Com isso, uma queda depois de minutos sem gravar lotes deixava os pares sem os BUs dessa janela.
  - O diário é lido em fluxo e o que passou do corte vai para o storage em lotes de até 500 registros, conforme é
    lido. Não há teto: a regra "republicar a mais é inofensivo" continua valendo, e a memória fica em um lote.
  - Execuções misturadas: o `secoes.log` só cresce, então a primeira linha dele é anterior ao primeiro lote da mesma
    execução. Se ela é **posterior ao último lote**, saída e storage da federação são de execuções diferentes. Era o
    caso do nó A: o diário recomeçou às 22:19 UTC de 16/10, numa operação manual que arquivou só a `saida`, e o
    último lote era das 20:21. Nada é republicado, e o motivo vira o alerta `federacao_diario_desalinhado`. Na
    `0.8.1`, esse caso não gerava alerta, embora o CHANGELOG dissesse que sim.
  - `republicar` devolve `Republicacao` (registros, lotes, desalinhado). O evento `federacao_republicados` ganhou
    `lotes`.
  - Testes: `republicar_sem_teto_em_lotes_limitados` (2.345 registros depois do corte → 5 lotes de no máximo 500,
    nada em memória no fim). Os outros dois de republicação foram adaptados; `gravar_lote_fraciona_em_tamanho_maximo`
    é da `0.8.1`.
  - A causa descrita na `0.8.1` está certa: a republicação do diário inteiro (499.207 BUs, ~8,5 GB) e o lote único
    para codificar tudo. Correção de detalhe: o buffer de `codificar` é a soma dos BUs, não `n × 16 KB`.
- **Medido no nó A da OCI**, com o estado real do simulado do 3220: retomada e partição com **480 MB de pico**
  (`VmHWM`) e 378 MB depois de 8 min, sem reinício nem OOM. Na `0.8.0`, ~11,75 GB e morte a cada ~4 min. O alerta
  `federacao_diario_desalinhado` saiu na partida.
- **Coleta não combinada durante a validação:** depois da retomada, o nó A seguiu o `no.env`, que aponta para o site
  do TSE no pleito 3220, e coletou por ~8 min (20:33–20:42 de Brasília):
  - ~88 req/s em média, abaixo do teto de 95;
  - ~25 mil `aux.json` e ~12 mil BUs baixados de novo. O `cs.json` do 3220 traz hoje *hashes* novos para seções já
    aplicadas (alerta `reenvio_identico`: mesmo BU, totais inalterados), e o serviço não usou os BUs dos pacotes
    locais (`bu_do_pacote` 0).

  Parado à mão. O nó A fica parado até o Luciano decidir o `no.env` dele.
- `deploy/oci/README.md`: arquivar `saida`, `dados` e `federacao` juntos (o `60_simulado.sh` já faz isso).
- `cargo test --release --workspace` passa (57 testes em `uv-aquisicao`).

## [0.8.1] — 2026-10-17

- **Correção de falta de memória (OOM) na retomada do serviço com federação ativa (`aquisicao/src/federacao/no.rs`):**
  - **Causa raiz:** na inicialização do serviço com partição configurada (`f.armazem`), `republicar()` busca no diário local (`secoes.log`) os BUs gravados nos últimos 30 s antes do último lote emitido (`corte_ms`). No nó A da OCI, após reinício de simulado, `secoes.log` havia sido reescrito com 499.207 seções (132 MB) às 22:19 UTC, enquanto o último lote `0000005291.uvtr` era de 20:21 UTC. Como todas as linhas eram posteriores ao corte, `republicar()` carregou todas as 499.207 seções inteiras para dentro de `f.saida: Mutex<Vec<(Cabecalho, Bytes)>>`, alocando ~8,5 GB de RAM. Em seguida, a thread publicadora chamou `gravar_lote(&f)`, que tentou serializar o vetor inteiro de uma vez em `codificar(&regs)`, solicitando um buffer de `Vec<u8>` de ~7,65 GB. A soma ultrapassou a memória física (12 GB de RAM na OCI, atingindo ~11,75 GB de RSS), acionando o kernel OOM killer 4–5 s após o log de partição.
  - **Correções aplicadas:**
    - **Fracionamento de lotes (`LOTE_MAX_REGISTROS = 500`):** `gravar_lote()` agora drena `f.saida` em pedaços de no máximo 500 registros (~6 MB por lote), eliminando alocações gigantescas de buffers de codificação.
    - **Teto na republicação (`MAX_REPUBLICAR = 1.000` seções / ~15 MB):** limita a quantidade máxima de registros que podem ser colocados na fila de retransmissão.
    - **Leitura eficiente do fim do arquivo:** em vez de ler o `secoes.log` inteiro do início (132 MB), faz seek para os últimos 2 MB (`seek_tail_bytes`) e processa em ordem reversa.
    - **Proteção contra diários dessincronizados (`MAX_JANELA_REPUBLICAR_MS = 120.000`):** se o timestamp dos BUs no final do diário tiver defasagem maior que 2 minutos em relação ao corte do lote, o sistema detecta que o arquivo pertence a outra execução/sessão, registra aviso no log e ignora com segurança a republicação.
  - **Medição de memória na retomada:**
    - Antes da correção: pico de **~11,75 GB RSS** (processo morto pelo kernel via SIGKILL).
    - Depois da correção: **~50–80 MB RSS** durante toda a inicialização e retomada da federação.
  - **Testes automatizados adicionados (`aquisicao/src/federacao/no.rs`):**
    - `gravar_lote_fraciona_em_tamanho_maximo`
    - `republicar_ignora_diario_desalinhado_ou_antigo`
    - `republicar_recupera_apenas_ultimos_bus_do_corte`
    - Suíte completa `cargo test --release --workspace` passa com sucesso.

## [0.8.0] — 2026-10-17

- **Etapa 8 concluída: previsão do matematicamente e do fim da apuração** — por decisão do Luciano, sem a validação
  independente prevista (como nas etapas 3 a 6). Entregas nas `0.7.6`–`0.7.8`:
  - motor (`urna_velox::previsao`) e backtest `uv-motor previsao`;
  - thread `uv-previsao` no `uv-servico`, mensagem `previsao` no protocolo (§4.7) e o cartão "Previsão
    (estimativa)" na página;
  - detecção ao vivo da parada da chegada dos BUs, com a previsão suspensa e adiada.
- **Critérios** ([relatório](docs/relatorios/RELATORIO_0.8.0.md)):
  - calibração e erro dentro da meta nas duas noites de 2024 (82–85 %, ≤ 8 min a partir de 50 %);
  - 1º turno de 2026 fora da meta pela parada da chegada dos BUs, aceito;
  - rodada < 1 s nos 5.569 municípios, e < 50 ms no 3221 com ≥ 8 threads;
  - CLS 0 no replay.
- **Pendências:** municípios pequenos (< 20 mil eleitores, 74 %) para a eleição municipal de 2028; estratos por zona;
  custo com 4 threads no 3221 a confirmar no ensaio geral.
- ROADMAP: etapa 8 ✅.
- **Implantação na OCI** (`deploy/oci/compilar_local_arm64.sh` + `50_atualizar.sh --sem-compilar`): o
  `uv-atualizador` não mexe nos nós, que estão com a trava manual do simulado do 3220. Os dois nós ficaram com a
  `0.8.0`:
  - **nó B:** ativo, página respondendo.
  - **nó A:** deixado **parado**. Desde a tarde de 17/10, ainda na `0.7.3`, ele estava num laço de falta de memória:
    retomada do simulado (~190 s, 499.207 BUs), depois ~12 GB poucos segundos após a partição, morto pelo kernel,
    30 reinícios a cada ~4 min. A `0.8.0` repete o laço. A investigar na etapa 9.
- Sem mudança de código em relação à `0.7.8`: `cargo test --release --workspace` passa.

## [0.7.8] — 2026-10-17

- **Parada da chegada dos BUs detectada ao vivo** (etapa 8, decisão do Luciano sobre o relatório §9.1, opção c):
  - `Previsor` acompanha o ritmo de chegada das seções no país (`detectar_parada`). Entre 5 % e 97 % apurado, se o
    ritmo dos últimos 5 min cai abaixo de 10 % do dos 30 min anteriores, a previsão fica `suspensa`. Ela volta ao
    normal quando o ritmo chega a 30 % do de antes.
  - Enquanto está suspensa, não se calcula previsão nova: a última previsão normal de cada tópico é adiada pelo tempo
    parado. Com a volta, tudo é recalculado.
  - Parâmetros novos `parada_janela_s`, `parada_ref_s`, `parada_razao` e `parada_retorno`.
  - O backtest lista as paradas no `resumo.md` e no `resumo.json`. Em 2026 pegou 19:31–20:00, além de duas curtas
    (18:01, 18:49); em 2024 1º turno, uma de 8 min; no 2º turno, nenhuma.
- **Protocolo** (§4.7, por acréscimo): `situacao` = `suspensa` e o campo `parada` (`desde`, `minutos`).
- **Página:** no cartão da previsão, a faixa ganha "(adiada)" e a nota, em cor de alerta, diz "chegada de BUs parada
  desde hh:mm — previsão adiada".
- **Relatório** `docs/relatorios/RELATORIO_0.8.0.md`:
  - §4.1 novo, com o efeito na calibração. A detecção não melhora o acerto do relógio em 2026 (Presidente 54 % →
    49 %; 59 % fora das suspensões), e 2024 não muda. O ganho é dizer na tela que a chegada parou.
  - §9 com as decisões.
- **ROADMAP:** municípios pequenos (< 20 mil eleitores, 74 % de cobertura) entram como pendência para a eleição
  municipal de 2028.
- **README:** seção nova "Para quem acompanha a eleição", antes da visão técnica: o que o Urna Velox mostra para
  quem não é técnico (apuração quase em tempo real, o matematicamente com a seção que decidiu, a contagem regressiva,
  a previsão com faixa, o letreiro de notícias da noite, a comparação com o TSE e a curva da noite).
- Sem mudança em totalização ou resultado: `cargo test --release --workspace` passa; não se aplica a conferência
  contra os arquivos oficiais.

## [0.7.7] — 2026-10-17

- **Previsão no serviço** (etapa 8): thread `uv-previsao` (`aquisicao/src/previsao.rs`), **fora da thread do motor**.
  Só lê os contadores atômicos e os candidatos; do motor recebe por canal os instantes em que cada circunscrição
  ficou matematicamente decidida. A cada `--previsao-intervalo` s do relógio do serviço (padrão 30; 0 desliga;
  `--previsao-threads`, padrão 4), registra o avanço de todos os tópicos publicados e prevê os que mudaram. A
  previsão vai para a difusão e para `<saida>/previsoes.csv`; o bloco `previsao` na `saude.json` traz rodadas,
  tópicos e custo.
- **Protocolo** (`docs/arquitetura/PROTOCOLO_WS.md` §4.7, por acréscimo): mensagem `previsao` para quem está inscrito no tópico.
  Traz `mat` (tipo, P10/P50/P90, `incerta`, probabilidade de definir antes do fim, fração com que deve sair),
  `fim99`, `fim100`, `base`, `chegou`, `definida` e `confirmacao`. Na inscrição vem também o `historico` das faixas
  da noite. Rota nova `GET /api/v1/previsao/<topico>`.
- **Página:** cartão "Previsão (estimativa)" entre "Apuração" e "Curva da apuração" (`site/src/previsao.js`), com
  altura fixa e linhas com reticência.
  - Mostra a faixa do matematicamente, o hodômetro do horário mais provável, a chance de sair antes do fim, a faixa
    P10–P90 da noite (tracejado verde = horário real depois que saiu) e o fim da apuração.
  - Depois da definição, mostra "saiu às hh:mm — dentro/fora da faixa prevista desde hh:mm", e o letreiro ganha o
    marco "Previsão confirmada".
  - Num tópico que não é a circunscrição, o matematicamente vem da circunscrição.
- **Relatório da etapa** em `docs/relatorios/RELATORIO_0.8.0.md`; ROADMAP atualizado. Replay do 1º turno de 2026 a 60×:
  - CLS 0 no desktop e no celular a noite inteira.
  - Em 45 de 47 circunscrições decididas com previsão antes, o real caiu na última faixa (nas outras 2, de 3 a 12 s
    antes do P10).
- Fechamento da `0.8.0` aguardando a decisão do Luciano sobre o 1º turno de 2026 (relatório §9).
- Sem mudança em totalização ou resultado: `cargo test --release --workspace` passa; não se aplica a conferência
  contra os arquivos oficiais.

## [0.7.6] — 2026-10-17

- **Previsão do matematicamente e do fim da apuração no motor** (etapa 8, `docs/handoffs/HANDOFF_0.8.0.md`): módulo novo
  `motor/src/previsao.rs`, puro (o instante entra como parâmetro) e determinístico com a semente.
  - Velocidade em aptos/min por média móvel com meia-vida de 10 min, mais a dispersão da janela.
  - Restante por estrato (UF no Brasil, município na UF, até 24 estratos) com encolhimento das proporções e dos
    votos por apto. O eleitorado do estrato é o publicado ou é estimado pelas seções.
  - Momento da definição pela mesma regra do motor (`matematica::avaliar`), por busca binária na fração do restante.
  - Monte Carlo (200 sorteios: deriva comum, amostragem por estrato, comparecimento, ordem de chegada entre estratos)
    para P10/P50/P90, a probabilidade de definir antes do fim e a fração apurada com que deve sair.
  - Conversão em horário misturando os gabaritos (curvas `τ(F)` das noites, ajustadas por deslocamento e escala nos
    últimos 30 min) com a velocidade, mais um erro relativo que cresce com poucas seções por chegar. Fim da apuração
    (99 % e 100 % das seções) pelo mesmo caminho.
  - Salvaguardas (≥ 5 % e ≥ 10 min; faixa > 3 h = "incerta") e confirmação depois da definição: última faixa, dentro
    ou fora, desde quando.
  - `Previsor` com uma série por tópico, rodada só dos que mudaram, em paralelo.
  - Gabaritos das três noites (14 curvas: Brasil, UF, municípios por porte) embutidos em `motor/src/gabaritos.json`.
- **`uv-motor previsao`** (`motor/src/backtest.rs`): reproduz a noite em ordem de chegada e prevê a cada 30 s do
  relógio da noite, só com o que se sabia. Grava `previsoes.csv` com o real ao lado, a calibração e o erro do P50 por
  faixa de apuração (`resumo.md`/`.json`), o custo da rodada (`--todos-topicos`) e os gabaritos da noite
  (`--gerar-gabaritos`).
- `scripts/backtest_previsao.sh`: as três noites, cada uma sem os próprios gabaritos; embute os gabaritos.
- `scripts/graficos_previsao.py`: faixa P10–P90 ao longo da noite, em SVG; gráficos em `docs/img/previsao/`.
- **Backtest** (relatório em `docs/relatorios/RELATORIO_0.8.0.md`), horário real dentro do P10–P90 com ≥ 20 % apurado:
  - **2024 2º turno:** 85,3 %, erro do P50 de 8,1 min a partir de 50 %.
  - **2024 1º turno:** 81,5 %, 7,8 min.
  - **2026 1º turno:** Governador 68,1 %, Senador 60,2 %, Presidente 53,3 %. A chegada dos BUs ao TSE parou das 19h30
    às 20h50; a fração com que cada disputa se decidiu foi prevista com erro de 0,06 ponto percentual.
  - **Custo:** 16.982 tópicos em 27 ms com 24 threads, 5.569 municípios em 224 ms com 2.
- Sem mudança em totalização ou resultado (a previsão só lê o estado): `cargo test --release` passa (8 testes novos
  em `previsao`); não se aplica a conferência contra os arquivos oficiais.

## [0.7.5] — 2026-10-17

- **Site: versão no topo sem deslocar a página.** A versão (`v0.7.x`) entrava como `<sup>` dentro da marca e alargava
  a primeira coluna do topo; no celular a conexão e o botão de tema eram empurrados para fora da tela, a página ficava
  mais larga que a janela e a tabela de resultados inteira aparecia deslocada. Agora a versão é uma etiqueta de 9 px
  logo abaixo de "Velox", fora do fluxo (`position: absolute`, sem ocupar largura); a marca fica do tamanho do texto
  (`justify-self: start`), os itens do topo podem encolher (`min-width: 0`) e o `html` corta qualquer transbordo
  lateral (`overflow-x: clip`). Abaixo de 360 px a marca e a conexão encolhem um pouco para a primeira linha caber.
  Conferido no Chromium em 320, 375, 390 e 1280 px: nada passa da largura da janela e a versão fica colada em "Velox".
- Sem mudança em totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.7.4] — 2026-10-17

- **Arquivos de indexação e SEO técnico (Google Search):** adicionados `robots.txt` e `sitemap.xml` apontando para o domínio canônico oficial (`https://urnavelox.duckdns.org`), com regras de `Allow` na raiz/estáticos e `Disallow` nas rotas de streaming e APIs internas (`/api/`, `/api/v1/`, `/ws`, `/saude.json`).
- **Suporte nativo no servidor HTTP (`uv-servico`):** endpoints `/robots.txt` e `/sitemap.xml` servidos com os tipos de conteúdo corretos (`text/plain; charset=utf-8` e `application/xml; charset=utf-8`) e cabeçalhos de cache apropriados a partir de tabela estática fixa embutida.
- **Build do site e borda:** automação do `site/build.mjs` atualizada para empacotar os arquivos de SEO em `site/dist/` de forma portável e integrada.
- Sem mudança em totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.7.3] — 2026-10-17

- **Proteção automatizada contra vazamento de dados sensíveis e credenciais:** rotina horária e validação no hook para inspecionar pendências no repositório antes do versionamento (arquivos modificados, staged e untracked), garantindo a conformidade com as regras de higienização do projeto.
- **Validação no pre-commit:** hook `.githooks/pre-commit` passa a inspecionar se há pendências irregulares antes de autorizar o commit.
- **Roteiros de implantação da borda e balanceamento:** documentação (`docs/operacao/GUIA_URL_E_BALANCEAMENTO.md`) e scripts de borda e balanceamento (`deploy/oci/`) para publicação segura com terminação TLS e cache.
- Sem mudança em totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.7.2] — 2026-10-17

- **README com a licença em um parágrafo:** livre para qualquer uso não comercial, com o link para o
  [`docs/legal/LICENCIAMENTO.md`](docs/legal/LICENCIAMENTO.md); saem os selos de licença do topo e a menção na lista de objetivos.
  O detalhe (uso livre, uso comercial, redistribuição, garantia, contato, contribuições) passa para o
  `LICENCIAMENTO.md`; a ADR-031 aponta para ele. `LICENSE` e `LICENCA.pt-BR.md` sem mudança.
- Sem mudança em código, totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.7.1] — 2026-10-17

- **Textos e documentação revistos para a publicação do código** (sem mudança na totalização): os documentos passam a
  usar só fontes públicas e a descrever a rede de operação de forma genérica; o teto de 100 req/s aparece como regra do
  projeto, com o HTTP 429 explícito acima dele; caminhos e identificadores de infraestrutura viram marcadores
  (`<ip-público-no-a>`, `<id>`); o `docs/legal/LICENCIAMENTO.md` fica só com as duas licenças e o contato; URLs do
  repositório apontam para `lukeboh/urna-velox-civica`.
- **TSE simulado:** atraso padrão de divulgação passa a mediana 120 s e p90 360 s (valores ilustrativos), com o teste
  `atraso_tem_a_mediana_e_o_p90_pedidos` ajustado; nada mais muda no simulador.
- Verificação: `cargo test --release` 94 passam, 0 falham, 31 `--ignored` (os mesmos da `0.7.0`). Nenhuma mudança em
  totalização ou resultado: não se aplica a conferência contra os arquivos oficiais.

## [0.7.0] — 2026-10-17

- **Etapa 7 concluída: comparação Urna Velox × TSE** ([docs/handoffs/HANDOFF_0.7.0.md](docs/handoffs/HANDOFF_0.7.0.md),
  [docs/relatorios/RELATORIO_0.7.0.md](docs/relatorios/RELATORIO_0.7.0.md)). Fechamento autorizado pelo Luciano em 17/10/2026.
  - Na página, o controle **"Urna Velox · Ambos · TSE"** mostra lado a lado, para a mesma abrangência e cargo, a posição,
    os votos e a % de cada candidato nas duas fontes, a diferença, as seções de cada uma e a idade do arquivo do TSE;
    marco "TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes" (`0.6.13`).
  - O `uv-servico` busca o `-u.json` só dos tópicos que algum cliente está vendo com a comparação ligada, no máximo uma
    vez a cada 30–60 s, com cache compartilhado e nada em segundo plano (`0.6.11`); `uv-simulador` com o TSE simulado
    (`0.6.12`); as seções não instaladas fecham a apuração como no oficial (`0.6.10`).
  - **Critério** (detalhes na `0.6.24`): simulador 2024-2T a 10× (10/10 telas, 51/51) e 2026-1T a 60× (10/10, 83/83),
    CLS 0, `fora_do_intervalo` 0, 0 HTTP 429, marcos 18/18 com o `eventos.csv`; site real 3220 e 3221 por 40 min cada,
    3/3 telas iguais aos arquivos, 0 HTTP 429.
- Relatório: caminho das evidências corrigido (`D:\dados\urna-velox\ensaios\20261017-14xx-etapa7-*`); ROADMAP com a
  etapa 7 concluída.

## [0.6.24] — 2026-10-17

- **Etapa 7: critério atingido** ([docs/relatorios/RELATORIO_0.7.0.md](docs/relatorios/RELATORIO_0.7.0.md); ROADMAP marcado). Falta o ok do
  Luciano para o `bump minor` → `0.7.0`.
  - **Simulador:** 2024-2T a 10× com 10 abas — 10/10 telas idênticas ao oficial nas duas colunas, 51/51, CLS 0, marcos
    "TSE confirmou" 10/10 com o `eventos.csv`; 2026-1T a 60× com 10 abas — 10/10, 83/83, **CLS 0 nas 10 abas**, marcos
    8/8. Rede: `fora_do_intervalo` 0, 0 buscas fora das telas, 0 HTTP 429; BU → publicação p99 23,5 ms (2026) e 3,0 ms
    (2024).
  - **Site real** (`ensaio_tse_real.sh`, 40 min cada, 3 telas): 3220 — 240 buscas (237 × 304), 3/3 telas iguais aos
    arquivos, 0 HTTP 429; 3221 — 243 buscas (240 × 304), 3/3, 0 HTTP 429.
- **`site/dist` regerado** com o selo de posição do TSE de largura fixa: o `uv-servico` embute o `dist`
  (`include_bytes!`), e a correção da `0.6.21` em `site/src/estilo.css` não tinha chegado ao serviço (a repetição do
  2026-1T ainda deu CLS 2,5·10⁻⁵ no mesmo elemento). Com o `dist` novo, CLS 0.
- **`ensaio_tse_real.sh`:** no 3221, `--aceitar-incompleto` (o `candidatos.json` preparado em 12/10 não tem o `cs.json`
  de TO; sem coleta, a comparação não depende das seções) e parada imediata se o `uv-servico` sair na partida (antes,
  esperava 5 min pela porta e seguia; nenhuma requisição ao TSE).
- Conferência: os votos não mudam (só CSS e roteiro); as conferências da noite (83/83 e 51/51) estão nos ensaios acima.

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
  - `docs/arquitetura/ARCHITECTURE_DECISIONS.md`: ADR-031 (classificação Source-Available / PolyForm Noncommercial).
  - `docs/legal/LICENCIAMENTO.md`: modelo de dupla licença e nome do titular.
  - Manifestos: `license = "PolyForm-Noncommercial-1.0.0"` (SPDX) no lugar de `license-file` (`motor`, `aquisicao`),
    e em `bench/rust` e `bench/js`.
- Sem mudança em totalização ou resultado: conferências não se aplicam.

## [0.6.14] — 2026-10-16

- **Ensaio da comparação contra o site real** (etapa 7, critério 3 do `docs/handoffs/HANDOFF_0.7.0.md`): roteiro novo
  `aquisicao/scripts/ensaio_tse_real.sh --confirmo-tse --combinado <nome> <3220|3221> [minutos]` — `uv-servico
  --sem-coleta --http` contra `https://resultados.tse.jus.br` e 3 abas da página (2 no PC, 1 no celular). A única coisa
  que vai ao TSE são as buscas da comparação (o `-u.json` de cada tela, ≤ 1 a cada 30 s por tópico). Horário liberado
  pelo Luciano em 16/10/2026 ("a qualquer tempo").
  - `uv-servico --sem-coleta` não faz mais a conferência automática do fim: ela baixaria todos os `-u.json` do pleito,
    fora do combinado (só os da tela); registra `conferencia_sem_coleta` no log.
  - `site/testes/comparacao.mjs --minimo <s>`: acompanha pelo menos esse tempo mesmo com tudo pronto (o 3220 já
    encerrado); `--oficial` opcional.
- **Posição do TSE com o desempate do motor**: votos iguais → o candidato mais idoso (`dt`) à frente, depois o `seq` e
  o número (`tse_resultados::resumo`; teste `resumo_empate_vai_para_o_mais_idoso`; `docs/arquitetura/PROTOCOLO_WS.md` §4.6).
- **Estado `desatualizado` na hora do site**: dois intervalos sem resposta válida contados no relógio do site (ou dois
  intervalos inteiros de tempo real, se ele não andou), a mesma conta da busca — no simulador acelerado, o estado não
  pisca entre `ok` e `desatualizado`.
- Suítes: unidade do workspace sem falha (`compilar_dev.sh`); o motor só mudou um comentário (`oficial.rs`), as
  conferências não mudam.

## [0.6.13] — 2026-10-16

- **Comparação Urna Velox × TSE na página** (etapa 7, `docs/handoffs/HANDOFF_0.7.0.md` item 4). Módulo novo
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

- **TSE simulado no `uv-simulador`** (etapa 7, `docs/handoffs/HANDOFF_0.7.0.md` item 5). `--tse-simulado <candidatos.json>`
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
  pelo `seq` e depois pelo número; sai o campo `posicao_do_arquivo` (`docs/arquitetura/PROTOCOLO_WS.md` §4.6).
- **Rede da comparação na hora do site**: a próxima busca de um tópico exige também que o relógio do site (o `Date` das
  respostas) tenha andado um intervalo — ou um intervalo inteiro de tempo real, se ele não andou (simulador parado:
  antes, a espera acelerada deixava buscar a cada 0,25 s real com o relógio parado; achado na fumaça desta etapa).
- **Conferência automática espera os arquivos completos**: com `secoes_totalizadas` < `secoes` no `-u.json` (o TSE
  ainda totalizando), a conferência do fim da noite tenta de novo a cada minuto (até 60 vezes; sem arquivo, até 10) e
  não alerta `conferencia_divergente` enquanto espera; `conferencia.json` ganhou `incompletos`.
- **`uv-servico --sem-coleta`**: nenhum `cs.json`, `aux.json` ou BU (nem sondagem) — só a publicação do que carregou na
  partida, a difusão e a comparação com o TSE. Para o ensaio contra o site real (3220 e 3221) sem coleta.
- `docs/operacao/OPERACAO.md` (TSE simulado, `--sem-coleta`, conferência). Suítes: servico 12/12, federacao 6/6; unidade do
  workspace sem falha.

## [0.6.11] — 2026-10-16

- **Comparação Urna Velox × TSE — servidor e protocolo** (etapa 7, `docs/handoffs/HANDOFF_0.7.0.md`). Módulo novo
  `aquisicao/src/tse_resultados.rs`: o `uv-servico --http` busca no site de resultados do TSE o `-u.json` **só do
  tópico que algum cliente inscreveu em `tse:<tópico>`**, no máximo uma vez a cada `--tse-intervalo` (padrão 30 s, de
  30 a 60, hora do site; fora disso o serviço não parte), contado a partir da resposta anterior; cache compartilhado
  (uma busca serve todos os clientes); **nada sem inscrito** (a inscrição cai com o `cancelar` ou o fim da conexão);
  até `--tse-max-topicos` (200) tópicos ao mesmo tempo — o novo espera vaga; `--sem-tse` desliga. Regra combinada com
  o Luciano em 16/10/2026 (`CLAUDE.md`).
  - Rede: a mesma da coleta (100 req/s do órgão, pausa ≥ 10 min depois de um 429), com **prioridade abaixo da coleta**
    (`Limitador::esperar_baixa`: só pega uma vaga livre do limitador; depois de 10 s esperando, entra na fila normal) e
    `If-None-Match`/`If-Modified-Since` (`Rede::get_com`, `Resposta.modificado`).
  - Protocolo (versão 1, por acréscimo; `docs/arquitetura/PROTOCOLO_WS.md` §4.6): mensagem `tse` (instantâneo na inscrição e uma a
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
  - `docs/operacao/OPERACAO.md` (comparação, opções e saúde).

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
  - `docs/operacao/OPERACAO.md` (Sondagem), `docs/relatorios/RELATORIO_SIMULADO_3220_ORDEM.md` e ROADMAP (item marcado).

## [0.6.9] — 2026-10-16

- **ROADMAP: `1.1.0` — Multipleito** (pedido do Luciano, 16/10; depois da eleição): o serviço descobre os pleitos no
  `ele-c.json` do TSE sem `PLEITO` configurado e trata cada um pela situação — agendados só exibidos, em curso
  totalizados (vários ao mesmo tempo, na mesma cota de 100 req/s), encerrados com o resultado completo (a totalização
  do Urna Velox comparada com a do TSE, como na etapa 7) ou só o resultado final do TSE quando não houve totalização;
  seletor de pleito na página. O bloco `1.x` passa a começar na `1.2.0`. Só documentação.

## [0.6.8] — 2026-10-16

- **Simulado do 3220 na OCI na ordem de recebimento do TSE** (16/10, 07:18–10:16, binário `0.6.5`, item da etapa 9 —
  ensaio geral, `0.9.0`), [relatório](docs/relatorios/RELATORIO_SIMULADO_3220_ORDEM.md): 499.207 BUs a ~95 req/s, 0 HTTP 429;
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
    arquivo do TSE), com um controle "Urna Velox · Ambos · TSE". Plano em [docs/handoffs/HANDOFF_0.7.0.md](docs/handoffs/HANDOFF_0.7.0.md).
  - **Etapa 8 (`0.8.0`, alvo 21/10) — previsão do matematicamente e do fim da apuração:** horário estimado, com
    intervalo P10–P90, pela velocidade de chegada dos BUs (aptos/min) e pela tendência dos votos por estrato, validado
    por backtest nas três noites. Plano em [docs/handoffs/HANDOFF_0.8.0.md](docs/handoffs/HANDOFF_0.8.0.md).
  - **Ensaio geral: etapa 7 → etapa 9 (`0.9.0`, alvo 23/10)**, para ensaiar a noite já com as telas novas;
    `docs/handoffs/HANDOFF_0.7.0.md` → [docs/handoffs/HANDOFF_0.9.0.md](docs/handoffs/HANDOFF_0.9.0.md) (cabeçalho novo). Os itens do ensaio que
    não dependem das telas seguem em paralelo; o `0.9.0` só sai depois do `0.8.0`.
- **Regra de rede combinada (CLAUDE.md):** durante a apuração, a comparação pode buscar o `-u.json` **só da
  abrangência e cargo na tela** de algum cliente com ela ligada, **no máximo a cada 30–60 s** por (abrangência, cargo),
  com cache compartilhado, nada em segundo plano, dentro dos 100 req/s do órgão e com a regra do 429.
- Atualizados: [ROADMAP.md](ROADMAP.md) (tabela, plano, mapa da renumeração de 16/10, linha do tempo, etapas 7–9,
  `1.0.0`, metas), [CLAUDE.md](CLAUDE.md), [docs/processo/VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md), [docs/arquitetura/GLOSSARIO.md](docs/arquitetura/GLOSSARIO.md),
  [docs/operacao/OPERACAO.md](docs/operacao/OPERACAO.md), [README.md](README.md), [docs/handoffs/HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md),
  [docs/handoffs/HANDOFF_MERGE_2.0_MAIN.md](docs/handoffs/HANDOFF_MERGE_2.0_MAIN.md) e o cabeçalho deste CHANGELOG.

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
  [relatório](docs/relatorios/RELATORIO_DEPLOY_SEM_SSH.md) — a lista do `sudoers` equivale a root (`uv-operar.sh` faz
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
  ([docs/relatorios/RELATORIO_DEPLOY_SEM_SSH.md](docs/relatorios/RELATORIO_DEPLOY_SEM_SSH.md)):
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

- **Handoff da implantação sem a porta 22** ([docs/handoffs/HANDOFF_DEPLOY_SEM_SSH.md](docs/handoffs/HANDOFF_DEPLOY_SEM_SSH.md)): na rede
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
  o 1º simulado do 3220 na OCI ([relatório preliminar](docs/relatorios/RELATORIO_SIMULADO_1T_2026.md)).
- **Renumeração do roadmap** (decisão do Luciano, 15/10): escalabilidade segura `2.0.0` → etapa 6, `0.6.0` (federação)
  e `1.x` (borda, quórum Ed25519, Wasm); ensaio geral `0.6.0` → etapa 7, `0.7.0`; proporcionais `3.0.0` → `2.0.0`.
  ROADMAP, VERSIONAMENTO, CLAUDE.md, ESCALABILIDADE_SEGURA e ARCHITECTURE atualizados; `docs/HANDOFF_2.0.0.md` →
  `docs/handoffs/HANDOFF_0.6.0.md` e o antigo `docs/handoffs/HANDOFF_0.6.0.md` (ensaio) → `docs/handoffs/HANDOFF_0.7.0.md`, com nota de
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
  topo dele; o `docs/handoffs/HANDOFF_MERGE_2.0_MAIN.md` fica marcado como superado.
- **Testes depois do merge** (em série, `--test-threads 1` nos ignorados, `TMPDIR` na RAM, máquina sem outra carga):
  workspace 84/84; **conferência oficial do motor 13/13** (7 de integração: 2026 83/83, 2024 5.574/5.574 e 51/51 com
  situação e matematicamente; 6 de matemática); ensaios do serviço 9/9 (476 s); ensaios da federação 6/6 (271 s).

## [2.0.0-alpha.14] — 2026-10-15

- **Conclusão oficial com 100% de sucesso do 1º Simulado Urna Velox (1º Turno 2026):**
  - **499.207 de 499.207 seções publicadas** no país inteiro baixadas da CDN oficial do TSE (`https://resultados.tse.jus.br`) e aplicadas pelo Nó A.
  - **0 erros HTTP 429** durante ~2 horas de execução contínua com cadência de 95 req/s.
  - **Paridade de 100,0000%** entre o Nó A (coletor primário) e o Nó B (réplica federada passiva, tráfego externo = 0 req/s), recebendo todos os lotes via rede privada `10.0.1.0/24`.
  - **Apuração consolidada para Presidente:** 119.300.580 votos válidos apurados, Cand 22 com 56.104.371 votos (47,03%) e Cand 13 com 53.879.483 votos (45,16%), com marco de 2º turno matematicamente decidido às 19:38:21 (seq 480.484).
  - Atualizado [docs/relatorios/RELATORIO_SIMULADO_1T_2026.md](docs/relatorios/RELATORIO_SIMULADO_1T_2026.md) com os totais finais consolidados.

## [2.0.0-alpha.13] — 2026-10-15

- **Roadmap reestruturado e preparação do merge da 2.0 para a `main`:** por solicitação do Luciano, a versão `2.0.0` (Módulo de Escalabilidade Segura) substitui oficialmente o antigo marco `0.6.0` (Ensaio Geral) no [ROADMAP.md](ROADMAP.md), consolidando a federação de nós via HRW, a mitigação de 429, a Web UI embutida e o ensaio geral multi-nó na nuvem OCI. O marco subsequente é deslocado para a `2.1.0` (operação de campo no 2º turno de 2026 em 25/10/2026).
- **Handoff oficial de merge:** criado [docs/handoffs/HANDOFF_MERGE_2.0_MAIN.md](docs/handoffs/HANDOFF_MERGE_2.0_MAIN.md) com a governança, topologia OCI e roteiro passo a passo para a fusão da branch `feature/2.0-escalabilidade` na branch `main`.
- **Relatório preliminar do 1º Simulado Urna Velox (1º Turno de 2026):** criado [docs/relatorios/RELATORIO_SIMULADO_1T_2026.md](docs/relatorios/RELATORIO_SIMULADO_1T_2026.md) registrando a execução na nuvem OCI Ampere A1 (Nós A e B) contra a CDN oficial do TSE, mantendo 95 req/s contínuos, 0 erros HTTP 429 e sincronia privada sub-2s entre pares.

## [2.0.0-alpha.12] — 2026-10-15

- **Pendência de teste registrada** ([docs/handoffs/HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md), etapa A; ROADMAP): o critério A completo (3 nós, 1º turno de 2026 a 60× com `kill -9` e reinício; 2º turno de 2024 a 10×) **ainda não tem número válido com a `alpha.11`** — falta um disco onde rodá-lo nesta máquina (~45 GB no pico). Tentativas de 15/10: dentro da distro enche o `ext4.vhdx` do C:; o D: pelo `/mnt/d` provoca falsas mortes pela latência por arquivo; uma imagem ext4 no D: montada por loop ficou 2× mais lenta e travou sob carga (um nó 40 min em E/S); o `wsl --mount --vhd` exige administrador da própria conta. Caminhos para retomar no handoff (distro em modo esparso, OCI depois da RC, ou `wsl --mount` com administrador da própria conta). Por decisão do Luciano, parado por ora.
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
- **Incidente de 15/10** ([docs/handoffs/HANDOFF_SESSAO_2026-10-14.md](docs/handoffs/HANDOFF_SESSAO_2026-10-14.md), "Ambiente"): a primeira tentativa do critério A completo (3 nós, 2026 a 60×) encheu o C: aos ~100 mil BUs (o vhdx já tinha 173 GB de `dados/` de ensaios antigos) e o WSL parou com `I/O error`. Limpeza (164 GB: `dados/` de 58 ensaios e `~/uvreal/v2`; distro de 234 para 70 GB) e compactação do vhdx pelo Luciano. O roteiro do critério A ganhou travas de disco e de memória e a limpeza no fim. O critério A não foi medido (nenhum número desta tentativa vale).
- Só documentação.

## [2.0.0-alpha.8] — 2026-10-15

- **Decisões do Luciano em 15/10** registradas em duas ADRs novas:
  - **ADR-029 — vários órgãos na mesma infraestrutura, com IP de saída dedicado por órgão garantido pela operação.** Vários órgãos podem rodar na mesma infraestrutura; eles **formam uma federação** (colaboram na coleta: cada seção pedida uma vez pela federação, menos pressão na CDN do TSE, todos mais rápidos) e **cada órgão tem máquina e IP público reservado exclusivos**, sem NAT comum, para que nenhum perca vazão por dividir IP com outro. A divisão por IP no código fica como rede de segurança e passará a gerar alerta grave de operação (a implementar).
  - **ADR-030 — entrega aos leitores sem storage (etapa B, opção A):** cada nó é a origem dos próprios arquivos de resultado e uma CDN em modo proxy só faz cache (TTL ~2 s), com vários nós como origem; a origem fica exposta só pela CDN e só com os caminhos públicos (o `/fed/v1/...` continua só entre os pares). **Revê a ADR-021** (storage R2/KV com upload).
- [ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md): princípio 1, §4.1, nota no §5, REQ-ESC-01 (WebSocket da ADR-026 continua para poucos consumidores de baixa latência), REQ-ESC-02 (lote → visível pelo cache < 200 ms + TTL), REQ-ESC-04 (CDN em proxy, sem storage pago) e REQ-ESC-08 (órgãos com máquina e IP exclusivos). [HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md): etapa B confirmada e seção "Vários órgãos na infraestrutura do operador"; `CLAUDE.md` da branch (regra dos órgãos hospedados); [HANDOFF_IMPLANTACAO_OCI.md](docs/handoffs/HANDOFF_IMPLANTACAO_OCI.md): a primeira implantação na OCI fica para depois da versão RC da 2.0; ROADMAP.
- Só documentação.

## [2.0.0-alpha.7] — 2026-10-14

- **Implantação de dois nós federados em servidores Arm da OCI** ([docs/handoffs/HANDOFF_IMPLANTACAO_OCI.md](docs/handoffs/HANDOFF_IMPLANTACAO_OCI.md), scripts em `deploy/oci/`): `00_criar_oci.sh` (VCN, sub-rede, security list, duas instâncias `VM.Standard.A1.Flex` com IP privado fixo e **IP público reservado** cada uma — o IP de saída que vai no `federacao.json`), `10_implantar.sh` (gera `federacao.json` e `no-*.env`, empacota a branch com `git archive`, prepara os servidores — usuário, Rust, `chrony`, `iptables` com a porta 7700 aberta só para o par —, compila no A e copia para o B, instala a unidade systemd sem iniciar, monta o `candidatos.json` uma vez e mede o IP de saída de cada nó pelo eco), `20_iniciar.sh`/`21_parar.sh` (travas: o site real exige `--confirmo-tse` e, antes de 26/10/2026, `--combinado`), `30_verificar.sh`, `40_ensaio.sh` (2º turno de 2024 inteiro com o `uv-simulador` no nó A, limite de 100 req/s por IP de origem, ponte para o B chegar com outro IP), `50_atualizar.sh` (um nó por vez) e `servidor/` (`preparar.sh`, `compilar.sh`, `uv-iniciar.sh`, `urna-velox.service`). `deploy/oci/nos.env` e `deploy/oci/gerado/` fora do git.
- **`"coleta": false` por nó no `federacao.json`** (`federacao/config.rs`, `estado.rs`, `no.rs`): o nó só recebe dos pares e totaliza, nunca pede seção ao TSE, e fica fora da partição com o motivo `so_recebe`. Para um segundo servidor do **mesmo órgão** (a regra de 100 req/s é por órgão): a validação passa a recusar só dois nós **que coletam** do mesmo órgão; o `10_implantar.sh` recusa "mesmo órgão + os dois coletando". 2 testes novos.
- Verificação: `bash -n` e `shellcheck -S warning` em todos os scripts, sem erros; topologia do ensaio reproduzida no WSL (`ensaio_topologia.sh`, 2º turno de 2024, AM e RO, 120×, ponte em Python no lugar do `socat`): o B chega ao simulador como `127.0.0.1` e o A sai por `127.0.0.2`; os dois nós **2/2 idênticos ao oficial**, 5.102 BUs cada, `aux.json` 2.643 + 2.459 = 5.102 (cada seção pedida uma vez), **0 HTTP 429**, ~100 lotes copiados por nó, no máximo 1.012 req num segundo por IP (limite 12.000 a 120×). Nada rodou ainda numa conta da OCI. `cargo test --release -p uv-aquisicao --lib` **39/39** (15/10); motor sem mudança e sem `tokio`/`hyper`/`axum` (`cargo tree -p urna-velox -e normal`).

## [2.0.0-alpha.6] — 2026-10-14

- **Handoff da sessão de 14/10** ([docs/handoffs/HANDOFF_SESSAO_2026-10-14.md](docs/handoffs/HANDOFF_SESSAO_2026-10-14.md)): onde está cada coisa (worktree, branch, versões), as decisões do Luciano nesta conversa (2.0 = escalabilidade; versões alpha; 100 req/s por órgão contados por IP de saída; sem storage compartilhado entre os nós; etapa B sem storage, a confirmar), o que cada alpha entregou, o estado verificado (unidade 37/37, federação 6/6, nó único 9/9), os próximos passos, as armadilhas do ambiente e o incidente do `git add -u` na `main`.
- [docs/handoffs/HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md), etapa B: direção proposta **sem storage** — cada nó serve os próprios arquivos de resultado e uma CDN só faz cache na frente (sem R2); critério B reescrito nesse sentido; marcado "a confirmar".
- Só documentação.

## [2.0.0-alpha.5] — 2026-10-14

- **Federação sem storage compartilhado: os nós trocam heartbeats e BUs diretamente** (decisão do Luciano em 14/10: um storage comum é mais uma peça que os órgãos teriam de montar e combinar). ADR-028 revista (itens 5 e 6, alternativas (e) storage compartilhado e (f) *push*), REQ-ESC-05 e o princípio 4 da [ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md), handoff e ROADMAP.
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

- **Etapa A da 2.0: a federação ligada ao `uv-servico` e ao simulador** (ADR-028; `docs/handoffs/HANDOFF_0.6.0.md`, "Estado da etapa A").
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
  - **ADR-028** ([docs/arquitetura/ARCHITECTURE_DECISIONS.md](docs/arquitetura/ARCHITECTURE_DECISIONS.md)): um nó por órgão; limite de 100 req/s contado por IP de saída **medido**; nós no mesmo IP dividem o teto; HRW **ponderado e só com inteiros** (identidades virtuais, FNV-1a 64 + *splitmix64*, sem ponto flutuante para que nós em sistemas diferentes concordem); 429 pausa só o IP; troca de BUs pelo storage, com a verificação da assinatura da urna como pré-requisito; `Armazem` como abstração. ADR-022 aponta para ela; índice com a ADR-027 (faltava) e a 028.
  - **Módulo `aquisicao/src/federacao/`** (sem dependência nova, sem rede, ainda não ligado ao `uv-servico`): `config` (`federacao.json`: ids e órgãos únicos, IP declarado válido, teto > 100 req/s só com `acordo`, padrões 95 req/s de coleta, peso 5 req/s, *heartbeat* 10 s, morte 30 s, pausa 600 s); `particao` (HRW ponderado); `estado` (quem participa: descarta desconhecido, morto, em pausa, IP divergente; agrupa por IP medido; peso = ⌊vazão/unidade⌋); `armazem` (`ArmazemLocal`: gravação atômica, `ETag` = SHA-256, listagem por prefixo, chaves validadas); `publicar_batimento`/`ler_batimentos` (um nó não publica pelo outro).
  - **Testes (12 novos, todos passando):** valores fixos do hash e de 5 donos (travam a partição entre versões); uniforme com pesos iguais (300 mil seções, ±2 %); proporcional ao peso (9/9/19); saída de um nó só libera as seções dele (~1/3); entrada só toma para si; mudança de peso só mexe no nó que mudou; ordem não importa; grupos por IP (95 → 47,5 + 47,5, pesos 19/9/9); mortos, pausados, IP divergente e desconhecidos fora; pausa de um nó devolve ao outro do mesmo IP os 95 req/s; *heartbeat* mais recente vale; storage local com `ETag`/listagem e chaves recusadas; 3 órgãos pelo storage com 285 req/s somados.
- Verificação: `cargo test --release --workspace` (target próprio, `/home/dev/uvreal/target-v2`): motor 17 + fixtures sem mudança, `uv-aquisicao` 36/36, sem avisos. Nenhuma mudança no motor nem na totalização (as conferências `--ignored` não foram refeitas: nada do caminho delas mudou). A igualdade dos valores fixos foi conferida só em Linux x86-64 (não há Rust no Windows desta máquina).
- ROADMAP: item da federação 🟡 com o que entrou e o que falta.

## [2.0.0-alpha.2] — 2026-10-14

- **Merge da `main` (`0.4.4` a `0.4.6`)** e regra de rede da federação revista com o Luciano: **100 req/s por órgão, contados por IP de saída**; órgãos diferentes podem colaborar. A restrição de "teto global da federação" da `alpha.1` deixa de valer.
- [docs/handoffs/HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md): um nó por órgão (no `federacao.json`); cada nó mede o IP de saída (eco da própria federação) e o publica no *heartbeat*, conhecendo o dos outros; IP medido ≠ declarado → não coleta; nós no mesmo IP dividem os 100 req/s, com HRW ponderado pela capacidade; 429 pausa só o IP, cujas seções passam aos outros; simulador passa a registrar e limitar por IP de origem (nós em `127.0.0.2`, `127.0.0.3`…); critérios A e E reescritos (por IP; meta p90 < 60 s com 3 órgãos).
- [docs/arquitetura/ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md): REQ-ESC-08 reescrito com essa regra (o mesmo texto vai para a `main`).
- Nenhuma mudança na totalização (só documentação e versão).

## [2.0.0-alpha.1] — 2026-10-14

- **Início da 2.0 (módulo de escalabilidade segura) na branch `feature/2.0-escalabilidade`**, a partir da `0.4.3`, num worktree próprio (`D:\git\lukeboh\urna-velox-v2`). Primeira versão alpha (regras em [docs/processo/VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md)).
- **[docs/handoffs/HANDOFF_0.6.0.md](docs/handoffs/HANDOFF_0.6.0.md):** por que a 2.0 (o gargalo é a aquisição: 100 req/s = ~2.850 BUs/min contra pico de ~6.500; entrega por WebSocket não chega a centenas de milhares de navegadores); **restrição do teto global da federação** (REQ-ESC-08 da especificação conflitava com a regra de 100 req/s somando tudo: o teto passa a ser da federação, dividido entre os nós; acima disso só com acordo escrito; nada contra o site real antes de 26/10); lacunas da especificação a decidir (troca de BUs entre nós pelo storage, verificação da assinatura da urna como pré-requisito, `Armazem` local/R2, chaves e `federacao.json`, relógio); etapas A a E com critérios (federação no simulador, publicação na borda, assinatura/quórum/spot-check, Wasm/SPA, ensaio com o atraso recepção → navegador comparado ao TSE); validação independente antes da `2.0.0`.
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

- docs: **handoff da marca para o Claude Design** ([docs/handoffs/HANDOFF_MARCA.md](docs/handoffs/HANDOFF_MARCA.md)): pedido de logo e
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
`0.4.1`–`0.4.11` ([docs/relatorios/RELATORIO_0.5.0.md](docs/relatorios/RELATORIO_0.5.0.md)); critérios medidos lá: tela × oficial 83/83 (2026
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

- docs: **handoff da etapa 6, ensaio geral** ([docs/handoffs/HANDOFF_0.7.0.md](docs/handoffs/HANDOFF_0.7.0.md)): contexto, leitura, o que fazer (atraso disponibilização → BU aplicado medido e corrigido, ensaios de ponta a ponta pelo WebSocket e pela página com conferência oficial, matematicamente × totalização do TSE em 2026, falhas provocadas, dois ensaios seguidos sem intervenção, runbook da noite, binário Linux, cobertura do motor), critério, regras, ambiente com as ferramentas da etapa 5, pendências para 25/10, armadilhas achadas nesta sessão e validação independente. Pressupõe a etapa 5 fechada antes ([docs/handoffs/HANDOFF_0.4.10.md](docs/handoffs/HANDOFF_0.4.10.md) §3).
- ROADMAP: etapa 5 marcada como feita em PATCH até a `0.4.13`, faltando a validação; etapa 6 com o link do handoff.

## [0.4.13] — 2026-10-14

- **`aquisicao/acervo/` entra no repositório** (pedido do Luciano): os scripts dele que montam e mantêm o acervo local de zips de BUs do Portal de Dados Abertos do TSE (`gerenciar_acervo.py`, `acervo_tse.py`, `README.md`: descoberta pela API CKAN, ≤ 90 req/s com recuo de 10 min depois de 429, zips sem compressão para leitura por mmap, índice por UF com sha256, validação ASN.1 de amostras). Já eram citados em `aquisicao/README.md` e em `aquisicao/results/PREFEITO_2024.md`, mas ficavam fora dos commits desde a etapa 1; agora quem clona consegue reproduzir a eleição municipal de 2024. O handoff da sessão deixa de pedir para excluí-la do `git add`.

## [0.4.12] — 2026-10-14

- docs: handoff da sessão de 14/10 atualizado para finalizar a etapa 5 em outra sessão ([docs/handoffs/HANDOFF_0.4.10.md](docs/handoffs/HANDOFF_0.4.10.md)): o que foi entregue da `0.4.9` à `0.4.11`, o que falta (medir de novo BU → publicação depois da mudança no gravador, retotalização da noite inteira, validação independente do RELATORIO §8 com a lista de rotas atualizada) e como testar.

## [0.4.11] — 2026-10-14

- **Curva completa depois de voltar no tempo no replay** (fecha o item em andamento da 0.4.10): a retotalização devia publicar a cada 2 min da noite, mas `fim_bloco - ult_pub` com `ult_pub = i64::MIN` estourava (em release dá negativo) e nenhuma publicação intermediária saía; agora `saturating_sub`. Com o gravador mandando todas as versões à difusão e a difusão amostrando as superadas (0.4.10), voltar para 18:30 dá a curva desde 17:08, e avançar até 21:00 dá 150 pontos contínuos. A retotalização até 18:30 continua em 1,8 s.
- Legenda da curva: "seções (0–100%)" e os nomes sem "(%)", que encolhem com reticências quando não cabem (no celular o 2º candidato sumia).
- Vídeos de 10 s da página (desktop e celular) gravados no Chrome com GPU: ao vivo, histórico aberto, linha do tempo do histórico voltando e filtro; CLS 0 nos dois.
- Verificado: `compilar_dev` sem avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9 (o que faltava na 0.4.10). RELATORIO_0.5.0 §2.1 e §9 atualizados.

## [0.4.10] — 2026-10-14

Checkpoint pedido pelo Luciano com trabalho em andamento (ver [docs/handoffs/HANDOFF_0.4.10.md](docs/handoffs/HANDOFF_0.4.10.md)): `compilar_dev` sem avisos; os ensaios `--ignored` do motor e do serviço **não** foram rodados nesta versão.

- **Curva da apuração com as linhas dos candidatos de verdade** (o Luciano viu as linhas constantes): a página só tinha os pontos desde que abriu, numa escala 0–100 em que os décimos do fim da noite viram retas, e guardava só os dois primeiros do momento. Agora o serviço guarda a curva de cada tópico (um ponto por minuto da noite) e a entrega em `GET /api/v1/curva/<topico>` ([docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §6); a página desenha a noite inteira ao abrir, com o % de todos os candidatos em cada ponto, e as linhas dos candidatos ganham escala própria ajustada aos valores (rótulos à direita; os minutos com menos de 1 % das seções ficam fora da escala), com a área das seções em 0–100 %. Testes `curva_um_ponto_por_passo` e `curva_com_versoes_superadas`. Conferido no Chrome com GPU: linhas variando, CLS 0.
- **Em andamento — curva depois de voltar no tempo:** as publicações da retotalização chegavam em rajada e só a última versão de cada arquivo passava pelo gravador e pela difusão, então a curva recomeçava no instante escolhido. O gravador passa a mandar todas as versões à difusão e a difusão amostra as superadas que fecham um ponto (`amostrar_curva_superada`, sem interpretar o JSON). Compilado, ainda não testado.
- **Histórico do feed** (pedido do Luciano: filtrar as mensagens e voltar ao que já passou): botão ☰ à esquerda do letreiro abre o painel "O que já passou" (`site/src/historico.js`), com filtro por categoria (Decisões, Viradas, Andamento), filtro por texto e uma linha do tempo própria que percorre os itens sem mexer na apuração. Painel fixo de altura fixa: CLS 0 no desktop (escuro e claro) e no celular.
- Roadmap (etapa 6): pendência "Atraso disponibilização → BU aplicado", o achado da 0.4.9, com o plano de medição.
- Fora do repositório: `candidatos.json` das quatro eleições baixadas (2026 1º e 2º turno, 2024 1º e 2º turno) regerados offline com o campo `agregada`, sem baixar nada de novo (o de 2026 2º turno continua incompleto: falta o `cs.json` de TO).

## [0.4.9] — 2026-10-14

- **Seções agregadas fora do total de seções** (pergunta do Luciano: por que 517.179 e não 499.207?): o `cs.json` do TSE lista também as seções **agregadas** — juntadas a uma seção principal, sem urna nem BU próprio (os votos vêm no BU da principal), marcadas com `nsp`. No 1º turno de 2026: 517.179 listadas = 499.207 com BU + 41 não instaladas + **17.931 agregadas**; o TSE conta 499.248 (`s.ts`). O `uv-coletor preparar` marca `agregada: true` nas seções do `candidatos.json` ([docs/arquitetura/FORMATO_CANDIDATOS.md](docs/arquitetura/FORMATO_CANDIDATOS.md)); o `Publicador` as tira de `secoes.total` (nunca abaixo das apuradas: uma agregada que vier com BU passa a contar). O total publicado passa a ser o do TSE (499.248) e o anel termina a noite em 100 %. Elas continuam no catálogo. Os `candidatos.json` dos ensaios da etapa 5 foram regerados `--offline` (iguais fora o campo novo). Teste novo no motor: `agregadas_fora_do_total_de_secoes`.
- **Replay como um vídeo** (pedidos do Luciano; [docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §11, [aquisicao/src/controle_replay.rs](aquisicao/src/controle_replay.rs)):
  - **Velocidade máxima 300×**: o máximo em que o serviço acompanha o pico da noite. Medido em 14/10: a 200× a fila de coleta fica em até 4,5 mil seções; a 300× em até 68 mil (~3,5 s de atraso, recuperado em seguida); a 600× em 152 mil (~8 s); a 1000× em mais de 10 s.
  - **Posição na noite**, `POST /api/v1/replay/posicao`: ir a um **instante já visto** — para trás, ou para a frente até onde a simulação já chegou — faz o serviço parar, guardar o diário das seções no diário-mestre do replay (`<dados>/replay_diario.log`, junto dos pacotes com os BUs já baixados), apagar `--saida` e recomeçar **retotalizando do disco** todos os BUs com disponibilização até o instante: em ordem cronológica, em blocos de 30 s da noite (eventos e marcos com o horário do bloco), sem nenhuma requisição. Até as 18:30 (168 mil BUs) leva 2,2 s, a noite inteira (499 mil) 5,8 s, e a página volta em 7–16 s com o reinício. O corte é exato (último BU = o instante). **Além do que já foi visto**: retotaliza até o ponto mais distante e, dali, **busca** a 300× (até 900× com a fila de coleta quase vazia).
  - Conferido no 1º turno de 2026: depois de passar a noite e retotalizar até 21:00, 18:30, 23:30, 19:00 e o fim, a conferência com os oficiais deu **83/83** e **832/832**. Os eventos são os mesmos 144 dos ensaios contínuos.
  - `uv-simulador`: `POST /controle/posicao?t=T[&x=N]`; `POST /controle/velocidade?x=N&ate=T&depois=M` (anda até `T` e lá muda de velocidade); `GET /controle` com `t`, `inicio`, `fim` e `alvo`; o relógio para 6 h depois dos oficiais. `uv-servico`: reinício em processo (`--controle-replay`; ir a um instante já visto apaga `--saida`); `GET /api/v1/replay` com o relógio do simulador, `inicio`, `fim`, `busca` e `sessao`, também no `estado`.
  - **Página — barra Replay** (`site/src/replay.js`): fica presa acima do letreiro, em largura inteira, como a barra de um vídeo. Tem tocar/pausar, o horário da simulação, a linha do tempo da noite com as horas marcadas e o alvo de uma busca, e a velocidade. O espaço da barra é reservado antes do primeiro desenho: nada da página fica coberto (antes o cartão flutuante tapava a curva) e o CLS fica em 0. Ao ir para um instante, aparece um véu "Indo para…" e todas as abas se recarregam na sessão nova. Conferido no Chrome com GPU, temas escuro e claro e no celular.
  - `simular_site.sh`: o simulador fica parado no começo da noite até o serviço subir; o andamento vem do total publicado; velocidade de 1 a 300.
- Achado ao conferir a retotalização (RELATORIO §7): nos ensaios contínuos a 60× da etapa 5, os eventos saíam em média 11 min da noite depois da disponibilização do BU que os decidiu (até 46 min no pico). Os roteiros desses ensaios usavam `--intervalo-cs 30/aceleração`, que relê os `cs.json` sem parar e disputa a CPU com a coleta. Isso não se aplica à operação a 1×; o efeito na operação real ainda precisa ser medido.
- Verificação (binário deste código; versão impressa ainda `0.4.8`): `compilar_dev.sh` sem avisos; testes de unidade do motor 18/18 e do `uv-aquisicao` 19/19 (novos: `relogio_para_no_alvo`, `agregadas_fora_do_total_de_secoes`); motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9 (com outra sessão rodando ensaios na mesma máquina). Sem `--controle-replay` só muda o total de seções publicado.

## [0.4.8] — 2026-10-14

- **Replay com velocidade controlada pela página** (pedido do Luciano: acelerar e frear a totalização do replay em tempo real, de 1× a 1000×; [docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §11):
  - `uv-simulador`: o relógio simulado ganha base móvel — `POST /controle/velocidade?x=N` (0 = relógio parado) e `GET /controle`, caminhos que não existem no site real; mudar de velocidade não faz o relógio saltar nem voltar (teste `relogio_muda_sem_salto`).
  - `uv-servico --controle-replay` (só contra servidor local e com `--http`): `POST /api/v1/replay/velocidade` com `{"acelerar": x}` (0 = pausa, 1 a 1000) repassado ao simulador, `GET /api/v1/replay`, bloco `replay` na mensagem `estado` (reenviado na hora da mudança). Sem CORS: só `application/json` (415) e a mesma origem (403). As esperas do serviço seguem a velocidade atual (escala atômica; o acompanhamento dos `cs.json` dorme em fatias de 200 ms, então uma espera longa feita a 1× não segura a volta a 1000×); teto local de requisições de 1000×. Sem `--controle-replay`, nada muda.
  - Página: painel **Replay** (`site/src/replay.js`), fixo no canto e recolhível — pausa/continua, régua de 1× a 1000× em degraus, atalhos 1×/10×/60×/300×/1000× e o relógio do site; as outras abas acompanham pelo `estado`. Conferido no Chrome do Windows com GPU (desktop escuro e claro, celular): cliques, teclado na régua e pausa mudam o simulador; CLS 0.
  - **Roteiro** `aquisicao/scripts/simular_site.sh [aceleração] [porta]`: sobe simulador e serviço com a noite do 1º turno de 2026 (sem requisição ao TSE), mostra o endereço e o andamento, `Ctrl+C` encerra. Documentado em `docs/operacao/OPERACAO.md` §8 e `site/README.md`.
  - Documentação: PROTOCOLO_WS §1, §4.3, §6, §11; OPERACAO §8; `site/README.md`; `RELATORIO_0.5.0.md` §2.1 e §9.
- Verificação (binário deste código; versão impressa ainda `0.4.6`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; testes de unidade do `uv-aquisicao` 18/18 (novo: `relogio_muda_sem_salto`); motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Nenhuma mudança na totalização.

## [0.4.7] — 2026-10-14

- Regras de rede da 2.0 trazidas da branch `feature/2.0-escalabilidade` (`2.0.0-alpha.2`) para a `main`: [docs/arquitetura/ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md) REQ-ESC-08 reescrito (um nó por órgão, 100 req/s por órgão contados por IP de saída, cada nó publica o seu e conhece o dos outros, nós no mesmo IP dividem o limite com HRW ponderado, 429 pausa só o IP atingido); ROADMAP, seção `2.0.0`, com a contagem por IP de saída. Texto idêntico ao da branch.
- Nenhuma mudança na totalização (só documentação).

## [0.4.6] — 2026-10-14

- **Correção do commit da `0.4.5`:** ele levou por engano alterações de código de outra sessão que estavam em andamento no checkout da `main` e ainda não tinham sido commitadas (`aquisicao/src/bin/uv-servico.rs`, `difusao.rs`, `servico.rs`, `simulador.rs`, `site/src/app.js`, `site/src/estilo.css`, `site/dist/app.js`, `site/dist/estilo.css`). Este commit volta esses 8 arquivos ao estado da `0.4.4` no repositório; **as alterações continuam no diretório de trabalho**, sem commit, como estavam, para quem as fez commitar com a própria versão.
- `CLAUDE.md`: o limite de 100 req/s vale **por IP de saída** (o que a CDN do TSE vê). Cada nó da federação publica o seu IP de saída e conhece o dos outros; nós com o mesmo IP de saída contam como um só e dividem os 100 req/s desse IP.
- Nenhuma mudança na totalização.

## [0.4.5] — 2026-10-14

- **Limite de requisições ao TSE é por órgão** (regra do projeto, 14/10): cada órgão acessa pelo seu IP e não passa de 100 req/s (acima disso, HTTP 429); órgãos diferentes podem fazer consórcio e colaborar. `CLAUDE.md` ("Rede e servidores do TSE"): 100 req/s **por órgão**, nunca vários IPs/proxies/processos **do mesmo órgão**; federação da 2.0 = um nó por órgão, cada um com o seu limite, vazão somada. ROADMAP (seção `2.0.0`): volta a coleta K × 100 req/s (um nó por órgão) e o critério; a nota de teto global da `0.4.4` deixa de valer.
- Nenhuma mudança na totalização (só documentação).

## [0.4.4] — 2026-10-14

- ROADMAP, seção `2.0.0`: o teto de requisições ao TSE é **da federação inteira** (100 req/s somando todos os nós, regra do `CLAUDE.md`), não de cada nó — a ESCALABILIDADE_SEGURA.md (REQ-ESC-08) falava em K × 100 req/s, o que seria contornar o limite com vários IPs; coleta mais rápida só com teto maior acordado com o TSE ou menos requisições por BU. Critério da 2.0 resumido a partir do handoff (`docs/handoffs/HANDOFF_0.6.0.md`, na branch `feature/2.0-escalabilidade`, versão `2.0.0-alpha.1`).
- Nenhuma mudança na totalização (só documentação).

## [0.4.3] — 2026-10-14

- **Versões MAJOR corrigidas: `2.0.0` = módulo de escalabilidade segura, `3.0.0` = cargos proporcionais** (decisão do Luciano em 14/10). O topo do ROADMAP, a [ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md) e as ADR-021 a 025 já diziam isso; estavam desatualizados a seção "`2.0.0` — cargos proporcionais" do ROADMAP (agora `3.0.0`, com uma seção nova da `2.0.0`), a meta "distribuição de vagas" (`3.0.0`), o mapa das fases antigas (Fase 3 → `3.0.0`), o `CLAUDE.md`, a tabela de [VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md), o cabeçalho deste CHANGELOG e um comentário em `aquisicao/src/preparo.rs`.
- **Versões alpha** para uma MAJOR desenvolvida em paralelo (a 2.0, na branch `feature/2.0-escalabilidade`): `X.0.0-alpha.N`, sem tag; `scripts/versao.sh bump alpha` sobe o `N`, `verificar` aceita o sufixo `-alpha.N`, `tag` ignora pré-releases; regras em VERSIONAMENTO.md.
- Nenhuma mudança na totalização (só documentação, um comentário e o script de versão).

## [0.4.2] — 2026-10-14

- **Etapa 5 (continua na `0.4.x`) — cor do partido, medida com GPU e três correções de tela** ([docs/relatorios/RELATORIO_0.5.0.md](docs/relatorios/RELATORIO_0.5.0.md) §2.2, §4, §6):
  - **Cor do partido** (`site/src/cores.js`): cada candidato leva um tom da cor do seu partido — cor de referência da Wikipédia (`Module:Political party`, consultada em 14/10/2026; partido sem cor própria usa a da federação). Da cor vale o matiz; luminosidade e croma são os da interface em cada tema (OKLCH), e matizes parecidos na mesma disputa (vários vermelhos, vários azuis) recebem tons diferentes da própria cor, a uma distância mínima em OKLab. `light-dark()` resolvido pelo tema; fixa na noite. `site/testes/cores.mjs`: os 31 partidos dos `candidatos.json` de 2026 e 2024 têm cor e nenhuma das 55 + 51 disputas repete cor.
  - **Medida no Chrome 150 do Windows com GPU** (NVIDIA RTX A2000, por CDP; 2026 a 10×): **CLS 0** nos temas escuro e claro, desktop 1440×900 e celular 360×780; ~54 fps (o teto da tela da sessão RDP desconectada; quadro p50 18,4 ms = vsync), quadro p99 19–21 ms, no máximo 1 tarefa longa (≤ 58 ms, na carga), heap 2–6 MB.
  - Corrigido o que a medida mostrou: a **barra de rolagem** empurrava o cabeçalho 15 px ao aparecer (CLS 8·10⁻⁵; `scrollbar-gutter: stable`); a **pílula vazia** reservada em toda linha cortava os nomes no celular ("FLAV…") — fica fora do fluxo até aparecer (`@starting-style`), CLS 0 conferido; **alerta ilegível no tema claro** (amarelo/verde/rosa escurecidos no claro).
  - Documentação: `RELATORIO_0.5.0.md` §1–§7 e §9, `site/README.md`, ROADMAP.
- Verificação (binário deste código; versão impressa ainda `0.4.1`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; motor `--ignored` 13/13; ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Nenhuma mudança no servidor além dos arquivos embutidos da página; nenhuma mudança na totalização.

## [0.4.1] — 2026-10-14

- **Etapa 5 — site de exemplo: página de resultados embutida no `uv-servico`, marcos da noite e nomes** ([docs/handoffs/HANDOFF_0.5.0.md](docs/handoffs/HANDOFF_0.5.0.md), relatório [docs/relatorios/RELATORIO_0.5.0.md](docs/relatorios/RELATORIO_0.5.0.md), ADR-027; a etapa fica na `0.4.x` até a validação independente — sem `bump minor`):
  - **Página** (`site/`, embutida com `include_bytes!` em `aquisicao/src/pagina.rs`): `GET /` e `GET /static/<arquivo>` de uma tabela fixa, CSP `default-src 'self'; connect-src 'self'`, nada externo. Brasil → UF → município por URL e seletores, todos os cargos e consultas, candidatos com partido e vice, %, votos, barras, selo de matematicamente eleito/2º turno com horário, apuração (anel, seções, último BU, **eleitores que faltam para o matematicamente**), curva da apuração, comparecimento/abstenção/brancos/nulos, estado da conexão e avisos do serviço, letreiro de marcos e curiosidades. Glassmorfismo tátil, brilho arco-íris (`conic-gradient` com `@property`) só nos destaques, hodômetros com largura reservada, troca de posição por `translateY`, gráficos SVG com valores interpolados, tema claro opcional, `prefers-reduced-motion`, celular 360 px. Fonte Archivo variável (OFL) embutida. Build: `cd site && npm run build` (esbuild); `dist/` versionado.
  - **Protocolo v1 por acréscimo** ([docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §3, §4.5, §6, §10): `op: marcos`, mensagem `marco` (`primeira_secao`, `secao_decisiva`, `faltam_para_mat`, `virada`, `pct_secoes`, `bu_marco`, `ultima_secao`), `GET /api/v1/marcos`, `GET /api/v1/nomes` (`ETag`/304).
  - **Motor:** `matematica::faltam_para_mat` (pura, sem rede: busca binária sobre `avaliar`) com 2 testes novos; `candidatos.json` com campos opcionais `nomes_ufs`, `municipios[].nome`, `partido`, `vices` (o motor ignora).
  - **Serviço:** `aquisicao/src/marcos.rs` (novo) na thread do motor, `<saida>/marcos.csv` relido na retomada; saúde `custos_ms.marcos`/`faltam` e `difusao.marcos`; `Amostra::com_janela` (janelas pequenas para as medidas novas). `uv-coletor preparar` lê os nomes dos `cm.json`/`-u.json` do cache (`--offline` regera sem requisição) e `nome_urna` passa a vir do `nmu`.
  - Documentação: `site/README.md`, `docs/operacao/OPERACAO.md` §8, `docs/arquitetura/FORMATO_CANDIDATOS.md`, `ARCHITECTURE.md` §2.8, ADR-027, ROADMAP (itens da etapa 5 com `v0.4.1`, sem concluir a etapa).
  - `uv-coletor preparar --pleito 3221` em 13/10 às 22:57: 29 requisições, sem 429; **`cs.json` de TO ainda 404**.
- Verificação (binário deste código; versão impressa ainda `0.4.0`, compilado antes do *bump*): `compilar_dev.sh` sem avisos; testes de unidade do motor 17/17 (2 novos de `faltam_para_mat`) e do `uv-aquisicao` 17/17 (novos: tabela da página, marcos, CSV dos marcos); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço **9/9** (duas vezes; a segunda com o código final exceto `Amostra::com_janela`, que depois passou pelos B finais); `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. **Conferência com os oficiais**: 2026 1º turno a 60× 83/83 e 832/832 (em todas as rodadas), 2024 2º turno 51/51 e 102/102 (a 60× e a 10× com `kill -9`); **números lidos da tela da página** × `-u.json`: 83/83 e 51/51. Marcos × `secoes.log`/`eventos.csv`: 0 divergências (primeira seção 29/29 e 72/72, seção decisiva 55/55 e 51/51, `faltam` = 0 no `seq` de cada evento e 0 subidas). D1 com 10 abas: BU → cliente p99 8,8 ms. CLS 0 nos ensaios finais. B com 0 clientes: p50 inalterado; p99 hoje 15–18 ms também com o binário da `v0.4.0` (máquina mais ruidosa que em 13/10), detalhes no relatório §5.1. Medida da página num navegador com GPU fica para o próximo PATCH. Nenhuma mudança na totalização.

## [0.4.0] — 2026-10-13

- **Etapa 4 concluída — Publicação em tempo real** ([ROADMAP](ROADMAP.md), [docs/relatorios/RELATORIO_0.4.0.md](docs/relatorios/RELATORIO_0.4.0.md)): `uv-servico --http` com WebSocket e REST ([docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md), ADR-026), `uv-carga`, critérios D1–D4 e limites executados na `0.3.3`/`0.3.4`. Critério do ROADMAP: **BU → cliente p99 7,7 ms** com 1.000 clientes na noite de 2026 a 60× (meta < 50 ms); até 10.000 clientes, 35,6 ms.
- **Concluída por decisão do Luciano (13/10/2026), sem a validação independente** prevista no fim do [HANDOFF_0.4.0.md](docs/handoffs/HANDOFF_0.4.0.md). O roteiro dessa validação continua no handoff e no §8 do relatório, para quem quiser fazê-la depois.
- Sem mudança de código desde a `0.3.4`: só versão, ROADMAP (etapa 4 ✅), status do relatório e cabeçalho do [HANDOFF_0.5.0.md](docs/handoffs/HANDOFF_0.5.0.md) (a etapa 5 parte da `0.4.0`, em PATCH `0.4.x`). Verificação: a mesma da `0.3.4` (`compilar_dev.sh` sem avisos, motor `--ignored` 13/13, serviço `--ignored` 9/9); `scripts/versao.sh verificar` coerente.

## [0.3.4] — 2026-10-13

- **Etapa 4: relatório e critérios executados** ([docs/relatorios/RELATORIO_0.4.0.md](docs/relatorios/RELATORIO_0.4.0.md); a etapa fica na `0.3.x` até a validação independente — sem `bump minor`):
  - **D1** (1º turno de 2026 a 60×, 1.000 clientes): **BU → cliente p99 7,7 ms** (meta < 50 ms), 0 conexões perdidas, 1.500/1.500 últimas mensagens idênticas ao arquivo, eventos = `eventos.csv` em 1.000/1.000 conexões, conferência 83/83 e 832/832; **D2** (+100 lentas): lentas juntadas e cortadas (`cliente_lento`), as outras com p99 8,0 ms, pico de RSS 432 MB; **D3** (2024 2º turno a 60×, 200 clientes): `kill -9` com reconexão (nenhum `instantaneo` com `seq` menor; publicação final = A da etapa 3, 51/51), rajada de 429 (`estado` com a pausa e a retomada 10 s depois), reenvio idêntico + anulação (alertas no `estado`, São Paulo corrigido e igual ao arquivo); **D4** (2024 2º turno a 10×, um cliente em todos os tópicos): 51/51 últimas mensagens = arquivo, `seq` só sobe, eventos 102/102, 102/102 situações; **escala**: 3.000 / 6.000 / 10.000 clientes com p99 11,7 / 21,3 / 35,6 ms (o limite de 50 ms não foi alcançado até 10.000), ~125 KB de RSS por conexão; **limites** (item 5): 6.000 conexões com `--max-clientes 5000` → 1.000 recusadas com 503; 1 MB → 1009; 1.000 msg/s → 1008 depois de 39 `erro taxa`; 219 tópicos → 200 aceitos e `limite_topicos`; *handshake* incompleto fechado em 10 s; caminhos do cliente no REST → 404; `POST` → 405; `ETag`/304; a apuração seguiu idêntica ao oficial durante os ataques (MA 219/219).
  - `uv-carga --limites`: o teste de tópicos vai em mensagens pequenas (com 300 tópicos numa só, caía no limite de tamanho, não no de tópicos) e o `ETag` é testado no primeiro tópico existente (antes, `br-c0001`, inexistente nos ensaios de 2024).
  - [docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §9: exemplos reais das mensagens (D4). ROADMAP: item "Carga" da etapa 4 marcado e o critério com o número medido (sem concluir a etapa).
  - **Handoff da etapa 5** ([docs/handoffs/HANDOFF_0.5.0.md](docs/handoffs/HANDOFF_0.5.0.md)) e ROADMAP da etapa 5 com os pedidos do Luciano: glassmorfismo tátil e glow arco-íris nos destaques, gráficos suaves, nenhum pulo na tela (só `transform`/`opacity`, CLS = 0), números em hodômetro, feed de marcos e curiosidades (primeira seção totalizada, seção que determinou o matematicamente, eleitores que faltam para o matematicamente, primeiro governador/senador/prefeito matematicamente eleito…); o que falta no servidor para isso (nomes de municípios e partidos no `candidatos.json`, mensagem `marco`, função `faltam_para_mat` no motor) por acréscimo ao protocolo.
- Verificação: `compilar_dev.sh` sem avisos (0 com `--all-targets`); testes de unidade do `uv-aquisicao` 14/14; motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Só o `uv-carga` mudou de código; nenhuma mudança na totalização nem no serviço.

## [0.3.3] — 2026-10-13

- **Difusão: correções guiadas pelas medidas do B e do D1** (1º turno de 2026 a 60×; [docs/relatorios/RELATORIO_0.4.0.md](docs/relatorios/RELATORIO_0.4.0.md) §1.2):
  - **Montagem preguiçosa da mensagem**: na `0.3.2` a thread `uv-difusao` montava a mensagem de toda versão de tópico (845 mil na noite, com *parse*, cópia e alocação) mesmo sem cliente, e disputava CPU e o alocador com o motor — **com `--http` e 0 clientes, BU → publicação p99 subia de 4,3 para 8,4 ms**. Agora ela só guarda o `Arc` dos bytes; a mensagem é montada uma vez, pela primeira conexão que a usa (`OnceLock`), nas threads HTTP; o REST serve os bytes do arquivo direto. Com 0 clientes: p50 0,805 / p99 4,80 / máx. 45 ms (sem `--http`, mesmo binário: 0,815 / 4,34 / 44).
  - **Aviso `seq_pulado` no máximo a cada 5 s por tópico e conexão** (com a soma das juntadas): com o `br-c0001` mudando ~100 vezes por segundo, um aviso por versão pulada era quase metade das mensagens (D1 da `0.3.2`: 21,9 M avisos em 50,9 M mensagens; agora 96,6 mil em 60,1 M). [docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) §4.4 atualizado.
  - **Threads `uv-http` com prioridade menor** (nice 10, `difusao::prioridade_baixa`): cedem a CPU ao motor, à gravação e à coleta (D1 da `0.3.2`: lote do motor p50 0,055 → 0,335 ms com 1.000 clientes; agora 0,205).
  - O bloco `difusao` da `saude.json` é calculado pela `uv-difusao` (o motor só copia o último); na parada, a saúde final recebe o bloco completo depois de fechadas as conexões.
  - `uv-carga`: `--amostras` (mensagens reais da conexão 0, para o protocolo) e, no relatório, as mudanças de `estado` e os avisos/eventos de apuração vistos pela conexão 0.
- Verificação: `compilar_dev.sh` sem avisos (0 com `--all-targets`); testes de unidade 14/14 do `uv-aquisicao`; motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 9/9; `cargo tree -p urna-velox -e normal` sem `tokio`/`hyper`/`axum`. Com o binário deste commit (versão impressa ainda `0.3.2`: compilado antes do *bump*), 1º turno de 2026 a 60×, simulador/serviço/carga em núcleos separados (`taskset`): **B sem `--http`** 83/83 e 832/832, BU → publicação p50 0,815 ms e p99 4,34 ms (o B da etapa 3, `0.2.6`, dava 0,675/4,3: a diferença é do ritmo ocioso dos `cs.json` da `0.2.8`, que faz 1,23 M requisições contra 1,03 M — não da difusão); **B com `--http` e 0 clientes** 83/83, p50 0,805 e p99 4,80 ms; **D1** (1.000 clientes) 83/83, **BU → cliente p50 3,2 ms, p99 7,7 ms**, máx. 47 ms, 1.500/1.500 últimas mensagens idênticas ao arquivo, eventos = `eventos.csv` em 1.000/1.000 conexões (3 de Presidente), 0 conexões perdidas, 0 regressões de `seq`; **D2** (+100 lentas) as 100 cortadas por `cliente_lento`, as 1.000 com p99 8,0 ms, pico de RSS do serviço 432 MB. Com 1.000 clientes o BU → publicação do serviço sobe para p50 2,9 / p99 10 ms (CPU e memória divididas com a carga e o simulador na mesma máquina; ver o relatório). Nenhuma mudança na totalização.

## [0.3.2] — 2026-10-13

- **Etapa 4 — difusão em tempo real no `uv-servico`** ([docs/handoffs/HANDOFF_0.4.0.md](docs/handoffs/HANDOFF_0.4.0.md); a etapa não está concluída: faltam D1–D4 completos, os limites do item 5 e a validação independente):
  - `--http <end:porta>` liga **WebSocket** (`/ws`) e **REST** (`/api/v1/estado`, `/topicos`, `/resultado/<tópico>`, `/eventos`); sem o parâmetro nada muda. Também `--max-clientes` (5.000) e `--http-threads` (4). Contrato em **[docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md)**: tópicos = abrangência × cargo (`br-c0001`…), `instantaneo` na inscrição, `atualizacao` a cada publicação, `evento` (matematicamente eleito/2º turno, revogação, apuração encerrada/reaberta), `estado`, `aviso` (`seq_pulado`, `desligamento`), `erro`. Toda mensagem de resultado leva `seq`, `secoes_apuradas`, `t_bu`, `t_envio` e **os bytes do arquivo publicado** como `resultado`, sem campo novo dentro dele.
  - Arquitetura (ADR-026, `aquisicao/src/difusao.rs`): a thread `uv-gravacao` converte cada JSON em `Bytes` e o entrega, **antes de gravar**, a uma thread nova `uv-difusao` (canal sem limite: a totalização nunca espera um cliente), que monta a mensagem do tópico uma vez e a põe num `tokio::sync::watch` (último valor por tópico); `axum` 0.8 num runtime próprio; cada conexão observa os `watch` dos seus tópicos e, lenta, recebe só a versão mais nova (aviso `seq_pulado`); 30 s sem conseguir escrever ou 4 MB de buffer derrubam a conexão. Limites: 200 tópicos, 20 msg/s (rajada 40), 4 KB por mensagem recebida, 10 s de *handshake*, `TCP_NODELAY`. Na partida a camada só escuta depois da publicação a frio; na parada limpa avisa e fecha as conexões depois da gravação final. `saude.json` ganha o bloco `difusao` (clientes, inscrições, mensagens/s, bytes/s, juntadas, desconexões por motivo, BU → envio e BU → difusão).
  - **O motor não mudou** (sem rede e sem `tokio`: `cargo tree -p urna-velox` sem `tokio`/`hyper`/`axum`); a única mudança no caminho da publicação é a `uv-gravacao` guardar `Bytes` em vez de `String` e entregar uma cópia de referência à difusão quando `--http` está ligado.
  - **`uv-carga`** (novo binário): N conexões em rampa, perfis de inscrição (`misto`, `br`, `todos`, lista), mede BU → cliente e servidor → cliente (histogramas), conta `seq` pulados, regressões e conexões perdidas, `--lentos`, `--reconectar`, `--limites`, e no fim confere a última mensagem de cada tópico **byte a byte** com o arquivo publicado e os eventos com o `eventos.csv`.
  - Documentação: [docs/arquitetura/PROTOCOLO_WS.md](docs/arquitetura/PROTOCOLO_WS.md) (novo), [docs/operacao/OPERACAO.md](docs/operacao/OPERACAO.md) §7 (ligar, `ulimit`, proxy reverso com TLS, saúde, clientes lentos), `ARCHITECTURE.md` §2.7, ADR-026, ROADMAP (itens da etapa 4 marcados com `v0.3.2`, sem concluir a etapa).
- Verificação: `compilar_dev.sh` sem avisos (0 também com `--all-targets`); testes de unidade novos `difusao::testes` (4: tópico do arquivo, mensagem com os bytes exatos, último valor na thread de difusão, e **protocolo de ponta a ponta num servidor real**: REST com `ETag`/304 e 404 para caminhos de fora da tabela, instantâneo/atualização byte a byte, limites de tópicos, taxa — 1008 —, tamanho — 1009 — e clientes — 503 —, eventos, estado, desligamento 1001); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço **9/9** — os 6 da etapa 3 sem `--http` (a publicação continua idêntica ao oficial: AM/RO 2/2, MA) e 3 novos: `difusao_d4_reduzido` (cliente WebSocket no teste, todos os tópicos: `seq` só sobe, última mensagem = arquivo, eventos = `eventos.csv`, 2/2), `difusao_cliente_lento` (50 conexões que não leem + 10 normais: serviço no mesmo tempo, BU → publicação p99 < 50 ms, normais 20/20 idênticas), `difusao_kill9_reconexao` (`kill -9` com 2.500 BUs e reinício: 20 clientes reconectam, nenhum `instantaneo` com `seq` menor, 40/40 idênticas, eventos 20/20). Fumaça AM+RO a 120× com 20 clientes: 40/40 idênticas, BU → cliente p99 7,3 ms. Nenhuma mudança na totalização: a conferência com os oficiais não muda. Dependências novas no `uv-aquisicao`: `axum` 0.8 (só `ws`, `tokio`, `http1`), `tokio-stream`, `tokio-tungstenite` 0.29 (a mesma versão que o `axum` usa: um `tungstenite` só na árvore).

## [0.3.1] — 2026-10-13

- **Handoff da etapa 4** ([docs/handoffs/HANDOFF_0.4.0.md](docs/handoffs/HANDOFF_0.4.0.md)): publicação em tempo real por WebSocket e REST mínimo dentro do `uv-servico` (`--http`), motor ainda sem rede e sem `tokio`, mesmos bytes do arquivo publicado para todos os clientes, último valor por tópico (cliente lento nunca atrasa a totalização), protocolo em `docs/arquitetura/PROTOCOLO_WS.md`, cliente de carga `uv-carga` e critérios D1–D4 com o simulador (D1: BU → cliente < 50 ms no p99, 1º turno de 2026 a 60×, 1.000 clientes).
- **Tags só nas versões "zeradas"** (`X.Y.0`, a pedido do Luciano): `scripts/versao.sh tag` (chamado pelo hook `post-commit`) não cria mais tag nos PATCH. As 43 tags de PATCH foram apagadas, no repositório local e no GitHub; ficam `v0.0.0`, `v0.1.0`, `v0.2.0` e `v0.3.0`. [docs/processo/VERSIONAMENTO.md](docs/processo/VERSIONAMENTO.md) e `CLAUDE.md` atualizados. Esta versão é a primeira sem tag.
- Verificação: sem mudança de código Rust; `compilar_dev.sh` sem avisos; `scripts/versao.sh verificar`; `scripts/versao.sh tag` na `0.3.1` responde "sem tag".

## [0.3.0] — 2026-10-13

- **Etapa 3 concluída — operação automatizada** ([ROADMAP](ROADMAP.md)): `uv-servico` roda a noite sozinho a partir do `candidatos.json` — acompanha os `cs.json` (dois ritmos: ocioso em rodízio e trabalhando), coleta `aux.json` e BUs dentro dos limites (≤ 100 req/s e pausa ≥ 10 min depois de qualquer 429 contra o TSE, impostos pelo host no código), totaliza em fluxo, calcula o "matematicamente", publica só o que mudou, retoma depois de `kill -9` pelo diário de seções, pede de novo sozinho as seções pendentes, trata reenvio e anulação depois de totalizada, confere com os oficiais no fim e mostra a apuração ao vivo com `--painel`; `uv-simulador` reproduz o site do TSE em `127.0.0.1` para os ensaios.
- Critérios ([docs/relatorios/RELATORIO_0.3.0.md](docs/relatorios/RELATORIO_0.3.0.md), binário `0.2.6`; repetidos depois nas `0.2.7`–`0.2.9` onde a mudança tocava o serviço): A — 2º turno de 2024 a 10×, 51/51 e 102/102, 0 HTTP 429, 0 duplicados, 0 revogações; B — 1º turno de 2026 a 60×, 83/83 e 832/832, 3 eventos de Presidente, BU → publicação p50 0,675 ms (meta < 1 ms); C — `kill -9` ×3, rajada de 429, reenvio, anulação depois de publicada, queda do site e rede ruim, sem perda nem duplicação; teste real no pleito 3221 por 31 min, só `cs.json`, 0 HTTP 429.
- **Concluída por decisão do Luciano sem a validação independente prevista no [handoff](docs/handoffs/HANDOFF_0.3.0.md)** (outra sessão relendo o diff e repetindo A, B e C): os critérios foram executados e medidos pela mesma sessão que fez as correções da `0.2.6`.
- Pendente para 25/10 (relatório §7): `cs.json` de TO ainda 404 (o `candidatos.json` do 3221 segue `incompleto`); `s.ts` pré-apuração parcial em 9 UFs para Presidente (o fim automático pode não vir: usar `CONFERIR`); destinação presumida "Válido".
- Verificação: versão apenas (sem mudança de código desde a `0.2.9`); `compilar_dev.sh` sem avisos; `scripts/versao.sh verificar`.

## [0.2.9] — 2026-10-13

- **`--intervalo-cs-ocioso` passa a controlar o gasto**: na `0.2.8` cada ciclo ocioso lia `4 × intervalo` UFs, então o custo ficava em ~4 req/s qualquer que fosse o intervalo (aumentá-lo só juntava as mesmas leituras em lotes maiores) — contraintuitivo. Agora, contra o TSE, cada ciclo lê sempre 4 UFs em rodízio e o intervalo (mínimo 1 s) decide o gasto: 1 s = 4 req/s (padrão, inalterado; cada UF relida a cada ~7 s), 2 s = 2 req/s, 5 s = 0,8 req/s. Contra o simulador, todas as UFs a cada ciclo, como antes. [docs/operacao/OPERACAO.md](docs/operacao/OPERACAO.md) §2 atualizado.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaio `criterio_a_reduzido` OK; site real, pleito 3221, 60 s ocioso com `--intervalo-cs-ocioso 2`: 144 requisições, todas `cs.json` (28 do primeiro ciclo completo + 4 a cada 2 s), 2,4 req/s em média (com 1 s, na `0.2.8`: 4,4), 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar`.

## [0.2.8] — 2026-10-13

- **`uv-servico`: dois ritmos de releitura dos `cs.json`** (a pedido do Luciano):
  - **ocioso** (fila vazia e nada novo no último ciclo — antes da eleição e nos intervalos da noite): um ciclo por `--intervalo-cs-ocioso` (novo, padrão 1 s), lendo só algumas UFs em rodízio; contra o TSE no máximo 4 req/s (`OCIOSO_REQ_S`: 4 UFs por ciclo de 1 s, cada UF relida a cada ~7 s com 28 UFs). Percebe a primeira seção publicada em segundos, e não em até 30 s;
  - **trabalhando** (seções chegando ou na fila): todas as UFs a cada `--intervalo-cs` (padrão 30 s, como antes). Ao achar seção nova no rodízio, o ciclo seguinte é completo e imediato; quando um ciclo não traz nada e a fila esvazia, volta a ocioso. `saude.json` → `ritmo_cs`; `servico.log` → evento `ritmo_cs` a cada troca.
  - **Os dois intervalos agora são em hora do site**: com `--aceleracao-local` são divididos pela aceleração (antes o `--intervalo-cs` era em segundos reais e tinha de ser ajustado à mão: `--intervalo-cs 0.06` a 500×). Os ensaios `aquisicao/tests/servico.rs` e os exemplos de [docs/operacao/OPERACAO.md](docs/operacao/OPERACAO.md) não passam mais `--intervalo-cs`. **Quem passar `--intervalo-cs` ajustado à mão contra o simulador agora o terá dividido de novo** (o mínimo é 50 ms); contra o TSE nada muda nesse parâmetro.
  - Custo contra o TSE antes da eleição: ~4 req/s (~14 mil requisições por hora, quase todas 304), eram ~0,5 req/s; dentro do ≤ 5 req/s do teste pré-eleição. Com 5 UFs por ciclo, 60 s deram 5,38 req/s (o primeiro ciclo é completo): por isso 4.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaios `--ignored` do serviço 6/6 (agora sem `--intervalo-cs`); 1º turno de 2026 a 500× só com `--aceleracao-local 500` (sem `--intervalo-cs`): 83/83 e 832/832 em 75 s; site real, pleito 3221, 60 s ocioso: 264 requisições, todas `cs.json` (228 com 304), 4,4 req/s em média contando o primeiro ciclo completo, 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar`.

## [0.2.7] — 2026-10-13

- **`uv-servico --painel [prefixo]`**: painel no terminal com a totalização ao vivo de uma abrangência — por padrão `br-c0001` (Presidente no Brasil); `--painel sp-c0003` = Governador de SP (prefixo do arquivo em `publicacao/`). Mostra seções apuradas, BUs aplicados, último BU, comparecimento, válidos, brancos e nulos, cada candidato com nome (do `candidatos.json`), votos, %, barra, o "matematicamente" com a sequência e a hora em que aconteceu, e a situação final. Desenhado pela thread `uv-gravacao` a partir do JSON que ela acabou de gravar, no máximo 4 quadros por segundo e sempre na publicação encerrada — nada roda na thread do motor. Num terminal redesenha a tela; redirecionado, escreve uma linha de resumo por quadro. Exemplo em [docs/operacao/OPERACAO.md](docs/operacao/OPERACAO.md) §2. Teste de unidade `painel_desenha`.
- Verificação: `compilar_dev.sh` sem avisos (47 testes de unidade); motor `--ignored` 13/13 (sem mudança de número); ensaio `criterio_a_reduzido` OK (2/2 idênticos); 1º turno de 2026 a 500× com `--painel` contra o `uv-simulador`: 195 quadros em 76,6 s (de 0 % a "DEFINIDO (ENCERRADA)"), 83/83 arquivos e 832/832 situações idênticos — o painel não muda a publicação. Nenhuma mudança na totalização.

## [0.2.6] — 2026-10-13

- **Etapa 3 — verificação completa com o binário final e correções** ([docs/relatorios/RELATORIO_0.3.0.md](docs/relatorios/RELATORIO_0.3.0.md); a etapa não está concluída, falta a validação independente):
  - **Registro do que entrou na `0.2.4` sem constar do CHANGELOG** (o commit `14a0298`, "docs: relatório de auditoria", também trouxe código do serviço): thread `uv-gravacao` para gravar a publicação fora do laço do motor; cache de totais por (UF, cargo) no `Publicador` (`ativar_cache_uf`/`invalidar_uf`/`invalidar_tudo`, teste `publicador_cache_por_uf`); latências com histograma da execução inteira (`p50_total`, `p99_total`, `max_total`); `M_ARENA_MAX=2` e `malloc_trim` depois de cada ciclo de `cs.json` (RSS do B: pico 365 → 288 MB); 4 threads de rede; sondagem das não instaladas só no exterior (no B de 12/10 pedia 2.596 `aux.json` 404 em UFs que ainda iam receber BU); `Situ::Bu` com a impressão do hash; `docs/operacao/OPERACAO.md`, `docs/arquitetura/AQUISICAO.md` §5.1, `ARCHITECTURE.md` §2.6.
  - **Seções pendentes pedidas de novo sozinhas** (`servico.rs` `Pend`, `repedir_pendentes`, `Pedido::Recoletar`): uma seção publicada cujo `aux.json`/BU não veio depois das novas tentativas era pedida de novo só se a disponibilização mudasse ou numa retomada; com o site fora do ar por 2 min reais a 60× (ensaio C-queda, 20 UFs), 160 seções ficaram pendentes e a apuração nunca encerrou. Agora o motor as pede de novo a cada 2, 4, 8, 16, 32 min (hora do site), até 12 vezes (alerta `pendente_sem_solucao`), só as que têm disponibilização; `secoes.repedidas` na saúde.
  - **`sync_data` fora da thread do motor** (thread `uv-sincronia`, `Escritor::descarregar_e_copiar`): o `fsync` dos ~30 arquivos (pacotes e diário) a cada 2 s parava o motor e atrasava os BUs que chegavam nesse intervalo.
  - **`saude.json`**: custos novos (`lote`, `saude`, `verificacao`, `sincronia_disco`); uma ordenação por amostra; contagem por UF sem uma `String` por seção.
  - **Alerta `secoes_esperadas_incoerentes` mais completo**: compara Brasil × soma das UFs mesmo com UFs sem `s.ts` e acusa `s.ts` < 90 % da lista do `cs.json` (Presidente/Governador, fora o exterior). No 3221 em 13/10 ele passava calado ("1 UF sem s.ts") com SP em 33.403 de 91.326 seções e o Brasil 494.864 × 358.396 na soma das UFs; agora lista CE, ES, GO, MA, PA, PE, PI, RJ e SP.
  - **`uv-simulador` recusa reenvio/anulação marcados antes da disponibilização da seção** (viravam em silêncio "dois hashes desde o início"/"anulada desde o início", e o ensaio não provava o que dizia).
  - Teste `segundo_turno_2026_candidatos_pre_apuracao`: lê os `-u.json` pré-apuração de `fontes/u/` (onde o `preparar` os guarda) e exige pelo menos um; falhava desde a regeração do `candidatos.json` do 3221 (12/10 21:31). Nenhum número mudou.
  - `candidatos.json` do 3221 regerado em 13/10 (72 requisições, ≤ 5 req/s): o `cs.json` de SE já existe (500.111 seções, eram 494.044); o de TO continua em 404, e o arquivo segue `incompleto`.
  - Ensaio `queda_do_simulador`: queda de 60 s reais no pico de chegada de AM e RO (era 10 s às 17:40, quando já não havia seção em coleta), exigindo 0 pendentes.
- Verificação (binário 0.2.6, detalhes e comandos no [relatório](docs/relatorios/RELATORIO_0.3.0.md) §3–5): `compilar_dev.sh` sem avisos, 46 testes de unidade; motor `--ignored` 13/13 (2026 83/83, 2024 5.574/5.574 e 51/51, situação e matematicamente, **sem mudança de número**); ensaios `--ignored` do serviço 6/6. Contra o `uv-simulador`: **A** (2024 2º turno, 10×) 51/51 e 102/102 pela conferência do próprio serviço e pelo `uv-motor conferir-publicacao`, 0 HTTP 429, 0 duplicados, 0 revogações, matematicamente com mediana de 1.562 s de antecedência (cenário "site" do `PREFEITO_2024_2T.md`: 1.561 s); **B** (2026 1º turno, 60×) 83/83 e 832/832, 3 eventos de Presidente, 0 revogações, BU → publicação p50 0,675 ms e p99 4,3 ms (0.2.5: 0,825 e 172,5 ms); **C** `kill -9` ×3 (publicação idêntica à do A), rajada de 429 (pausa de 10 s reais = 600 s simulados, 0 requisições nela), dois reenvios idênticos, anulação depois de publicada (só o arquivo do município diverge do oficial, como esperado), queda de 120 s e queda com `--timeout 2` (45 pendentes pedidas de novo e resolvidas), rede com 2 % de 503 e latência de 20–400 ms. **Teste real** no 3221 (31 min, binário 0.2.6): 868 requisições, todas `cs.json`, 0 `aux.json`, 0 HTTP 429, saída 0 no `SIGTERM`, publicação a frio idêntica à do `uv-motor totalizar` (43 arquivos, "apurando"); rede real da sessão ≈ 2.030 requisições, nenhum BU.

## [0.2.5] — 2026-10-13

- Ajustes nos gráficos do benchmark comparativo (não fazem parte desta cópia).

## [0.2.4] — 2026-10-13

- **Auditoria de controle do projeto:**
  - Levantamento completo de requisitos e afirmações que não conferem no código (parsing zero-copy vs alocações dinâmicas na heap, promessa de `rasn`/`rayon`/`seqlock`/`epoca` ausentes no código, falta de validação de assinaturas ECDSA da urna, badge de licença em desacordo com licença Source-Available).
  - Identificação de problemas e bugs invisíveis (vulnerabilidade de Path Traversal/Zip Slip em `zip.rs`, descompressão sem teto máximo/risco de Zip Bomb, divergência de percentuais por arredondamento flutuante em vez de truncamento oficial do TSE, e risco de concorrência na escrita desacoplada).
  - Mapeamento de recursos necessários pendentes para o 2º turno de 2026 (servidor Linux dedicado de produção com IP exclusivo para evitar penalidade de 429 da Akamai, pendência dos `cs.json` de SE e TO no pleito 3221 e inconsistência no `s.ts` de SP nos `-u.json` pré-apuração).

## [0.2.3] — 2026-10-13

- **Módulo de Escalabilidade Segura (arquitetura da v2.0.0)** ([docs/arquitetura/ESCALABILIDADE_SEGURA.md](docs/arquitetura/ESCALABILIDADE_SEGURA.md)):
  - Especificação completa da federação colaborativa de nós coletores sem coordenação central, balanceamento determinístico por *Rendezvous Hashing* (HRW) com tolerância a falhas e dinamicidade via *heartbeats* leves em storage público.
  - Distribuição massiva aos clientes invertida: eliminação do *push* ativo via conexões abertas permanentes no servidor único, adotando *Static-File Edge Broadcast* via CDN global com custo R$ 0,00 de tráfego de saída (Edge Cloudflare R2 / KV / Pages com SPA hospedada no GitHub Pages).
  - Segurança Zero-Trust e validação descentralizada por quórum criptográfico de operadores autorizados (chaves Ed25519) e integridade dos votos amarrada às assinaturas digitais originais da urna eletrônica do TSE (ECDSA / ICP-Brasil).
  - Adoção de SPA / PWA universal e motor em WebAssembly (Wasm) no navegador, eliminando a necessidade e burocracia de aplicativos nativos de lojas mobile (Android/iOS).
  - Novos registros de decisões de arquitetura em [docs/arquitetura/ARCHITECTURE_DECISIONS.md](docs/arquitetura/ARCHITECTURE_DECISIONS.md): ADR-021 (Static-File Edge Broadcast), ADR-022 (Rendezvous Hashing), ADR-023 (Zero-Trust e quórum Ed25519), ADR-024 (Wasm no navegador) e ADR-025 (SPA/PWA em vez de apps nativos).
  - Lista formal com 17 requisitos de engenharia rastreáveis (`REQ-ESC-01` a `REQ-ESC-17`).
  - Alinhamento de roadmap: `v1.0.0` (majoritários em nó único), `v2.0.0` (escalabilidade segura) e `v3.0.0` (cargos proporcionais).

## [0.2.2] — 2026-10-12

- **Etapa 3 — operação automatizada** ([docs/handoffs/HANDOFF_0.3.0.md](docs/handoffs/HANDOFF_0.3.0.md), itens 1 a 4; a etapa não está concluída):
  - **Motor — retirar e substituir um BU** (ADR-019): `Estado::retirar` (mesmo agregador, `fetch_sub` em votos, comparecimento, `aptos`, `secoes`, `bus` e contadores de conferência; limpa o bit de `vistos`), `substituir`, `aplicar_secao` (anulação dada pela situação do `aux.json`), `aplicada`. `Acompanhamento::bu_alterado` reavalia a circunscrição e guarda o motivo, que vai com qualquer revogação de "matematicamente" (`Evento::motivo`). Testes `motor/tests/retirada.rs` com BUs reais das fixtures: aplicar → retirar volta ao estado zero (todos os contadores e o checksum); substituir A por B = aplicar só B; anular depois = anulada desde o início; caso real MA 07315/0023/0354. `pacote::indice`/`ler_bu_em`/`Escritor::gravar_pos`/`descarregar`; `Publicador::nao_instalada`/`secoes`.
  - **`uv-simulador`** (novo binário, ADR-020): site de resultados em `127.0.0.1` com os caminhos do real (`cs.json` reconstruído no instante simulado com `ETag`/304, `aux.json` no formato real, `-bu.dat` dos `.uvpk`, `-u.json` oficiais depois do último BU), relógio acelerado, `--horario recebido` (2026: o `cs.json` foi coletado dias depois da noite), `--corte`, e falhas por parâmetro: rajadas de 429, latência e 5xx com semente fixa, reenvio idêntico, anulação depois de publicada, queda; `simulador.log` com cada requisição.
  - **`uv-servico`** (novo binário): `candidatos.json` → `cs.json` de todas as UFs com `If-None-Match` → `aux.json` → BU → motor numa thread nativa (o motor continua sem rede e sem `tokio`) → matematicamente a cada lote → publicação atômica só das abrangências que mudaram → eventos. Diário de seções só de acréscimo (`secoes.log`) e `.uvpk` só de acréscimo: retomada depois de `kill -9` sem perder nem duplicar BU, BUs órfãos do pacote reaproveitados; reenvio idêntico (alerta, totais inalterados), reenvio diferente e anulação depois de totalizada (substituição com alerta); não instaladas contadas em `s.st`; situação desconhecida = pendente com alerta. Fim da apuração pelo `s.ts` dos candidatos, com fila vazia e toda seção publicada resolvida, ou por `CONFERIR`/`--conferir-a-cada`; conferência automática (`conferencia.md`/`.json`, ≤ 5 req/s); publicação reaberta se chegar BU depois; contra o site real nunca sai sozinho. `saude.json` a cada 2 s, `servico.log` em linhas JSON, `alertas.csv`; parada limpa com `SIGTERM`.
  - **Rede** (`coletor.rs`): `limites`/`base_local`/`Rede::para` — contra qualquer host que não seja `127.0.0.1`/`localhost`/`::1`, teto de 100 req/s e pausa global ≥ 600 s depois de **qualquer** 429, qualquer que seja a configuração, e aceleração recusada; contra o simulador, teto e pausa escalados pela aceleração e a rede só aceita URLs dele. Testes de unidade `servidor_local` e `limites_nao_afrouxam_contra_o_tse`.
  - **Sondagem das não instaladas** (única exceção a "seção sem disponibilização não gera requisição"): só depois de 30 min (hora do site) sem BU novo, com toda seção publicada resolvida, em abrangências quase completas (≤ 5 % ou 50 seções), nunca a do Brasil, ≤ 1.500 por abrangência e 3.000 na noite, ≤ 2 req/s — milhares de 404 seguidos já provocaram 429 a ~70 req/s (docs/arquitetura/AQUISICAO.md §1.3).
  - `candidatos.json`: `abrangencias[].secoes` (`s.ts` pré-apuração), opcional ([docs/arquitetura/FORMATO_CANDIDATOS.md](docs/arquitetura/FORMATO_CANDIDATOS.md)); os quatro `candidatos.json` locais regerados offline (iguais aos anteriores fora esse campo). Alerta na partida quando o `s.ts` é incoerente — **o do 3221 baixado em 12/10 é** (Brasil 488.941 ≠ soma das UFs; SP 33.403 seções com 91.326 no `cs.json`); o eleitorado das circunscrições (Brasil e os 5 Governadores) está coerente.
  - Crate `uv-aquisicao` passa a ter biblioteca (`src/lib.rs`) usada pelos três binários; dependências `hyper` 1, `hyper-util`, `http-body-util`, `libc`.
- Verificação: `compilar_dev.sh` sem avisos (15 + 4 + 1 + 7 + 4 + 5 + 8 testes); `--ignored` do motor 13/13 (2026 83/83, 2024 5.574/5.574 e 51/51, situação e matematicamente, sem mudança de número); ensaios `--ignored` do serviço contra o simulador 6/6: critério A reduzido (AM e RO a 120×, 2/2 idênticos, 0 HTTP 429, 0 sondagem), `kill -9` três vezes (mesmo resultado da execução sem queda, 5.102 "aplicado" no diário), rajada de 429 (nenhuma requisição durante a pausa), reenvio idêntico, anulação depois de publicada (MA, 1º turno de 2024: 219/219 arquivos e 608/608 situações idênticos), queda do simulador.

## [0.2.1] — 2026-10-12

- [docs/handoffs/HANDOFF_0.3.0.md](docs/handoffs/HANDOFF_0.3.0.md): plano da etapa 3 (operação automatizada) para outra sessão executar: retirar/substituir BU no motor (reenvio e anulação depois de totalizada), site simulado local `uv-simulador` (os BUs de 2024 e 2026 não podem ser baixados de novo), serviço `uv-servico` (acompanha `cs.json`, coleta ≤ 95 req/s com pausa de 10 min após 429, totaliza em fluxo, matematicamente, publicação, diário de seções e retomada, saúde em arquivo, conferência automática no fim), critérios A/B/C no simulador e teste real sem BUs no pleito 3221. ROADMAP aponta para o handoff.

## [0.2.0] — 2026-10-12

- **Etapa 2 concluída — catálogo e candidatos pré-eleição** ([ROADMAP](ROADMAP.md)): `uv-coletor preparar` monta o `candidatos.json` só com arquivos publicados antes da apuração ([docs/arquitetura/FORMATO_CANDIDATOS.md](docs/arquitetura/FORMATO_CANDIDATOS.md)) e o `uv-motor` totaliza, publica e reproduz a noite com ele (`--candidatos`).
- Revalidação ([docs/relatorios/VALIDACAO_0.2.0.md](docs/relatorios/VALIDACAO_0.2.0.md) §6) dos seis itens da validação sobre a `v0.1.7`: votos idênticos ao oficial nos três conjuntos; com a destinação presumida "Válido", só as 33 situações (32 municípios) de candidatos cassados/indeferidos de 2024 divergem; noite de 2026 com 3 eventos de Presidente, 0 revogações; 3221 a frio com os 43 arquivos certos. Correções e revalidação feitas pela mesma sessão.
- Pendente para 25/10: `cs.json` de SE e TO do 3221 (404 em 12/10) — o `preparar` sai com código 2 até serem publicados.

## [0.1.7] — 2026-10-12

- **Correções da validação da etapa 2** ([docs/relatorios/VALIDACAO_0.2.0.md](docs/relatorios/VALIDACAO_0.2.0.md) §3; detalhes e evidências em [docs/relatorios/RELATORIO_0.2.0.md](docs/relatorios/RELATORIO_0.2.0.md) §8):
  - `uv-coletor preparar` reescrito (`aquisicao/src/preparo.rs`): baixa ele-c, `mun-e…-cm.json`, os `-u.json` de cada abrangência e os `cs.json`, com cache em `--fontes`, 1 conexão, ≤ 5 req/s e parada no 1º HTTP 429; `--offline`, `--atualizar`, `--cargos`, `--ignorar-destinacao`; fonte ausente → `incompleto` e código de saída 2.
  - Só a circunscrição do cargo vira circunscrição (Presidente: 1, no Brasil); as outras abrangências vão para `cargos[].abrangencias` com o eleitorado. Na noite de 2026: 3 eventos de Presidente (eram 64), `nao_confere` 0 (era 23).
  - Municípios de cada eleição a partir do `cm.json` (`eleicoes[].municipios`): o 3221 passa a ter 28 UFs e 5.757 municípios; cargo municipal só tem região nos municípios da eleição.
  - `Publicador`: abrangências do `candidatos.json` (consultas do 3221 por UF; a frio, 43 arquivos com os mesmos nomes dos `-u.json` pré-apuração, eram 721) e eleição por circunscrição (`Candidatos::eleicao_de`); `st` vazio enquanto o `estado` é "apurando".
  - `Catalogo::de_arquivo_candidatos` devolve `Result` (UF inválida é erro, não pânico).
  - `candidatos.json` ganha `destinacao_presumida` e `incompleto`; premissas (destinação "Válido", circunscrição "finalizada", contador de votáveis desconhecidos) em [docs/arquitetura/FORMATO_CANDIDATOS.md](docs/arquitetura/FORMATO_CANDIDATOS.md); `ARCHITECTURE.md` §3.1 com o slot `OFF_VNT`.
  - Testes: `*_pre_eleicao` falham sem o arquivo e, em 2024 1º turno, admitem só as 33 situações (32 municípios) com candidato não apto no oficial; `matematica::*_candidatos` (3 conjuntos, 0 revogações); `segundo_turno_2026_candidatos_pre_apuracao`; fixture e teste unitário cobrem abrangências, eleição por circunscrição e UF inválida. 13 testes `--ignored` passam.

## [0.1.6] — 2026-10-12

- [docs/relatorios/VALIDACAO_0.2.0.md](docs/relatorios/VALIDACAO_0.2.0.md): validação independente da etapa 2 como entregue na `v0.1.5` — **não aprovada para 0.2.0**: Presidente com 29 circunscrições (matematicamente "eleito no AC"), pleito 3221 sem SE e TO, consultas publicadas por município (683 arquivos), `st` "Eleito" com zero voto, testes "pré-eleição" montados com os arquivos oficiais já apurados; verificado e correto: `OFF_VNT`, desempenho, eleitorado ≥ Σ aptos nos três conjuntos, rede. As correções vêm nas versões seguintes (feitas por esta sessão, a pedido do Luciano).
- Publicação das pendências: `v0.1.4` (estava só local) e `v0.1.5` (trabalho sem commit). **A mensagem do commit da `v0.1.5` (`d0813df`) saiu errada** — repete a da `v0.1.3` ("docs: handoff da etapa 2 usa os arquivos…") por falha ao gravar o arquivo da mensagem; o conteúdo é o da entrada [0.1.5] abaixo. Não reescrevi o histórico já publicado.

## [0.1.5] — 2026-10-12

- **Catálogo e candidatos pré-eleição** ([docs/handoffs/HANDOFF_0.2.0.md](docs/handoffs/HANDOFF_0.2.0.md), passos 1, 2, 4 e 5):
  - Especificação e modelo de dados estruturado `candidatos.json` (`docs/arquitetura/FORMATO_CANDIDATOS.md`), contendo pleito, UFs, municípios, seções, eleições, cargos, vagas e candidatos homologados com número, nome, destinação e data de nascimento.
  - Implementado carregador `Candidatos::ler(caminho)` e construtor direto de catálogo `Catalogo::de_arquivo_candidatos(arq)` sem necessidade de processar BUs.
  - Subcomando `uv-coletor preparar --pleito <P> --saida <DIR>` implementado para consolidação automática de candidatos a partir dos metadados pré-eleição ou de fontes locais em cache.
  - Binário `uv-motor` (comandos `totalizar`, `conferir`, `noite`) passa a aceitar a opção `--candidatos <caminho>` para carregar o catálogo e os candidatos pré-eleição.
  - Preparado e verificado o caso de teste real do 2º turno de 2026 (pleito 3221): 26 UFs, 5.543 municípios, 494.044 seções, 11 eleições (Presidente com 2 candidatos, Governador nas 5 UFs com 2 candidatos, 6 suplementares de Prefeito e 3 consultas); totalização a frio sem BUs executa com sucesso gerando 721 arquivos de publicação com estado `"apurando"`.
  - Novos testes de integração automatizados (`--ignored`) executando a totalização e conferência com catálogo pré-eleição nos 3 conjuntos reais: 2026 1º turno (83/83 idênticos), 2024 1º turno (5.574/5.574 idênticos) e 2024 2º turno (51/51 idênticos).
  - Fixture versionada `motor/tests/fixtures/candidatos_exemplo.json` e testes unitários de carregamento adicionados.

## [0.1.4] — 2026-10-12

- **Votável desconhecido vira alerta e nulo técnico** ([docs/handoffs/HANDOFF_0.2.0.md](docs/handoffs/HANDOFF_0.2.0.md), passo 3):
  - Adicionado slot `OFF_VNT` no layout da região (`motor/src/catalogo.rs`), com `EXTRA = N_LEGENDA + 5` (105). Essa mudança no layout altera o checksum estrutural por região, sem alterar totais de votos nominais, brancos ou comparecimento.
  - Votos nominais em candidatos ausentes do catálogo pré-eleição são contabilizados em `OFF_VNT`, contam na soma da seção para validação de integridade de comparecimento (`soma == comp * vagas`) e são registrados como alertas consultáveis em `Estado::lista_alertas_vnt()` (com cargo, município, número votado e votos).
  - `Totais` em `motor/src/resultado.rs` ganha campo `vnt: u64` lido diretamente do vetor de tally e consolidado nas UFs e no Brasil.
  - `Publicador::valor` e `conferir_votos` unificam o cálculo de `vnt`: somam `t.vnt` às sobras nominais, garantindo compatibilidade total com o catálogo montado de BUs e com o catálogo pré-eleição.
  - Testes unitários e testes completos (`--ignored`: 83/83 em 2026, 5.574/5.574 e 51/51 em 2024) 100% aprovados.

## [0.1.3] — 2026-10-12

- [docs/handoffs/HANDOFF_0.2.0.md](docs/handoffs/HANDOFF_0.2.0.md): o TSE já gerou os arquivos de candidatos do 2º turno de 2026 (pleito 3221), segundo o Luciano. Eles passam a ser a fonte principal e o caso de teste real da etapa 2, usados desde o passo 1, e a validação vai conferir os candidatos do 3221 com esses arquivos. Nenhuma mudança de código.

## [0.1.2] — 2026-10-12

- **"Cadastro" passa a se chamar "candidatos"**, a pedido do Luciano, para não confundir com o cadastro de eleitores: módulo `motor/src/cadastro.rs` → `motor/src/candidatos.rs`, tipo `Cadastro` → `Candidatos` (`Candidatos::dos_oficiais`, `dos_arquivos_oficiais`), campo `Config::cadastro` do replay → `candidatos`; o contador de seções fora do catálogo passa de `fora_cadastro` a `fora_catalogo` ("fora do catálogo" nas mensagens). Documentação atual (ARCHITECTURE, ROADMAP, FORMATO_RESULTADO, `motor/README.md`) e [docs/handoffs/HANDOFF_0.2.0.md](docs/handoffs/HANDOFF_0.2.0.md) atualizados: o arquivo da etapa 2 será `candidatos.json` (`docs/arquitetura/FORMATO_CANDIDATOS.md`, opção `--candidatos`). As entradas antigas deste CHANGELOG mantêm o nome da época.
- Só renomeação, sem mudança de comportamento: testes normais (30) e completos (`--ignored`: 2026 83/83, 2024 5.574/5.574 e 51/51, com situação e matematicamente) passam iguais.

## [0.1.1] — 2026-10-12

- [docs/handoffs/HANDOFF_0.2.0.md](docs/handoffs/HANDOFF_0.2.0.md): plano da etapa 2 (catálogo e cadastro pré-eleição, alvo 16/10) para ser executado por outra sessão e validado de forma independente — sondagem das fontes publicadas antes da apuração (pleito 3221 como prova), `cadastro.json` local documentado, `uv-coletor preparar`, `uv-motor … --cadastro`, votável desconhecido como alerta e nulo técnico sem depender do catálogo dos BUs, testes com o cadastro pré-eleição nos três conjuntos, preparo do pleito 3221; quem executa para na `0.1.x`, e o `bump minor` para `0.2.0` vem depois da validação. Link no ROADMAP.
- Nenhuma mudança de código ou de resultado.

## [0.1.0] — 2026-10-12

**Etapa 1 do roadmap concluída: o motor** ([handoff](docs/handoffs/HANDOFF_0.1.0.md), passos 0.0.23–0.0.27). O critério de pronto foi atingido:

- Crate **`urna-velox`** (`motor/`) no workspace, com o binário **`uv-motor`** (renomeado de `urna-velox` a pedido do Luciano: todos os executáveis do projeto começam com `uv-`); `uv-coletor` depende dele; versão e hooks coerentes (`scripts/versao.sh`, `.githooks/pre-commit`, um `Cargo.lock` na raiz).
- **Testes automatizados reproduzem as conferências da etapa 0** com a situação de cada candidato calculada pelo motor: 1º turno de 2026 **83/83** abrangências e **832/832** situações; Prefeito de 2024 **5.569/5.569** municípios (+ 5 consultas; **15.042/15.042** situações) e **51/51** no 2º turno (**102/102**) — pelo estado e pelas publicações JSON. Unidade + fixtures: `cargo test --release -p urna-velox` (30 testes, < 1 s); completos: `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (6 testes, ~15 s).
- **Matematicamente eleito / 2º turno** no motor: nas noites reproduzidas (2026, 2024 1º e 2º turnos) nunca revogado e sempre igual ao eleito oficial; 2º turno de 2024 com o motor: mediana de 26 min antes da totalização do TSE.
- **Formato JSON documentado** ([docs/arquitetura/FORMATO_RESULTADO.md](docs/arquitetura/FORMATO_RESULTADO.md)) e usado pelo replay e pelo `uv-motor totalizar`.
- **Desempenho** (i7-14700, WSL), antes (v0.0.22) → depois: totalização em lote numa thread, 2026 2,86 → **2,14 µs por BU** e 2024 1º turno 1,75 → 1,57 µs (mesma ordem; variação de cache); **com situação matemática recalculada por lote**, 3,1–3,3 µs (2026), 2,1–2,3 µs (2024 1º turno), 0,4 µs (2º turno) — critério < 10 µs; eleição geral inteira em memória com 28 threads **0,119 s** (critério < 1 s). Replay 60× de 2026 com publicação JSON e matematicamente: latência p50 5,1 ms, p99 12,8 ms.
- **Cobertura** (`scripts/cobertura.sh`, só testes sem `--ignored`): motor **84,3 %** das linhas e 81,9 % das funções (90 % das linhas sem o binário `main.rs`); `matematica.rs` e `situacao.rs` 100 %. `uv-aquisicao` 9,9 % (o núcleo saiu de lá). Novo `tests/publicacao.rs`: publicação em pasta e conferência dos arquivos, publicação sem cadastro, noite e antecedência sobre o TSE, retomada de pacote com registro incompleto.
- Documentação: [ARCHITECTURE.md](ARCHITECTURE.md) §2.5 (o crate, módulos e fluxo) e tabela do layout da região atualizada; [motor/README.md](motor/README.md); [README.md](README.md) com os comandos reais do binário (a seção antiga descrevia comandos que não existem); `CLAUDE.md`: a verificação contra os oficiais passa a ser pelos testes automatizados (os scripts Python ficam como conferência secundária); ROADMAP com a etapa 1 ✅ e a cobertura de cargos com a situação "no motor".
- Próxima etapa: **0.2.0, catálogo pré-eleição** (cadastro e catálogo montados antes do primeiro BU, a partir do site do TSE, para o pleito 3221).

## [0.0.27] — 2026-10-12

Etapa 1, passo 5: **formato JSON do resultado, documentado e usado pelo replay**, e o binário `urna-velox totalizar | conferir`.

- [docs/arquitetura/FORMATO_RESULTADO.md](docs/arquitetura/FORMATO_RESULTADO.md) (versão 1): um JSON por (eleição, cargo, abrangência), `<abr>-c<cargo>-e<eleição>.json` — `v`, `eleicao`, `turno`, `cargo`, `abr`, `circ` (circunscrição), `seq`, `ultimo_bu`, `encerrada`, `secoes {total, apuradas}`, `eleitorado`, `apurado`, `comparecimento`, `vb`, `vn`, `vnt`, `validos`, `legenda`, `estado` (`apurando`, `matematicamente_eleito`, `matematicamente_2t`, `definido`, `sem_eleito`) e `cand[] {n, v, pct, dvt, st, mat}`; a situação final (`st`) sai só na publicação encerrada; durante a apuração vale `mat` (estado, sequência e horário). É o contrato do WebSocket (etapa 4) e do site (etapa 5). Eventos do "matematicamente" em CSV (`replay_eventos.csv`).
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

Etapa 1 ([handoff](docs/handoffs/HANDOFF_0.1.0.md)), passo 1: **workspace Cargo e crate `urna-velox`** (motor), sem mudança de comportamento.

- `Cargo.toml` na raiz com o workspace (`motor`, `aquisicao`; `bench/rust` fica de fora), `profile.release` único e **um `Cargo.lock` só**, na raiz (o de `aquisicao/` foi movido).
- Novo crate **`urna-velox`** em `motor/` (biblioteca `urna_velox` + binário `urna-velox`, que por ora só mostra a versão e inspeciona um BU): `bu.rs` e `pacote.rs` movidos sem mudança; `totaliza.rs` dividido em `catalogo.rs` (catálogo, `cargo_municipal`, `ColetaCandidatos`, layout das regiões) e `estado.rs` (estado denso, agregador, `totalizar_lote`, CSVs de totais); `chave(município, zona, seção)` passa a ser do motor (`tse::chave` a reexporta).
- `uv-aquisicao` (`uv-coletor`) depende do motor (`urna-velox = { path = "../motor" }`); coletor, `tse`, `zip`, `replay` e `main` ficam na aquisição.
- `scripts/versao.sh` (manifesto `motor/Cargo.toml`, pacotes `urna-velox` e `uv-aquisicao` no `Cargo.lock` da raiz), `.githooks/pre-commit`, `scripts/cobertura.sh` (inclui o motor) e `scripts/testar_versao.sh` atualizados; `compilar_dev.sh` compila e testa o workspace inteiro.
- Novo `aquisicao/scripts/conferir_tres.sh`: as três conferências (catálogo → totalização → comparação oficial) com um binário (`UV_BIN`), em 17 s.
- **Regressão** — binário da `v0.0.22` (compilado da tag) × binário novo, mesmos dados: **2026 83/83 abrangências** e 794 situações idênticas; **2024 1º turno 5.569/5.569** municípios (15.032 situações) e consultas 5/5; **2024 2º turno 51/51** (102 situações). Checksums do estado iguais (2026 `6122989966576576`, 2024-1t `410969429627600`, 2024-2t `143536550724`); `catalogo.txt`, `totais_uf.csv` e `totais_municipio.csv` **idênticos byte a byte** nos três conjuntos. Conferência do estado sem alarmes (0 cargos inválidos, desconhecidos, duplicados, divergentes, erros).
- Desempenho, uma thread (sem mudança esperada; ruído de cache): 2026 2,18 µs/BU (antes 2,86), 2024-1t 1,57 µs/BU (antes 1,75).

## [0.0.22] — 2026-10-12

- [docs/handoffs/HANDOFF_0.1.0.md](docs/handoffs/HANDOFF_0.1.0.md): decisões do Luciano registradas — crate em `motor/` (pacote e binário `urna-velox`), fixtures até 5 MB, e no "matematicamente" o empate é decidido **pela idade, como manda a lei** (custo de ultrapassagem `v_i − v_j` se o adversário é mais idoso, `+1` se não é; sem data, vale o caso do adversário mais idoso).
- Handoff reescrito com **caminhos absolutos** (repositório no Windows e no WSL, dados em `/home/dev/uvreal`); passos do plano renumerados para `0.0.23`–`0.0.27` antes da `0.1.0`.

## [0.0.21] — 2026-10-12

- Versão 0.0.20 ratificada: `VERSION`, manifestos, `Cargo.lock`, tag `v0.0.20` e `origin/main` coerentes (`scripts/versao.sh verificar`). Nenhuma mudança de código ou de resultado.
- [docs/handoffs/HANDOFF_0.1.0.md](docs/handoffs/HANDOFF_0.1.0.md): plano da etapa 1 (motor, `0.1.0`, alvo 14/10) — workspace com o crate `urna-velox`, cadastro (destinação, nascimento, eleitorado, vagas, turno), situação final e **matematicamente eleito / matematicamente 2º turno** com as fórmulas, formato JSON por abrangência, fixtures e testes de integração que reproduzem as conferências (83/83, 5.569/5.569, 51/51), passos com versões e critério de pronto. Link no ROADMAP.
- `CLAUDE.md`: `versao.sh bump patch` sem resumo na linha de comando (o resumo vai no CHANGELOG), como já era feito.

## [0.0.20] — 2026-10-12

- Roadmap reorganizado em etapas com datas: 0.1.0 motor (situação, matematicamente eleito/2º turno, formato JSON), 0.2.0 catálogo pré-eleição, 0.3.0 operação automatizada (`urna-velox servir`), 0.4.0 HTTP + WebSocket, 0.5.0 site de exemplo, 0.6.0 ensaio geral e runbook.
- 1.0.0 passa a ser o sistema completo para cargos majoritários (automatizado, resultado por WebSocket, site de exemplo), com meta 23/10/2026 para acompanhar o 2º turno de 2026; cargos proporcionais vão para a 2.0.0.
- Usar o sistema em eleição real não muda a versão; a regra vale também em `docs/processo/VERSIONAMENTO.md` e `CLAUDE.md`.

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
- **Aquisição** ([AQUISICAO §1.1.1](docs/arquitetura/AQUISICAO.md)): 478.026 BUs dos zips de dados abertos no acervo local (65,1 GB, 46,7 min a 23,3 MB/s, uma requisição por UF); `aux.json` de 500.341 seções em 1 h 33 min a 89,7 req/s, 0 HTTP 429, para o horário de recebimento.
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
- **Guia de Licenciamento ([`docs/legal/LICENCIAMENTO.md`](docs/legal/LICENCIAMENTO.md)):** Dupla licença e canal oficial de contato.
- Atualização da seção de licença no [`README.md`](README.md) e apontamento `license-file = "../LICENSE"` nos manifestos.

## [0.0.16] — 2026-10-12

Eleições municipais de 2024, passos 1 e 2 do [handoff](docs/handoffs/HANDOFF_2024.md): formato levantado e **cargos de circunscrição municipal** no totalizador.

- **Levantamento de 2024** ([AQUISICAO §1.1.1](docs/arquitetura/AQUISICAO.md)): ciclo `ele2024`, pleitos 452 (1º turno) e 453 (2º turno), eleição 619 (Prefeito e Vereador) e 5 consultas populares (633–637); `bu.asn1` de 2024 estruturalmente igual ao de 2026; resultado oficial por município (`<uf><mun>-c0011-e000619-u.json`). O leitor de BU não precisou mudar.
- **Catálogo por município** para Prefeito, Vereador e consultas (cargos 11, 13 e 25..99): o mesmo número é um candidato diferente em cada município. Linhas `K <município> <cargo> …` no `catalogo.txt`, busca binária na lista do município, consolidação por UF somando por número (`Estado::somar_na_uf`, usada também pelo replay). Catálogos antigos (sem `K`) continuam com circunscrição estadual. Teste de unidade novo (`cargo_municipal_por_municipio`).
- `uv-coletor totalizar --totais-municipio` (uf, município, cargo, tipo, código, votos).
- Scripts: `baixar_oficial_municipal.py` (arquivos oficiais por município, teto de 15–20 req/s, disjuntor no primeiro 429, retoma), `comparar_municipal.py` (votos, brancos, nulos, nulos técnicos, comparecimento, seções e situação de Prefeito — 2º turno só acima de 200 mil eleitores — e das consultas); `coleta_nacional.sh` aceita `UV_CICLO`, `UV_PLEITO`, `UV_UFS`, `UV_DADOS`, `UV_SAIDA`; `totalizar_comparar.sh` aceita `UV_BIN`.
- **Conferência — Prefeito, AC, 1º turno de 2024** (2.118 BUs do acervo local, sem rede): **22 de 22 municípios idênticos** ao oficial e a situação dos 60 candidatos idêntica. Estado de 6.670 contadores, contra 19.800 com as listas por UF.
- **Regressão de 2026:** Presidente, Governador e Senador seguem idênticos em **83 de 83 abrangências**, situação de 794 candidatos idêntica; `totais_uf.csv` com as mesmas linhas do da `v0.0.13`.

## [0.0.15] — 2026-10-12

- [`docs/handoffs/HANDOFF_2024.md`](docs/handoffs/HANDOFF_2024.md): handoff para a próxima etapa — totalizar Prefeito e consultas populares das eleições municipais de outubro de 2024 (marco dos cargos majoritários, Fase 2), com contexto, regras, ambiente, plano e critério de pronto.
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
