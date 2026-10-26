# Bugs do Urna Velox

Lista viva (regra no [CLAUDE.md](../CLAUDE.md), seção "Bugs"). Cada bug entra aqui assim que é visto, **só com o
sintoma**: sem análise nem correção até o Luciano definir a ordem de ataque. Na correção, primeiro o teste que comprova o
bug (falhando), depois a correção (teste passando e suíte inteira verde).

Estados: **registrado** (só o sintoma) → **na fila** (ordem definida) → **teste vermelho** (teste comprova o bug) →
**corrigido** (teste verde, versão) · ou **descartado** (com o motivo).

## Ordem de ataque

Definida em 18/10/2026 (ponderação e motivos em [HANDOFF_BUGS_2026-10-18.md](HANDOFF_BUGS_2026-10-18.md), §3): **1** B-008 → **2** B-009 → **3** B-014 → **4** B-013 → **5** B-011 → **6** B-010 → **7** B-012 → **8** M-001 → **9** B-004 → **10** B-005 → **11** M-002 → **12** B-003 → **13** B-002 → **14** B-006 → **15** B-001 → **16** B-007.

Em 22/10/2026 o Luciano deixou a ordem dos bugs visuais do site (B-029 a B-046) com o Claude, com a regra de sempre (o teste
que comprova antes da correção). Ordem seguida: B-029/B-030 → B-045 → B-035 → B-031/B-032/B-036/B-043 → B-042 → B-047 → B-034 → B-044 → (0.8.54) B-033, B-037, B-038, B-039, B-040, B-041. B-046 fica: precisa da série real de previsões para reproduzir.

22/10/2026 (Luciano aprovou a sugestão do Claude): **1** B-053 → **2** B-052 → **3** B-054 (a previsão pelo ritmo usa o ritmo certo; o relógio cortado aparece no "ainda incerto"). Registrados no branch `claude/bugs-previsao-ritmo` como B-029, B-030 e B-031 e renumerados no merge com a main (0.9.3) para B-052, B-053 e B-054, porque esses números já eram dos bugs visuais da revisão de 22/10.

## Lista

| id | registrado em | quem viu | onde | sintoma | estado | ordem | teste | versão |
|---|---|---|---|---|---|---|---|---|
| B-001 | 18/10/2026 | Claude (ciclo diário) | modo rápido do ciclo, dois nós | Os BUs coletados pelo nó par chegam em lotes e são aplicados com atraso: a ordem de totalização se afasta da ordem de recebimento no TSE (1º turno de 2026: ~10 min da noite na mediana). | resolvido pelo B-011 (medido) | 15 | medição abaixo: mediana 3–6 s da noite na 0.8.17 | 0.8.27 |
| B-002 | 18/10/2026 | Claude (ciclo diário) | `deploy/oci/servidor/uv-ciclo-resumo.py` | A medida de ordem do `RESUMO.json` (atraso p99) é dominada pelos BUs recebidos dias depois (entram no corte de 02:00): p99 de 2,8 dias no 1º turno de 2026. | corrigido | 13 | `deploy/oci/testes/test_ciclo_resumo.py`; saída abaixo (junto do B-003) | 0.8.25 |
| B-003 | 18/10/2026 | Claude (ciclo diário) | `uv-ciclo-resumo.py` | Um reinício intencional (`63_ciclo.sh --reiniciar`) aparece no `RESUMO.json` como falha, igual a uma queda real. | corrigido | 12 | `deploy/oci/testes/test_ciclo_resumo.py`; saída abaixo | 0.8.25 |
| B-004 | 17/10/2026 | Claude (envio do acervo) | `deploy/oci/lib.sh`, função `para` | Com arquivo grande (tar de 3,7 GB), o `para` devolve erro ("Run Command SUCCEEDED:" sem saída) embora o arquivo tenha chegado inteiro ao nó. | corrigido | 9 | `deploy/oci/testes/test_uv_oci.py`; saída abaixo | 0.8.15 |
| B-005 | 17/10/2026 | Claude (ciclo diário) | conferência do 1º turno de 2024 (452) | 33 situações de candidatos diferem do oficial ("Eleito" × "Não eleito") e 41 "matematicamente" não conferem: candidatos depois indeferidos/cassados; o `candidatos.json` pré-eleição não tem a destinação final (`dvt`). Votos idênticos. | corrigido | 10 | `motor/tests/integracao.rs::primeiro_turno_2024_simulacao_destinacao_final`; saída abaixo | 0.8.17 |
| B-006 | 18/10/2026 | Claude (teste do `--reiniciar`) | federação, `saude.json` | Depois de uma retomada, o contador `federacao.publicados` do nó recomeça (o B mostrou 91 mil publicados ao fim, tendo coletado ~240 mil), o que confunde a telemetria. | corrigido | 14 | `federacao::no::contadores_sobrevivem_a_retomada`, `tests/federacao.rs::kill9_e_reinicio`; saída abaixo | 0.8.26 |
| B-007 | 18/10/2026 | Claude | repositório | Três scripts de análise dos acessos, sem commit, de outra sessão: decidir se entram no repositório ou saem. | corrigido | 16 | decisão do Luciano; `deploy/oci/testes/test_scripts_em.py` | 0.8.18 |
| B-008 | 18/10/2026 | Luciano | site público, troca de eleição das 07h de 18/10 (3221 → 1º turno de 2024) | Às 07h em ponto, ao abrir o site, o navegador oferecia **salvar um arquivo** em vez de mostrar a página; durou uns 2 a 3 minutos. | corrigido | 1 | `deploy/oci/testes/pagina_indisponivel.sh` (+ `site/testes/indisponivel.mjs`); saída abaixo | 0.8.9 |
| B-009 | 18/10/2026 | Luciano | site público, 1º turno de 2024 (ciclo das 07h de 18/10), Prefeito de São Paulo/SP | O contador de **eleitores que faltam para o "matematicamente eleito" ficou travado** (pelo menos nessa eleição, a que ele olhou). | corrigido | 2 | `faltam_acompanha_a_apuracao` (`aquisicao/tests/servico.rs`, `--ignored`); saída abaixo | 0.8.10 |
| B-010 | 18/10/2026 | Luciano | site público, eleições de 1º turno (ciclo de 18/10) | **Sempre aparece 2º turno.** Regra que o Luciano lembrou: no 1º turno, presidente, governador ou prefeito com mais de 50% dos votos válidos é eleito; e nos municípios pequenos (abaixo de um limiar de eleitorado — referência: 200 mil eleitores, CF art. 29, II) não há 2º turno, independentemente da votação. | não reproduzido (18/10: o Luciano não lembra onde viu; avisa se aparecer de novo) | 6 | sem teste: motor e situações conferem (abaixo) | |
| B-011 | 18/10/2026 | Luciano | site público, card "Curva da apuração", 1º turno de 2024 (ciclo de 18/10; eixo 18:27–08:00) | **A curva tem comportamentos estranhos e pula, principalmente a de seções.** Nas capturas: a linha de seções sobe devagar a noite toda e dá um salto vertical no fim (~08:00); as linhas dos candidatos também saltam no fim; no começo há trechos retos e um pico isolado (um candidato vai a 28,5 % e volta). Capturas: [curva 1](bugs/B-011-curva-1.jpg), [curva 2](bugs/B-011-curva-2.jpg). | corrigido | 5 | `curva_sem_salto_no_modo_rapido` (`aquisicao/tests/servico.rs`, `--ignored`); saída abaixo | 0.8.13 |
| B-012 | 18/10/2026 | Claude (nas capturas do B-011), registro pedido pelo Luciano | site público, card "Curva da apuração" | **Dois candidatos com cores quase iguais:** na [curva 2](bugs/B-011-curva-2.jpg), RICARDO e PABLO aparecem em verdes muito parecidos; as linhas e a legenda ficam difíceis de distinguir. | corrigido | 7 | `site/testes/cores_distancia.mjs`; saída abaixo | 0.8.14 |
| B-013 | 18/10/2026 | Luciano | site público, card "Previsão", 1º turno de 2024 (ciclo das 07h de 18/10, modo rápido) | **Não viu a previsão funcionando** durante o simulado das 07h. Ele também perguntou se a previsão é do "matematicamente" (eleito / 2º turno) ou do fim da totalização: o card mostra as duas, e isso não ficou claro para ele na tela. | corrigido | 4 | `previsao_acompanha_servico_atrasado` (`aquisicao/tests/servico.rs`, `--ignored`) e `site/testes/previsao_pagina.mjs`; saída abaixo | 0.8.12 |
| B-014 | 18/10/2026 | Luciano | site público no celular, 1º turno de 2024 (ciclo das 07h de 18/10, modo rápido) | **O celular parecia travar de vez em quando e esquentou** durante a totalização. O chip de ritmo mostrava em alguns momentos mais de 10–12 mil BUs por minuto (números reais do modo rápido: o 1º turno de 2024 inteiro, 478 mil BUs, passou em ~8 min). Ele suspeita do volume de mensagens do servidor para a página, num ritmo bem acima do de uma noite real. Pergunta dele: a página recebe só o que está na tela? Resposta levantada no código (`site/src/app.js`, `atualizarTopicos`): **não** — a página assina até ~190 tópicos (todos de Brasil/UF e os municipais, primeiro os da UF da tela), mais todos os eventos e todos os marcos; cada tópico manda o resultado inteiro a cada mudança (o servidor junta versões e limita a 20 mensagens/s por conexão). | corrigido | 3 | `aquisicao/scripts/ensaio_carga_pagina.sh` (+ `site/testes/carga_pagina.mjs`); saída abaixo | 0.8.11 |
| B-015 | 18/10/2026 | Claude (implantação do B-008) | `deploy/oci/02_configurar_borda.sh` | A etapa que instala o Caddy nos nós não faz nada: o comando enviado com `em "$N" <<CMD` chega vazio ao nó (Run Command sem script), e o roteiro diz "Nó configurado e ativo". | corrigido | | `deploy/oci/testes/test_scripts_em.py`; saída abaixo (junto do B-007) | 0.8.18 |
| B-016 | 18/10/2026 | Luciano | site público, card "Previsão", simulado das 13h de 18/10 (1º turno de 2026, modo rápido, 0.8.10 nos nós) | **O gráfico da previsão não é atualizado** durante a apuração (ele não sabe se é por causa da velocidade máxima). | corrigido | 4 (com o B-013) | o mesmo do B-013 | 0.8.12 |
| B-017 | 19/10/2026 | Luciano | borda (Caddy), rotas estáticas e `uv-servico` (`/api/v1/replay`) | **Erros 404 e 5xx recorrentes nos logs de acesso:** (1) a rota `/api/v1/replay` devolve 404 na inicialização da página quando não há replay ativo (186 ocorrências); (2) falta de `/apple-touch-icon.png` e `/apple-touch-icon-precomposed.png` na raiz gera 404 no Safari/iOS (32 ocorrências); (3) erros 502/503 em `/` e `/ws` quando o `uv-servico` está reiniciando/trocando de ciclo na porta 8080. | registrado | | | |
| B-018 | 19/10/2026 | Luciano | site público no celular, faixa (0.8.29) | **Sumiu o chip da velocidade de processamento dos BUs (BUs/min).** Causa conhecida (introduzida na 0.8.29): para caber o botão "▶ Playlist", o CSS de até 640 px esconde o chip de ritmo quando a playlist aparece. | corrigido | 17 | `site/testes/playlist_pagina.mjs` (faixa inteira à vista a 360, 390 e 1366 px); saída abaixo | 0.8.31 |
| B-019 | 19/10/2026 | Luciano | painel dos nós em produção (0.8.34), visão geral | **Só aparece um dos nós:** o outro vem como "painel fora do alcance" com `error sending request for url (http://10.0.1.11:8095/painel/interno/visao?…)`. | corrigido | 1 (19/10) | `deploy/oci/testes/test_liberar_portas.py`; `painel::testes::par_fora_do_ar_aparece_como_inalcancavel` (a causa no erro); saída abaixo | 0.8.35 |
| B-020 | 19/10/2026 | Luciano | painel dos nós no celular (iPhone), aba Configuração | **A tela de configuração não é responsiva:** a página fica mais larga que a tela e o Safari reduz tudo (cabeçalho ocupando só um terço da largura). | corrigido | 1 (19/10) | `site/testes/painel_celular.mjs`; saída abaixo | 0.8.35 |
| B-021 | 19/10/2026 | Claude (nas capturas do B-020) | painel dos nós, aba Configuração | Durante o ciclo de simulados a tela mostra o `no.env` (pleito 3220, servidor do TSE) como se fosse o que o nó está rodando, enquanto a visão geral mostra o simulado (pleito 452, servidor local): nada diz que aquela configuração é a do `urna-velox.service`, parado durante o ciclo. | corrigido | 1 (19/10) | `site/testes/painel_celular.mjs` (aviso do ciclo); saída abaixo | 0.8.35 |
| B-022 | 20/10/2026 | Luciano | site público (0.8.37), vitrine do 2º turno de 2026 (item `3221-espera` da playlist, 14h–15h de 20/10) | **Não aparece o botão da Playlist**, e o selo do topo diz "ao vivo" sem nenhuma apuração em andamento. | corrigido | 1 (20/10) | `site/testes/modo_pagina.mjs`; saída abaixo | 0.8.39 |
| B-024 | 20/10/2026 | Luciano | site público no celular (0.8.39), 16h49 de 20/10, replay do 452 | **Não aparece a contagem "Próxima eleição AO VIVO"** (nem o "▶" da playlist no selo), que deveria estar sempre à vista. | corrigido | 1 (20/10) | `deploy/oci/testes/test_caddyfile.py` (B leva a playlist ao A) e `site/testes/modo_pagina.mjs` (playlist fora do ar na partida); saída abaixo | 0.8.40 |
| B-025 | 20/10/2026 | Luciano (acessou `http://<IP do nó>:8080/`) | Security List da sub-rede e firewall dos nós | **A porta 8080 (o `uv-servico` direto, sem o Caddy) abre de qualquer lugar da internet**, nos dois nós. | corrigido | 1 (20/10) | `deploy/oci/testes/test_liberar_portas.py` (`FecharPortas`, `test_b025_*`); saída abaixo | 0.8.41 |
| B-023 | 20/10/2026 | Claude (no plano do chip REPLAY, 20/10) | site público, explicação que abre sozinha "na primeira visita do dia" | Quem volta depois das 21h (Brasília) vê a explicação abrir de novo no mesmo dia. | corrigido | com o B-022 (o aviso diário pedido em 20/10 depende dele) | `site/testes/modo_pagina.mjs` (cenário "dia local"); saída abaixo | 0.8.39 |
| B-026 | 20/10/2026 | Claude (revisão do teste de carga de 20/10) | site público, `site/src/conexao.js`, acima de `--max-clientes` | Quem chega depois que os nós atingem o limite de clientes WebSocket (503 "limite de clientes" no `/ws`, visto no degrau de 12.000 do teste de carga) **não recebe resultado nenhum**: a página só fica tentando reconectar (espera crescente até 30 s), sem mostrar a apuração. | corrigido | 1 (20/10, passo 1 do plano de borda) | `site/testes/ws_recusado.mjs`; saída abaixo | 0.8.44 |
| B-027 | 21/10/2026 | Claude (implantação do site `dados`) | Caddy dos dois nós, depois do `02_configurar_borda.sh` com `DOMINIO_DADOS` (15h18 no A, 15h20 no B) | **O site principal ficou fora do ar nos dois nós por ~1h20** (porta 443 recusada nos dois IPs; o `uv-servico` respondia na 8080). O Caddy não subia: `loading initial config: ... opening log writer ... /var/log/caddy/urna-velox-dados.log`; o arquivo existia com dono `root` e modo `0600`. Restabelecido à mão às 16h37 (B) e 16h38 (A): `chown -R caddy:caddy /var/log/caddy` e `systemctl restart caddy`. | registrado | | | |
| B-028 | 21/10/2026 | Claude (implantação do site `dados`) | Caddy dos dois nós, certificado Let's Encrypt de `dados.urnavelox.com.br` | Com o nome `dados` apontando para os dois IPs, **nenhum nó conseguia o certificado**: o desafio falhava no outro nó (`no active ACME challenge` / `tls: internal error`), e o Caddy repetia com intervalos crescentes. Obtido à mão (16h38–16h45) renomeando de forma temporária um dos registros `dados` na Cloudflare de cada vez (`dados-b`, depois `dados-a`). | registrado | | | |
| B-029 | 22/10/2026 | Luciano | site público, todos os contadores (hodômetros) | O **"." de milhar** fica um pouco mais baixo que os dígitos. Claude viu o mesmo na **vírgula decimal** dos percentuais ("47,31", "66,30"). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-030 | 22/10/2026 | Luciano | site público, horários em hodômetro ("Totalizado pelo Urna Velox às", previsão) | O **":" das horas** fica mais baixo que os dígitos, **bem deslocado** no iPhone ("10:06:29" em "Totalizado pelo Urna Velox às"; [captura](bugs/B-030-dois-pontos-hora.jpg)). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-031 | 22/10/2026 | Luciano | site público no iPhone, comparação "Ambos" (Presidente, Brasil) — [captura](bugs/B-031-ambos-celular.jpg) | O **"+" da diferença UV − TSE fica deslocado**: longe do número que acompanha ("+      1.654.136") e encostado no "TSE" do rótulo "UV − TSE". Claude viu o "+" separado do número também no desktop (1366 px). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-032 | 22/10/2026 | Luciano | site público no iPhone, comparação "Ambos" — [captura](bugs/B-031-ambos-celular.jpg) | O **selo da posição** ("1º", "2º"…) fica colado no "%" do Urna Velox ("%1º"), **cobre um dígito dos votos** do Urna Velox ("26.11█.387") e encosta no percentual do TSE. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-033 | 22/10/2026 | Luciano | site público no iPhone, botão ☰ do letreiro — [captura](bugs/B-031-ambos-celular.jpg) | O **ícone do menu (☰) é pequeno e não fica centralizado** no círculo do botão. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-034 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público, cartão "Apuração", de 961 a ~1150 px e no celular de 360 px | A caixa "Eleitores que faltam apurar para o matematicamente" **passa da borda de baixo do cartão** e o número fica cortado ao meio; "312.954 de" quebra antes de "472.027" e "Totalizado pelo Urna Velox às" ocupa duas linhas. Também no iPhone: o número "75.171.4…" fica cortado embaixo pela caixa ([captura](bugs/B-045-cartoes-borda.jpg)). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-035 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público, cabeçalho da corrida, de 961 a ~1180 px | O título vira **"Presidente …"**, o subtítulo "eleitorado de 155.912.…" e a linha das fontes "Ur…", embora haja espaço sobrando no cartão. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-036 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público no celular de 360 px, comparação "Ambos" | O rótulo **"UV − TSE" quebra em duas linhas** ("UV –" / "TSE") embaixo do número do candidato; os nomes ficam muito curtos ("SORAYA THRO…", "CONSTITUINT…"). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-037 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público no celular, pílula "© Urna Velox · Termos de uso" | A pílula fixa fica **por cima do conteúdo** ao rolar: das linhas dos candidatos e do título "Previsão (estimativa de horário)". Visível também na [captura](bugs/B-031-ambos-celular.jpg). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-038 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público no celular, letreiro com o histórico (☰) aberto | O rótulo **"pausado" aparece por cima do horário** do item do letreiro ("19:20:05pausado"). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-039 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público no celular, cabeçalho da corrida | **Espaço vazio grande** (~50 px) entre o subtítulo ("1º turno · eleitorado de …") e o controle "Urna Velox · Ambos · TSE" enquanto não há selo do matematicamente. Visível também na [captura](bugs/B-031-ambos-celular.jpg). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-040 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público no celular, barra de identificação presa no alto ao rolar | Uma **faixa de ~15 px acima da barra** deixa ver o conteúdo que passa por baixo (o pedaço de cima da linha de um candidato). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-041 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público, hodômetros de "Seções apuradas" e do chip "BUs/min" | **Vão vazio à esquerda do número**: em Governador/SP, "67.211" começa recuado em relação ao rótulo "Seções apuradas"; no chip, o "0" fica deslocado para a direita. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.54 |
| B-042 | 22/10/2026 | Claude (revisão visual de 22/10, capturas com dados falsos) | site público, letreiro, histórico e folha de detalhe | Preposição errada antes do nome da UF: **"Governador em Bahia"**, "em Rio de Janeiro" (o certo: "na Bahia", "no Rio de Janeiro"). | corrigido | 22/10 (ordem do Claude) | `site/testes/preposicoes.mjs`; saída abaixo | 0.8.53 |
| B-043 | 22/10/2026 | Luciano | site público no iPhone (0.8.39), replay do 1º turno de 2024, Prefeito de Bujari (AC), comparação "Ambos" — [captura](bugs/B-043-nome-some-eleito.jpg) | **O nome do candidato eleito não aparece** na linha dele: no lugar do nome fica só a pílula "eleito às 16:33:51" (o selo do topo diz "Matematicamente eleito: PADEIRO às 16:33:51"). | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-044 | 22/10/2026 | Claude (na captura do Luciano do B-030) | site público no iPhone (domínio duckdns), Presidente, Brasil, com a chegada de BUs parada, cartão "Previsão" — [captura](bugs/B-030-dois-pontos-hora.jpg) | Com a previsão "em horário ainda incerto (…)" e "chegada de BUs parada", o horário "mais provável" mostra **"00:00:0"**, cortado na borda direita do cartão. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-045 | 22/10/2026 | Claude (na captura do Luciano do B-030); confirmado pelo Luciano ([captura](bugs/B-045-cartoes-borda.jpg)) | site público no iPhone (domínio duckdns), Presidente, Brasil, com a chegada de BUs parada, cartões "Apuração" e "Previsão" — [captura](bugs/B-030-dois-pontos-hora.jpg) | Os cartões **passam da borda direita da tela** (a borda direita deles não aparece; "último BU no TSE: 19:05:08 de 04/10" encosta na borda). Na captura do Luciano, os cartões vão além da borda direita da barra presa no alto (a linha vermelha marca onde deveriam terminar) e o "às" de "Totalizado pelo Urna Velox às" e o último dígito de "499.248" ficam nessa borda. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-046 | 22/10/2026 | Claude (na captura do Luciano do B-030) | site público no iPhone (domínio duckdns), Presidente, Brasil, com a chegada de BUs parada, gráfico do cartão "Previsão" — [captura](bugs/B-030-dois-pontos-hora.jpg) | A faixa da previsão aparece **serrilhada**, em zigue-zague contínuo, em vez de uma faixa que converge. | registrado | | | |
| B-047 | 22/10/2026 | Claude (nas capturas da correção do B-045) | site público, cartão "Apuração", caixa "Eleitores que faltam" ainda sem valor | Aparecem **zeros-fantasma ("0.000") fora da caixa**, à esquerda do cartão, enquanto o número dos eleitores que faltam ainda não chegou. | corrigido | 22/10 (ordem do Claude) | `site/testes/visual_pagina.mjs`; saída abaixo | 0.8.53 |
| B-048 | 22/10/2026 | Luciano | página `/playlist` (`deploy/oci/servidor/uv-playlist.html`) | **A identidade visual (fontes e estilo) da playlist é totalmente diferente da do Urna Velox** (página da apuração). | corrigido | 22/10 (pedido do Luciano) | `site/testes/playlist_identidade.mjs`; saída abaixo | 0.8.55 |
| B-049 | 23/10/2026 | Claude (pergunta do Luciano sobre os BUs do exterior a partir da 1h) | replays do ciclo (`uv-ciclo.sh`: `uv-simulador --horario recebido`, `uv-servico --ordem`) e ensaios | No 1º turno de 2026 o TSE publicou 781 das 1.310 seções do exterior **antes das 17h** (`zz-p003220-cs.json`: a primeira às 01:49:24, Wellington; Camberra às 03:40, Tóquio às 06:33, Budapeste às 13:44), mas o `aux.json` dessas seções traz o recebimento num lote só, às **17:08:44–17:08:50** (Wellington, Tóquio, Hamamatsu, Budapeste, Lisboa); as recebidas depois das 17h (Miami, Boston, Nova York, Los Angeles) têm recebimento = publicação. Os replays usam o recebimento: começam às 17h e **nunca reproduziram uma noite com BUs chegando de madrugada**. | corrigido | 23/10 (decisão do Luciano: nos replays, o que saiu antes das 17h entra junto às 17h) | `simulador::testes::piso_das_17h`; saída abaixo | 0.8.58 |
| B-050 | 23/10/2026 | Claude (mesma análise) | site público, selo e faixa, na madrugada de 25/10 | Com o primeiro BU do exterior aplicado (≈ 01:49), o selo passa a "AO VIVO" e a apuração aparece, mas a contagem "Próxima eleição AO VIVO · domingo (25/10), 17h · faltam 15h" continua na faixa e no letreiro até as 17h: as duas coisas ao mesmo tempo. | corrigido | 23/10 (decisão do Luciano: mostrar a totalização antes das 17h, explicando) | `site/testes/antes_17h.mjs`; saída abaixo | 0.8.57 |
| B-051 | 23/10/2026 | Claude (alerta nos reinícios da 0.9.0 nos nós, vitrine do 3221) | serviço, fim da apuração | O alerta `secoes_esperadas_incoerentes` aparece em toda partida do 3221: o `s.ts` pré-apuração do `candidatos.json` está parcial em várias UFs (Brasil 494.864 × 499.248 no catálogo; SP 33.403 × 103.904 seções no `cs.json`; CE, ES, GO, MA, PA, PE, PI, RJ). O fim automático da apuração usa esse `s.ts`: pode encerrar a apuração antes da hora (e reabrir a cada BU) ou nunca encerrar. O Luciano pediu que isso seja automático, sem depender de regerar o `candidatos.json`. | corrigido | 23/10 (pedido do Luciano) | `servico::sts_desatualizado_nao_encerra_cedo` (`--ignored`, dados reais); saída abaixo | 0.9.2 |
| B-052 | 22/10/2026 | Luciano (B-029 no branch `claude/bugs-previsao-ritmo`) | site público no celular, card "Previsão", replay do 1º turno de 2026 (10h06 de 22/10, ~53 % das seções, Brasil · Presidente) | **A previsão fica constantemente indefinida** em várias situações: "Matematicamente 2º turno sai em horário ainda incerto" e "Fim da apuração: 99 % em horário ainda incerto", embora a "chance de sair antes do fim" seja 100 %. Pedido dele junto: sempre dar alguma previsão, ainda que imprecisa, e reajustá-la continuamente (como o ETA do Waze), inclusive no modo muito rápido — por exemplo, BUs restantes ÷ velocidade de processamento de BUs. Capturas: [previsão 1](bugs/B-052-previsao-1.jpg), [previsão 2](bugs/B-052-previsao-2.jpg). | corrigido | 2 (22/10) | `aquisicao/src/previsao.rs::previsao_sempre_com_horario` e `site/testes/previsao_cartao.mjs`; saída abaixo | 0.9.3 (0.8.49 no branch) |
| B-053 | 22/10/2026 | Luciano (B-030 no branch `claude/bugs-previsao-ritmo`) | site público, chip de ritmo (BUs/min) | **A velocidade mostrada chega a 200–300 mil BUs por minuto** em vários momentos, claramente irreal. | corrigido | 1 (22/10) | `site/testes/ritmo_pagina.mjs`; saída abaixo | 0.9.3 (0.8.49 no branch) |
| B-054 | 22/10/2026 | Claude (nas capturas do B-052) (B-031 no branch `claude/bugs-previsao-ritmo`) | site público no celular (iPhone), card "Previsão" | Ao lado de "em horário ainda incerto" aparece o relógio **"00:00:0" com "mais provável"**, cortado na borda direita do card. | corrigido | 3 (22/10) | `site/testes/previsao_cartao.mjs`; saída abaixo (junto do B-052) | 0.9.3 (0.8.49 no branch) |
| B-055 | 24/10/2026 | Luciano (vitrine do 3221 × site de resultados do TSE) | `uv-coletor preparar --atualizar`, `candidatos.json` do 3221 nos nós | "Eleitores que faltam apurar para o matematicamente" **157.563.461** no Urna Velox, eleitorado apto **158.745.502** no TSE (Brasil, Presidente, 2º turno, antes da apuração). | corrigido | imediato (Luciano, 24/10) | `preparo::testes::atualizar_baixa_de_novo_os_u_json`, `tse_resultados::testes::eleitorado_divergente_alerta_uma_vez_por_topico`; saída abaixo | 0.9.7 |
| B-056 | 25/10/2026 | Luciano ("no primeiro turno a zz já divulgava às 1h") e Claude (conferência no site do TSE) | `uv-servico`, acompanhamento dos `cs.json`, exterior (ZZ) do 3221 | Às 03h de 25/10 o `aux.json` e o BU de Wellington (ZZ 30805/0001/1088) já estavam no ar (BU "Recebido" 01:51:38), mas o `zz-p003221-cs.json` ainda era o de 09/10 sem nenhuma disponibilização; o `uv-servico` só pedia seção com disponibilização no `cs.json` e a sondagem só rodava depois do primeiro BU e com poucas seções faltando, então não via nenhum BU do exterior | corrigido | imediato (Luciano, 25/10) | `servico::testes::exterior_sondado_antes_do_cs_json` | 0.9.21 |
| B-057 | 25/10/2026 | Luciano (site de terceiros pegava BUs do exterior antes) e Claude (conferência do 1º turno) | `uv-servico`, sondagem antecipada do exterior (ZZ) do 3221 | Mesmo com a sondagem antecipada do B-056, o exterior era sondado a 2 req/s, cada seção a cada 300 s e na ordem do catálogo: uma volta nas 1.351 seções leva ~11 min, então um BU podia esperar até ~11 min para ser visto; sem contingência para um 429 durante o reforço | corrigido | imediato (Luciano, 25/10) | `servico::testes::exterior_sondado_pelo_calendario` | 0.9.22 |
| B-058 | 25/10/2026 | Luciano ("por que a hora do Totalizado pelo Urna Velox às ainda está zerada?") | site público, cartão "Apuração" (hodômetro "Totalizado pelo Urna Velox às") depois de reiniciar o `uv-servico` | Depois de um reinício do serviço (troca de binários às 04:31 e 05:28 de 25/10) a hora fica **zerada** mesmo com seções apuradas: o `br-c0001` e o `zz-c0001` publicados têm `seq` 32 e 32 seções apuradas, mas `ultimo_bu` e `totalizado_em` vazios; o `/api/v1/estado` mostra `ultimo_bu` com `site` e `sistema` nulos. Só volta quando um BU novo é aplicado | corrigido | 25/10, 17h58 (Luciano: "Ataque"; visto de novo na implantação da 1.0.4, com o TSE parado desde 17:11:45) | `servico::testes::ultimo_bu_vem_do_diario_na_retomada` (antes: `left: None, right: Some(1792948305)`; depois: ok) | 1.0.5 |
| B-059 | 25/10/2026 | Luciano ("estamos atrás do site concorrente") e Claude (comparação município a município às 05:46) | `uv-servico`, sondagem antecipada do exterior, qualquer município | Às 05:46 o concorrente tinha 45 urnas do exterior e nós 35; as quatro cidades iguais e **Nagoya (30198) com 13 × 3**. Os BUs de Nagoya chegaram ~20 min antes do horário do 1º turno, fora da janela "devida" do calendário do B-057 (5 min antes), e ficaram a 300 s por seção; um município que já tinha BU não era sondado com mais frequência que os outros | corrigido | imediato (Luciano, 25/10) | `servico::testes::exterior_municipio_quente_sondado_a_cada_20s` | 0.9.24 |
| B-060 | 25/10/2026 | Luciano ("já temos mais de 237 BUs recebidos no painel do TSE, mas só 199 publicados: deixamos algum BU do exterior de lado? o sistema está polando todos de tempos em tempos?") e Claude (medição às 13h05) | `uv-servico`, sondagem do exterior (ZZ) | Os dois nós e o site concorrente com as mesmas 199 seções (município a município), mas `sondadas` parado em 1.170 de 1.351 seções sondáveis: ~180 seções do exterior nunca foram consultadas em 48 min de serviço. A faixa quente (140 seções a cada 20 s) enchia o orçamento da rodada e metade das seleções eram do outro nó (descartadas sem requisição); sem garantia de varredura completa | corrigido | imediato (Luciano, 25/10) | `servico::testes::sondagem_cobre_todas_as_secoes_do_no` | 0.9.27 |
| B-061 | 25/10/2026 | Claude (ensaio do mapa M-017 a 300×, celular com CPU 4× mais lenta) | página, cartão do mapa das seções (`site/src/mapa.js`) | `IndexSizeError: Failed to execute 'arc' on 'CanvasRenderingContext2D': The radius provided (-0.46) is negative` no console durante a apuração acelerada | corrigido | junto do mapa (1.0.0, ainda não implantado) | `site/testes/mapa_pagina.mjs`, "B-061 explosão adiantada (Brasil / globo)" | 1.0.0 |
| B-062 | 25/10/2026 | Claude (suíte da página da 1.0.0: `antes_17h` falhando de vez em quando) | página, faixa e letreiro (próxima eleição ao vivo) | Na madrugada da eleição, com BUs do exterior já aplicados, o letreiro anunciava "Próxima eleição AO VIVO" ao abrir a página quando a playlist chegava antes do primeiro `estado` (o item, à vista, só sai quando passa da borda esquerda) | corrigido | junto da 1.0.0 | `site/testes/antes_17h.mjs`, "madrugada, estado depois da playlist" | 1.0.0 |
| B-063 | 25/10/2026 | Claude (conferência da implantação da 1.0.0) | `deploy/oci/uv_oci.py` (transporte OCI dos scripts de operação) | `./30_verificar.sh` parava no nó A com `oci.exceptions.ServiceError … InvalidParameter … content.source.text size must be between 1 and 4096` | corrigido | imediato | `deploy/oci/testes/test_uv_oci.py` (`LimiteDoTexto`) | 1.0.1 |
| B-064 | 25/10/2026 | Luciano | site público (1.0.0), mapa das seções (M-017), 15h36 do 2º turno de 2026 | **O número de seções do mapa não bate com o número de seções** da apuração. O Luciano acredita que o mapa conta as seções sem levar em conta as agregações. Em produção: mapa com 517.179 seções, página com 499.248. | corrigido | imediato (Luciano, 15h47: "Ataque agora") | `aquisicao/src/mapa.rs::b064_total_do_mapa_igual_ao_da_pagina` (antes: `left: Some(2), right: Some(1)`); `site/testes/mapa_pagina.mjs`, cenário B-064 (antes: `FALHA B-064 agregada da noite 4796 ≠ 4794`); depois: verdes | 1.0.2 |
| B-065 | 25/10/2026 | Luciano ("nosso concorrente está com mais seções", 16h38) | `uv-servico`, exterior (ZZ) do 3221, 2º turno, antes das 17h | Às 16h40 o concorrente (`dados.ascaribe.tech/real/exterior.json`) tinha **776 de 1.351** urnas do exterior apuradas; o Urna Velox, **708** (`zz-c0001`), o mesmo número das 15h46, com `ultimo_bu` vazio. Diferença: Londres 0 × 48, Lisboa 83 × 103; o `aux.json` de Londres 29971/0001/0985 no ar desde 15:47:02. | corrigido (não implantado) | imediato (Luciano, 16h38: "corrija e deixe pronto") | `servico::testes::exterior_sondado_com_o_cs_json_atrasado` (antes: `left: Nao, right: Antecipada`; depois: ok) | 1.0.3 |

