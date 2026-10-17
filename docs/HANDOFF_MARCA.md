# Handoff — Urna Velox: logo e ícone de aplicativo (para o Claude Design)

*Preparado em 15/10/2026, na `0.5.1` (etapa 5, site de exemplo, concluída). Para usar: abra o Claude Design, cole este
documento e anexe as imagens da seção "Anexos". Todos os caminhos são absolutos (repositório =
`D:\git\lukeboh\urna-velox`, no WSL `/mnt/d/git/lukeboh/urna-velox`). Este documento:
`D:\git\lukeboh\urna-velox\docs\HANDOFF_MARCA.md`.*

## O pedido

Criar a **marca do Urna Velox**: um **símbolo** (que vira o ícone de aplicativo e o favicon) e o **logotipo** (símbolo +
nome), **totalmente harmônicos com a página de resultados que já existe** — mesma paleta, mesma fonte, mesmo
"vidro tátil" e o mesmo brilho arco-íris. Quem olha a página e o ícone tem que sentir que são a mesma coisa.

## O que é o Urna Velox (para o conceito)

* **Motor independente e de código aberto de totalização eleitoral**, em Rust. Lê os Boletins de Urna (BUs) que o TSE
  publica, soma tudo em memória e mostra o resultado do Brasil, de cada UF e de cada município **segundos depois de cada
  BU chegar** — numa máquina comum. Serve para **auditoria cívica**: qualquer pessoa confere o resultado oficial por
  conta própria.
* **"Velox"** = veloz, em latim. Os três valores que a marca deve passar: **velocidade**, **transparência/conferência**
  (os números batem com o oficial, BU por BU) e **independência/neutralidade**.
* Público: cidadãos, jornalistas, pesquisadores e órgãos que querem acompanhar a apuração; a página roda no celular e no
  computador. Primeiro uso público: noite do 2º turno de 2026 (25/10/2026).

## Restrições que não são de gosto (obrigatórias)

1. **Não parecer oficial.** O Urna Velox **não é do TSE** nem da Justiça Eleitoral: nada de brasão, balança da justiça,
   bandeira do Brasil, combinação verde-amarelo-azul da bandeira, nem desenho que imite a urna eletrônica oficial ou o
   logotipo do TSE. Pode evocar "urna" e "voto" de forma abstrata.
2. **Neutralidade política.** Nenhuma cor de partido dominando (vermelho sozinho, azul sozinho, verde-amarelo, laranja
   etc.). Por isso o arco-íris da interface é bom: **todas as cores juntas = nenhum lado**. Sem símbolos que lembrem
   partidos (estrela, punho, mão fazendo número etc.).
3. **Nada de pessoas**, rostos, mãos votando ou bonecos.
4. **Funciona pequeno.** O símbolo tem de ser legível em **16×16 px** (favicon) e bonito em 1024 px. Sem texto dentro do
   ícone. Formas simples, traço grosso.
5. **Técnico** (a página é servida pelo próprio binário com `Content-Security-Policy: default-src 'self'`): os SVG não
   podem carregar nada externo (sem fontes externas, sem imagens embutidas por URL); texto do logotipo **convertido em
   curvas**; o favicon SVG deve ser pequeno (idealmente < 2 KB) e sem filtros pesados (`feGaussianBlur` só nas versões
   grandes, se houver).

## A linguagem visual que já existe (para seguir)

Fonte: `D:\git\lukeboh\urna-velox\site\src\estilo.css` e `D:\git\lukeboh\urna-velox\site\README.md`.

**Tema escuro (padrão)**

| papel | cor |
|---|---|
| fundo | `#0d1431` (azul-noite), segundo plano `#141d45` |
| aurora do fundo (gradientes radiais) | violeta `rgba(112, 86, 255, .30)`, ciano `rgba(0, 196, 255, .18)`, rosa `rgba(255, 92, 168, .14)` |
| vidro | branco 6,5 % (`rgba(255,255,255,.065)`), forte 11 %; borda branca 15 %, filete de luz no topo 32 % |
| texto | `#eef1ff`, secundário `#a8b2d8`, terciário `#7680a8` |
| estados | ok `#4fe0b0`, alerta `#ffc24b`, erro `#ff6b8b` |

**Tema claro (opcional na página):** fundo `#e9edf8` / `#f6f7fc`, texto `#121836` / `#4a5378`, vidro branco 58–78 %,
mesma aurora com menos opacidade.

**Brilho arco-íris** (só nos destaques: selo de "matematicamente eleito", linha do candidato decidido, marcos novos):
`conic-gradient` girando (5 s por volta) com, nesta ordem:
`#ff5f7e → #ffb84d → #f9f871 → #4fe0b0 → #4fc3ff → #8c6bff → #ff5fd2 → #ff5f7e`, anel de 1,5 px + halo desfocado
(10 px, 55 %). É a assinatura visual mais forte da página.

**Gradiente do ícone atual e do anel de % de seções:** ciano `#4fc3ff` → violeta `#8c6bff` → magenta `#ff5fd2`.

