# Handoff — bugs e melhorias do ciclo diário (18/10/2026)

*Preparado em 18/10/2026, na `0.8.8`. Lista viva: [/mnt/d/git/lukeboh/urna-velox/docs/BUGS.md](/mnt/d/git/lukeboh/urna-velox/docs/BUGS.md)
(14 bugs, 2 melhorias). Este documento define a **ordem de ataque** (ponderada abaixo, aprovada pelo Luciano ao pedir o
handoff) e como trabalhar sem quebrar o ciclo diário que está no ar até 23/10. Para iniciar a sessão, cole o prompt do
fim ou peça para ler este arquivo.*

**Caminhos** (todos absolutos): repositório = `D:\git\lukeboh\urna-velox` (Windows, PowerShell, Git) =
`/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`, usuário `dev`: scripts, cargo); dados = `/home/dev/uvreal`;
binários de desenvolvimento = `/home/dev/uvreal/target-dev/release/`; linha do tempo das eleições =
`/mnt/d/dados/urna-velox/linha-do-tempo/`; evidências do ciclo = `/mnt/d/dados/urna-velox/ciclo-diario/` (baixar com
`/mnt/d/git/lukeboh/urna-velox/deploy/oci/63_ciclo.sh --telemetria`).

## 1. Leia antes (nesta ordem)

1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes. **Em especial a seção "Bugs"** (regra de
   18/10/2026): primeiro o caso de teste automatizado que comprova o bug, rodado e **visto falhando** (saída anotada no
   `docs/BUGS.md`); só depois a correção; no commit, o teste fica no repositório e a suíte inteira passa.
2. `/mnt/d/git/lukeboh/urna-velox/docs/BUGS.md` — o sintoma de cada item, como foi registrado.
3. `/mnt/d/git/lukeboh/urna-velox/docs/operacao/CICLO_DIARIO.md` — o ciclo no ar (agenda, modos, telemetria, o que não usar,
   `--binarios`/`--reiniciar`).
4. `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md` — tópicos, inscrições, mensagens (B-014, B-009, B-011).
5. `/mnt/d/git/lukeboh/urna-velox/site/README.md` — como gerar `site/dist` (o binário embute) e os testes da página.

## 2. Regras de trabalho (resumo; valem as do `CLAUDE.md`)

* **Um bug por vez, na ordem da §3.** Para cada um: (1) teste que comprova, vermelho, saída no `BUGS.md`; (2)
  correção; (3) teste verde + suíte inteira (`cargo test --release -p urna-velox`, `-p uv-aquisicao --lib`, testes da
  página `site/testes/rodar.mjs`; os `--ignored` com dados reais quando mexer em totalização, resultado ou situação);
  (4) **um commit por bug**, com `scripts/versao.sh bump patch` (PATCH `0.8.x`; nada de `minor`), CHANGELOG e o
  `BUGS.md` atualizados (estado, teste, versão). Push com `git push --follow-tags`.
* **Mexeu em totalização, resultado ou situação:** conferir contra os oficiais (2026 83/83, 2024 5.574/5.574 e 51/51)
  e registrar no CHANGELOG.
* **Mudou a página (`site/src`):** `npm run build` (gera `site/dist`, que vai no commit) e recompilar; conferir com
  captura a 1366 px e a 390 px (celular).
* **Bug que não dá para reproduzir em teste automatizado:** anotar por quê e combinar com o Luciano antes de corrigir.
* **Rede:** o limite de 100 req/s vale só para buscar arquivos no TSE; nenhum teste daqui precisa do TSE (use o
  `uv-simulador` com o acervo local). Nada contra o TSE em 25 e 26/10 sem combinar.

## 3. Ordem de ataque (ponderada)

**Critérios** (nota de 0 a 3 cada): **G** = gravidade na noite de 25/10 (resultado errado, página fora do ar,
funcionalidade central quebrada); **P** = público atingido (todo visitante, só celular, só operação/telemetria);
**L** = apontado pelo Luciano (3) ou só pela análise (1); **E** = esforço/risco da correção (3 = pequeno e seguro);
**N** = relevância para a noite de 25/10, que é **2º turno** (3 = aparece na noite; 0 = só nas simulações ou na
operação). **Peso** = 3·G + 2·P + 2·L + E + 2·N; a ordem segue o peso. Desempate: o que derruba ou erra para todos os
visitantes vem antes. Prazo: tudo o que muda a página ou o serviço da noite deve estar no ar **até 22/10** (o ciclo de
23/10, cadência real, é o ensaio final com a versão de 25/10).