## Melhorias

Pedidos que não são bug (mesma regra de registro: só o pedido, sem desenho nem análise até a ordem de ataque).

| id | registrado em | quem pediu | onde | pedido | estado | ordem | versão |
|---|---|---|---|---|---|---|---|
| M-001 | 18/10/2026 | Luciano | site público, topo da página | **Identificação clara e permanente da eleição que está sendo vista.** Ele fica meio perdido ao olhar a página. Referência: o aplicativo oficial da Justiça Eleitoral deixa claro no topo qual é a eleição ("Eleições 2026 · Geral Ordinária"), a abrangência ("Anamã, AM") e o turno (1º/2º) — [captura](bugs/M-001-referencia-app-je.jpg). O que ele pediu: (1) eleição, abrangência e turno **nunca somem** ao rolar a tela; (2) a marca Urna Velox **pode sumir** ao rolar; (3) o usuário pode esconder essa faixa se quiser; (4) algo **simples, que ocupe pouco espaço e seja informativo**, sem perder a qualidade e a identidade visual atuais (que ele considera muito boas); (5) os controles do topo (incluindo o selo de simulado) estão muito bons, mas a disposição dessas informações e a identificação correta da eleição precisam melhorar. | feito | 8 | 0.8.16 |
| M-002 | 18/10/2026 | Luciano | serviço, federação e nós (topologia) | **Medir a vazão de ponta a ponta para achar uma topologia que escale.** Contexto: o motor soma os ~500 mil BUs em menos de 1 s (prova de conceito), mas o ciclo rápido de 18/10 levou ~8 min (coleta HTTP de `aux.json` + BU por seção, troca de lotes entre os nós — o nó A esperou o B com a fila zerada —, publicação das abrangências e WebSocket, diário em disco, nós ARM de 2 núcleos). Objetivo: medir onde vai o tempo em cada etapa (incluindo o I/O para obter os dados, do TSE e do disco) e avaliar uma topologia que permita **aumentar a vazão quando precisar** (mais nós, outra divisão de papéis etc.); se houver possibilidade de melhorar, ele quer avaliar. | medido; decisão de topologia com o Luciano | 11 | 0.8.21 |
| M-003 | 18/10/2026 | Luciano | site público (horários na tela e card "Previsão") | **Separar o horário que vem do TSE do horário do próprio sistema.** Hoje a página mostra datas e horas de totalização tiradas do TSE (dos BUs e dos arquivos de divulgação); ele quer que fique distinto, na tela, o que é horário do TSE e o que é horário do Urna Velox (quando o sistema de fato totalizou). E **toda a previsão de horário** (matematicamente, fim da apuração) deve ser baseada no que o sistema está fazendo, **também no simulado**. Esclarecimento (18/10, 20h29): no simulado valem a data e a hora reais em que o sistema totalizou; para comparar com o TSE (eleição real, em outro dia), ajustar a diferença de data entre a eleição real e o simulado, para medir a distância de tempo entre as duas. | feito (teste `horarios_do_sistema_no_simulado`; saída abaixo) | pedida em 18/10 (antes da fila) | 0.8.28 |
| M-004 | 20/10/2026 | Luciano | site público, topo e faixa | **"Replay" no lugar de "simulado"** (os dados são reais; como a reprise de um jogo de futebol). O chip AO VIVO/SIMULADO e o da Playlist viram **um chip só, com destaque, em âmbar** ("REPLAY", sem subtítulo). **Contagem regressiva para a próxima eleição ao vivo sempre à vista**, também durante um replay ("próxima eleição ao vivo: 2º turno das Eleições Gerais 2026, domingo (25/10), 17h"), com bastante destaque — uma propaganda —, inclusive no letreiro; aviso na primeira entrada do dia. | feito (teste `site/testes/modo_pagina.mjs`) | 1 (20/10) | 0.8.38 |
| M-005 | 20/10/2026 | Luciano | borda (Caddy) e página | **Oficializar a mudança de endereço** de `urnavelox.duckdns.org` para `urnavelox.com.br`: redirecionamento permanente (HTTP 301) no Caddy de tudo o que chegar pelo DuckDNS (inclusive `robots.txt` e `sitemap.xml`) e `<link rel="canonical" href="https://urnavelox.com.br/">` no `<head>`. | feito (teste `deploy/oci/testes/test_caddyfile.py`); falta implantar | 1 (20/10) | 0.8.38 |
| M-010 | 24/10/2026 | Luciano | página, faixa de alertas | **Pensar numa forma de apresentar a "destinação dos votos presumida 'Válido'" que deixe poucas dúvidas.** Até lá, o aviso sai da página (o alerta continua no estado, nos logs e no `30_verificar.sh`). (M-006 a M-009 estão na branch `merge/ws-autenticado`.) | aviso retirado da página na 0.9.17; apresentação a pensar | a definir | 0.9.17 (retirada) |
| M-011 | 24/10/2026 | Luciano | página, topo | **Chip "Anuncie Aqui" bem no topo, em destaque âmbar**, com link só para o e-mail `urna.velox@proton.me`. Complemento (24/10, 20h59): **usa todo o espaço horizontal que sobrar no topo**. | feito (teste `site/testes/topo_anuncie.mjs`) | 24/10 | 0.9.17; espaço todo na 0.9.18 |
| M-012 | 24/10/2026 | Luciano | página, topo | **Esconder temporariamente o selo "EM BREVE" / Playlist** (prioridade: a totalização do 2º turno); voltar com ele só depois da totalização (tirar o `hidden` do `#conexao` no `site/src/index.html`). Complemento (24/10, 21h00): o aviso "Próxima eleição AO VIVO" também sem o link da playlist (volta junto com o selo). | escondido na 0.9.17; link do aviso fora na 0.9.18; voltar depois da totalização | 24/10 | 0.9.17 / 0.9.18 |
| M-013 | 24/10/2026 | Luciano | página, linha dos termos de uso e da privacidade | **"Pode ficar sempre ao fundo, não precisa ficar sempre visível. Atrapalha a visualização. Deixe para ser visível só quando o usuário rolar até lá embaixo. E aí pode estar centralizado."** (no celular já tinha subido na branch `merge/ws-autenticado`, M-009; aqui, todas as larguras) | feito (teste `site/testes/termos.mjs`) | 24/10 | 0.9.19 |
| M-014 | 24/10/2026 | Luciano | página, faixa (chip da próxima eleição) e letreiro | **"Quero que o texto da próxima eleição fique rodando igual um letreiro se necessário, mas é que quero avisar que a partir da 01h da madruga já podem chegar BUs do exterior."** | feito (teste `site/testes/promo_letreiro.mjs`) | 24/10 | 0.9.19 |
| M-015 | 25/10/2026 | Luciano ("o pessoal não está percebendo que tem um botão para selecionar a origem dos dados: TSE, Ambos ou Urna Velox") | página, cartão da corrida (seletor de fonte) | **Fazer o usuário perceber e entender o seletor de fonte**, de um jeito que **nunca pule** a tela, pensando no celular (onde está a maioria): rótulo "De onde vêm os números" com o ⓘ, nomes **Urna Velox · Comparar · TSE oficial** (o do meio era "Ambos"), uma frase de uma linha embaixo que muda com o modo, o estado "O TSE ainda não começou a totalizar" no lugar do arquivo velho com zeros, folha do ⓘ com a explicação e um botão por fonte, dica da primeira visita (destaque no controle e a frase "Toque em Comparar…") e medição de uso (POST em `/api/v1/uso/<evento>`, contado no log do Caddy) | feito (teste `site/testes/seletor_fonte.mjs`, 212 verificações de 320 a 1440 px, claro e escuro) | 25/10 | 0.9.25 |
| M-016 | 25/10/2026 | Luciano ("quando troca de versão, o usuário tem que fazer um refresh? perde a conexão?") | página, aviso de versão nova | **Avisar quem deixou a aba aberta que há versão nova**, sem recarregar sozinho (não interromper a leitura) e sem nada pular: a mensagem `estado` já traz a `versao` do servidor; se for **maior** que a da página (o `?v=` do `app.js`), aparece o aviso fixo "Nova versão disponível · Atualizar · ✕". Só maior (numa troca de nós um já está na nova e o outro na antiga: isso não é versão nova). O ✕ dispensa a versão; uma ainda maior traz o aviso de volta | feito (teste `site/testes/aviso_versao.mjs`, 106 verificações) | 25/10 | 0.9.26 |
| M-017 | 24/10/2026 | Luciano | página, novo cartão e painel | **Mapa das seções**: cada BU totalizado "explode" e deixa um ponto no mapa do Brasil (no fim, o Brasil todo iluminado pelas ~500 mil seções), só o Brasil escuro sobre o fundo do portal, com opção de **globo 3D** que se gira (o exterior chega em arcos); local de votação do TSE se houver, senão a cidade. **Todo cartão do portal se maximiza e se reorganiza.** Mockup aprovada no chat em 24/10. | feito na branch `claude/mapa-secoes` (testes `site/testes/mapa_pagina.mjs`, `mapa::testes`, `difusao::testes::mapa_das_secoes_ponta_a_ponta`); servidor **ainda sem compilar** (feito numa sessão sem as dependências do Rust) | a definir (não implantar antes do 2º turno sem ensaio) | 1.0.0 |
| M-018 | 25/10/2026 | Luciano | página, topo | **"Pode remover o 'anuncie aqui'. Não vou mais querer mostrá-la."** Sai o chip "Anuncie Aqui" do M-011 (e o link mailto); com o selo escondido (M-012), a grade do topo fica com marca, navegação e tema, o tema no fim da linha | feito (teste `site/testes/topo_anuncie.mjs`: antes, "sem o chip", "nenhum Anuncie" e "nenhum mailto" falhavam nas 10 combinações; depois, tudo ok) | 25/10 | 1.0.0 |
| M-019 | 25/10/2026 | Luciano (17h39, com o TSE sem BU novo desde 17:11:45) | página, faixa do topo | **Aviso enquanto o TSE não publica BUs**, com o texto "TSE sem novos BUs desde 17h11 · retomamos automaticamente quando voltarem", que **some sozinho** quando volta. | feito (teste `site/testes/sem_bus.mjs`) | 25/10, imediato | 1.0.4 |
| M-020 | 25/10/2026 | Luciano (18h03: "controle anti-exposição do TSE") | `uv-servico`, aplicação dos BUs; painel dos nós | **Trava**: se o TSE soltar os BUs de uma vez, o Urna Velox fica muito à frente; segurar o processamento sempre que estiver mais de X à frente do TSE (10, parâmetro alterável no painel), medido só pelo arquivo de Presidente do Brasil. Decisões (18h10): X em **pontos percentuais** de seções totalizadas; sem dado do TSE, **segura no último teto**; ler o `br-c0001` do TSE **sempre** (1 req/30 s por nó; muda o combinado de 16/10). 18h18: **sem mostrar nada ao usuário**. | feito (testes `trava::testes`) | 25/10, imediato | 1.0.6 |

