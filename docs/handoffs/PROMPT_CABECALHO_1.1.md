# Prompt — melhoria do cabeçalho da página (etapa `1.1`)

Preparado em 30/10/2026 (versão no ar: `1.0.57`). Para colar no início de uma sessão nova.

---

Vamos começar a **melhoria do cabeçalho da página** do Urna Velox. Ela entra na etapa **`1.1`** (multipleito), por
decisão minha de 30/10/2026: registre como melhoria nova no `docs/BUGS.md` com destino `1.1` e acrescente uma linha
no texto da etapa `1.1.0` do `ROADMAP.md`, no mesmo commit (regras do `CLAUDE.md`: bugs, melhorias e roadmap andam
juntos; versão PATCH por commit; CHANGELOG).

**Leia antes:**
- `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` (regras do projeto);
- `/mnt/d/git/lukeboh/urna-velox/site/README.md` (regras da página: nada pula — CLS < 0,01 —, celular a 360 px, os
  dois temas, testes em `site/testes/`);
- `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_1.1.0.md` (§3.3: o seletor de pleito no topo é critério da
  `1.1.0`);
- `/mnt/d/git/lukeboh/urna-velox/docs/itens/M-024.md` (o que já mudou na escolha do pleito em 30/10);
- `/mnt/d/git/lukeboh/urna-velox/docs/itens/M-001.md`: em 18/10/2026 já houve **três protótipos do topo** (a: faixa
  fina fixa; b: o topo fixo troca a marca pela identificação; c: cartão à moda do aplicativo da Justiça Eleitoral),
  com as capturas em `docs/itens/img/M-001-prototipo-{a,b,c}-{390,1366}.png`; escolhi a **C**, que é o cartão de
  hoje (`site/src/identificacao.js`). Comece por elas: o que da A e da B vale retomar agora, com o multipleito.

**O que é o cabeçalho hoje** (de cima para baixo, em `site/src/index.html`, `app.js`, `identificacao.js`,
`estilo.css`):
1. `header.topo`: marca "Urna Velox" com a versão, o selo do modo (`#conexao`: AO VIVO / REPLAY / CONSULTA / EM BREVE,
   que também abre o aviso com a playlist) e o botão do tema;
2. a escolha da abrangência ("Brasil", "Estado…" e, nos municipais, o município);
3. a faixa (`#faixa`): chip do seletor de pleito (no celular só o ícone "▤"), ritmo (BUs/min), modo HTTP, "parado",
   promo da próxima eleição ao vivo, avisos de pausa e alerta (B-077: a ordem dos chips não pode fazer nada pular);
4. o cartão de identificação (`#ident`): "Eleições 2026 · Gerais · trocar ▾", o turno "1º/2º" (só indica, não é
   botão), o local e o cargo; recolhível; vira a barra presa no alto (`.ident-barra`) ao rolar;
5. as pílulas de cargo ("Presidente", "Governador"…).

**Problemas que já vi no celular** (capturas de 30/10):
- o cabeçalho ocupa cerca de um terço da tela a 390 px antes do primeiro número;
- informação repetida: o pleito aparece no chip da faixa e no cartão; o cargo, no cartão e nas pílulas; o local, nos
  seletores e no cartão;
- o turno "1º 2º" parece um botão e não é;
- o seletor de pleito ainda é pouco evidente (a M-024 ajudou, mas o chip "▤" da faixa não diz nada);
- o selo do modo e a faixa disputam atenção com o que importa (o resultado).

**O que eu quero desta sessão:**
1. **Diagnóstico com medidas**, sem mudar nada: altura de cada parte do cabeçalho a 360, 390, 768 e 1366 px, nos dois
   temas, ao vivo, no replay e na consulta (Playwright, sem servidor, como os testes de `site/testes/`); capturas em
   `/tmp`; a lista de informações repetidas e de controles que parecem clicáveis e não são.
2. **Duas ou três propostas de layout** (protótipos lado a lado, com capturas a 390 e 1366 px), cada uma dizendo o que
   some, o que junta e o que muda de lugar. Ponto de partida que me agrada: **uma barra só** com a eleição (o seletor
   de pleito), o turno, o local e o cargo — tocando em cada parte, escolhe-se aquilo — e o modo (ao vivo / replay /
   consulta) como um selo pequeno nessa mesma barra; a faixa só com o que muda durante a noite. Me pergunte antes de
   implementar.
3. Depois da minha escolha: implementar com testes (CLS < 0,01, nada sobreposto, tudo dentro da tela de 360 a 1920 px,
   os dois temas, ao vivo/replay/consulta), ajustando os testes que dependem da estrutura atual
   (`faixa_estavel.mjs`, `pleitos_pagina.mjs`, `consulta_pagina.mjs`, `consulta_celular.mjs`, `visual_pagina.mjs`,
   `modo_pagina.mjs`, `validacao_1_1.mjs`), e conferir no Chromium e no WebKit (`consulta_celular.mjs` já roda nos dois).
4. Implantar nos nós como nas versões `1.0.50`–`1.0.57` (`deploy/oci/compilar_local_arm64.sh`,
   `63_ciclo.sh --binarios` e `--reiniciar`) e conferir no ar (`validacao_1_1.mjs`, `consulta_desempenho.mjs`).

**Cuidados:**
- O cartão de identificação tem altura reservada no `index.html` (nada pula quando ele se preenche); a barra presa
  entra só por `transform`/`opacity`.
- O selo do modo é o acesso à playlist (aviso com o link) e diz "CONSULTA" inteiro no celular (B-084).
- O seletor de pleito abre o painel "Escolha a eleição" (`site/src/pleitos.js`); o cartão já abre o painel pelo
  "trocar ▾" (M-024).
- Nada de recurso externo na página (`pagina::testes::tabela_fixa`, B-071).
