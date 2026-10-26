# Site de exemplo — página de resultados do Urna Velox (etapa 5)

A página que o `uv-servico --http <end:porta>` serve em `http://<end:porta>/`: acompanha a apuração **só pelo
protocolo público** (`/mnt/d/git/lukeboh/urna-velox/docs/PROTOCOLO_WS.md`) e é também o modelo para quem quiser
integrar. Decisão de arquitetura: ADR-027 (`/mnt/d/git/lukeboh/urna-velox/docs/ARCHITECTURE_DECISIONS.md`).

## Onde fica cada coisa

| caminho | o quê |
|---|---|
| `/mnt/d/git/lukeboh/urna-velox/site/src/` | fontes: `index.html`, `estilo.css`, `app.js` (tela, inscrições, desenho por quadro), `conexao.js` (WebSocket e reconexão), `hodometro.js`, `graficos.js` (anel e curva), `feed.js` (letreiro de marcos e folha de detalhe), `historico.js` (painel com o que já passou pelo letreiro), `textos.js` (nomes e textos do feed), `cores.js` (cor de cada candidato a partir da do partido), `replay.js` (barra do replay: velocidade e linha do tempo), `comparacao.js` (comparação Urna Velox × TSE: controle, células do TSE e da diferença, estado da fonte), `previsao.js` (cartão da previsão do matematicamente e do fim da apuração), `mapa.js` (cartão "Mapa das seções": cada seção totalizada acende um ponto; vistas Brasil e Exterior (o globo); protocolo §4.8), `mapa.json` (a geografia do mapa, gerada por `site/geo/gerar_mapa.py`; ver `site/geo/README.md`), `cartoes.js` (todo cartão do painel se maximiza e se reorganiza; ordem em `uv-cartoes`), `util.js`, `icone.svg`; `documento.html`, `documento.css` e `documento.js` (molde das páginas de texto `/termos` e `/privacidade`, que o `site/documentos.mjs` gera no build a partir de `docs/LICENCIAMENTO.md`, `LICENSE`, `LICENCA.pt-BR.md` e `docs/PRIVACIDADE.md`: as regras ficam no próprio site, sem depender do GitHub; mudou um desses arquivos, rode `npm run build`) |
| `/mnt/d/git/lukeboh/urna-velox/site/dist/` | o que o binário embute (gerado por `build.mjs`, **versionado**): `index.html`, `app.js`, `estilo.css`, `archivo.woff2` (Archivo variável, latim, eixos de peso e largura; licença OFL em `LICENCA-FONTE.txt`), `icone.svg` |
| `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/pagina.rs` | tabela fixa dos arquivos embutidos (`include_bytes!`), CSP e `index.html` com a versão |
| `/mnt/d/git/lukeboh/urna-velox/site/testes/` | ensaios: `rodar.mjs` (Playwright/Chromium: acompanha um replay, mede CLS, fps, tarefas longas, memória, reconexão, e lê os números da tela), `conferir.py` (tela × `-u.json` oficiais), `conferir_marcos.py` (`marcos.csv` × `secoes.log`/`eventos.csv`), `cores.mjs` (todo partido de um `candidatos.json` tem cor e nenhuma disputa repete cor: `node testes/cores.mjs <candidatos.json>`), `comparacao.mjs` (N abas da comparação com o TSE em modos variados; no fim, tela × `-u.json` oficial nas duas colunas, CLS, marco "TSE confirmou"; usado por `aquisicao/scripts/ensaio_comparacao.sh`), `sonda.mjs` (depuração), `indisponivel.mjs` (B-008: uma URL abre como página HTML no Chromium e no Firefox, nunca como download; usado por `deploy/oci/testes/pagina_indisponivel.sh`, que precisa de `npx playwright install firefox`), `carga_pagina.mjs` (B-014: a página como celular numa disputa durante um replay; mensagens e bytes por minuto, tópicos de resultado recebidos, tarefas longas, capturas a 390 e 1366 px; usado por `aquisicao/scripts/ensaio_carga_pagina.sh`), `ws_recusado.mjs` (B-026, sem servidor real: `/ws` recusando com 503 → modo HTTP, consulta do tópico da tela e do estado com `If-None-Match`, aviso, volta ao WebSocket; `node testes/ws_recusado.mjs`), `visual_pagina.mjs` (B-029 a B-047, sem servidor real: dados de tamanho real em 12 larguras de 360 a 1920 px, nos dois temas e nos modos "Urna Velox" e "Ambos"; mede caixas — separadores na linha de base dos dígitos, nada da linha do candidato por cima de outra coisa, nome à vista ao lado da pílula, cartões e textos dentro das margens; `node testes/visual_pagina.mjs`), `preposicoes.mjs` (B-042: "na Bahia", "no Paraná"; sem navegador), `antes_17h.mjs` (B-050 e o aviso "Oficial às 17h": BUs do exterior de madrugada, sem servidor), `playlist_identidade.mjs` (B-048: a página `/playlist`, `deploy/oci/servidor/uv-playlist.html`, com a identidade visual da apuração — tema, tokens, fonte, aurora, marca, vidro; sem servidor), `ritmo_pagina.mjs` (B-053, sem servidor: o chip de BUs/min no relógio do serviço, sem salto depois de uma pausa da conexão nem num replay que recomeça; `node testes/ritmo_pagina.mjs`), `mapa_pagina.mjs` (M-017, sem servidor: o mapa das seções carrega, acende pelas mensagens `mapa`, relê a fotografia em outra época, conta o exterior, troca de vista sem mudar o palco, maximiza e volta ao lugar, reorganiza pelo teclado e guarda a ordem; CLS 0 e capturas a 1366 e 390 px; `node testes/mapa_pagina.mjs`), `previsao_cartao.mjs` (B-052 e B-054, sem servidor: o cartão "Previsão" sempre com horário — "por volta de …" pelo ritmo atual quando a faixa é larga —, sem "00:00:00" esmaecido e com o relógio dentro do cartão a 390 e 1366 px; `node testes/previsao_cartao.mjs`) |