## Testes que comprovam (saída antes e depois)

### B-008 · página vira download (0.8.9)

**Causa achada.** No nó A, o `ciclo.log` de 18/10 mostra o serviço parado às 07:00:02 e o novo no ar às 07:02:25
(o `uv-ciclo.sh` espera ~1 min e o simulador do 452 leva ~1 min para subir): **os dois nós ficaram sem `uv-servico`
por ~2,5 min**, a duração vista pelo Luciano. Nesse intervalo o Caddy (`servidor/Caddyfile.template`) responde
`502`/`503` **vazio e sem `Content-Type`**, mas com o `X-Content-Type-Options: nosniff` do bloco `header`; sem tipo e
sem poder farejar, o navegador do visitante oferece salvar o arquivo. A página do próprio `uv-servico` sempre foi
`text/html` (não era ela).

**Teste:** `bash deploy/oci/testes/pagina_indisponivel.sh` — sobe o Caddy v2.11.7 (a versão dos nós) com o
`Caddyfile.template` dos nós (só a porta do serviço e o log trocados), abre `/` no Chromium e no Firefox do Playwright
(`site/testes/indisponivel.mjs`) com o serviço fora do ar (antes e depois da verificação de saúde) e exige página
`text/html`, nunca download; `/api/v1/estado` tem de responder JSON; com o serviço de volta, a página dele passa.
Os motores sem interface do Playwright mostram página de erro em vez de baixar, mas a resposta é a mesma que virou
download no navegador do visitante.

Antes (0.8.8, vermelho):

```
1) serviço fora do ar (troca de eleição)
FALHOU chromium http://127.0.0.1:38443/ · status 503 · (sem Content-Type)
  - Content-Type (sem Content-Type) (esperado text/html)
FALHOU firefox http://127.0.0.1:38443/ · status - · (sem resposta)
  - navegação falhou: page.goto: NS_ERROR_NET_ERROR_RESPONSE
  FALHOU: página HTML, não download
  ...
    /api/v1/estado com Content-Type ''
  FALHOU: API responde JSON
2) serviço de volta
  ok: a página do serviço passa
pagina_indisponivel: 3 caso(s) falharam
```

Correção: `handle_errors 502 503 504` no Caddyfile — `503` com `Retry-After: 10` e `Cache-Control: no-store`; para
`/`, uma página HTML pequena ("O serviço está sendo atualizado e volta em instantes", recarrega sozinha a cada 10 s);
para `/api/*` e `/ws`, `{"erro":"servico_indisponivel","tentar_em_s":10}` em JSON.

Depois (0.8.9, verde):

```
1) serviço fora do ar (troca de eleição)
ok chromium http://127.0.0.1:38443/ · status 503 · text/html; charset=utf-8
ok firefox http://127.0.0.1:38443/ · status 503 · text/html; charset=utf-8
  ok: página HTML, não download
  ok: página HTML também com os nós fora da rotação
  ok: API responde JSON
2) serviço de volta
  ok: a página do serviço passa
pagina_indisponivel: ok
```

### B-009 · eleitores que faltam para o "matematicamente" travado (0.8.10)

**Causa achada.** Replay local do 452 a 500× (como o ciclo das 07h): o `marcos.csv` do serviço teve só **três**
valores de `faltam_para_mat` para Prefeito de São Paulo na noite inteira (9.295.379 no seq 37.607, 5.903.434 no
243.926 e 0 no 430.621, o "matematicamente 2º turno"), com 6.708 valores para as 5.574 circunscrições juntas. A conta
(uma busca binária com dezenas de avaliações do "matematicamente") rodava na verificação de 1 s da thread do motor
com **orçamento de 0,5 ms por rodada** para todas as circunscrições, em rodízio: com os BUs chegando depressa, cada
circunscrição só era recontada a cada muitos minutos e o contador da tela ficava parado. A página e a difusão
entregavam certo o que recebiam.

**Teste:** `faltam_acompanha_a_apuracao` (`aquisicao/tests/servico.rs`): replay só de SP do 1º turno de 2024 a 120×
contra o `uv-simulador`; para `sp71072-c0011`, enquanto a disputa está aberta, nenhum trecho com mais de 5 % dos BUs
da noite pode passar sem valor novo no `marcos.csv`.
`UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test servico -- --ignored faltam_acompanha_a_apuracao`

Antes (0.8.9, vermelho):

```
faltam_acompanha_a_apuracao: 3 valores de faltam_para_mat para São Paulo antes do matematicamente (seq 81597 de 102602);
maior trecho sem valor novo: 30447 BUs (29.7 %)
test result: FAILED
```

(a 500× o mesmo teste dava 0 valores antes do "matematicamente": parado em 79,5 % da noite.)

Correção: a conta vai para a thread `uv-faltam` (`marcos::ContaFaltam`, prioridade baixa). A verificação de 1 s só
manda a fotografia (votos e restante) das circunscrições em disputa que receberam BU e recolhe as respostas da rodada
anterior; resposta de fotografia mais velha que o último valor mandado (o 0 do evento) é descartada. Sem orçamento:
as 645 circunscrições de SP cabem numa rodada.

Depois (0.8.10, verde):

```
faltam_acompanha_a_apuracao: 52 valores de faltam_para_mat para São Paulo antes do matematicamente (seq 81594 de 102602);
maior trecho sem valor novo: 3989 BUs (3.9 %)
test result: ok
```

### B-014 · celular trava e esquenta (0.8.11)

**Medida antes de mudar.** `ensaio_carga_pagina.sh`: replay do 452 inteiro a 500× (o modo rápido do ciclo), página
aberta como celular (390 px, CPU 4× mais lenta pelo CDP) em Prefeito de São Paulo, contando o que chega pelo
WebSocket. Em 92 s a página recebeu **61.326 mensagens e 16 MB** (pico de 11,5 MB num minuto), resultado de **191
tópicos**, e a thread principal passou 13 % do tempo em tarefas longas. O grosso não eram os resultados (1,1 MB):
eram **47.032 marcos (11,4 MB)** — sobretudo o `faltam_para_mat` das 5.574 circunscrições, que desde a 0.8.10 (B-009)
sai 1×/s de cada uma — e **11.200 eventos (2,5 MB)** de todos os prefeitos do país.

**Teste:** `bash aquisicao/scripts/ensaio_carga_pagina.sh` (a página vem embutida no serviço: `npm run build` e
`compilar_dev.sh` antes). Falha se a página recebe resultado de mais de 3 tópicos (a tela e a circunscrição dela),
se algum minuto passa de 1.500 KB ou se a thread principal fica mais de 25 % do tempo em tarefas longas.

Antes (0.8.10, vermelho):

```
carga_pagina: 92 s, 61326 mensagens, 16021 KB (máx. 11500 KB/min), 191 tópicos de resultado, tarefas longas 11.9 s (13 %)
por tipo: marco 47032 msgs 11368 KB · evento 11200 msgs 2480 KB · resultados 1726 msgs 1257 KB · previsao 521 msgs 313 KB
FALHOU
  - recebeu resultado de 191 tópicos (teto 3: a tela e a circunscrição dela)
  - 11500 KB num minuto (teto 1500)
```

Correção (a direção combinada em 18/10): **resultado inteiro só da tela e da circunscrição dela** (`atualizarTopicos`;
o ritmo da faixa passa a usar também o `ultimo_bu.seq` do `estado`); no protocolo, `op: marcos` aceita
`faltam: [tópicos]` (só esse `faltam_para_mat`: o da tela e o de Presidente) e `eventos`/`marcos` aceitam
`escopo: "uf"` (de fora dos tópicos inscritos, só Brasil e UFs). Sem os campos novos o servidor se comporta como
antes. Saíram do letreiro "disputa mais apertada / mais folgada" e "comparecimento por UF", que precisavam do
resultado de todos os tópicos (podem voltar como marcos calculados no servidor).

Depois (0.8.11, verde):

```
carga_pagina: 70 s, 466 mensagens, 406 KB (máx. 404 KB/min), 1 tópicos de resultado, tarefas longas 3.4 s (5 %)
por tipo: atualizacao 266 msgs 297 KB · previsao 118 msgs 87 KB · marco 69 msgs 16 KB · estado 8 · evento 4 · instantaneo 1
ok
```

### B-013 e B-016 · a previsão não aparece / o gráfico da previsão não é atualizado (0.8.12)

**Causa achada.** No nó A, durante o simulado das 13h de 18/10 (1º turno de 2026, 500×), o `previsoes.csv` parou em
`2026-10-05 08:00:01` com a base em **21 % das seções** (`br-c0001`, situação "prevista", faixa ainda incerta), embora
o serviço tenha chegado a 499.207 seções aplicadas; a `saude.json` dizia `"definidas": 0`. A thread de previsão anda
pelo **relógio do site** (o `Date` do simulador): no modo rápido o relógio do simulador chega ao fim da noite muito
antes de os nós ARM aplicarem os BUs, e daí em diante a previsão não fazia mais nenhuma rodada — o cartão e o gráfico
ficavam parados (e o "matematicamente" nunca era registrado na previsão). Numa máquina rápida o serviço acompanha o
simulador e o defeito não aparece.

**Teste:** `previsao_acompanha_servico_atrasado` (`aquisicao/tests/servico.rs`): replay só do AC do 1º turno de 2026 a
500× com a linha do tempo do ciclo, o simulador respondendo em 200–300 ms e o serviço com 8 seções ao mesmo tempo
(a noite do simulador acaba na metade da coleta, como nos nós); a última previsão de `ac-c0003` tem de ter ao menos
90 % das seções apuradas (ou ser a "definida").

Antes (0.8.11, vermelho):

```
previsao_acompanha_servico_atrasado: 486 previsões de ac-c0003; primeira ("2026-10-04 18:18:37", "prevista", 0.0573);
última ("2026-10-05 02:53:10", "prevista", 0.4881)
a previsão parou com 48.8 % das seções apuradas (prevista às 2026-10-05 02:53:10)
```

Correção: `previsao::relogio` — se o relógio do site está parado há mais de 3 s e os BUs continuam entrando, o relógio
da previsão anda sozinho na velocidade do site (1/`escala`); na noite real o relógio do site anda sempre e nada muda.
Junto (segunda parte do B-013), o cartão diz o que prevê: "Matematicamente 2º turno **sai** entre 01:57 e 02:30",
"**Fim da apuração:** 99 % … · 100 % …" e o título "Previsão (estimativa de horário)".

Depois (0.8.12, verde):

```
previsao_acompanha_servico_atrasado: 1044 previsões de ac-c0003; primeira ("2026-10-04 18:18:47", "prevista", 0.0573);
última ("2026-10-05 13:03:21", "definida", 1.0)
test result: ok
```