| ordem | id | o quê (resumo) | G | P | L | E | N | peso | por quê |
|---|---|---|---|---|---|---|---|---|---|
| 1 | B-008 | página vira download por 2–3 min na troca das 07h | 3 | 3 | 3 | 2 | 3 | 29 | site fora do ar para todos; a mesma situação (serviço subindo) acontece na partida da noite real |
| 2 | B-009 | "eleitores que faltam para o matematicamente" travado (SP) | 3 | 3 | 3 | 2 | 3 | 29 | funcionalidade central da noite (a contagem regressiva do "matematicamente") |
| 3 | B-014 | celular trava e esquenta; a página assina ~190 tópicos e recebe resultados inteiros de tudo | 2 | 3 | 3 | 2 | 3 | 26 | o público da noite está no celular; também reduz a carga do servidor por cliente |
| 4 | B-013 | previsão não apareceu no simulado das 07h; o card não deixa claro o que prevê | 2 | 3 | 3 | 2 | 3 | 26 | funcionalidade da etapa 8, prometida para a noite |
| 5 | B-011 | curva da apuração pula (seções e candidatos) | 2 | 3 | 3 | 2 | 2 | 24 | gráfico visível em todas as telas da noite |
| 6 | B-010 | sempre aparece 2º turno no 1º turno (regra dos 50 % dos válidos e do limiar de 200 mil eleitores) | 3 | 3 | 3 | 1 | 0 | 22 | situação errada na tela é erro de resultado e aparece todo dia nos 1º turnos do ciclo; o 25/10 é 2º turno (N = 0) |
| 7 | B-012 | dois candidatos com verdes quase iguais na curva | 1 | 3 | 2 | 3 | 3 | 22 | no 2º turno cada disputa tem **dois** candidatos: cores iguais confundem a tela inteira; correção pequena |
| 8 | M-001 | identificação permanente da eleição, abrangência e turno no topo | 1 | 3 | 3 | 1 | 3 | 22 | o visitante precisa saber o que está vendo; melhoria, depois dos bugs da página |
| 9 | B-004 | `para` (envio aos nós) devolve erro com arquivo grande que chegou | 1 | 1 | 1 | 3 | 2 | 14 | operação: as implantações de 19–22/10 podem precisar enviar arquivos grandes |
| 10 | B-005 | situação divergente no 1º turno de 2024 (destinação final ausente no `candidatos.json`) | 2 | 1 | 1 | 2 | 1 | 14 | votos idênticos; afeta a conferência das simulações e, na noite real, só candidato cassado com destinação presumida |
| 11 | M-002 | medir a vazão de ponta a ponta e avaliar uma topologia que escale | 0 | 1 | 3 | 0 | 0 | 8 | a medição pode ser feita antes de 25/10 sem tocar no que está no ar; **mudança de topologia só depois de 25/10** |
| 12 | B-003 | reinício intencional reprova o `RESUMO.json` | 0 | 1 | 1 | 3 | 0 | 7 | telemetria do ciclo |
| 13 | B-002 | medida de ordem do `RESUMO.json` dominada pelos BUs recebidos dias depois | 0 | 1 | 1 | 3 | 0 | 7 | telemetria do ciclo |
| 14 | B-006 | contador `federacao.publicados` zera na retomada | 0 | 1 | 1 | 2 | 0 | 6 | telemetria |
| 15 | B-001 | a ordem do modo rápido se afasta da do TSE (lotes do par) | 0 | 1 | 1 | 1 | 0 | 5 | só no modo rápido da simulação; na cadência real a ordem é a do TSE |
| 16 | B-007 | três scripts sem commit em `deploy/oci` | 0 | 0 | 1 | 3 | 0 | 5 | arrumação (perguntar ao Luciano se entram ou saem) |

## 4. Cada item: por onde começar e o que prova o conserto

As pistas abaixo são **pontos de partida**, não diagnóstico (a análise começa quando o item é atacado).

### 1 — B-008 · página vira download
* **Sintoma:** às 07:00 de 18/10, na troca 3221 → 1º turno de 2024, o navegador oferecia salvar um arquivo por 2–3 min.
* **Onde começar:** o que responde em `/` enquanto o `uv-servico` sobe (o `uv-ciclo.sh` para o serviço, espera ~1 min e
  sobe o novo; o 1º turno de 2024 tem 5.574 abrangências para publicar a frio): `aquisicao/src/difusao.rs` e
  `aquisicao/src/pagina.rs` (cabeçalhos `Content-Type` da página e de respostas de erro/indisponível),
  `deploy/oci/servidor/Caddyfile.template` (o Caddy com dois nós e verificação de saúde em `/api/v1/estado`).