## Desenvolver

No WSL (`Ubuntu-24.04`):

```bash
cd /mnt/d/git/lukeboh/urna-velox/site
npm install                 # esbuild, a fonte (@fontsource-variable/archivo) e playwright
npx playwright install chromium   # só para os ensaios
npm run build               # src/ → dist/
bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh   # o binário embute dist/
```

Depois de mudar qualquer coisa em `src/`: `npm run build` e recompilar (o `dist/` vai no commit). Para ver: rode um
ensaio com o simulador e `--http 127.0.0.1:18080` e abra `http://localhost:18080/` no Chrome do Windows (o WSL2 encaminha
`localhost`). Navegação por URL: `#/br/c0001` (Presidente, Brasil), `#/sp/c0003` (Governador, SP),
`#/sp71072/c0011` (Prefeito, São Paulo).

Nada externo em tempo de execução: sem CDN, sem fonte remota, sem rastreamento; a CSP do servidor
(`default-src 'self'; connect-src 'self'`) bloquearia qualquer tentativa. Estilos só por arquivo e CSSOM
(`element.style`), nunca atributo `style` no HTML.

## Regras de desenho (pedidas, não opcionais)

* **Nenhum pulo na tela (CLS = 0):** toda mudança por `transform`/`opacity`; caixas com tamanho reservado (a lista de
  candidatos ganha a altura certa antes de entrar no documento; faixa de alertas, selo, cartões e feed têm altura fixa;
  texto que muda fica no último item da linha ou em caixa de largura fixa); troca de posição dos candidatos por
  `translateY` (cada linha é absoluta, na posição do seu lugar); fonte embutida esperada (`document.fonts`) antes do
  primeiro desenho dos números. A página mede o próprio CLS (`PerformanceObserver` `layout-shift`) em `window.__uv.cls`.
* **Hodômetro:** cada dígito é uma coluna que rola por `translateY`; largura reservada para o maior número possível
  (eleitorado da abrangência), zeros à esquerda invisíveis, separadores fixos. Valor novo no meio da animação só muda o
  alvo (transição CSS). **Cada dígito gira sempre no mesmo sentido**, como num hodômetro de verdade (de 7 para 2 passa
  por 8, 9, 0, 1, também quando o número diminui); os contadores regressivos (`regressivo: true`: eleitores que faltam
  para o matematicamente, no cartão e no feed) giram no sentido oposto. A coluna tem 0–9 três vezes e, quando o dígito
  sairia dela, volta 10 ou 20 posições sem transição (o mesmo dígito na cópia vizinha). Teste:
  `node testes/hodometro.mjs` (amostra cada coluna a cada quadro e confere que nunca anda contra o sentido; e, no
  modo enxuto, CLS 0 e o número na borda — à esquerda ou centrado — quando dígitos aparecem e somem).
* **Ritmo de desenho ≠ ritmo de mensagens:** a 60× chegam milhares de mensagens por segundo; cada uma só guarda o texto
  do último resultado do tópico (o JSON é interpretado quando a tela precisa) e o desenho acontece no máximo uma vez por
  `requestAnimationFrame`.
