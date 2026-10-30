# Handoff — Urna Velox: etapa 1.1, multipleito (`1.1.0`)

*Preparado em 29/10/2026, ao fim da sessão que fez o merge do branch `claude/multipleito-1.1` (`1.0.35`), corrigiu os
bugs vistos nos testes (`1.0.36`) e implantou o multipleito nos dois nós da OCI (`1.0.37`). **Ponto de partida: a
`1.0.x` mais recente** (`git fetch` + `git log`). Quem continua fica em PATCH (`1.0.x`); o `bump minor` para `1.1.0` só
quando o critério da etapa no [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md) estiver cumprido e o Luciano
concordar. Para iniciar a sessão, cole este documento ou peça para lê-lo, e leia antes o
[CLAUDE.md](/mnt/d/git/lukeboh/urna-velox/CLAUDE.md).*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` (Windows) = `/mnt/d/git/lukeboh/urna-velox`
(WSL `Ubuntu-24.04`: scripts, cargo); dados = `/home/dev/uvreal`; acervo de dados abertos =
`/mnt/d/dados/tse-arquivos-urna`; dados por pleito = `/mnt/d/dados/urna-velox/pleitos/<AAAA-MM-DD-código>`; evidências
arquivadas = `/mnt/d/dados/urna-velox/ensaios`; temporários de roteiros = `/mnt/d/dados/uv-tmp`. Implantação:
`/mnt/d/git/lukeboh/urna-velox/deploy/oci` (sem SSH: [HANDOFF_DEPLOY_SEM_SSH.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_DEPLOY_SEM_SSH.md)).
Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_1.1.0.md`.

## 0. Etapa concluída: `1.1.0` (30/10/2026)

**O Luciano deu a etapa por concluída em 30/10/2026, 17h04** ("considere que a versão 1.1 está completa"). Versão
`1.1.0`, tag `v1.1.0`, implantada nos dois nós. O que fechou a etapa depois deste documento: o cabeçalho novo com o
seletor de pleito (M-025, `1.0.62`) e, na própria `1.1.0`, B-089 (o letreiro preso em "pausado" no celular), M-026 (a
seta da escolha da eleição maior) e M-027 (o letreiro esmaece nas bordas). **Ficam para depois, fora do critério:** a
prova do modo em curso com um pleito real (suplementares de dezembro de 2026), guardar as parciais do TSE vistas numa
noite ao vivo no acervo de versões e o `60_simulado.sh` como "totalizar de novo um pleito encerrado". O resto deste
documento fica como histórico da etapa.

## 1. A visão (alinhada com o Luciano em 29/10/2026)

* **Motor: um pleito por vez.** Só em dois casos ele totaliza: a **noite ao vivo** (pleito em curso no `ele-c.json`) e o
  **replay** — a totalização refeita, no ciclo de pleitos da **playlist** (votos, rotação a cada meia hora redonda).
* **Visitante: qualquer pleito, a qualquer momento, com toda a riqueza de uma eleição totalizada.** Todo pleito
  encerrado é consultável independente do que está no ar: **arquivos estáticos consolidados ao longo da noite** (uma
  fotografia por minuto; nos pleitos com poucos lotes, uma por lote, para não haver passo vazio), servidos **só por
  cache/CDN** — continuam abertos durante uma noite ao vivo, sem custo para os nós.
* **Slider de cada visitante** (cada um com a sua linha do tempo; diferente do replay de hoje, que é um só para todos):
  play, pausa, reposicionar, acelerar; ele comanda **tudo** na tela — contadores, cartões, mapas, curva, marcos,
  "matematicamente", selos (art. 224 etc.) e a coluna do TSE. No **pleito em curso**, do início da noite até o último
  minuto consolidado, com um botão "ao vivo" que volta ao tempo real.
* **TSE lado a lado em todo o slider:** com as **parciais oficiais de divulgação** ao longo da noite, quando disponíveis
  (quando houver); sem elas, a coluna do TSE
  é **reconstruída pela hora de recebimento de cada BU** (`aux.json`; é o "TSE simulado" que já existe no
  `uv-simulador --tse-simulado`), e a tela diz que é reconstrução.