Na página: `site/testes/previsao_pagina.mjs` (com `UV_PLEITO=3220 UV_TESTE_PAGINA=testes/previsao_pagina.mjs
aquisicao/scripts/ensaio_carga_pagina.sh 500 '#/br/c0001'`) anota o cartão a cada segundo e falha se o gráfico ficar
parado mais de 20 s ou for redesenhado menos de 10 vezes antes do "matematicamente".

### B-011 · a curva da apuração pula (0.8.13)

**Causa achada.** A mesma raiz do B-016, do lado da curva. A curva de cada tópico (`difusao`, um ponto por minuto)
usa como eixo o horário do site em que o BU foi aplicado. No modo rápido do ciclo o relógio do simulador corre a 500×
sem esperar os nós: na captura do Luciano a linha de seções chega a ~20 % ao longo de 18:27–08:00 (o relógio do
simulador vai até o limite de 08:00) e os outros ~80 % entram todos no mesmo instante, quando o relógio para — o salto
vertical, junto com o dos candidatos. Na noite real o relógio do TSE é o da chegada dos BUs e isso não acontece. O pico
isolado do começo (um candidato a 28,5 % e de volta) são as primeiras seções, de poucos votos: a escala já ignora o
trecho abaixo de 1 % das seções, mas a linha é desenhada.

**Teste:** `curva_sem_salto_no_modo_rapido` (`aquisicao/tests/servico.rs`): AC do 1º turno de 2026 a 500× com os
argumentos do modo rápido do ciclo (`SIM_RAPIDO`), o simulador respondendo em 200–300 ms e o serviço com 8 seções ao
mesmo tempo (o serviço fica atrás do relógio, como nos nós ARM); na curva de `ac-c0003` nenhum passo de um minuto
pode subir mais de 15 pontos percentuais das seções.

Antes (0.8.12, `SIM_RAPIDO` vazio como no ciclo; vermelho):

```
curva_sem_salto_no_modo_rapido: 374 pontos, de 04/10/2026 17:15:05 a 05/10/2026 02:53:14; maior passo 51.5 p.p.
(ponto 372: ("05/10/2026 02:52:25", 48.5) → ("05/10/2026 02:53:14", 100.0))
a curva de seções salta 51.5 pontos percentuais de um minuto para o outro
```

Correção: `uv-simulador --acompanhar-servico <pct>` — o relógio não passa do instante em que as seções
disponibilizadas e ainda sem o BU servido seriam mais que essa % das seções com BU; volta a andar, na mesma
aceleração, quando o serviço busca os BUs (sem BU novo por 10 s, anda solto: seção nunca buscada não trava a noite).
O `uv-ciclo.sh` passa `--acompanhar-servico 1` no modo rápido; `simulador_resumo.json` ganhou `acompanhar` (folga,
BUs servidos, freadas). A cadência real e o TSE de verdade não mudam. Vale também para o B-001 (ordem do modo rápido)
e para a previsão do modo rápido.

Depois (0.8.13, `SIM_RAPIDO = --acompanhar-servico 1`; verde):

```
curva_sem_salto_no_modo_rapido: 139 pontos, de 04/10/2026 17:11:09 a 04/10/2026 20:59:17; maior passo 4.3 p.p.
test result: ok
```

### B-010 · "sempre aparece 2º turno" — não reproduzido (18/10/2026)

O que se conferiu: a regra está no motor (`candidatos::Sistema::para`: Presidente e Governador por maioria absoluta dos
votos válidos; Prefeito por maioria absoluta só acima de `LIMITE_2T_PREFEITO` = 200.000 eleitores, abaixo disso
maioria simples; testes `sistema`/`situacao` de unidade). No replay local do 1º turno de 2024 (0.8.11), das 5.574
prefeituras: 5.466 de até 200 mil eleitores terminaram "definido" com um "Eleito" (nenhuma com 2º turno), 52 maiores
com eleito no 1º turno e 51 com "2º turno" — o mesmo do oficial (as conferências `--ignored` 5.574/5.574 situações).
As previsões do "matematicamente" (`previsoes.csv`) também batem com o desfecho na quase totalidade. Sem saber em que
parte da tela o "2º turno" apareceu, não há teste que comprove o bug; combinado com o Luciano: fica como não
reproduzido e ele avisa se vir de novo (anotar a disputa e o cartão).

### B-012 · dois candidatos com cores quase iguais (0.8.14)

**Causa achada.** `site/src/cores.js` dá a cada candidato o matiz do partido e, quando dois da mesma disputa ficam
parecidos, escolhia o **primeiro** tom alternativo a pelo menos 0,075 (OKLab, só no tema escuro) dos já escolhidos —
ou o mais distante, se nenhum chegasse lá. Com vários partidos da mesma família de cor (verdes: MDB e PRTB em São
Paulo; vermelhos: PT, PSOL, PSTU, PCB, PCO, Mobiliza) os pares ficavam a 0,05: quase a mesma cor na curva e na legenda.

**Teste:** `node site/testes/cores_distancia.mjs <candidatos.json>…` — em todas as disputas dos `candidatos.json` de
2024 (1º e 2º turnos) e do 1º turno de 2026, a menor distância OKLab entre dois candidatos da mesma disputa, **nos dois
temas**, tem de ser ≥ 0,12 nas disputas de até 8 candidatos (todas as de 2º turno e quase todas as de Prefeito e
Governador) e ≥ 0,075 nas maiores.

Antes (0.8.13, vermelho):

```
cores_distancia: 5446 disputas; 2650 com algum par abaixo do limiar (0.12 até 8 candidatos, 0.075 acima) em OKLab;
pior 0.047; pior até 8 candidatos 0.051
```

Correção: cada candidato tem 25 tons possíveis da própria cor (luminosidade ±0,07/±0,14 e matiz ±12°/±24°: o verde
continua verde) e a disputa é resolvida em rodadas — cada um troca para o tom que mais o afasta do mais próximo, nos
dois temas, até todos ficarem a 0,14 ou nada melhorar; com folga, fica o tom mais perto da cor do partido. A cor
continua dependendo só de quem está na disputa (não muda durante a noite).

Depois (0.8.14, verde):

```
cores_distancia: 5446 disputas; 0 com algum par abaixo do limiar (0.12 até 8 candidatos, 0.075 acima) em OKLab;
pior 0.080; pior até 8 candidatos 0.122
```

### B-004 · `para` devolve erro com arquivo grande que chegou (0.8.15)

**Causa achada.** O `para` (e o `em`) roda o comando no nó pelo Run Command da OCI e lê o resultado na linha
`UV_RES:` do texto da saída. Quando o Run Command já está "SUCCEEDED" mas o texto ainda veio vazio (o caso do tar de
3,7 GB: a cópia demorada termina e o texto não acompanha), o código desistia na primeira consulta e devolvia erro
("Run Command SUCCEEDED:" sem saída), embora o arquivo tivesse chegado inteiro.

**Teste:** `python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'` (`test_uv_oci.py`, clientes da OCI
falsos; para isso o Python do `_oci_op` saiu do heredoc do `lib.sh` para `deploy/oci/uv_oci.py`, mesma lógica):
estado final sem texto e arquivo inteiro no nó → `para` tem de dar 0; arquivo com tamanho errado → erro; texto que
chega depois do estado final → o resultado dele.

Antes (0.8.14, vermelho):

```
FAIL: test_saida_vazia_mas_o_arquivo_chegou — AssertionError: 1 != 0
FAIL: test_texto_chega_depois_do_estado — AssertionError: Tuples differ: (1, b'') != (0, b'ok\n')
Ran 5 tests — FAILED (failures=2)
```

Correção: com o estado "SUCCEEDED" e sem a linha `UV_RES`, consulta de novo (até 10 vezes, 3 s entre elas); se ainda
assim não vier, o `para` confere o tamanho do destino no nó (`stat -c %s`) e dá 0 se for igual ao do arquivo local
(com aviso), senão erro com os dois tamanhos.

Depois (0.8.15, verde): `Ran 5 tests — OK`. Conferido também contra a OCI de verdade (nó A): `em`, `para` de um
arquivo pequeno para `/tmp`, `em` com `bash -s` e `de` — todos com código 0.

### M-001 · identificação da eleição sempre à vista (0.8.16)

Três protótipos mostrados ao Luciano em capturas (a: faixa fina fixa; b: o topo fica fixo e troca a marca pela
identificação; c: cartão à moda do aplicativo da Justiça Eleitoral). Escolhida a **C**.

**Teste:** `UV_PLEITO=452 UV_TESTE_PAGINA=testes/identificacao.mjs bash aquisicao/scripts/ensaio_carga_pagina.sh 60
'#/sp71072/c0011'` (`site/testes/identificacao.mjs`, a 1366 e a 390 px, durante um replay): o cartão mostra eleição,
tipo, turno marcado, local e cargo; rolada a página, a identificação continua à vista presa no alto (topo ≤ 12 px, uma
linha ≤ 48 px) com turno e local; o botão recolhe, a escolha sobrevive a um recarregamento e o botão reabre; CLS < 0,01.

Antes (0.8.15, vermelho):

```
page.evaluate: TypeError: Cannot read properties of null (reading 'getBoundingClientRect')   (não há cartão)
```

Primeira versão (cartão que virava `position: fixed` ao rolar), também vermelha — a troca para `fixed` desloca o
próprio cartão na tela e conta como deslocamento de layout:

```
1366px: … rolado top 8 alt 40 · CLS 0.0082/0.2264/0.001
FALHOU — 1366px: CLS 0.0082 no topo, 0.2264 rolado; 390px: CLS 0 no topo, 0.0893 rolado
```

Correção: cartão estático no lugar reservado do `index.html` (88 px; 80 no celular; 40 recolhido) e, separada, uma
barra de uma linha sempre `position: fixed`, que entra por `transform`/opacidade quando o cartão sai da tela (não conta
no CLS). Texto do local alinhado à esquerda e "Gerais/Municipais" com lugar reservado, para nada andar ao preencher.
Recolhido: cartão e barra viram só o botão (lembrado no `localStorage`, aplicado ao carregar o módulo). O título da aba
também passa a dizer cargo, local e turno. `body` com `overflow-x: clip` (era `hidden`).

Depois (0.8.16, verde):

```
1366px: Eleições 2024 · Municipais · 1º turno · São Paulo (SP) · Prefeito · rolado top 8 alt 40 · CLS 0.0014/0.0014/0.001
390px: Eleições 2024 · Municipais · 1º turno · São Paulo (SP) · Prefeito · rolado top 8 alt 40 · CLS 0/0/0.0078
ok
```

### B-005 · situações divergentes no 1º turno de 2024 (0.8.17)

O `candidatos.json` do acervo das simulações foi montado com `preparar --ignorar-destinacao` (todos "Válido", como na
véspera). Votos idênticos, mas 33 situações "Eleito" × "Não eleito" (oficial) em 32 municípios com candidato depois
anulado/anulado sub judice, e o alerta `destinacao_presumida` em toda simulação.

**Teste:** `motor/tests/integracao.rs::primeiro_turno_2024_simulacao_destinacao_final` (`--ignored`, `UV_DADOS`):
totaliza os BUs do 452 com `2024-1t/preparo-simulacao/candidatos.json` e exige 5.574 arquivos, 0 situações
divergentes, conferência idêntica (direta e pelas publicações JSON) e 15.042 situações.

Antes (com o `candidatos.json` que está hoje no acervo dos nós, vermelho):

```
mg45616-c0011-e000619-u.json (cargo 11): situação do candidato 45 THIAGO CÂMARA: oficial 'Não eleito', Urna Velox 'Eleito'
… (33 no total)
assertion `left == right` failed: situações divergentes: 33
  left: (5574, 33)
 right: (5574, 0)
```

Correção: `deploy/oci/candidatos_simulacao.sh` gera os `candidatos.json` dos três pleitos das simulações (452, 453,
3220) com o `preparar` **sem** `--ignorar-destinacao`, só do cache (`--offline`: os `-u.json` finais; nenhuma
requisição) — 270, 1 e 38 candidatos com destinação diferente de "Válido", `destinacao_presumida` falso. Fora a
destinação, os três são iguais aos do acervo atual (conferido). Com `--enviar`, troca o arquivo no acervo dos nós A e
B (vale a partir da próxima eleição do ciclo). A noite real não muda: sem `dvt`, destinação presumida e alerta.

Depois (0.8.17, verde):

```
2024 1º turno, simulação: 5574 arquivos oficiais conferidos: 5574 idênticos, 0 com divergência; cargo 11: 5569/5569; …
  | situação: 15042 de 15042 candidatos idênticas
2024 1º turno, simulação, publicações JSON: 5574 idênticos … | situação: 15042 de 15042 candidatos idênticas
test result: ok. 1 passed
```

Nos nós: **pendente de envio** (`./candidatos_simulacao.sh --enviar`, com o OK do Luciano).

### B-007 e B-015 · scripts de análise dos acessos e heredoc sem "bash -s" (0.8.18)

B-007: o Luciano decidiu o destino dos três scripts de outra sessão (análise dos logs de acesso do Caddy). Ao
revisá-los, um deles tinha o mesmo defeito do B-015: `em "$N" << 'CMD'` sem `"bash -s"`, que no Run Command da OCI
chega vazio ao nó. Os dois itens foram resolvidos juntos, com um teste para o padrão em todos os scripts.

**Teste:** `deploy/oci/testes/test_scripts_em.py` (`python3 -m unittest discover -s deploy/oci/testes -p 'test_*.py'`):
nenhum `em <nó> <<` sem `"bash -s"` nos `.sh` de `deploy/oci` e `deploy/oci/servidor`; `bash -n` em todos os `.sh`;
compilação de todos os `.py`.

Antes (vermelho):

```
FAIL: test_em_com_heredoc_tem_bash_s
heredoc para "em" sem "bash -s" (chega vazio ao nó):
02_configurar_borda.sh:49: em "$N" <<CMD
```

Correção: `"bash -s"`; em `02_configurar_borda.sh`, erro (e não "Nó configurado e ativo") se a instalação
falhar.

Depois (0.8.18, verde): `Ran 8 tests — OK` (3 novos + os 5 do `test_uv_oci.py`).

### M-002 · vazão de ponta a ponta e topologia — medição (0.8.21)

Relatório: [RELATORIO_M002_VAZAO.md](RELATORIO_M002_VAZAO.md). Nada mudou nos nós (decisão de 18/10: medir sem tocar).

* **Noite real:** quem limita é o teto de 100 req/s por IP (2 requisições por BU → ~50 BU/s por nó). Chegada ao TSE
  com pico de 115 BU/s (1 min). 2 nós: atraso máx. 0,8 min (452) / 0,5 min (3220); 1 nó: 45 / 38 min; 3 nós: ~0.
  Sem folga hoje: um nó fora no pico → ~40 min.
* **Simulações (modo rápido):** limitadas pela CPU dos nós de 2 núcleos: ~1.100 BU/s para o par; `uv-servico` ~1,1
  núcleo; ~1 ms de CPU por BU; totalização < 1 % do tempo; `fsync` do diário com p99 de 3–10 s.
* **Opções:** manter 2 nós; 3º nó com IP próprio (N+1); 1 requisição por BU; simulações mais leves (cs.json no modo
  rápido); nós maiores. Escolha pendente com o Luciano.

**Teste das medidas:** `aquisicao/scripts/testes/test_m002.py` (`python3 -m unittest discover -s
aquisicao/scripts/testes -p 'test_*.py'`): fila com capacidade folgada não atrasa; rajada de 6.000 BUs a 50 BU/s
sai em 119 s (59 s com 2 nós); o cs.json desconta da capacidade; `medir_vazao.py` numa execução sintética (fases,
vazão, CPU em núcleos, fila, repetidos da federação). Primeira rodada: 1 falha — a expectativa do teste estava errada
(a fila zera no segundo 119, não no 120; o código estava certo). Depois: `Ran 4 tests — OK`.

### B-003 e B-002 · RESUMO.json: reinício intencional e ordem dominada pelos tardios (0.8.25)

Os dois ficam no `deploy/oci/servidor/uv-ciclo-resumo.py`.

* **B-003:** o resumo contava toda troca de pid do `uv-servico` como falha; o `63_ciclo.sh --reiniciar` não deixava
  rastro. Agora o `--reiniciar` anota a hora em `atual/reinicios_intencionais` (o `arquivar` guarda o arquivo nas
  evidências) e o resumo desconta a troca de pid vista até 60 s depois de uma anotação (uma troca por anotação),
  com a observação "N reinício(s) intencional(is)". Queda sem anotação continua reprovando.
* **B-002:** os BUs recebidos no TSE dias depois da noite entram na simulação no corte (`disponivel` = 02:00,
  `recebido` = a data real). Um só aplicado no meio da noite levava o "maior recebido até aqui" para dias à frente e
  todo BU seguinte parecia atrasado. Agora ficam fora da medida de ordem e são contados à parte
  (`recebidos_depois_do_corte`).

**Teste:** `deploy/oci/testes/test_ciclo_resumo.py` (execuções sintéticas; `python3 -m unittest discover -s
deploy/oci/testes -p 'test_*.py'`).

Antes (vermelho):

```
FAIL: test_recebidos_depois_do_corte_nao_dominam_o_atraso — 543097.0 not less than or equal to 60
FAIL: test_reinicio_intencional_nao_reprova — 'falhou' != 'aprovado'
Ran 5 tests — FAILED (failures=2)
```

Depois (0.8.25, verde): `Ran 5 tests — OK` (`deploy/oci/testes`: 13/13).

Com as evidências reais de 18/10 (resumo antigo × novo, atraso p99 da ordem):

| execução | antes | depois | tardios fora da medida |
|---|---|---|---|
| 452 07h, nó A | 1.057.088 s (12 dias) | 19.539 s (5,4 h) | 322 |
| 3220 13h, nó A | 246.600 s (2,9 dias) | 26.929 s (7,5 h) | 2.923 |
| 3220 13h, nó B | 245.824 s | 26.034 s | 2.923 |

As horas que restam são desordem real dessas execuções (relógio do simulador à frente dos nós no modo rápido,
corrigido no B-011, 0.8.13; o B-001 trata da ordem do modo rápido). O reinício do nó B no 452 das 07h continua
reprovando, como deve: não havia anotação.

### B-006 · contadores da federação recomeçam na retomada (0.8.26)

Os contadores da federação (`publicados`, `lotes_gravados`, `recebidos`, `ja_aplicados`…) eram só memória
(`ContFed`, `AtomicU64`): o processo novo começava do zero, enquanto o `seq` dos lotes era recuperado do armazém
(`bus/<nó>/ultimo`). O nó B mostrou 91 mil publicados ao fim, tendo coletado ~240 mil.

**Teste:** `aquisicao/src/federacao/no.rs::contadores_sobrevivem_a_retomada` (700 registros em 2 lotes, 42 recebidos
e 7 já aplicados; um `Federado` novo no mesmo armazém — a retomada — tem de começar com esses valores e seguir
acumulando). E, de ponta a ponta, `aquisicao/tests/federacao.rs::kill9_e_reinicio` (`--ignored`) passou a exigir que o
`lotes_gravados` final do b seja o último lote dele no armazém e que os `publicados` não fiquem abaixo do valor de
antes do `kill -9`.