* **Glassmorfismo tátil:** cartões translúcidos com `backdrop-filter`, borda clara e sombra; inclinam de leve sob o
  ponteiro ou o dedo (só `transform`). **Brilho arco-íris** (`conic-gradient` girando por `@property --angulo`) só nos
  destaques: selo e linha do matematicamente eleito/2º turno e os marcos novos do feed (no máximo dois).
* **`prefers-reduced-motion`:** sem giro, sem rolagem dos dígitos (troca com *fade*), feed mostra um item por vez.
* **Celular:** 360 px sem rolagem horizontal; nomes estreitados pelo eixo de largura da fonte em vez de cortados.
* **Cor do partido:** cada candidato leva um tom da cor do seu partido (`src/cores.js`; cores de referência da
  Wikipédia, `Module:Political party`, consultada em 14/10/2026; partido sem cor própria usa a da federação). Da cor
  do partido vale só o matiz: luminosidade e croma são os da interface em cada tema (OKLCH), e candidatos da mesma
  disputa com matizes parecidos (vários vermelhos, vários azuis) recebem tons diferentes da própria cor, mais claros ou
  mais escuros, com distância mínima em OKLab. A cor é um `light-dark()`, resolvido pelo `color-scheme` do tema, e não
  muda durante a noite (escolhida na ordem do número). Partido sem matiz (UP, preto) vira grafite.

## Barra do replay (só no site simulado)

Quando `GET /api/v1/replay` responde (serviço com `--controle-replay` contra o `uv-simulador`), a página monta a barra
**Replay** (`src/replay.js`) **antes do primeiro desenho** e reserva o espaço dela (`:root.com-replay`, `--player-h`):
fica presa acima do letreiro, em largura inteira, e nada da página fica escondido embaixo dela nem pula quando ela
aparece. Como a barra de um vídeo: tocar/pausar, horário da simulação (interpolado entre as consultas, uma por
segundo), linha do tempo da noite com as horas marcadas e o alvo de uma busca, e a velocidade (1× a 300×). Soltar a
linha do tempo manda `POST /api/v1/replay/posicao` (PROTOCOLO_WS §11): num instante já visto o serviço retotaliza do
disco e a página mostra um véu ("Indo para 19:30: retotalizando…") até a sessão nova responder, e então se recarrega;
além do visto, a barra mostra "avançando até 21:00 · 63%". Toda aba percebe a sessão nova (`sessao` no `estado` e no
`GET`) e se recarrega. Simulação pronta: `aquisicao/scripts/simular_site.sh`.

## Curva da apuração

`Curva` (`src/graficos.js`): ao abrir (ou recarregar, ou depois de voltar no tempo no replay) a página pede
`GET /api/v1/curva/<topico>` (PROTOCOLO_WS §6) e desenha a noite inteira até ali; depois acrescenta os pontos que
chegam ao vivo. Duas escalas: a área das seções apuradas vai de 0 a 100 %; as linhas dos dois primeiros candidatos
usam uma escala própria, ajustada ao intervalo dos valores (rótulos à direita, pelo menos 4 p.p. de altura, sem contar
os minutos com menos de 1 % das seções, que oscilam demais) — na escala 0–100 os décimos do fim da noite pareciam
retas. Cada ponto guarda o % de todos os candidatos, então uma virada não troca as linhas de dono. Domínios com mola.

## Histórico do feed

O botão ☰ à esquerda do letreiro abre o painel **O que já passou** (`src/historico.js`): tudo o que entrou no feed
(marcos, eventos do matematicamente, primeiros eleitos), em ordem de BU aplicado, com filtro por categoria
(**Decisões**: eleitos, 2º turno, seção decisiva, primeiros eleitos; **Viradas**; **Andamento**: primeira e última
seção, % de seções, BU nº N, apuração encerrada), filtro por texto (nome, UF, município) e uma **linha do tempo
própria** (slider): arrastar percorre os itens e mostra o horário de cada um, sem mexer na apuração. Rolar a lista
move o slider; com o slider no fim, o painel acompanha os itens novos. Clicar num item mostra o detalhe ali mesmo.
Os itens fixos (eleitores que faltam, disputa mais apertada/folgada, comparecimento) mudam no lugar e não entram.
O painel é fixo e de altura fixa (nada da página pula), cria as linhas só quando aberto e guarda até 20 mil itens.

## Curiosidades (o que entrou)