* **Navegação:** cada pleito, abrangência e cargo tem **endereço próprio e compartilhável**; um **seletor de pleito** no
  topo absorve o painel "Pleitos do TSE" de hoje (em curso, agendados com "faltam N dias", encerrados, cargos, consultas
  populares, cidade/UF, filtro). O painel deixa de ser uma janela à parte.

## 2. Onde estamos (`1.0.37`, 29/10/2026)

Entregue e **no ar nos dois nós** (detalhe no [CHANGELOG](/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md), `1.0.35` a
`1.0.37`):

| item do ROADMAP | situação |
|---|---|
| Descoberta (`uv-coletor pleitos`, `pleitos.json`, `GET /api/v1/pleitos`) | ✅ no ar: 43 pleitos, `uv-pleitos.timer` (15 min) |
| Acervo automático (`montar_acervo.sh`, `uv-pleitos.sh`) | ✅ no ar: 2 acervos por rodada, 0 HTTP 429; prova `replay_acervo.sh` idêntica ao oficial (3080, 3237, 3239…) |
| Playlist e ciclo dinâmicos (`uv_pleitos.py`) | ✅ no ar: suplementares votáveis e na rotação, troca a cada meia hora redonda |
| Página: painel "Pleitos do TSE" | ✅ no ar (vai ser absorvido pelo seletor de pleito: §3.3) |
| B-070 (art. 224 em todo cargo majoritário) | ✅ corrigido na `1.0.35`; o 3237 na produção: 1/1 arquivo e 3/3 situações |
| Em curso (modo `aovivo`, vigia) | 🟡 implantado; falta a prova com um pleito em curso de verdade (os suplementares de dezembro) |
| Agendados | ⬜ |
| Operação (`no.env` sem `PLEITO`; `60_simulado.sh`) | ⬜ (hoje o `no.env` dos dois nós ainda tem `PLEITO=3221`; o modo `aovivo` põe o pleito por cima) |
| Consulta de qualquer pleito (slider), lado do TSE no slider, seletor de pleito | ⬜ (visão de 29/10: §3) |

Bugs: B-071 a B-075 corrigidos na `1.0.36`, B-076 descartado; **B-077 aberto** (destino `1.0.x`: o chip do ritmo da
faixa troca de lugar no computador durante a apuração; CLS pequeno, abaixo do limite dos testes; mexer nele é mudar o
layout da faixa — o Luciano decide quando).

## 2b. Noite de 29 para 30/10/2026 (`1.0.42` a `1.0.46`): série de consulta e slider

Trabalho autônomo autorizado pelo Luciano ("pode continuar a noite toda… tome as decisões… amanhã eu reviso").
**Implantado nos dois nós em 30/10/2026, de manhã (`1.0.47`)**, depois de a outra sessão terminar: 38 pleitos com
série em cada nó (os 5 gerados aqui + 33 suplementares gerados pelo passo 3 do `uv-pleitos.sh`), conferido em
`https://urnavelox.com.br/#/p/3220` etc. Na implantação, o nó B estava fora da rotação desde 29/10 21h30 (B-080):
realinhado com `63_ciclo.sh --iniciar 3220 cadencia`. Os passos abaixo ficam como roteiro (a ação nova
`63_ciclo.sh --consultas` leva as séries geradas fora dos nós).