* **Teste que comprova:** subir o serviço com um conjunto grande e consultar `/` em laço durante a partida, exigindo
  `Content-Type: text/html` (ou uma página de "carregando") em toda resposta; também com o Caddy na frente se der.
* **Pronto quando:** em nenhum instante da partida/troca o navegador recebe algo que vire download.

### 2 — B-009 · eleitores que faltam travado
* **Sintoma:** Prefeito de São Paulo/SP, 1º turno de 2024, o contador parou.
* **Onde começar:** `site/src/app.js` (contador "faltam", `ui.faltH`), a mensagem/campo que o alimenta no servidor
  (`docs/arquitetura/PROTOCOLO_WS.md`, resultado/`matematicamente`), `motor/src/matematica.rs`.
* **Teste que comprova:** replay do 452 (SP) pelo simulador, registrando a sequência do valor publicado/mostrado; o teste
  falha se o valor fica parado enquanto as seções da circunscrição avançam e o "matematicamente" não saiu.

### 3 — B-014 · celular trava; push de tudo
* **Sintoma e fato levantado:** a página assina até ~190 tópicos (Brasil, UFs, municípios), todos os eventos e marcos;
  cada tópico manda o resultado inteiro a cada mudança (limite de 20 mensagens/s por conexão). No modo rápido o celular
  trava e esquenta.
* **Direção combinada na conversa de 18/10:** resultado inteiro **só do tópico da tela**; dos outros, no máximo um
  resumo pequeno e só se a tela usar; letreiro pelos marcos do servidor. Confirmar com medida antes.
* **Onde começar:** `site/src/app.js` (`atualizarTopicos`), `site/src/conexao.js`, `aquisicao/src/difusao.rs`.
* **Teste que comprova:** cliente de teste (Playwright ou `uv-carga`) que mede mensagens e bytes recebidos por minuto
  numa replay do 452 a 500×; o teste fixa um teto (a definir com a medida) e falha hoje.

### 4 — B-013 · previsão
* **Sintoma:** não apareceu no simulado das 07h (modo rápido, 500×); o card não deixa claro que mostra o
  "matematicamente" e o fim da apuração.
* **Onde começar:** `site/src/previsao.js`, thread de previsão no serviço (`aquisicao/src/previsao.rs`, intervalo em
  hora do serviço), `docs/relatorios/RELATORIO_0.8.0.md`.
* **Teste que comprova:** replay a 500× e a 1× exigindo mensagens `previsao` no WebSocket e o card visível.

### 5 — B-011 · curva pula
* **Sintoma:** linha de seções sobe devagar e salta no fim (~08:00); candidatos saltam no fim; trechos retos e um pico
  isolado no começo (capturas em `docs/bugs/`).
* **Onde começar:** `site/src/graficos.js` e a origem dos pontos da curva (curva por tópico na difusão; retotalização a
  cada 2 min em `aquisicao/src/servico.rs`).
* **Teste que comprova:** comparar a série da curva com a série real (seções apuradas × hora da noite, do diário
  `secoes.log`) numa replay; falha onde houver salto que o diário não tem.

### 6 — B-010 · sempre 2º turno
* **Regra (Luciano):** no 1º turno, presidente, governador ou prefeito com mais de 50 % dos votos válidos é eleito; nos
  municípios abaixo do limiar de eleitorado (referência: 200 mil eleitores, CF art. 29, II) não há 2º turno (maioria
  simples).
* **Onde começar:** `motor/src/situacao.rs`, `motor/src/matematica.rs`, `motor/src/candidatos.rs` (eleitorado e
  limiar), textos da página (`site/src/textos.js`, `app.js`: selo "Matematicamente 2º turno").
* **Teste que comprova:** casos de unidade (prefeito com >50 % no 1º turno; município pequeno com maioria simples;
  governador e presidente) e a conferência das situações do 452 e do 3220 contra os oficiais (não pode piorar:
  3220 832/832).

### 7 — B-012 · cores parecidas
* **Onde começar:** `site/src/cores.js` (há teste em `site/testes/cores.mjs`).
* **Teste que comprova:** para os candidatos de uma disputa (os dois do 2º turno, e os N da curva), distância mínima de
  cor (OKLab) acima de um limiar; falha hoje com o par RICARDO/PABLO (captura em `docs/itens/img/B-011-curva-2.jpg`).