Do servidor (mensagem `marco`, com histórico): primeira seção totalizada do Brasil, de cada UF e de cada circunscrição
municipal; seção que garantiu o "matematicamente" de cada circunscrição; eleitores que faltam para o
"matematicamente" (hodômetro decrescente no cartão da apuração e no feed: Presidente, as 8 disputas mais perto de se
decidir e a da tela); viradas de liderança (com a contagem); 25/50/75/90/99/100 % das seções do Brasil e de cada UF;
BU nº 10 mil, 50 mil, 100 mil…; última seção da noite. Da página (a partir de resultados e eventos): todo
"matematicamente eleito/2º turno" de candidato; primeiro governador, senador e prefeito matematicamente eleitos do país
e o primeiro prefeito de cada UF; primeira UF 100 % apurada; disputa mais apertada e mais folgada no momento;
maior e menor comparecimento por UF; ritmo da apuração (BUs por minuto); apuração encerrada.

## Comparação com o TSE (etapa 7)

Controle **Urna Velox · Comparar · TSE oficial** (M-015; a chave interna do meio segue `ambos`) no cartão da corrida, com o rótulo "De onde vêm os números", o ⓘ (folha com a explicação e um botão por fonte), uma frase de uma linha por modo e a dica da primeira visita (`src/comparacao.js`; escolha no `localStorage`, em
`try/catch`, ou `?comparar=uv|ambos|tse` na URL; o uso do seletor vai como POST vazio em `/api/v1/uso/<evento>`, contado no log do Caddy). Teste: `node testes/seletor_fonte.mjs`. Com "Ambos" ou "TSE" a página se inscreve em `tse:<tópico>`
(`docs/PROTOCOLO_WS.md` §4.6) **só do tópico da tela e só com a aba visível**; com "Urna Velox", nada. "Ambos" põe
lado a lado, por candidato, Urna Velox (% e votos), TSE (posição, % e votos) e a diferença (UV − TSE), com destaque de
fundo na linha quando a ordem diverge; "TSE" troca a lista, a apuração, os votos e o selo pelos do arquivo do TSE. O
cabeçalho das fontes mostra as seções de cada uma, a idade do arquivo do TSE e o estado (aguardando, desatualizado,
indisponível, pausado). Marco no feed: "TSE confirmou o matematicamente às hh:mm — Urna Velox às hh:mm, N min antes".
Mesmas regras de sempre: hodômetros, CLS 0, nada de rolagem horizontal no celular.

## Previsão (etapa 8)

Cartão **"Previsão (estimativa)"** entre "Apuração" e "Curva da apuração" (`src/previsao.js`), alimentado pela
mensagem `previsao` (`docs/PROTOCOLO_WS.md` §4.7), que chega sozinha para os tópicos inscritos: a faixa de horário do
"matematicamente" ("entre 20:10 e 20:35", ou "ainda incerto", ou "deve ir até o fim da apuração"), o hodômetro do
horário mais provável (P50), a chance de sair antes do fim e a fração apurada com que deve sair, a faixa P10–P90 de
cada previsão da noite (a previsão convergindo; tracejado verde = horário real depois que saiu) e o fim da apuração
("Seções: 99% entre … · 100% …"). Num tópico que não é a circunscrição (Presidente numa UF), o "matematicamente" é o da
circunscrição e o fim é o da tela. Depois da definição: "saiu às hh:mm — dentro da faixa prevista desde hh:mm" (ou
"fora da faixa", com honestidade) e o marco "Previsão confirmada" no letreiro. Se a chegada dos BUs para no país
(`situacao` = `suspensa`, desde a `0.7.8`), a faixa ganha "(adiada)" e a nota, em cor de alerta, diz "chegada de BUs
parada desde hh:mm — previsão adiada". Antes de 5 % apurado e 10 min de
apuração: "aguardando base". Altura fixa, cada linha com reticência: CLS 0.

## Aviso de versão nova (M-016)

A página não recarrega sozinha quando o servidor troca de versão (o WebSocket reconecta; o JavaScript antigo continua). A
mensagem `estado` traz `versao`; se for **maior** que a da página (o `?v=` do `app.js`), `verificarVersao` (`src/app.js`)
mostra um aviso fixo "Nova versão disponível · Atualizar · ✕", fora do fluxo (nada se move) e sem recarregar sozinho. Só
maior (numa troca de nós, um já na nova e outro na antiga não é "versão nova"); versão ilegível nunca avisa. Teste:
`node testes/aviso_versao.mjs`.