| item | versão | situação |
|---|---|---|
| §3.1 série de consulta: acervo de versões no `uv-servico` (toda versão de todo arquivo publicado, com a hora da noite; `--serie`, `--sem-serie`) e o `uv-serie` (índice, um tópico por arquivo com a última versão de cada minuto, mapa, marcos, eventos, previsões, nomes, tópicos; só grava se a última versão = a publicação final) | `1.0.43` | ✅ local; proposta em [PROPOSTA_SERIE_CONSULTA.md](/mnt/d/git/lukeboh/urna-velox/docs/propostas/PROPOSTA_SERIE_CONSULTA.md) (formato aprovado) |
| §3.2 lado do TSE na série (`tse/<tópico>.json`, como o `dados` da mensagem `tse`): das parciais oficiais (`--tse-parciais`) ou reconstruído pela hora de recebimento (`--tse-dados --linha-do-tempo`, marcado `reconstrucao`) | `1.0.44` | ✅ local |
| §3.3 endereço por pleito `#/p/<pleito>/<abr>/c<cargo>` e "Consultar a noite" no painel dos pleitos | `1.0.45`, `1.0.50` | ✅ seletor de pleito no 1º chip da faixa, que abre a lista dos pleitos; B-077 corrigido junto (`1.0.50`, decidido sem o Luciano: revisar) |
| §3.4 slider de cada visitante (`site/src/serie.js`: `ConexaoSerie` + a barra do replay com relógio local), comandando todos os componentes e a coluna do TSE | `1.0.45`, `1.0.50`, `1.0.51` | ✅ "● ao vivo" no pleito em curso (série refeita pelo vigia a cada 2 min, `uv-serie --em-curso`; `1.0.50`); T14 achou o B-081 (o 452 travava no celular), corrigido na `1.0.51` |
| §3.5 `uv-servico --consulta <pasta>` serve `/consulta/*` (gzip pré-comprimido, cache 5 min / 1 h) e `aquisicao/scripts/gerar_consultas.sh` | `1.0.45`, `1.0.47`, `1.0.49` | ✅ implantado; `dados.urnavelox.com.br/consulta/*` pela Cloudflare (conferido em 30/10: `cf-cache-status: HIT`, 5 min no índice, 1 h no resto; 30 s num pleito em curso) |
| §3.6 agendados e operação | `1.0.50` | ✅ catálogo pré-eleição dos agendados sozinho (`uv-pleitos.sh`, passo 2b); `no.env` sem `PLEITO` nos dois nós desde 30/10 (cópia em `/etc/urna-velox/no.env.antes-sem-pleito-*`); o `60_simulado.sh` ficou como está (revisar) |

**Séries geradas nesta máquina** (`D:\dados\urna-velox\consulta\<pleito>\serie`, índice em `D:\dados\urna-velox\consulta\indice.json`;
é a árvore que vai para `/srv/urna-velox/consulta` nos nós):

| pleito | tópicos | instantes | série comprimida | última versão × publicação final | lado do TSE (última versão × `-u.json` final do acervo) |
|---|---|---|---|---|---|
| 452 (1º turno de 2024) | 5574 | 275 | 21.73 MB | 5574/5574 | reconstrucao: 5574 tópicos, 5574 idênticos |
| 453 (2º turno de 2024) | 51 | 173 | 0.67 MB | 51/51 | reconstrucao: 51 tópicos, 51 idênticos |
| 3220 (1º turno de 2026) | 83 | 912 | 6.37 MB | 83/83 | parciais: 83 tópicos, 79 idênticos, divergentes: am-c0003-e006259, am-c0005-e006259, rj-c0003-e006259, rj-c0005-e006259 |
| 3221 (2º turno de 2026) | 45 | 207 | 1.84 MB | 45/45 | parciais: 45 tópicos, 44 idênticos, divergentes: rr-c0001-e006258 |
| 3237 (suplementar de Governador de RR) | 1 | 146 | 0.03 MB | 1/1 | reconstrucao: 1 tópicos, 1 idênticos |

**Para implantar (sugestão; com o Luciano):**
1. `deploy/oci/servidor/compilar.sh` já inclui o `uv-serie`; `63_ciclo.sh --instalar` leva `replay_acervo.sh` e
   `gerar_consultas.sh` e cria `/srv/urna-velox/consulta`.
2. Atualizar os nós (`50_atualizar.sh`): o `uv-iniciar.sh` e o `uv-ciclo.sh` passam `--consulta /srv/urna-velox/consulta`
   quando a pasta existe.
3. As séries: copiar `D:\dados\urna-velox\consulta` para `/srv/urna-velox/consulta` dos dois nós, ou deixar o passo 3 do
   `uv-pleitos.sh` gerar sozinho a partir dos acervos de lá (com `nice`, nunca na noite de uma eleição nem no dia
   seguinte; um geral leva ~10 min a 500×). Os acervos dos nós têm os suplementares; os gerais estão só no nó A.
4. Caddy/CDN: a rota `/consulta/*` para o `uv-servico` (ou um `file_server` com `precompressed gzip` na mesma pasta), com
   cache na borda. Conferir com `site/testes/consulta_pagina.mjs` adaptado para a URL real.