### 8 — M-001 · identificação da eleição no topo
* **Pedido:** eleição, abrangência e turno sempre visíveis (a marca pode sumir ao rolar), recolhível pelo usuário,
  compacto, sem perder a identidade visual. Referência: `docs/itens/img/M-001-referencia-app-je.jpg`.
* **Como fazer:** propor 2–3 variações com captura (1366 e 390 px) e deixar o Luciano escolher antes de implementar;
  depois, teste de página que garante a faixa visível após rolar.

### 9 a 16
* **9 · B-004:** `deploy/oci/lib.sh`, `_oci_op` (`rodar_remoto`, saída do Run Command). Teste: script com arquivo
  grande (ou saída simulada vazia) exigindo código 0 quando o destino tem o tamanho certo.
* **10 · B-005:** destinação final dos candidatos (`dvt`) a partir dos `-u.json` oficiais finais para as eleições já
  encerradas (simulações), mantendo o alerta `destinacao_presumida` na noite real. Teste: conferência do 452 com
  5.574/5.574 situações.
* **11 · M-002:** primeiro medir onde vai o tempo por etapa (coleta, I/O, troca de lotes, publicação, diário), por nó,
  no 452 ou 3220 a 500× com a telemetria do ciclo; relatório com opções de topologia; nada muda nos nós antes de 25/10
  sem decisão do Luciano.
* **12 · B-003 e 13 · B-002:** `deploy/oci/servidor/uv-ciclo-resumo.py`. Testes em Python com `secoes.log`/telemetria
  sintéticos.
* **14 · B-006:** `aquisicao/src/federacao/` (contadores no `saude_json`). Teste: retomada no meio, contador acumulado.
* **15 · B-001:** só no modo rápido; medir antes de mudar (ligado a M-002).
* **16 · B-007:** perguntar ao Luciano.

## 5. Implantar sem quebrar o ciclo diário (até 23/10)

* **Agenda do ciclo** (`docs/operacao/CICLO_DIARIO.md`): 07h 1º turno de 2024, 11h 2º turno de 2024, 13h 1º turno de 2026,
  17h 2º turno de 2026 (3221, sem coleta). Dias pares desde 18/10 modo rápido, ímpares cadência real.
* **Para levar uma correção aos nós:** `cd /mnt/d/git/lukeboh/urna-velox/deploy/oci && ./compilar_local_arm64.sh &&
  ./63_ciclo.sh --binarios` (troca os executáveis sem parar nada; vale no próximo horário) e, se quiser já,
  `./63_ciclo.sh --reiniciar` (retoma de onde parou). Se a correção muda o resultado, recomece a eleição em vez de
  retomar: `./63_ciclo.sh --iniciar <pleito> <modo>`. Scripts do ciclo (`servidor/uv-ciclo*.py|sh`): `./63_ciclo.sh
  --instalar` (para a eleição em andamento; depois `--iniciar`).
* **Não usar durante o ciclo:** `10_implantar.sh`, `20_iniciar.sh`, `21_parar.sh`, `50_atualizar.sh`, `60_simulado.sh`
  (motivos no `CICLO_DIARIO.md`). Nunca reiniciar o `uv-ciclo-sim`.
* **Conferir depois de implantar:** `./63_ciclo.sh --estado` e o `RESUMO.json` da eleição seguinte
  (`/srv/urna-velox/ciclo/historico/<id>/` nos nós; `./63_ciclo.sh --telemetria` traz para o D:).

## 6. Prompt para a sessão que vai executar

> Leia `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_BUGS_2026-10-18.md` e os documentos da §1 dele. Ataque os itens de
> `/mnt/d/git/lukeboh/urna-velox/docs/BUGS.md` **na ordem da §3**, um por vez, seguindo a regra do `CLAUDE.md`: primeiro
> o caso de teste automatizado que comprova o bug, rodado e visto falhando (saída anotada no `BUGS.md`), depois a
> correção, teste e suíte verdes, um commit por item com `scripts/versao.sh bump patch`, CHANGELOG e `BUGS.md`
> atualizados, `git push --follow-tags`. O ciclo diário está no ar nos dois nós da OCI até 23/10: implante só pelo
> `deploy/oci/63_ciclo.sh` (§5) e nunca pelos roteiros listados como proibidos. Na M-001, mostre opções com captura antes
> de implementar. Antes de M-002 e de qualquer mudança de topologia nos nós, pergunte ao Luciano. Ao fim de cada item,
> me diga em 2–3 linhas o que o teste mostrou antes e depois.