Antes (vermelho):

```
assertion `left == right` failed: publicados/lotes recomeçaram
  left: (0, 0)
 right: (700, 2)
```

Correção: os contadores vão para o armazém local em `estado/<nó>/contadores.json` (fora de `nos/` e `bus/`, não é
servido aos pares) a cada lote gravado e a cada ~1 s no publicador; o `Federado::novo` os recupera. O
`lotes_gravados` vem do próprio `ultimo` (exato mesmo num `kill -9` entre o lote e os contadores). Numa eleição nova
o armazém é outro e tudo começa do zero, como antes.

Depois (0.8.26, verde): o teste unitário passa; `kill9_e_reinicio` ok (45 s); suíte completa ok.

### B-001 · ordem do modo rápido × ordem do TSE — medido; resolvido pelo B-011 (0.8.27)

Pista do handoff: só no modo rápido; medir antes de mudar. Medido com o `uv-ciclo-resumo.py` da 0.8.25 (sem os BUs
recebidos depois do corte, B-002) nas evidências do ciclo de 18/10 (`63_ciclo.sh --telemetria`):

| execução | versão | atraso p50 (tempo da noite) | atraso p99 | deslocamento p50 |
|---|---|---|---|---|
| 3220 13h, nó A / B | 0.8.10 (sem `--acompanhar-servico`) | 4,3 min / 2,8 min | 26.929 s / 26.034 s (7,5 h) | 79.708 / 63.640 |
| 3220 15h18, nó A / B | 0.8.17 (com `--acompanhar-servico 1`, B-011) | **6 s / 3 s** | 1.680 s / 481 s | 2.936 / 758 |

O que o B-001 descrevia (~10 min da noite na mediana) vinha do relógio do simulador à frente dos nós no modo rápido:
tudo ficava disponível de uma vez e os lotes do par chegavam embaralhados. Com o relógio acompanhando os nós (B-011,
0.8.13), a mediana caiu para 3–6 s da noite. O p99 que sobra (8–28 min da noite) é, a 500×, 1–3,4 s de tempo real:
a cadência da troca de lotes (0,1 s × 500 = 50 s da noite) mais a fila — o nó A, que também roda o simulador, fica
mais atrás. Na cadência real (1×) a ordem é a do TSE. **Nenhuma mudança de código**; fica como resolvido pelo B-011.

Para repetir: extraia a execução baixada (`no-a.tar.gz`), monte `saida/secoes.log` (descompactado), `saida/saude.json`,
`telemetria.jsonl`, `params` e `id` numa pasta e rode `python3 deploy/oci/servidor/uv-ciclo-resumo.py <pasta> /tmp`
(campo `ordem`).

### M-003 · horários do Urna Velox no relógio do sistema, separados dos do TSE (0.8.28)

**O que havia.** Todo horário que a página mostrava como do Urna Velox era, na verdade, do relógio do TSE: o
"Horário do último BU" e a curva usavam o `ultimo_bu` (o horário em que o BU apareceu no site, `Date`); a previsão
andava pelo relógio do site (o do simulador, no simulado); os eventos e marcos do feed traziam o horário do BU. Num
simulado de 2024 rodado hoje, a tela dizia "06/10/2024 20:15" em vez da hora em que o sistema totalizou, e a
comparação "Urna Velox × TSE" subtraía horários de dias diferentes.

**Teste:** `horarios_do_sistema_no_simulado` (`aquisicao/tests/servico.rs`, `--ignored`): AC do 1º turno de 2026 a
500× no modo rápido do ciclo; exige a publicação com `ultimo_bu` (TSE, outubro de 2026) **e** `totalizado_em` (relógio
do sistema, dentro da janela do ensaio), a âncora `relogio_sistema` no `/api/v1/estado` (aceleração 500), a curva e a
previsão no relógio do sistema, os eventos do "matematicamente" com `sistema`, e a curva sem salto (critério do B-011).
`UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test servico -- --ignored horarios_do_sistema_no_simulado`

Antes (0.8.27, vermelho):

```
horarios_do_sistema_no_simulado: ensaio 18/10/2026 20:38:57 a 18/10/2026 20:41:41; ultimo_bu "2026-10-04T20:58:20-03:00"; totalizado_em ""
  relogio_sistema null
  curva: 136 pontos, 136 fora do relógio do sistema, maior passo 4.4 p.p.
  previsão: instante "2026-10-04 20:58:33" relogio null · 1 horários, 1 fora
  eventos do matematicamente: 6, 6 sem horário do sistema
M-003:
  - totalizado_em "" fora do relógio do sistema
  - estado sem a âncora do relógio do sistema: null
  - curva: 136 pontos, 136 fora do relógio do sistema
  - previsão fora do relógio do sistema: ["2026-10-04 20:58:33"]
  - 6 de 6 eventos sem horário do sistema
test result: FAILED
```

Implementação (`aquisicao/src/relogio.rs`): a **âncora** liga as duas escalas — no primeiro `Date` do site (`noite0`) o
relógio do sistema marcava `sistema0`; com a aceleração do simulado (1 na noite real), um instante `t` do TSE equivale a
`sistema0 + (t − noite0) / acelerar` no sistema. Isso é o ajuste pedido: desloca a data da eleição real para o início
do simulado e, no modo rápido, comprime pela velocidade, para medir a distância entre as duas apurações.
* Publicação: `totalizado_em` (relógio do sistema, ms) logo depois do `ultimo_bu` (que continua sendo o do TSE).
* Eventos e marcos: `sistema`; `estado`: `relogio_sistema` (`docs/PROTOCOLO_WS.md`, acréscimos da `0.8.28`).
* Previsão: a conta corre na escala da noite com o relógio **do sistema** (não mais o do site) e todos os horários da
  mensagem saem no relógio do sistema (`relogio: "sistema"`); o `previsoes.csv` fica na escala da noite.
* Curva: eixo no relógio do sistema, um ponto por minuto da noite (0,12 s a 500×).
* Página (`site/src/relogio.js`): "Totalizado pelo Urna Velox às 21:17:25" e, embaixo, "último BU no TSE: 20:59:17
  de 04/10"; feed com o horário do sistema (o do TSE no detalhe); "TSE confirmou … às hh:mm do TSE (≙ hh:mm aqui) —
  Urna Velox às hh:mm, N s antes", com a antecedência também no tempo da noite; cabeçalho da comparação com o arquivo
  do TSE e o equivalente no simulado; previsão com segundos quando acelerada. Teste de unidade:
  `node site/testes/relogio.mjs`; da tela: `site/testes/horarios_pagina.mjs`.

Depois (0.8.28, verde):

```
horarios_do_sistema_no_simulado: ensaio 18/10/2026 21:14:48 a 18/10/2026 21:17:31; ultimo_bu "2026-10-04T20:59:17-03:00"; totalizado_em "2026-10-18T21:17:25.157-03:00"
  relogio_sistema {"agora":"2026-10-18T21:17:26.511-03:00","noite0":"2026-10-04T17:08:48-03:00","sistema0":"2026-10-18T21:14:54.339-03:00","acelerar":500.0}
  curva: 811 pontos, 0 fora do relógio do sistema, maior passo 0.4 p.p.
  previsão: instante "2026-10-18 21:17:25" relogio "sistema" · 1 horários, 0 fora
  eventos do matematicamente: 6, 0 sem horário do sistema
test result: ok
```

Suíte: unidade ok; motor `--ignored` (2026 83/83, 2024 5.574/5.574 e 51/51, situações e "matematicamente") ok;
serviço `--ignored` 16/16 — o `kill9_tres_vezes` acusou o campo novo na comparação com a execução sem queda (ele
depende do instante, como o `ultimo_bu`): o teste passou a descartá-lo também. Votos e situações não mudam.

### B-018 · sumiu o chip de BUs/min no celular (0.8.31)

Introduzido na 0.8.29: o CSS de até 640 px tinha `.faixa .chip.playlist:not(.oculto) ~ .chip.ritmo { display: none; }`
para caber o botão "▶ Playlist". Teste: `site/testes/playlist_pagina.mjs` passou a exigir os três chips da faixa
(dados reais, playlist, BUs/min) inteiros e dentro da margem lateral de 16 px, a 1366, 390 e 360 px.

Antes (binário 0.8.29, o de produção):

```
ok    desktop: chip BUs/min inteiro à vista (1141–1263 de 1366 px)
ok    celular-390: chip playlist inteiro à vista (168–246 de 390 px)
FALHA celular-390: chip BUs/min inteiro à vista (oculto)
ok    celular-360: chip playlist inteiro à vista (168–246 de 360 px)
FALHA celular-360: chip BUs/min inteiro à vista (oculto)
2 falha(s)
```

Correção: no celular o chip da simulação mostra só "Dados reais" (o "do TSE" e o resto ficam no `title` e na
explicação que abre ao tocar), o ritmo não é mais escondido e perde a largura fixa de 150 px.

Depois:

```
ok    desktop: chip BUs/min inteiro à vista (1141–1263 de 1366 px)
ok    celular-390: chip dados reais inteiro à vista (16–110 de 390 px)
ok    celular-390: chip playlist inteiro à vista (120–198 de 390 px)
ok    celular-390: chip BUs/min inteiro à vista (208–330 de 390 px)
ok    celular-360: chip dados reais inteiro à vista (16–110 de 360 px)
ok    celular-360: chip playlist inteiro à vista (120–198 de 360 px)
ok    celular-360: chip BUs/min inteiro à vista (208–330 de 360 px)
tudo certo
```

### B-019, B-020 e B-021 · painel em produção: só um nó, tela de configuração larga no celular (19/10/2026)

Pedido do Luciano em 19/10 ("veja tudo"), com duas capturas do iPhone: ordem de ataque 1, os três juntos.

**B-020 (e B-021) — teste:** `site/testes/painel_celular.mjs` serve a página do painel com uma API falsa (dois nós,
valores longos como os de produção) e confere a 360, 390, 430 px e no desktop, nos dois temas e nas três abas, que nada
passa da borda direita; confere também os campos com letra de 16 px (abaixo disso o Safari do iPhone aproxima a página
ao tocar no campo) e o aviso do ciclo de simulados. Antes da correção (0.8.34):

```
FALHOU 360 px · escuro · config: nada passa da tela (360 px) — 163 elementos, ex.: div (1236 > 360), section.caixa (1236 > 360), …
FALHOU 390 px · escuro · config: nada passa da tela (390 px) — 162 elementos, ex.: div (1236 > 390), …
FALHOU 430 px · claro · config: nada passa da tela (430 px) — 161 elementos, ex.: div (1236 > 430), …
FALHOU 390 px · config: campos com letra de 16 px ou mais (o Safari do iPhone não aproxima ao tocar) — 17 abaixo
FALHOU 390 px · config: avisa que o nó está no ciclo de simulados (uv-ciclo-no ativo, urna-velox parado)
painel_celular: 8 falha(s)
```

Causa: abaixo de 900 px a grade da configuração tinha uma coluna `1fr`, que não encolhe abaixo do conteúdo mais
largo (a tabela da auditoria, com o JSON da mudança numa linha só): a página ia a 1.236 px. Correção: colunas
`minmax(0, 1fr)`, tabelas que quebram o texto das células, campos com 16 px no celular e, quando o `uv-ciclo-no` está
ativo e o `urna-velox` parado, um aviso no topo da configuração. Depois: `painel_celular: tudo ok` (24 conferências de
largura + 2).

**B-019 — teste:** `deploy/oci/testes/test_liberar_portas.py` (clientes falsos da OCI). A porta 8095 do painel precisa
estar aberta para a sub-rede na Security List; o `02_configurar_borda.sh` procurava a lista pelo nome com a OCI CLI e,
sem achar, seguia em silêncio — no log da implantação da 0.8.34 aparece "Atualizando Security List…" sem o
"atualizada com sucesso". Antes da correção:

```
AttributeError: module 'uv_oci' has no attribute 'liberar_portas'
Ran 5 tests — FAILED (errors=5)
```

Correção: `uv_oci.py liberar-portas` chega à lista pelo caminho da instância (VNIC → sub-rede → Security List), só
acrescenta as regras que faltam (nunca apaga) e falha alto sem lista; o `02` usa ele e para com erro se não conseguir;
o `80_painel.sh` confere no fim, de cada nó, o painel do par (`/painel/vivo` na 8095). O painel passa a mostrar a causa
do erro (conexão recusada × tempo esgotado). Depois: 5 testes ok (suíte `deploy/oci/testes`: 33 ok).


### B-022 e B-023 · vitrine do 3221 sem playlist e dizendo "ao vivo"; aviso do dia em UTC (0.8.39)

Visto pelo Luciano às 14h40 de 20/10 na 0.8.37: no ar estava o item `3221-espera` da playlist (vitrine do 2º turno
de 2026, 14h–15h), que o `uv-ciclo.sh` sobe **sem** `--simulacao`. A página só mostrava o chip "Dados reais" e o
"▶ Playlist" quando o serviço mandava `simulacao.rotulo`, e o selo do topo dizia "ao vivo" sempre que o WebSocket
estava conectado. O B-023 apareceu no mesmo código: o "dia" da explicação automática era
`new Date().toISOString().slice(0, 10)`, o dia em UTC (às 21h de Brasília já é o dia seguinte).

**Teste:** `site/testes/modo_pagina.mjs`, sem servidor nenhum (a página de `dist/` servida por rotas do Playwright,
WebSocket falso com `routeWebSocket`, playlist com `proxima_ao_vivo` fixa, relógio fixo em America/Sao_Paulo), a
1366, 390 e 360 px: replay (terça 15h), vitrine (14h40), ao vivo (domingo 18h) e "dia local" (10h e 22h30 do mesmo
dia). `cd site && node testes/modo_pagina.mjs`. Antes (página da 0.8.37), 52 falhas; as do B-022 e do B-023:

```
FALHA desktop · espera: o selo não diz "ao vivo" sem apuração ("ao vivo")
FALHA desktop · espera: selo com data-modo="espera"
FALHA desktop · espera: contagem para a próxima eleição ao vivo na faixa
FALHA desktop · espera: o selo leva à playlist (B-022)
FALHA celular-390 · espera: o selo leva à playlist (B-022)
FALHA celular-360 · espera: o selo leva à playlist (B-022)
FALHA dia local: às 22h30 do mesmo dia o aviso não abre de novo (B-023)
FALHA desktop · replay: selo "ao vivosimulação"
FALHA desktop · replay: selo em âmbar (rgba(255, 255, 255, 0.067))
modo_pagina: 52 falha(s)
```

(As outras falhas são da melhoria M-004 pedida junto: selo REPLAY em âmbar, contagem da próxima eleição ao vivo na
faixa e no letreiro, aviso com a próxima eleição.)

**Correção (com a M-004):** o selo do topo virou um botão que diz o modo — REPLAY (com rótulo), AO VIVO (com BU
aplicado ou apuração encerrada), EM BREVE (sem rótulo e sem BU: a vitrine e a noite oficial antes das 17h) — e abre
o aviso, que tem o link da playlist sempre que a playlist responde (em qualquer modo). A contagem para a próxima
eleição ao vivo (campo `proxima_ao_vivo` do `uv-playlist`) fica na faixa e no letreiro. O dia do aviso é o do
visitante (`site/src/proxima.js`, `diaLocal`). Depois:

```
ok    desktop · espera: o selo não diz "ao vivo" sem apuração ("EM BREVE▶ Playlist")
ok    desktop · espera: o selo leva à playlist (B-022)
ok    celular-390 · espera: o selo leva à playlist (B-022)
ok    celular-360 · espera: o selo leva à playlist (B-022)
ok    celular-360: contagem inteiro à vista (16–221 de 360 px)
ok    celular-360: BUs/min inteiro à vista (…)
ok    dia local: às 22h30 do mesmo dia o aviso não abre de novo (B-023)
ok    dia local: no dia seguinte (22/10, 8h) o aviso abre de novo
modo_pagina: tudo certo
```

O `site/testes/playlist_pagina.mjs` (que precisa do ciclo inteiro: serviço com `--simulacao`, Caddy e `uv-playlist`)
foi atualizado para o selo novo (o B-018 continua conferido: selo, contagem e BUs/min inteiros a 360 px), mas não foi
rodado nesta versão: o ensaio com o ciclo inteiro fica para a implantação.

### B-024 · sem a contagem "Próxima eleição AO VIVO" no celular (0.8.40)

Captura do Luciano às 16h49 de 20/10 (0.8.39, replay do 452, Bujari/AC): sem o chip da contagem na faixa e sem o "▶"
no selo — os dois só aparecem quando a página lê a playlist. O `uv-playlist` só roda no nó A, mas o domínio aponta
para os dois nós (o `02_configurar_borda.sh` manda cadastrar os dois IPs no DNS) e o Caddy do B mandava
`/api/v1/playlist` para `127.0.0.1:8090`, que no B não existe: 503 (medido: pelo A 200, pelo B 503). A página, sem a
playlist na partida, só tentava de novo 15 min depois.

**Testes:** `deploy/oci/testes/test_caddyfile.py` (o B leva a playlist ao A; o A confia no B para o IP do visitante;
os dois roteiros passam o papel; `caddy adapt` aceita os dois) e `site/testes/modo_pagina.mjs` (playlist com 503 nas
duas primeiras buscas: a contagem tem de aparecer em até 75 s). Antes:

```
FAIL: test_b024_no_b_leva_a_playlist_ao_a — '127.0.0.1:8090' unexpectedly found in '@playlist path … reverse_proxy 127.0.0.1:8090 …'
FAIL: test_b024_no_a_confia_no_b_para_o_ip_do_visitante — 'trusted_proxies static 198.51.100.20 10.0.1.11' not found
FAIL: test_b024_roteiros_passam_o_papel_da_playlist — 'remoto:' not found in 02_configurar_borda.sh
FALHA B-024: com a playlist fora do ar na partida, a contagem aparece assim que ela volta (≤ 75 s)
```

**Correção:** o `gerar_caddyfile.sh` ganhou o papel da playlist — no B, `remoto:<IP público do A>` (o Caddy do B
leva `/playlist` e `/api/v1/playlist` ao HTTPS do A, com `tls_server_name` do domínio: a sub-rede só abre 7700–7701,
8080 e 8095 entre os nós); no A, `local:<IPs do B>` (`trusted_proxies`, e o `X-Real-IP` passa a ser o `{client_ip}`:
o voto continua sendo um por visitante, não um pelo B). A página, sem a playlist, tenta de novo em 25–35 s. Depois:
`test_caddyfile` 8 ok (com o `caddy` 2.11.7), `deploy/oci/testes` 43 ok, `modo_pagina: tudo certo` (com
"ok B-024: …"), `painel_celular`, `termos` e `cargo test -p uv-aquisicao --lib` (98) ok.

### B-025 · a 8080 aberta para a internet (0.8.41)