**Decisões tomadas nesta noite (para revisar):**
* Formato da série (aprovado): uma série por tópico + arquivos de eventos com hora, em vez de fotografia + diferença.
* O eixo é a hora da noite (`ultimo_bu`/recebimento no TSE); a 500× um lote pode andar mais de um minuto (no 3221,
  209 instantes para 298 minutos com chegada): a série de um replay mais lento ou da noite ao vivo é mais fina.
* Lado do TSE: parciais oficiais quando há; senão, o TSE simulado com 1 s de atraso a partir do recebimento,
  marcado como reconstrução; as parciais de zona ficam de fora (o Urna Velox não publica zona).
* A consulta abre parada no fim da noite (o resultado final); tocar no fim recomeça do início; voltar no tempo
  recomeça a página e reentrega tudo até o instante (o letreiro e o histórico são limpos).
* O selo diz "CONSULTA"; o aviso leva de volta ao que está no ar. A previsão não é reproduzida no slider (o cartão fica
  "aguardando base").

**Achados:**
* O `-u.json` "oficial" do RR no acervo do 3221 (`D:\dados\urna-velox\pleitos\2026-10-25-3221\acervo\oficial`) é de antes
  do fim da totalização (geração 22:45:56, `tf = n`); a última parcial (23:26:57, `tf = s`) é a final. Vale trocar o
  arquivo do acervo pelo final.
* No 3220, o lado do TSE (parciais da noite) tem 4 tópicos diferentes do `-u.json` do acervo: Governador e Senador do
  AM e do RJ (não analisado). Antes do limite "só a noite" eram 23: as parciais trazem regenerações de 15/10 e 24/10.
* Tempo de geração nesta máquina (500×): suplementar < 30 s; 453 ~4 min; 3221 ~3 min; 3220 ~12 min; 452 ~27 min (a
  maior parte gravando ~22 mil arquivos pequenos no D: pelo WSL; nos nós, com disco local, bem menos). A reconstrução do
  TSE só guarda versão quando os números mudam (sem isso, o 452 passava de 7 GB de memória).
* Na consulta de um geral, a primeira abertura baixa o mapa (~620 KB) e os marcos (o 452 tem 12 mil); falta medir no
  celular (T14).
* Outra sessão publicou a `1.0.42` (B-078, B-079) e removeu a rota `/painel/qualidade` do Caddyfile (`bd728c6`), embora
  o Luciano tivesse dito para deixar como está.

## 3. O que falta para a `1.1.0`

Ordem sugerida (o Luciano decide): 3.1 e 3.2 primeiro (são a base do slider e não mexem na página), depois 3.3 a 3.5,
por fim 3.6 e 3.7. Cada passo com versão PATCH, CHANGELOG e testes (regras do CLAUDE.md); o que mexe em totalização ou
resultado, conferido contra os oficiais.

### 3.1 Fotografias consolidadas de cada pleito (a base do slider)

* **O que:** para cada pleito totalizado (noite ao vivo e replay; para os encerrados, uma vez a partir do acervo), uma
  série de fotografias do estado publicado — tudo o que a página mostra (os tópicos de resultado de cada abrangência e
  cargo, o estado/saúde que a tela usa, marcos, curva, mapa das seções, "matematicamente", previsão) — **por minuto da
  noite**, ou **por lote** quando o pleito tem poucos lotes de totalização (suplementares com 1, 2, 3… lotes).
* **Proposta para decidir** (antes de implementar, uma proposta curta em `docs/propostas/` para o Luciano): base +
  diferenças (uma fotografia completa a cada N minutos e, entre elas, só o que mudou), em arquivos estáticos por pleito
  (`/<pleito>/serie/indice.json` com os instantes e `/<pleito>/serie/<instante>.json`), comprimidos, com cache longo
  (imutáveis depois do encerramento). Medir o tamanho numa geral (3220: ~600 minutos × 45 abrangências × cargos) e num
  suplementar; a meta é a CDN servir tudo sem os nós.
* **Onde nasce:** o `uv-servico` já tem tudo o que publica e a hora de cada BU; o mais simples é ele gravar a série
  enquanto totaliza (o replay rápido do acervo gera a série de um encerrado em segundos ou minutos), com a hora **da
  noite** (`hora_site`) como eixo, não a do sistema.