**Fonte:** **Archivo** (variável, licença OFL; eixos de peso 100–900 e largura 62–125 %), embutida na página. Números em
`tabular-nums`. O nome hoje aparece no topo como **"Urna"** peso 800, largura 125 %, cor `#eef1ff` + **"Velox"** peso 300,
cor `#a8b2d8` (dá para manter essa ideia ou propor outra, sempre com a Archivo).

**Formas:** cantos bem arredondados (cartões 26 px, blocos 16 px, pílulas totalmente redondas), vidro translúcido com
desfoque e sombra suave, inclinação leve sob o dedo. Gráficos: anel de progresso (seções apuradas), barras finas,
curva suave da apuração. Números rolando como **hodômetro** (cada dígito sobe numa coluna; contadores regressivos
descem) — é outro elemento muito característico.

**Ícone atual (provisório):** `D:\git\lukeboh\urna-velox\site\src\icone.svg` — um "U" em traço grosso com o gradiente
ciano → violeta → magenta, sobre quadrado arredondado `#0d1431`. Pode ser o ponto de partida ou ser substituído.

## Ideias de caminho (não obrigatórias — proponha 3 conceitos diferentes)

* **U + velocidade:** o "U" (de Urna) com rastros/linhas de movimento ou um corte inclinado que sugira "veloz".
* **Anel de apuração:** o anel de % de seções da página (arco em gradiente) formando um "U" ou uma urna estilizada;
  o anel incompleto = "apurando", completo = "conferido".
* **Hodômetro/conferência:** um dígito rolando ou um sinal de "confere" (check) integrado ao U.
* **Vidro + arco-íris:** o símbolo como peça de vidro (translúcida, com filete de luz) contornada pelo anel arco-íris —
  nas versões grandes; nas pequenas, versão chapada em gradiente.

Para cada conceito, mostrar: o símbolo em 1024, 180, 32 e 16 px; no fundo escuro e no claro; e o logotipo horizontal.

## Entregáveis (do conceito escolhido)

1. **Símbolo** em SVG mestre (1024×1024, grade e zona de segurança indicadas).
2. **Favicon** `icone.svg` (substitui `D:\git\lukeboh\urna-velox\site\src\icone.svg`; < 2 KB, sem filtros) e PNG 16, 32
   e 48 px (ajustados à mão para a grade de pixels, se precisar).
3. **Ícone de aplicativo**: PNG 180 (Apple touch), 192 e 512 (PWA), **512 "maskable"** (conteúdo dentro de 80 % do
   quadro, fundo `#0d1431` sangrando até a borda) e uma versão **monocromática** (uma cor, para tema do sistema e
   notificações).
4. **Logotipo** horizontal (símbolo + "Urna Velox") e empilhado, em SVG com o texto em curvas, versões para fundo escuro
   e claro, e uma versão de uma cor.
5. **Versão animada opcional** (só CSS dentro do SVG ou do HTML): o anel arco-íris girando como na página — com
   alternativa parada (a página respeita `prefers-reduced-motion`).
6. **Mini-guia** (1 página): área de proteção, tamanho mínimo, cores com hex, usos certos e errados.

## Anexos (enviar junto)

Capturas da página atual (Chrome com GPU, 2026 1º turno no replay):
* `D:\git\lukeboh\urna-velox\docs\img\marca\ref_desktop_escuro.png` — tela do Presidente/Brasil, tema escuro.
* `D:\git\lukeboh\urna-velox\docs\img\marca\ref_desktop_claro.png` — mesma tela, tema claro.
* `D:\git\lukeboh\urna-velox\docs\img\marca\ref_celular_escuro.png` — celular (anel, curva, barra do replay).
* `D:\git\lukeboh\urna-velox\docs\img\marca\ref_video_desktop_folha.jpg` — quadros do vídeo, com o **selo e as linhas
  com brilho arco-íris** ("matematicamente 2º turno") e o painel do histórico.
* `D:\git\lukeboh\urna-velox\docs\img\marca\icone_atual.svg` — ícone provisório.
* Vídeos (opcional, mostram as animações): `D:\git\lukeboh\urna-velox\.git\uv_video\desktop.mp4` e `celular.mp4`.
* Folha de estilo com todas as cores: `D:\git\lukeboh\urna-velox\site\src\estilo.css`.

## Depois (integração no projeto — outra sessão do Claude, não o Claude Design)

Trocar `site/src/icone.svg`, pôr o logotipo no topo da página no lugar do texto "Urna Velox" (tamanho reservado, sem
mudar o layout: CLS = 0), acrescentar `apple-touch-icon` e um `manifest.webmanifest` com os PNG (todos embutidos no
binário por `aquisicao/src/pagina.rs`, tabela fixa), rodar `node build.mjs` em `site/`, `compilar_dev.sh` e os testes da
página; README e LICENCIAMENTO com a licença da marca. Versionamento como sempre (PATCH, CHANGELOG).