O Luciano abriu `http://<IP do nó>:8080/` de fora: é o `uv-servico` direto, sem o Caddy (sem HTTPS, sem o login do
painel para os `POST /api/…`, sem a playlist — por isso, lá, sem a contagem da próxima eleição ao vivo). Causa: a
Security List da sub-rede tinha `0.0.0.0/0 → tcp 8080`, criada à mão (nenhum roteiro do repositório cria); como ela
"cobria" a sub-rede, o `02_configurar_borda.sh` (`liberar-portas sub-rede 8080,…`) dizia "já liberadas" e a regra da
sub-rede nunca existiu — tirar a pública derrubaria o balanceamento entre os nós. Nos firewalls dos nós havia ainda
uma regra de antes da 0.7.4, sem origem (`--dport 8080 … ACCEPT`). Nenhum comando de controle estava ligado (o
controle do replay responde 404): não houve dano.

**Teste:** `deploy/oci/testes/test_liberar_portas.py` (clientes falsos da OCI). Antes:

```
FAIL: test_b025_sub_rede_nao_conta_com_regra_publica — Lists differ: [8095] != [8080, 8095]
ERROR: test_b025_tira_so_a_publica — AttributeError: module 'uv_oci' has no attribute 'fechar_portas'
ERROR: test_b025_idempotente / test_b025_regra_ampla_falha_alto — (o mesmo)
FAIL: test_b025_borda_libera_a_sub_rede_e_fecha_a_internet — 'fechar-portas 0.0.0.0/0' not found
Ran 10 tests — FAILED (failures=2, errors=3)
```

**Correção:** `liberar-portas sub-rede` não conta mais uma regra pública como cobertura; `uv_oci.py fechar-portas
<CIDR> <portas>` tira só as regras exatas dessas portas (uma regra ampla, de todo protocolo ou faixa, falha alto para
o operador decidir); o `02_configurar_borda.sh` libera a sub-rede e **depois** fecha para a internet as portas
internas (8080, 8090, 8095, 7700, 7701); o `instalar_caddy.sh` tira a regra antiga sem origem. Depois: 10 ok;
`deploy/oci/testes` 48 ok.

**Em produção (20/10, ~18h20), nesta ordem:** (1) `liberar-portas sub-rede` → "10.0.1.0/24 → 8080 acrescentadas";
(2) `fechar-portas 0.0.0.0/0` → "0.0.0.0/0 → 8080 tiradas"; (3) a regra antiga tirada do firewall dos dois nós (as
da 8080 que ficaram têm todas origem no par). Conferido antes e depois:

```
antes:  <A>:8080 direto -> 200 · <B>:8080 direto -> 200
depois: <A>:8080 direto -> 000 · <B>:8080 direto -> 000   (sem resposta de fora)
        pelos dois nós: site 200 · estado 200 · playlist 200 · painel 200
        de cada nó para o par: :8080/api/v1/estado 200 · :8095/painel/vivo 200
        página real pelos dois nós: WebSocket conectado · selo "REPLAY▶ Playlist" · contagem à vista
```


### B-026 · acima do `--max-clientes`, a página não mostra a apuração (passo 1 do plano de borda)

Visto no teste de carga de 20/10 (degrau de 12.000: 1.457 conexões recusadas com 503 "limite de clientes" no `/ws`).
O `site/src/conexao.js` só reconectava (1, 2, 4… até 30 s) e a página ficava em "conexão caiu · volta em Ns", sem
resultado nenhum. As rotas REST de que um caminho alternativo precisa já existem (`GET /api/v1/resultado/{topico}`
com `ETag`/304 e `GET /api/v1/estado`; `difusao.rs`, `PROTOCOLO_WS.md` §6): nenhuma rota nova.

**Teste:** `site/testes/ws_recusado.mjs` (sem servidor real: um `node:http` falso, para o navegador usar o cache HTTP
e ver a recusa como em produção; o `/ws` responde 503 no handshake, as rotas REST respondem; intervalos curtos pelo
`window.__uvConexao`: consulta a cada 1,5 s, nova tentativa do WebSocket a cada 5 s). Confere a 1366 e a 390 px: o
resultado da tela aparece e é atualizado, o estado chega, o aviso "atualizando a cada minuto (muitos acessos)" está
inteiro à vista, só o tópico da tela e o estado são consultados (com `If-None-Match` e 304), nenhuma consulta antes
do intervalo −20 %, o WebSocket é tentado de novo sem martelar e, quando volta a aceitar, a página volta ao tempo
real, as consultas param e o aviso some; e, com a base das consultas (`<meta name="uv-dados">`) fora do ar, a página
consulta o próprio domínio. Antes (0.8.43), **22 falhas**:

```
FALHA desktop: com o /ws recusando (503), o resultado da tela aparece (seq 120000, 2 candidatos no painel)
FALHA desktop: o resultado é atualizado pela consulta (seq 120517)
FALHA desktop: o estado da apuração chega por consulta (selo "conexão caiu · volta em 4s▶ Playlist", conexão "caiu")
FALHA desktop: aviso "atualizando a cada minuto (muitos acessos)" inteiro à vista (não aparece)
FALHA desktop: consulta só o tópico da tela (0 consultas: nenhuma)
FALHA desktop: consulta o estado da apuração (0 consultas)
FALHA desktop: If-None-Match (0 com, 0 respondidas com 304)
FALHA desktop: tenta o WebSocket de novo sem martelar (6 tentativas; intervalos depois do modo HTTP: 3337, 9275, 14474 ms)
FALHA desktop: quando o /ws aceita, a página volta ao tempo real
… (as mesmas a 390 px)
FALHA base fora do ar (http://127.0.0.1:9): o resultado aparece, consultado no próprio domínio
FALHA base fora do ar: as consultas chegam ao próprio domínio
ws_recusado: 22 falha(s)
```

**Correção (0.8.44):** `site/src/conexao.js` — depois de 3 falhas seguidas do WebSocket (uma conexão que durou 30 s
zera a conta), modo HTTP: a cada 60 s (±20 %) `GET /api/v1/resultado/<tópico da tela>` (com `If-None-Match`) e
`GET /api/v1/estado`, entregues ao mesmo `receber` do WebSocket; nova tentativa do WebSocket a cada ~2 min (±20 %),
e ao abrir volta ao tempo real. Base das consultas no `<meta name="uv-dados">` (vazia = o próprio domínio; fora do
ar → o próprio domínio). `app.js`/`estilo.css`: aviso "atualizando a cada minuto (muitos acessos)" na faixa, no
lugar do BUs/min, e explicação no título do selo. Ficam só no WebSocket: eventos, marcos, "faltam", comparação com o
TSE e previsão. Depois:

```
ok    desktop: com o /ws recusando (503), o resultado da tela aparece (seq 120000, 2 candidatos no painel)
ok    desktop: o resultado é atualizado pela consulta (seq 120517)
ok    desktop: o estado da apuração chega por consulta (selo "AO VIVO▶ Playlist", conexão "http")
ok    desktop: aviso "atualizando a cada minuto (muitos acessos)" inteiro à vista (36–310 px)
ok    desktop: consulta só o tópico da tela (5 consultas: /api/v1/resultado/br-c0001)
ok    desktop: If-None-Match (4 com, 3 respondidas com 304)
ok    desktop: nenhuma consulta antes do intervalo − 20 % (menor intervalo 1636 ms; mínimo 1200 ms)
ok    desktop: tenta o WebSocket de novo sem martelar (4 tentativas; intervalos depois do modo HTTP: 5783 ms)
ok    desktop: quando o /ws aceita, a página volta ao tempo real
ok    desktop: de volta ao WebSocket, as consultas param (0 depois de voltar)
… (o mesmo a 390 px: aviso em 16–290 px, 4 respostas 304)
ok    base fora do ar (http://127.0.0.1:9): o resultado aparece, consultado no próprio domínio
ws_recusado: tudo certo
```

Suíte do site (`modo_pagina`, `relogio`, `hodometro`, `termos`, `painel_celular`) e `cargo test -p uv-aquisicao --lib`
(98) ok.

### B-029 a B-036, B-042 a B-045 e B-047 · revisão visual do site (0.8.53)

Teste novo, `site/testes/visual_pagina.mjs`: a página de `dist/` com rotas e WebSocket falsos (nenhum servidor), dois
cenários com dados de tamanho real (Presidente/Brasil apurando, 8 candidatos com nomes longos, replay a 500× com a
chegada de BUs parada; Prefeito de Bujari/AC com o eleito definido), modos "Urna Velox" e "Ambos", 12 larguras de 360
a 1920 px, temas escuro e claro. Cada conferência mede caixas no navegador e leva o número do bug: linha de base dos
separadores do hodômetro × a dos dígitos (B-029/B-030, por um marcador de altura zero no fim do texto); nada da linha
da comparação por cima de outra coisa (B-032); o sinal a no máximo ¼ da fonte do número (B-031); o rótulo "UV − TSE"
numa linha (B-036); o nome com pelo menos 56 px (ou inteiro) ao lado da pílula (B-043); cartões dentro da margem
direita e todo texto dentro da área de conteúdo do cartão (B-045); a caixa dos eleitores que faltam e o número dela
dentro do cartão (B-034); título, subtítulo e linha das fontes sem reticência a partir de 961 px (B-035); o horário
"mais provável" dentro do cartão da previsão (B-044); nenhum hodômetro desenhando fora do cartão (B-047).

Antes (0.8.52), vermelho pelo motivo de cada bug:

```
FALHA B-029 presidente · uv · 390 px · escuro: "," fora da linha de base dos dígitos (3.0 px abaixo, fonte 19 px, em pct)
FALHA B-029 presidente · uv · 390 px · escuro: "." fora da linha de base dos dígitos (3.0 px abaixo, fonte 19 px, em val)
FALHA B-030 presidente · uv · 390 px · escuro: ":" fora da linha de base dos dígitos (4.0 px abaixo, fonte 26 px, em pv-hod)
FALHA B-031 presidente · ambos · 360 px · escuro: candidato 14: "+" a 32 px do número (fonte 12 px)
FALHA B-032 presidente · ambos · 360 px · escuro: candidato 13: "pct" cruza "pos" (187–243 × 237–261 px)
FALHA B-032 presidente · ambos · 360 px · escuro: candidato 13: "qtd" cruza "pos" (192–243 × 237–261 px)
FALHA B-035 presidente · uv · 961 px · escuro: h1 "Presidente — Brasil" truncado (307 > 210 px)
FALHA B-035 presidente · uv · 961 px · escuro: fA "Urna Velox 55,23% das seções" truncado (172 > 31 px)
FALHA B-036 presidente · ambos · 360 px · escuro: candidato 13: rótulo "UV − TSE" em mais de uma linha (28 px de altura, fonte 10 px)
FALHA B-043 prefeito-eleito · uv · 961 px · escuro: nome de 12 some ao lado da pílula "eleito às 22:54:38" (0 de 86 px à vista)
FALHA B-044 presidente · uv · 390 px · escuro: horário da previsão passa da borda do cartão (391 > 370 px)
FALHA B-045 presidente · uv · 360 px · escuro: cartão "vidro apuracao" passa da margem direita (390 > 344 px)
FALHA B-045 presidente · uv · 1024 px · escuro: cartão "vidro apuracao" passa da margem direita (1021 > 978 px)
visual_pagina: 526 conferências ok, 526 falha(s)
por bug: B-029 96 · B-030 48 · B-045 96 · B-044 48 · B-035 30 · B-032 56 · B-031 132 · B-036 8 · B-043 12
```

O B-034 não se reproduzia nessas larguras enquanto o B-045 alargava o cartão; com os cartões contidos, o teste
ampliado (texto dentro da área de conteúdo, hodômetro dentro do cartão) ficou vermelho também pelo B-034, pelo resto do
B-045 e pelo B-047 (visto nas capturas):

```
FALHA B-045 presidente · uv · 360 px · escuro: no cartão "vidro apuracao" (conteúdo 37–323 px), fora: "último BU no TSE: 19:05:08 de 04/10" (169–367)
FALHA B-047 presidente · uv · 360 px · escuro: no cartão "vidro apuracao" (conteúdo 37–323 px), fora: hodômetro em val (-92–70)
FALHA B-034 presidente · uv · 360 px · escuro: caixa "Eleitores que faltam" fora do cartão (embaixo 1428 × 1420 px)
```

O B-042 tem teste próprio, `site/testes/preposicoes.mjs` (sem navegador: importa `src/textos.js`), com as 27 UFs e o
exterior:

```
FALHA disputa(ba): "Governador em Bahia" (esperado "Governador na Bahia")
FALHA primeira seção (rj): "Primeira seção totalizada em Rio de Janeiro: seção 2, zona 1, Bujari (RJ)"
FALHA em(): "undefined", "undefined"
preposicoes: 35 falha(s)
```

**Correções (0.8.53):**
* B-029/B-030 (`estilo.css`): `.hod .vis` passa a ser `inline-flex` com `align-items: baseline`. Em linha comum, o dígito
  (inline-block com `overflow: hidden`) se alinhava pela borda de baixo da caixa, e o separador pela linha de base do
  texto: "." "," e ":" ficavam ~16 % da fonte abaixo dos dígitos. O hodômetro "enxuto" (eleitores que faltam) já era
  flex, e por isso estava certo nas capturas.
* B-045 (`estilo.css`): `.lado`, a coluna de números da apuração e os filhos dela com `min-width: 0` (o texto mais largo,
  "último BU no TSE: … de 04/10" do replay, alargava a coluna e os cartões), com reticência nesse texto.
* B-035 e B-045 no desktop: uma coluna só abaixo de 1200 px (antes, 960 px): na faixa de 961 a 1199 px a coluna da
  direita (~1/3) não cabia a apuração, e o selo do "matematicamente" (270 px reservados) deixava ~210 px para o título.
  De 1200 a 1365 px, a linha das fontes da comparação em duas linhas, como no celular.
* B-044: o horário "mais provável" com a largura do molde (`min-width` no lugar dos 88 px fixos): o "hh:mm:ss" do
  replay acelerado passava da borda.
* B-031 (`hodometro.js`, `comparacao.js`): opção `prefixo` do hodômetro; o sinal é o prefixo da diferença e desliza
  (`transform`) até o primeiro dígito à vista, em vez de ficar no começo da largura reservada (9 dígitos).
* B-032, B-036, B-043 no celular ("Ambos"): a linha do candidato em quatro faixas — o nome na largura inteira em cima;
  embaixo, a diferença (rótulo sobre o valor), o Urna Velox (% sobre votos) e o TSE (posição e % sobre votos), cada um
  na sua coluna (64 e 94 px); mesma altura de linha (74 px).
* B-043 em qualquer largura: o horário da pílula num `span` à parte; com a linha do nome estreita (container query, até
  300 px) a pílula fica em 64 px ("eleito", "2º turno"; o texto inteiro no `title`). A largura depende só da largura da
  linha, nunca do texto: nada se desloca.
* B-047: `.hod.nulo .vazio { opacity: 0 }` (sem valor, os zeros à esquerda do hodômetro apareciam a 22 %, e no enxuto
  eles ficam fora da caixa).
* B-034: no celular, o cartão da apuração com 316 px (antes 300) e menos espaço entre as colunas.
* B-042 (`textos.js`): `Nomes.em(cod)` com a preposição de cada UF ("no Brasil", "na Bahia", "em São Paulo",
  "em Bujari (AC)"), usada na disputa ("Governador na Bahia"), na primeira seção, no primeiro eleito da UF e na previsão
  ("se decide no Brasil").

Depois:

```
visual_pagina: 1004 conferências ok, 0 falha(s)
preposicoes: tudo certo
```

Suíte do site sem servidor (`hodometro`, `relogio`, `termos`, `ws_recusado`, `modo_pagina`, `painel_celular`) ok. Os
ensaios que precisam do `uv-servico` com dados reais (`rodar`, `comparacao`, `horarios_pagina`, `identificacao`,
`previsao_pagina`, `carga_pagina`) não rodaram nesta sessão (ambiente sem `~/uvreal`). Nada aqui mexe em totalização ou
resultado.

B-033, B-037 a B-041: logo abaixo (0.8.54). B-046 (faixa da previsão serrilhada) continua registrado.

### B-033, B-037, B-038, B-039, B-040 e B-041 · revisão visual do site, segundo lote (0.8.54)

Conferências novas no `site/testes/visual_pagina.mjs`: B-033 o ícone do botão do histórico desenhado (SVG) e centrado
no botão (±0,75 px) com pelo menos 16 px; B-037 a pílula "Termos de uso" com fundo de alfa ≥ 0,9; B-038 com o letreiro
pausado, o rótulo "pausado" fora da área dos itens; B-039 no celular, sem selo, no máximo 24 px entre o subtítulo e o
controle da comparação (com selo: ele à vista, sem cobrir título nem subtítulo); B-040 com a barra de identificação
presa no alto, duas capturas da faixa acima dela em rolagens diferentes (700 e 780 px) saem iguais; B-041 o número de
"Seções apuradas" alinhado ao rótulo (±1 px) e o conteúdo do chip de BUs/min centrado (±2 px). Para o B-041 o cenário
do Presidente passou a ter 18 % das seções (91.013 de 499.248: um dígito a menos que o total).

Antes (0.8.53):

```
FALHA B-033 presidente · uv · 390 px · escuro: ícone ☰ do letreiro é um caractere de texto (sem desenho próprio)
FALHA B-037 presidente · uv · 390 px · escuro: pílula dos termos translúcida (color(srgb 0.0509804 0.0784314 0.192157 / 0.72))
FALHA B-038 presidente · uv · 390 px · escuro: "pausado" por cima da área dos itens do letreiro (pausado 323–369 × itens 69–381 px)
FALHA B-039 presidente · uv · 390 px · escuro: 78 px vazios entre o subtítulo e o controle da comparação
FALHA B-040 presidente · uv · 390 px · escuro: a faixa acima da barra presa no alto mostra o conteúdo que rola por baixo
FALHA B-041 presidente · uv · 390 px · escuro: conteúdo do chip de BUs/min fora do centro (51 px à esquerda, 10 à direita)
FALHA B-041 presidente · uv · 390 px · escuro: número de "Seções apuradas" 11 px à direita do rótulo
visual_pagina: 1072 conferências ok, 188 falha(s)
por bug: B-033 56 · B-037 56 · B-039 8 · B-041 56 · B-038 8 · B-040 4
```

**Correções (0.8.54):**
* B-033 (`index.html`, `estilo.css`): o ☰ virou um SVG de três traços (`currentColor`), centrado pelo `place-items` do
  botão; o caractere dependia da fonte do sistema (no iPhone, pequeno e fora do centro).
* B-037: fundo da pílula dos termos a 94 % (antes 72 %) e borda.
* B-038: com o letreiro pausado, o trilho dos itens termina 76 px antes da borda direita (só o recorte muda: nada se
  desloca), e o "pausado" fica nesse espaço.
* B-039: no celular, o selo do "matematicamente" ocupa o lugar do subtítulo (que some quando ele aparece) e a altura do
  cabeçalho segue o título (uma ou duas linhas; não muda depois de montada a tela). Antes, uma faixa de 34 px ficava
  reservada embaixo, vazia até o selo aparecer.
* B-040: com a barra presa, uma faixa fixa no alto (28 px mais a área do relógio do iPhone, `safe-area-inset-top`) com
  a cor do fundo, em degradê, cobre o que rola por baixo.