* **Conferência:** a última fotografia de cada pleito idêntica ao oficial (a mesma conferência de hoje,
  `conferencia.json`); a regra do CLAUDE.md vale.

### 3.2 Lado do TSE na série

* **Parciais oficiais de divulgação, quando disponíveis**, em
  `/mnt/d/dados/urna-velox/pleitos/<AAAA-MM-DD-código>/tse-parciais/<item>/` (`lote-*.jsonl.gz`: uma linha por versão
  de cada arquivo de divulgação, com a hora da geração, o SHA-256 e o conteúdo). 2024 e os suplementares não têm
  parciais guardadas (só a versão final): para eles, a reconstrução abaixo.
* **Regra nova (Luciano, 29/10/2026): o sistema guarda toda versão de todo arquivo que vê ou publica** — as parciais do
  TSE baixadas durante uma noite ao vivo (a comparação) e as publicações do próprio Urna Velox —, num acervo só de
  acréscimo com a hora de cada versão, ainda que a série das consultas seja processada depois. Assim os próximos pleitos
  não dependem de outra fonte.
* Com as parciais oficiais (quando disponíveis): a coluna do TSE de cada instante sai delas.
* Sem elas: reconstrução pela hora de recebimento de cada BU no TSE (`aux.json`, a mesma do `tse_simulado.rs`),
  marcada como reconstrução na série e na tela.
* Os dois casos conferem no fim com o `-u.json` final oficial.

### 3.3 Seletor de pleito e endereço por pleito (página)

* O pleito entra na URL (hoje `#/<abrangência>/<cargo>`; ex.: `#/p/3220/br/c0001`), compartilhável; sem pleito na URL,
  a página abre no que está no ar (ao vivo ou o replay da vez).
* Seletor no topo com o que hoje está no painel "Pleitos do TSE" (`site/src/pleitos.js`): grupos (em curso, agendados
  com "faltam N dias", encerrados), cargos, consultas, cidade/UF, filtro, o "no ar" em destaque. O painel deixa de ser
  janela à parte.
* Regras da página (site/README.md): nada pula (CLS < 0,01), celular a 360 px, os dois temas, testes em `site/testes/`.

### 3.4 Slider de cada visitante

* Play, pausa, reposicionar, acelerar; comanda todos os componentes (contadores, cartões, mapas, curva, marcos,
  "matematicamente", selos, coluna do TSE). Reaproveitar a barra "Replay" de hoje (`site/src/replay.js`), que hoje
  controla o replay do servidor para todos: no slider, a mesma interface, mas local (só o visitante).
* No pleito em curso: até o último minuto consolidado; botão "ao vivo" volta ao tempo real (WebSocket).
* Desempenho no celular (o T14 da validação: não esquentar, não ficar lento).

### 3.5 Servir pela borda

* Os arquivos da série atrás do cache/CDN (o subdomínio `dados.urnavelox.com.br` já existe; ver
  [CHECKLIST_CLOUDFLARE.md](/mnt/d/git/lukeboh/urna-velox/docs/operacao/CHECKLIST_CLOUDFLARE.md)); durante uma noite ao
  vivo, nada da consulta pode chegar aos nós. Ensaio de carga como os de `scripts/teste_carga*`.

### 3.6 Agendados e operação

* **Agendados:** na página (no seletor: data, cargos, abrangência, "faltam N dias"); catálogo pré-eleição
  (`uv-coletor preparar`) montado sozinho quando os `cs.json` saírem.
* **Operação:** `no.env` sem `PLEITO` obrigatório (os dois nós hoje têm `PLEITO=3221`); o `60_simulado.sh` vira só
  "totalizar de novo um pleito encerrado".

### 3.7 Prova do pleito em curso e critério da etapa

* Os suplementares de dezembro de 2026: quando aparecerem no `ele-c.json`, em até ~15 min eles entram em "Agendados";
  no dia, o vigia troca sozinho o replay pelo modo `aovivo` (autorizado pelo Luciano; regra no CLAUDE.md). Conferir os
  testes T10 a T12 da validação (§4).
* Critério (ROADMAP): um nó sem `PLEITO` mostra 2024, 2026 e os suplementares (encerrados idênticos ao oficial), um
  agendado e um em curso reproduzidos pelo simulador, um pleito novo entra sozinho, e **todo encerrado é consultável no
  slider, com o lado do TSE, e o resultado final no slider é idêntico ao oficial**.

## 4. Validação com o Luciano (Fase 4 do prompt de 29/10) — em andamento

Roteiro dos testes T1 a T14 no prompt da sessão (página, celular, playlist, rotação, suplementar, B-070, art. 224, ao
vivo, painel, desempenho). Resultado até agora:

| teste | resultado |
|---|---|
| T1 painel "Pleitos do TSE" no computador | passou |
| T2 o mesmo no celular | passou |
| T3 "▶ na playlist" / "replay em preparo" | passou |
| T4 suplementares votáveis na /playlist | passou |
| T5 votado entra na meia hora seguinte | pendente (às 19h30 entrou o 3230 da rotação; confirmar com o Luciano se houve voto) |
| T6 rotação de 30 min sem votos, nós sincronizados | pendente de confirmação (a rotação trocou na meia hora redonda, nós sincronizados, às 16h00, 16h30, 17h30 e 19h30) |
| T7 replay de suplementar municipal (o 3226, Oiapoque/AP, estava no ar das 17h30 às ~19h10) | pendente (resposta do Luciano) |
| T8 B-070 na tela (3237) | não deu para testar (o resultado ficou na tela só das 17h24 às 17h30); na produção, 1/1 e 3/3 |
| T9 link do art. 224 (Chrome, Safari, Firefox) | não deu para testar |
| T10 a T12 pleito novo, dia da eleição, fim da noite ao vivo | pendentes até os suplementares de dezembro |
| T13 painel dos nós | pendente |
| T14 desempenho no celular | pendente |

No fim: `docs/relatorios/VALIDACAO_1.1_IMPLANTACAO_<AAAA-MM-DD>.md` (tabela teste × resultado × observação), no índice
`docs/README.md`, cada falha no `docs/BUGS.md` só com o sintoma e destino; os de dezembro como pendentes no relatório e
no ROADMAP. O `site/testes/playlist_pagina.mjs` roda contra os nós (ainda não rodou desde a `1.0.35`).

## 5. Pendências de decisão

* **`deploy/oci/servidor/Caddyfile.template`, rota `/painel/qualidade`:** apareceu às 15h28 de 29/10 no repositório e
  nos dois nós sem vir desta sessão (provavelmente outro agente na máquina do Luciano); entrou por engano no commit da
  `1.0.37`. Inofensiva por ora (atrás do login do painel; `/srv/urna-velox/qualidade` não existe). O Luciano decide se
  fica ou sai. Evitar dois agentes no mesmo repositório e nos mesmos nós ao mesmo tempo.
* **B-077** (layout da faixa): quando atacar.
* **Proposta da série (§3.1):** formato, passo e tamanho, antes de implementar.

## 6. Lembretes de processo (CLAUDE.md)

* Cada commit sobe a versão; identidade `Luciano Bohnert <luciano.bohnert@gmail.com>`; `git push --follow-tags`.
* Bug: registrar só o sintoma com destino; a ordem é do Luciano; primeiro o teste que comprova, depois a correção.
* Totalização ou resultado: conferência contra os oficiais antes do commit (`UV_DADOS=/home/dev/uvreal cargo test
  --release -p urna-velox -- --ignored --test-threads 1`; sem `UV_OFICIAIS`, o `situacao_oficial` usa as pastas do
  `UV_DADOS` desde a `1.0.36`).
* TSE: ≤ 95 req/s por IP de saída nas coletas, pausa de 10 min num 429, nada de coleta na noite de uma eleição sem
  combinar (exceto o modo `aovivo` e a comparação, já combinados).
* Disco: `scripts/disco.sh` antes e depois de cada ensaio; ensaios em `~/uvreal/ensaios/<nome>`, arquivados depois.
* `replay_acervo.sh` usa por padrão o `target/release` do repositório: passe `UV_BIN` com o binário certo (a primeira
  linha da saída diz qual `uv-servico` rodou; B-076).
* Commits feitos do WSL no `/mnt/d` podem esbarrar num `index.lock` passageiro; repita o comando.