* B-041 (`hodometro.js`, `app.js`): propriedade `arrastarEl` do hodômetro: em vez do número, desliza o elemento em
  volta dele, para o que vem depois andar junto — a linha "91.013 de 499.248" (alinhada ao rótulo) e o conteúdo do chip
  de BUs/min (com `centro`, centrado no chip). Medido também no quadro seguinte à montagem, mesmo sem valor novo.

Depois:

```
visual_pagina: 1260 conferências ok, 0 falha(s)
```

Suíte do site sem servidor (`hodometro`, `relogio`, `termos`, `preposicoes`, `ws_recusado`, `modo_pagina`) ok.

### B-048 · a playlist com outra identidade visual (0.8.55)

Teste novo, `site/testes/playlist_identidade.mjs`: abre a página da apuração (`site/dist/`) e a da playlist
(`deploy/oci/servidor/uv-playlist.html`) com rotas e API falsas, a 1366, 390 e 360 px, com o tema do sistema claro e
escuro e com e sem escolha guardada em "uv-tema", e compara o que o navegador calcula nas duas: cor do fundo, os tokens
do tema (`--fundo`, `--vidro`, `--texto-2`, `--raio-g`…), fonte e cor do texto, a aurora, a marca "Urna Velox" (largura,
peso, tamanho, cor; "Velox" leve), o cartão de vidro (cor, borda, canto dos cartões), o título na largura da Archivo do
app, o botão de tema ◐ e nada de rolagem horizontal.

Antes (0.8.54):

```
FALHA 1366 px · tema guardado (nenhum) · sistema light: fundo da página igual ao do app (app rgb(13, 20, 49), playlist rgb(233, 237, 248))
FALHA 1366 px · tema guardado (nenhum) · sistema light: cores e raios do tema iguais aos do app (diferem: --fundo app "#0d1431" × playlist "#e9edf8"; --fundo-2 app "#141d45" × playlist ""; …)
FALHA 1366 px · tema guardado (nenhum) · sistema light: o mesmo fundo (aurora) do app
FALHA 1366 px · tema guardado (nenhum) · sistema light: marca "Urna Velox" — font-stretch igual (app 125%, playlist 100%)
FALHA 1366 px · tema guardado (nenhum) · sistema light: "Velox" da marca — font-weight igual (app 300, playlist 800)
FALHA 1366 px · tema guardado (nenhum) · sistema light: "Velox" da marca — background-image igual (app none, playlist linear-gradient(90deg, rgb(79, 195, 255), rgb(140, 107, 255), rgb(255, 95, 210)))
FALHA 1366 px · tema guardado (nenhum) · sistema light: título na largura da marca do app (font-stretch 100%; app —)
FALHA 1366 px · tema guardado (nenhum) · sistema light: botão de tema ◐, como no app
playlist_identidade: 100 falha(s)
```

**Correção (0.8.55, `uv-playlist.html`):** o estilo da playlist passa a usar os mesmos tokens, fonte, aurora, marca e
vidro do `site/src/estilo.css` (com um comentário apontando o teste que confere que as duas páginas batem): tema escuro
por padrão, como o app (antes seguia o do sistema: num celular em modo claro, a playlist abria clara e o app escuro); o
claro só pela escolha em "uv-tema", agora também com o botão ◐ na playlist; marca "Urna **Velox**" larga, com "Velox"
leve (antes em degradê); títulos na Archivo larga do app (`font-stretch` 118 %) com o complemento leve ("das
Eleições"); cartões com o canto, a borda e o brilho dos do app; os destaques (botão "Ver ao vivo", barras) com as cores
do anel da apuração. Depois:

```
playlist_identidade: tudo certo
```

`deploy/oci/testes/test_playlist.py` (14) ok. A página entra nos nós com o `63_ciclo.sh` (instala o `uv-playlist.html`
em `/opt/urna-velox/bin/`); até lá, a playlist em produção continua com o estilo antigo.

### B-050 e o aviso "totalização antes das 17h" (0.8.57)

Decisão do Luciano (23/10): a página **mostra** a totalização antes das 17h e explica por quê — no Brasil a
totalização começa oficialmente às 17h, mas o Urna Velox trata cada boletim de urna assim que o TSE o publica, e os do
exterior podem chegar a partir de 1h da madrugada; então o Urna Velox começa antes.

Teste novo, `site/testes/antes_17h.mjs` (sem servidor: relógio do serviço no `estado`, marco da primeira seção às
01:49:24 em Wellington; 1366, 390 e 360 px; com e sem a playlist): de madrugada, selo AO VIVO sem a contagem
"Próxima eleição AO VIVO · faltam" (B-050) e sem o item dela no letreiro; chip "Oficial às 17h" inteiro à vista, que
abre a explicação; o aviso também no letreiro; CLS < 0,001. Às 17h05 e depois da meia-noite (26/10 01:00), nada do
aviso. Sem nenhum BU (03:10), a contagem para as 17h continua e o aviso não aparece.

Antes (0.8.56):

```
ok    desktop · madrugada: selo AO VIVO
FALHA desktop · madrugada: sem a contagem "Próxima eleição AO VIVO · faltam" (B-050) — "Próxima eleiçãoAO VIVO2º turno · Eleições Gerais 2026hoje(25/10), 17hfaltam13h 50min00s"
FALHA desktop · madrugada: o letreiro não anuncia a "próxima eleição AO VIVO" (B-050)
FALHA desktop · madrugada: chip do aviso "Oficial às 17h" à vista
FALHA desktop · madrugada: tocar no chip abre a explicação (não abriu)
… (23 falhas em 1366 e 390 px; a rodada foi interrompida pelo tempo limite na de 360 px)
```

**Correção (0.8.57, `app.js`, `estilo.css`):** com a noite oficial no ar (selo AO VIVO), a contagem "Próxima eleição
AO VIVO" some também antes das 17h (B-050). Enquanto a totalização está no ar e o relógio do serviço (Brasília) ainda
não passou das 17h do dia da eleição — o dia vem do marco da primeira seção do Brasil; sem ele, da playlist —, a faixa
mostra o chip "Oficial às 17h · por enquanto, só BUs do exterior" (no celular, "Oficial às 17h"), depois do ritmo
(nada do que está à vista anda quando ele aparece), e o letreiro um item fixo; os dois abrem a explicação:

> **Totalização antes das 17h.** No Brasil, a totalização dos votos começa oficialmente às 17h (horário de Brasília),
> quando termina a votação em todo o país. Os eleitores no exterior votam no horário local. Onde a votação termina
> antes, como na Nova Zelândia, na Austrália, na Ásia e na Europa, o TSE publica os boletins de urna (BUs) a partir de
> 1h da madrugada de domingo. O Urna Velox totaliza cada boletim de urna assim que o TSE o publica. Por isso começa a
> totalizar antes das 17h: até lá, os números são só dos eleitores do exterior.

Depois:

```
antes_17h: tudo certo
```

Suíte do site sem servidor (`modo_pagina`, `termos`, `ws_recusado`, `preposicoes`, `hodometro`,
`playlist_identidade`, `visual_pagina` com 1260 conferências) ok.

### B-049 · os BUs do exterior da madrugada nos replays (0.8.58)

Decisão do Luciano (23/10): reproduzir a madrugada em tempo real seria monótono para quem assiste; nos replays, **todos
os BUs que saíram antes das 17h entram juntos às 17h**. A noite real não muda (o serviço totaliza cada BU quando o TSE
o publica e a página explica, B-050).

Teste novo, `simulador::testes::piso_das_17h` (`aquisicao/src/simulador.rs`), com os horários reais do 1º turno de
2026: Wellington (publicada 01:49:24, recebimento agrupado às 17:08:44) e Budapeste (13:44:27) passam a 17:00:00;
Brasil (17:00:31) e Miami (20:57:03) ficam como estão; seção sem horário continua sem; sem piso, nada muda. Antes, com
a função que só devolvia o horário escolhido (o comportamento de então):

```
thread 'simulador::testes::piso_das_17h' panicked at aquisicao/src/simulador.rs:874:9:
assertion `left == right` failed: recebimento agrupado às 17:08
  left: Some(1791133724)
 right: Some(1791133200)
```

**Correção (0.8.58):** `simulador::com_piso` e a opção `--piso` do `uv-simulador` (padrão `17:00:00` no dia da
eleição; `--piso ""` desliga): o horário de uma seção publicada (disponibilização do `cs.json`) ou escolhida antes do
piso passa a ser o piso. Vale para os dois modos (`--horario recebido` e `disponivel`) e, pelo padrão, para os replays
do ciclo sem mudar o `uv-ciclo.sh`. No 3220, os 781 BUs do exterior publicados antes das 17h entram às 17:00:00 (antes,
às 17:08, pelo recebimento agrupado). Depois: `piso_das_17h` ok; `cargo test -p uv-aquisicao --lib` ok. Os ensaios
`--ignored` com dados reais não rodaram nesta sessão (sem `~/uvreal`); usam o 2º turno de 2024, sem exterior e sem
seção antes das 17h.


### B-051 · o fim da apuração pelo `s.ts` pré-apuração desatualizado (0.9.2)

Pedido do Luciano (23/10): o fim automático não pode depender de regerar o `candidatos.json` perto da eleição. O `s.ts`
pré-apuração do 3221 que está nos nós (baixado em 18/10) tem Brasil 494.864 e SP 33.403, e o catálogo do mesmo arquivo
(as seções listadas nos `cs.json`) tem 517.179 seções, 17.931 agregadas: 499.248, com SP 103.904. Até a 0.9.1 o fim da
apuração exigia, além de toda seção publicada resolvida, que cada abrangência chegasse ao seu `s.ts`; com o `s.ts` menor
que o real, a abrangência "completa" cedo e a apuração encerra (e reabre) sempre que a fila do que já foi publicado
zera.

**Base da correção (só leitura, nos dados locais):** o total do catálogo (seções listadas menos as agregadas do
`candidatos.json`) é igual ao `s.ts` oficial final em **todas** as abrangências: 1º turno de 2026 83/83, 2º turno de
2024 51/51, 1º turno de 2024 5.574/5.574.

Teste novo, `servico::sts_desatualizado_nao_encerra_cedo` (`aquisicao/tests/servico.rs`, `--ignored`): o 2º turno de
2024 (AM e RO, 5.102 BUs, a 120×) com o `s.ts` das 51 abrangências cortado para 30 %; tem de encerrar uma vez só, com
os 5.102 BUs, sem reabrir, e a conferência sair 2/2. Antes (0.9.1):

```
sts_desatualizado_nao_encerra_cedo: 51 s.ts cortados; encerramentos (seq): [1807, 4165, 4948, 5010, 5026, 5051, 5078, 5081, 5082, 5085, 5086]; reaberturas: 10; aplicadas 5086
thread 'sts_desatualizado_nao_encerra_cedo' panicked at aquisicao/tests/servico.rs:216:5:
assertion `left == right` failed: a apuração encerrou cedo e foi reaberta
  left: 10
 right: 0
```

A primeira "apuração encerrada" saiu com 1.807 dos 5.102 BUs, e o serviço parou com 5.086 (16 de fora).

**Correção (0.9.2):**
- `servico.rs`: o esperado de cada abrangência passa a ser o total de seções do catálogo (o mesmo número da página,
  `Publicador::secoes`), com o `s.ts` pré-apuração só como piso (`Motor::esperadas`, `Motor::completa`); entram todas
  as abrangências publicadas, não só as que têm `s.ts`. A sondagem das não instaladas e a `saude.json` usam o mesmo.
- Agregadas descobertas na noite saem do total: as do `cs.json` lido durante a apuração que o `candidatos.json` não
  tinha (`Comum::agregadas_ger`, `Motor::tirar_agregadas_do_cs`) e as sondadas sem `aux.json`, também na retomada
  (`Publicador::agregada`, `motor/src/resultado.rs`; teste de unidade em `agregadas_fora_do_total_de_secoes`).
- Alerta `secoes_esperadas_incoerentes` só quando o `s.ts` é **maior** que o catálogo (aí o fim ainda espera por
  seções que o `cs.json` não lista); `s.ts` menor vira o registro `sts_desatualizado` no `servico.log`. Sai o alerta
  `sem_secoes_esperadas` (o fim não depende mais do `s.ts`).

Depois:

```
sts_desatualizado_nao_encerra_cedo: 51 s.ts cortados; encerramentos (seq): [5102]; reaberturas: 0; aplicadas 5102
test sts_desatualizado_nao_encerra_cedo ... ok
```

### Bugs do branch `claude/bugs-previsao-ritmo` (B-052 a B-054; trazidos para a main na 0.9.3)

Registrados e corrigidos no branch `claude/bugs-previsao-ritmo` (0.8.48 e 0.8.49 do branch, saídos da 0.8.47) como B-029, B-030 e B-031; no merge com a main viraram **B-052** (antigo B-029, previsão sempre "em horário ainda incerto"), **B-053** (antigo B-030, chip de ritmo irreal) e **B-054** (antigo B-031, relógio "00:00:0" cortado), porque B-029 a B-031 já eram bugs visuais da revisão de 22/10. Nas duas seções abaixo, "0.8.48" e "0.8.49" são as versões do branch (na main, essas versões são outras entradas).

### B-053 · chip de ritmo com 200–300 mil BUs/min (0.8.49 no branch; 0.9.3 na main)

**Causa:** o ritmo era medido no relógio da página (`performance.now()`) com o maior `seq` recebido, numa janela de 60 s
que guarda pelo menos 2 amostras. Quando a página passa mais de 60 s sem mensagem (celular em segundo plano, WebSocket
caído e reconectando), as amostras da pausa ficam todas com o mesmo `seq` e as antigas saem da janela; na volta, o
salto do `seq` (os BUs aplicados durante a pausa inteira) é dividido pelos poucos segundos entre as duas últimas
amostras da página. Num replay que recomeça, o `seq` volta do zero e o maior visto prendia o chip em 0 até passar o
antigo, e então saltava.

**Teste:** `site/testes/ritmo_pagina.mjs` (Playwright, sem servidor, relógio da página controlado; serviço falso a
700 BUs/s = 42.000/min, `estado` a cada 2 s). Antes (0.8.48):

```
ok    a página abriu o WebSocket
ok    contínuo: chip 41.928 BUs/min (serviço a 42.000)
FALHA pausa de 90 s: o maior valor do chip nos 20 s seguintes é 152.234 BUs/min (serviço a 42.000)
FALHA sessão nova: o chip não salta (maior 142.747)
FALHA sessão nova: depois de 80 s o chip volta a 0 BUs/min

3 FALHA(S)
```

**Correção:** `site/src/app.js` — o ritmo vem do `estado` (≤ 10 s): BUs aplicados (`ultimo_bu.seq`) e o relógio do
serviço (`relogio_sistema.agora`) da mesma mensagem, janela de 60 s **no relógio do serviço**; série refeita quando o
`sistema0` muda (sessão nova do replay); amostra com `seq` menor (outro nó um pouco atrás, no modo HTTP) é ignorada.
Servidor sem `relogio_sistema`: o cálculo antigo. Depois:

```
ok    a página abriu o WebSocket
ok    contínuo: chip 42.000 BUs/min (serviço a 42.000)
ok    pausa de 90 s: o maior valor do chip nos 20 s seguintes é 42.000 BUs/min (serviço a 42.000)
ok    sessão nova: o chip não salta (maior 42.000)
ok    sessão nova: depois de 80 s o chip volta a 42.000 BUs/min

tudo ok
```

### B-052 e B-054 · previsão sempre "em horário ainda incerto"; relógio "00:00:0" cortado (0.8.49 no branch; 0.9.3 na main)

**Causa (B-052):** a faixa P10–P90 com mais de 3 h **da noite** vira `incerta`, e a página então só dizia "em horário
ainda incerto", sem horário nenhum. No modo rápido (âncora a 500×) os BUs entram no ritmo do serviço, devagar e
constante na escala da noite, e os sorteios pela curva das noites anteriores e pela velocidade se espalham por horas
da noite (3 h da noite = 22 s do sistema): a faixa quase sempre passa do limite. Na noite real acontece no começo da
apuração e, no Brasil, no 100 %. **Causa (B-054):** com a faixa larga o relógio "mais provável" ficava sem valor e o
hodômetro mostrava o "00:00:00" esmaecido de valor desconhecido; com segundos (simulado acelerado) os 8 caracteres a
26 px não cabiam nos 88 px reservados e saíam do cartão.

**Testes:** `aquisicao/src/previsao.rs::previsao_sempre_com_horario` (Governador de uma UF sintética, 4 municípios × 100
seções de 1.000 eleitores, uma seção a cada 250 s da noite; com metade apurada, a mensagem `previsao` tem de trazer um
horário para o matematicamente, os 99 % e os 100 % — o P50 com a faixa estreita, o do ritmo atual com a faixa larga —,
e o do 100 % pelo ritmo atual a ~50.000 s) e `site/testes/previsao_cartao.mjs` (o cartão sozinho, sem servidor, a 390 e
1366 px, com as mensagens do modo rápido). Antes (0.8.48):

```
thread 'previsao::testes::previsao_sempre_com_horario' panicked at aquisicao/src/previsao.rs:233:13:
mat: "em horário ainda incerto" sem horário nenhum: {"p10":"2026-10-05 07:34:10","p50":"2026-10-05 14:01:59","p90":"2026-10-06 01:50:48","incerta":true,"tipo":"matematicamente_2t","prob_antes_fim":1.0,"apurado_p50":0.9262}
test result: FAILED. 0 passed; 1 failed
```

```
FALHA celular-390 · faixa larga: o matematicamente tem horário ("em horário ainda incerto")
FALHA celular-390 · faixa larga: relógio "mais provável" —
FALHA celular-390 · faixa larga: fim da apuração com horário ("Fim da apuração: 99% em horário ainda incerto · 100% em horário ainda incerto")
FALHA celular-390 · relógio inteiro dentro do cartão (relógio até 384 px, cartão até 354 px)
ok    celular-390 · faixa estreita: "entre 10:11:50 e 10:12:50", relógio 10:12:20
FALHA celular-390 · sem ritmo: nenhum relógio "mais provável" (nem 00:00:00 esmaecido): —
… (as mesmas a 1366 px: relógio até 1345 px, cartão até 1315 px)
10 FALHA(S)
```

**Correção:** `motor/src/previsao.rs` — toda `Faixa` ganha o `eta`, o horário **pelo ritmo atual**, como o ETA de um
navegador de trânsito: o que falta (aptos até a fração `apurado_p50` em que o matematicamente deve sair; seções até 99 %
e 100 %) ÷ a velocidade média móvel (meia-vida de 10 min da noite), recalculado a cada rodada; `None` só sem ritmo
nenhum; adiado junto com a faixa numa parada. `aquisicao/src/previsao.rs`: `eta` na mensagem (relógio do sistema, como
as faixas; `PROTOCOLO_WS.md` §4.7). `site/src/previsao.js`: faixa larga → "por volta de 10:12:30" e esse horário no
relógio "mais provável" (nota "pelo ritmo atual · …"); faixa estreita como antes; sem horário nenhum, o relógio some
(nada de 00:00:00); `estilo.css`: com segundos, relógio a 21 px em 100 px. Faixas, P50, gráfico e confirmação não
mudam. Depois:

```
previsao_sempre_com_horario: mat {…,"incerta":true,"eta":"2026-10-05 18:53:37",…} · fim99 {…,"eta":"2026-10-05 20:40:00"} · fim100 {…,"eta":"2026-10-05 20:56:40"}
test previsao::testes::previsao_sempre_com_horario ... ok
```

```
ok    celular-390 · faixa larga: o matematicamente tem horário ("por volta de 10:12:30")
ok    celular-390 · faixa larga: relógio "mais provável" 10:12:30
ok    celular-390 · faixa larga: fim da apuração com horário ("Fim da apuração: 99% por volta de 10:13:20 · 100% por volta de 10:14:05")
ok    celular-390 · relógio inteiro dentro do cartão (relógio até 353 px, cartão até 354 px)
ok    celular-390 · faixa estreita: "entre 10:11:50 e 10:12:50", relógio 10:12:20
ok    celular-390 · sem ritmo: nenhum relógio "mais provável" (nem 00:00:00 esmaecido): (escondido)
… (as mesmas a 1366 px)
previsao_cartao: tudo ok
```

Suíte: `cargo test --release --workspace` verde (100 testes do `uv-aquisicao`, os do motor e as fixtures); da página,
`ritmo_pagina`, `previsao_cartao`, `modo_pagina`, `ws_recusado`, `hodometro` e `relogio` ok. **Não rodados nesta
sessão** (ambiente sem `UV_DADOS` nem servidor): os `--ignored` com dados reais (entre eles
`previsao_acompanha_servico_atrasado` e `horarios_do_sistema_no_simulado`) e os testes da página que precisam do
`uv-servico` (`previsao_pagina`, `horarios_pagina`, `playlist_pagina`, `identificacao`). Não mexe em totalização nem
em resultado (só a previsão e a página).

**Depois do merge na `main` (0.9.3):** o cartão da previsão ficou com uma regra só para a largura do relógio (a do B-044, `min-width`); `.pv-hod.seg` só muda a fonte (21 px) e `.pv-hod.vazio` esconde o relógio sem horário. Testes:

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

### B-055 · eleitorado do 3221 desatualizado no `candidatos.json` (`preparar --atualizar` usava o cache dos `-u.json`) (0.9.7)

Visto pelo Luciano em 24/10/2026, 13h17, na vitrine do 3221: "Eleitores que faltam apurar para o matematicamente"
**157.563.461** no Urna Velox × eleitorado apto **158.745.502** no site de resultados do TSE (Brasil, Presidente,
2º turno). Ordem: o Luciano mandou corrigir na hora (24/10, véspera do 2º turno).

**Causa.** A diferença, 1.182.041, é o eleitorado do TO. O TSE publicou os `-u.json` pré-apuração do 2º turno aos
poucos: `br-c0001-e006258-u.json` gerado em 12/10 com 155.823.368 (SE e TO com `te` 0), em 13/10 10:56 com 157.563.461
(entrou SE) e em 15/10 09:38 com 158.745.502 (entrou TO, `te` 1.182.041). O `candidatos.json` dos nós (regerado em
18/10 00:11, por causa do `cs.json` do TO) saiu com 54 arquivos do cache e 32 requisições: o `--atualizar` só baixava de
novo o `ele-c.json` e os `cs.json`, e os `-u.json` vieram de 13/10. Além do Brasil, ficaram desatualizados o
eleitorado e as seções de 11 abrangências de Presidente por UF (SP 10.847.521 × 34.123.591; RJ, PE, PA, CE, GO, MA, ES,
PI, MG, TO), a circunscrição de **Governador do ES** (2.538.949 × 2.987.603) e a lista de candidatos da suplementar de
Prefeito em TO 95338 (o TSE tirou o 10 do arquivo em 22/10). O "matematicamente" usa o eleitorado da circunscrição: com
o do catálogo menor que o real, o `R` (eleitorado ainda não apurado) fica subestimado.

**Correção operacional (24/10, antes do código):** `uv-coletor preparar --pleito 3221 --atualizar` com a pasta
`fontes/u` posta de lado (`~/uvreal/2026-2t/preparo-20261024`: 74 requisições a ≤ 5 req/s, 12 do cache, código 0,
517.179 seções, 0 pendências). Conferido: Brasil 158.745.502 e TO 1.182.041, iguais ao TSE. O arquivo novo (SHA-256
`e87fe93c…`) foi para `/srv/urna-velox/preparo/candidatos.json` dos nós A e B (o anterior ficou em
`candidatos.json.20261018`) e o 3221 foi reiniciado com `63_ciclo.sh --reiniciar` (13h40, retomada em 0,2 s);
`GET /api/v1/resultado/br-c0001` pelo domínio passou a dar `"eleitorado":158745502`.

**Teste que comprova:** `preparo::testes::atualizar_baixa_de_novo_os_u_json` (`aquisicao/src/preparo.rs`; um "TSE"
em 127.0.0.1 que troca o `-u.json` do Brasil e do TO entre duas rodadas do `preparar`, como de 13/10 para 15/10).
Antes (0.9.6):

```
thread 'preparo::testes::atualizar_baixa_de_novo_os_u_json' panicked at aquisicao/src/preparo.rs:331:9:
assertion `left == right` failed: preparar --atualizar ficou com o eleitorado do cache (B-055)
  left: (1000, 0)
 right: (1700, 700)
```

**Correção (0.9.7):**
- `preparo.rs`: com `--atualizar`, o `preparar` baixa de novo **todas** as fontes (`ele-c.json`, `cm.json`, `-u.json`
  e `cs.json`), não só o `ele-c.json` e os `cs.json`. Sem `--atualizar`, o cache continua valendo.
- Alerta novo `eleitorado_divergente` no `uv-servico` (`TseResultados::conferir_eleitorado`): quando a comparação com o
  TSE (etapa 7) baixa o `-u.json` de um tópico, o `e.te` é conferido com o eleitorado do `candidatos.json`; se difere
  (e o TSE não está com 0), um alerta por tópico, com os dois números e o que fazer. Não faz requisição a mais: usa o
  arquivo que a comparação já trouxe. Teste: `tse_resultados::testes::eleitorado_divergente_alerta_uma_vez_por_topico`.
- Não feito: o `uv-servico` trocar sozinho o eleitorado do catálogo pelo do TSE na partida (pediria requisições na
  partida, inclusive numa retomada durante a apuração; fica para o Luciano decidir).

Depois:

```
test tse_resultados::testes::eleitorado_divergente_alerta_uma_vez_por_topico ... ok
test preparo::testes::atualizar_baixa_de_novo_os_u_json ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 102 filtered out; finished in 2.44s
```

### B-056 · BUs do exterior no ar antes da disponibilização no `cs.json` (3221, 2º turno) (0.9.21)

Sintoma (25/10/2026, 03h de Brasília): `zz-p003221-cs.json` com `last-modified` de 09/10 e nenhuma das 2.715 seções com
`da`/`ha`, mas o `aux.json` e o BU de Wellington já no ar (BU "Recebido" às 01:51:38). No 1º turno (3220) o exterior
já era divulgado de madrugada. Detalhes e números: [HANDOFF_B056.md](HANDOFF_B056.md).

Teste que comprova: `servico::testes::exterior_sondado_antes_do_cs_json` (`quando_sondar` com os números do ZZ do 3221
de 25/10, sem seção listada no `cs.json`). Rodado no CI do GitHub (sem a estação de desenvolvimento), antes da correção:

```
test servico::testes::exterior_sondado_antes_do_cs_json ... FAILED
thread 'servico::testes::exterior_sondado_antes_do_cs_json' panicked at aquisicao/src/servico.rs:2435:9:
assertion `left == right` failed: B-056: o exterior não é sondado antes do cs.json
  left: Nao
 right: Antecipada
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 104 filtered out; finished in 0.00s
```

**Correção (0.9.21):** `Motor::sondar` passa a decidir por `quando_sondar`: num domingo (relógio do site), enquanto o
`cs.json` do ZZ (com `nsp`) não listar nenhuma seção, as seções do ZZ sem BU são sondadas já (≤ 2 req/s, cada uma a
cada 300 s, primeiro as dos municípios que já têm BU), sem esperar o primeiro BU. Com o `cs.json` listando seções,
volta a regra de 16/10. UFs do Brasil não são afetadas. `--sem-sondagem` desliga. Comentário do nome do BU em `tse.rs`
corrigido (`o<pleito:05>…`).

Depois:

```
test servico::testes::exterior_sondado_antes_do_cs_json ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Suíte (CI, Linux): `cargo test --release -p urna-velox` 28 + 4 + 1 + 7 + 4 + 6 testes ok; `-p uv-aquisicao` 105 ok, 0
falhas. Os testes `--ignored` com dados reais não foram rodados (sem os dados neste notebook).

### B-057 · exterior sondado devagar e fora de ordem (3221, 2º turno) (0.9.22)

Sintoma (25/10/2026, ~05h): depois do B-056 o nó A já via os mesmos 31 BUs do exterior que um site de terceiros, mas a
sondagem pedia 2 req/s, cada seção a cada 300 s e na ordem do catálogo (uma volta de ~11 min). Os BUs do exterior chegam em
ondas por fuso (em 04/10, no `cs.json` do 1º turno: 5 seções na hora 01, 109 na hora 06, o grosso entre 12h e 15h e entre 19h
e 22h) e não havia como priorizar o que devia estar chegando.

Teste que comprova: `servico::testes::exterior_sondado_pelo_calendario` (calendário do `cs.json` do turno anterior,
faixas por horário, virada da meia-noite e ritmo com e sem 429). Rodado no CI do GitHub antes da correção
(`faixa_calendario` ainda devolvendo sempre a faixa "futura"):

```
test servico::testes::exterior_sondado_pelo_calendario ... FAILED
thread 'servico::testes::exterior_sondado_pelo_calendario' panicked at aquisicao/src/servico.rs:2519:9:
assertion `left == right` failed: B-057: seção devida agora
  left: (2, 300)
 right: (0, 20)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 105 filtered out; finished in 0.00s
```

**Correção (0.9.22):** na sondagem antecipada do ZZ (B-056), o calendário do `cs.json` do turno anterior
(`<pasta do candidatos.json>/fontes/cs/zz-p<pleito-1>-cs.json`, lido na partida; sem o arquivo, nada muda) dá o horário
esperado de cada seção. Faixa 0 ("devida": de 5 min à frente a 2 h atrás do horário esperado) vai primeiro e é repetida a
cada 20 s; faixa 1 (devia ter chegado há 2–12 h e não veio): 60 s; faixa 2 (futura ou sem calendário): 300 s, como antes.
Reforço de 8 req/s por nó (cada órgão tem o seu teto de 100 req/s) só com a sondagem antecipada ligada e a fila de pedidos
abaixo de 20 (o exterior não toma requisições do Brasil). **Contingência:** depois de qualquer HTTP 429 desde a partida
do serviço, o reforço acaba (volta a 2 req/s e 300 s por seção) e o alerta `reforco_exterior_desligado` é emitido; a pausa
global de 10 min depois do 429 continua valendo.

### B-062 · próxima eleição anunciada no letreiro antes do primeiro estado (1.0.0)

Sintoma (25/10/2026, suíte da página da 1.0.0): `antes_17h` falhava de vez em quando em "madrugada: o letreiro não anuncia
a 'próxima eleição AO VIVO' (B-050)" — em larguras diferentes a cada execução; contra a 0.9.27 passava. Causa: na carga,
se a playlist chega antes do primeiro `estado` do WebSocket, `promoAtual()` ainda não sabe que a noite está ao vivo e põe o
item "Próxima eleição AO VIVO" no letreiro; quando o `estado` chega, o item é marcado para sair, mas um item à vista só sai
quando passa da borda esquerda (`feed.js`, para nada pular) — fica anunciando por vários segundos. A página da 1.0.0 faz
mais trabalho na carga (mapa, cartões), e a corrida passou a aparecer.

Teste (antes da correção): o `estado` chegando 1,5 s depois da playlist, às 03:10 do dia da eleição com 412 seções:
```
FALHA desktop · madrugada, estado depois da playlist: o letreiro não anuncia a "próxima eleição AO VIVO" (B-062)
FALHA celular-390 · madrugada, estado depois da playlist: o letreiro não anuncia a "próxima eleição AO VIVO" (B-062)
FALHA celular-360 · madrugada, estado depois da playlist: o letreiro não anuncia a "próxima eleição AO VIVO" (B-062)
antes_17h: 3 falha(s)
```
**Correção (1.0.0):** sem `estado` ainda, não há o que anunciar (`promoAtual()` devolve `null`); o chip e o item entram
quando o primeiro `estado` diz que a noite ainda não está ao vivo. Depois: `antes_17h: tudo certo` (duas execuções seguidas, com o cenário novo e o original da madrugada).

### B-061 · raio negativo nas explosões do mapa quando o quadro começa antes da explosão (1.0.0)

Sintoma (25/10/2026, ensaio do mapa, `~/uvreal/ensaios/mapa-1.0.0-retomada`): `IndexSizeError: … arc … The radius provided
(-0.460223) is negative.` na página, a 300×, no celular com a CPU 4× mais lenta.

Causa: a explosão nasce com `t0 = performance.now()` (na mensagem `mapa` ou no marco), mas o quadro é desenhado com o
horário do `requestAnimationFrame`, que é o começo do quadro e, com a página ocupada, pode ser dezenas de ms anterior a
`t0`. Com a fase `e < 0`, o crescimento `1 − (1 − e)³` fica negativo e o raio do anel `2 + 13·r·o` (r = 3,2 no selo de
marco) passa de zero para baixo a partir de ~35 ms de atraso. Os arcos do exterior já pulavam `e < 0`; as explosões não.

Teste (antes da correção, `node site/testes/mapa_pagina.mjs`): explosão com início 400 ms depois do horário do quadro,
na vista Brasil e no globo:
```
FALHA B-061 explosão adiantada (Brasil) 1366/escuro: Failed to execute 'arc' on 'CanvasRenderingContext2D': The radius provided (-22.0542) is negative.
FALHA B-061 explosão adiantada (globo) 1366/escuro: Failed to execute 'arc' on 'CanvasRenderingContext2D': The radius provided (-12.8595) is negative.
mapa_pagina: 30 conferências ok, 4 falha(s)
```
**Correção (1.0.0):** a explosão que ainda não começou fica na lista e não é desenhada naquele quadro (`if (e < 0)
continue`, nos dois laços: plano e globo). Depois: `mapa_pagina: 32 conferências ok, 0 falha(s)`.

### B-059 · município do exterior com BU não era sondado mais vezes, e a janela do calendário era estreita (0.9.24)

Sintoma (25/10/2026, 05:46): o site concorrente mostrava 45 urnas apuradas no exterior e o Urna Velox 35. Por município:
Camberra (29491) 7 × 7, Sydney (30562) 19 × 19, Wellington (30805) 5 × 5, Seul (30538) 1 × 1 e **Nagoya (30198) 3 × 13**. O
arquivo oficial de totais do TSE ainda mostrava 0 (a fonte dele também são os BUs). Os 3 de Nagoya eram os que estavam na janela
do calendário do 1º turno (05:30); os outros chegaram antes da janela (calendário: 06:00) e esperaram até 300 s cada.

Teste que comprova: `servico::testes::exterior_municipio_quente_sondado_a_cada_20s`. Rodado no CI do GitHub antes da correção
(`faixa_secao` ignorando o município quente):

```
thread 'servico::testes::exterior_municipio_quente_sondado_a_cada_20s' panicked at aquisicao/src/servico.rs:2558:9:
assertion `left == right` failed: B-059: município com BU não é sondado a cada 20 s
  left: (2, 300)
 right: (0, 20)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 106 filtered out; finished in 0.00s
```

**Correção (0.9.24), para qualquer município do exterior (nada específico de Nagoya):** (1) **município quente**: se o município
da seção já tem BU e ainda faltam seções, todas as dele entram na faixa 0 (a cada 20 s), qualquer que seja o horário esperado ou
mesmo sem calendário; (2) a janela "devida" do calendário abre **30 min antes** do horário esperado (era 5), para quem chega
adiantado em relação ao 1º turno. Nada muda no teto: reforço de 8 req/s por nó, só com a fila do Brasil quase vazia, e um 429
desliga o reforço (B-057). O teste do B-057 teve a asserção de "futura" movida de 01:43 para 01:18 (nova janela).

### B-060 · sondagem do exterior sem garantia de cobertura (3221, 2º turno) (0.9.27)

Sintoma (25/10/2026, 13h05): painel do TSE com mais BUs recebidos (237) que os 199 publicados. Nós e o site concorrente tinham as
mesmas 199 seções, município a município, então o painel do TSE conta BUs ainda não publicados (nenhum de nós os alcança antes da
publicação). Mas o `sondadas` do evento `sondagem` estava parado em **1.170** (de 1.351 sondáveis) nos dois nós havia minutos:
cerca de 181 seções nunca tinham sido consultadas em 48 min de serviço. Contas: faixa quente 140 seções (a 20 s = 7 seleções/s) e
frias 971 (a 300 s = 3,2/s) contra 8 seleções por rodada, das quais metade eram seções do outro nó, descartadas sem requisição
(`coletor`): cerca de 4 requisições reais por segundo por nó. O calendário do turno anterior erra em 20–30 min (Nagoya, B-059): uma
seção "futura" que chega antes do previsto podia esperar sem ser vista.

Teste que comprova: `servico::testes::sondagem_cobre_todas_as_secoes_do_no` (partição, cota das sondadas há mais tempo e uma simulação
de 7 min com 2.000 seções, 200 quentes e 800 frias por nó). Rodado no CI do GitHub antes da correção (`so_do_no` e `escolher_sondas`
com o comportamento de antes):

```
thread 'servico::testes::sondagem_cobre_todas_as_secoes_do_no' panicked at aquisicao/src/servico.rs:2609:9:
assertion `left == right` failed: B-060: a rodada devia disputar só as seções deste nó
  left: 20
 right: 10
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.00s
```

**Correção (0.9.27):** (1) `so_do_no`: só as candidatas da partição do nó disputam o orçamento da rodada (as do outro nó eram
descartadas depois, sem requisição, e gastavam metade dele); (2) `escolher_sondas`: com o reforço, **1/4 da rodada** (cota) vai
para as sondadas há mais tempo (as nunca sondadas primeiro), em qualquer faixa, e o resto por prioridade; (3) o reforço do
exterior passa de 8 para **12** seleções por rodada, que agora são todas requisições do nó (3× as ~4 reais de antes, bem abaixo dos
~70 req/s em que o TSE devolveu 429 em 12/10; com a fila do Brasil vazia, e um 429 ainda desliga o reforço); (4) o evento
`sondagem` ganha `candidatas` (devidas e deste nó) e `nunca` (nunca sondadas), para ver a cobertura no `servico.log`. Sem
mudança no Brasil, no teto de 95 req/s nem na contingência de 429.
