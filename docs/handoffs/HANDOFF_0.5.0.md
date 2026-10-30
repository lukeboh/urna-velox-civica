# Handoff — Urna Velox: Etapa 5, site de exemplo (`0.5.0`, alvo 22/10/2026)

*Preparado em 13/10/2026, no fim da etapa 4. **Ponto de partida: `v0.4.0`** (etapa 4 concluída por decisão do Luciano,
sem a validação independente; relatório em
[RELATORIO_0.4.0.md](/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.4.0.md)). Para iniciar a sessão, cole este
documento ou peça para lê-lo. **Quem executa fica em PATCH (`0.4.1`, `0.4.2`…) e entrega um relatório; o `bump minor`
para `0.5.0` só depois da validação independente descrita no fim** (ou por decisão explícita do Luciano).*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` (Windows, Git, PowerShell) =
`/mnt/d/git/lukeboh/urna-velox` (WSL `Ubuntu-24.04`: scripts, cargo); dados = `/home/dev/uvreal`; ensaios da etapa 4 =
`/home/dev/uvreal/etapa4`. Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.5.0.md`.

## Contexto

A etapa 4 pôs no `uv-servico --http` um **WebSocket** e um **REST** que entregam, em tempo real, o resultado de cada
abrangência × cargo (os bytes do arquivo publicado), os eventos do "matematicamente" e o estado do serviço
(`/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md`, ADR-026). Medido: BU → cliente p99 7,7 ms com 1.000 clientes
na noite de 2026 a 60× (relatório da etapa 4, §3).

**Objetivo desta etapa:** uma página de resultados **elegante, dinâmica e sem nenhum pulo na tela**, servida pelo
próprio binário, que acompanha a apuração só pelo WebSocket público e mostra, além dos números, **marcos e
curiosidades da noite** que ficam visíveis em qualquer tela. Ela é também o modelo para quem quiser integrar.

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes (versionamento, rede, verificação).
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — "Etapa 5" (com as observações de design do Luciano) e "Etapa 6".
3. `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/PROTOCOLO_WS.md` — **o contrato**; a página não usa nada fora dele (o que faltar
   entra nele, só por acréscimo — ver "Servidor").
4. `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/FORMATO_RESULTADO.md` — os campos de cada resultado.
5. `/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.4.0.md` — números de mensagens por segundo (o `br-c0001` muda
   ~100 vezes por segundo a 60×), avisos `seq_pulado`, reconexão.
6. `/mnt/d/git/lukeboh/urna-velox/docs/operacao/OPERACAO.md` §7 e `/mnt/d/git/lukeboh/urna-velox/ARCHITECTURE.md` §2.7.
7. Código: `aquisicao/src/difusao.rs` (rotas, `Hub`, eventos), `aquisicao/src/servico.rs` (`gravar_eventos`,
   `aplicar_bu`, `saude`), `aquisicao/src/preparo.rs` (como o `candidatos.json` é montado), `motor/src/matematica.rs`
   (`custo`, `garantido`, `avaliar`), `motor/src/acompanhamento.rs`, `aquisicao/src/bin/uv-carga.rs` (cliente de
   referência).

## Requisitos de design (pedidos do Luciano — não são opcionais)

1. **Visual moderno: glassmorfismo "tátil"** — cartões translúcidos com `backdrop-filter: blur()`, borda clara fina,
   sombra suave e resposta ao toque/ponteiro (leve inclinação/realce por `transform`, sem mudar tamanho). **Glow nos
   destaques com variação de cor em arco-íris**, como no app do Gemini e no "modo IA" do Google: borda ou halo com
   `conic-gradient` animado (girando o ângulo por `@property --angulo` ou por `transform: rotate` de uma camada), com
   desfoque. Tema escuro como padrão, claro opcional; respeitar `prefers-reduced-motion` (sem giro, sem hodômetro:
   troca com *fade*).
2. **Gráficos suaves, com transições dinâmicas elegantes**: barras e anéis de votos, % de seções apuradas, curva da
   apuração ao longo da noite. SVG ou `<canvas>` desenhados à mão (sem biblioteca que redesenha do zero e "pisca");
   valores interpolados (*easing*) entre a versão anterior e a nova.
3. **Nenhum pulo na tela.** Toda mudança visual por `transform`/`opacity` (CSS ou Web Animations API) — nunca por mudar
   `width`/`height`/`top`/`left`/`margin` de algo que empurra o resto. Tamanhos reservados de antemão (inclusive do
   feed e dos cartões que aparecem depois); fontes com `font-variant-numeric: tabular-nums` e carregadas antes da
   primeira pintura (embutidas, `font-display: block` curto). **Troca de posição de candidatos** por FLIP
   (`transform: translateY`), nunca reflow. **Meta medida: CLS = 0** (`PerformanceObserver` de `layout-shift`) durante
   a noite inteira.
4. **Números com animação de hodômetro**: cada dígito é uma coluna 0–9 que rola por `translateY` até o valor novo
   (votos, %, seções, eleitores que faltam); separadores de milhar fixos; largura reservada para o maior número possível
   (eleitorado da abrangência).
5. **Marcos e curiosidades** sempre visíveis em qualquer tela (Brasil, UF, município, qualquer cargo): um **feed** que
   passa (carrossel/*ticker* horizontal ou vertical, por `transform`, pausável ao passar o mouse/tocar, com
   `aria-live="polite"`) e, ao tocar, abre o detalhe. Os marcos persistem (quem chega depois ou reconecta vê o histórico).
   Lista mínima (pedida):
   * **primeira seção totalizada** do país e de cada UF/cargo exibido: número da seção, zona, município e UF, horário;
   * **a seção que determinou o "matematicamente eleito" / "matematicamente 2º turno"** de cada circunscrição
     (número, zona, município, UF, horário);
   * **quantos eleitores faltam apurar para o "matematicamente"** de cada circunscrição em disputa — um número
     **decrescente** (hodômetro) ao longo da noite, até virar o selo de "matematicamente eleito/2º turno";
   * **primeiro governador, primeiro senador e primeiro prefeito matematicamente eleitos** do país (e o primeiro de
     cada UF), com o horário.
   Sugestões a mais (escolha e registre no relatório o que entrou): virada de liderança (troca do 1º lugar) e quantas
   houve; marcos de 25/50/75/90/99/100 % das seções do Brasil e de cada UF; primeira UF e primeira capital 100 %
   apuradas; última seção do país; disputa mais apertada no momento (menor diferença % entre 1º e 2º) e a mais folgada;
   maior e menor comparecimento por UF; o BU nº 100 mil, 250 mil…; ritmo da apuração (BUs/min); quanto tempo antes da
   totalização do TSE o "matematicamente" saiu (só no fim, da conferência). Nada de curiosidade que exponha pessoa
   (eleitor ou mesário): seção, zona e município apenas.
6. Funciona no celular (toque, telas de 360 px, sem rolagem horizontal) e mostra **quando a conexão cai e volta**
   (selo discreto, reconexão com espera crescente como no §5 do protocolo; ao voltar, o `instantaneo` substitui tudo
   sem pulo).
7. Nada externo em tempo de execução: a página é servida pelo binário com todos os recursos embutidos (sem CDN, sem
   fonte do Google, sem rastreamento). Pode usar uma etapa de *build* (ex.: `esbuild`) desde que o resultado vá para o
   repositório como arquivo estático e o binário o embuta (`include_bytes!`).

## Decisões já tomadas (mude só se o Luciano pedir)
- **Onde fica:** a página é estática, embutida no `uv-servico` e servida pela mesma camada HTTP (`GET /` e
  `/static/<arquivo>` de uma tabela fixa embutida — **nenhum caminho vindo do cliente chega ao disco**, como o REST).
  Fontes da página em `/mnt/d/git/lukeboh/urna-velox/site/` (novo). O motor continua sem rede.
- **Só o protocolo público:** a página usa só `/ws` e `/api/v1/*`. O que faltar entra no `PROTOCOLO_WS.md` como
  **acréscimo** (campo novo ou tipo de mensagem novo, versão 1 continua válida; quem lê ignora o que não conhece). Nada
  dentro de `resultado` muda (é o arquivo publicado, conferido byte a byte).
- **Ritmo de desenho ≠ ritmo de mensagens:** a 60× chegam ~100 versões/s do `br-c0001`; a página guarda o último valor
  de cada tópico e redesenha no máximo a cada `requestAnimationFrame`, com as animações em andamento redirecionadas
  para o valor novo (sem recomeçar). Na noite real (1×) chegam poucas por segundo.
- **Curiosidades: cliente onde der, servidor onde precisar.** O que sai dos resultados e eventos (viradas, marcos de %,
  primeiro eleito por cargo, disputa apertada, comparecimento) a página calcula; o que exige saber **qual seção** foi
  aplicada ou a conta do "matematicamente" fica no servidor (abaixo).

## Servidor (o que falta para a página — por acréscimo ao protocolo v1)
1. **Nomes** — hoje o `candidatos.json` não tem nome de município nem partido (só `codigo`/`uf` dos municípios e
   `nome`/`nome_urna` dos candidatos). Eles existem nas fontes que o `uv-coletor preparar` já baixa: `cs.json` →
   `abr[].mu[].nm` (nome do município; `abr[].ds` = nome da UF) e `-u.json` pré-apuração → `carg[].agr[].par[]`
   (`sg` = sigla do partido, `cand[].vs[]` = vice/suplentes). Acrescentar ao `candidatos.json` (campos novos; o motor
   ignora) **sem nenhuma requisição nova ao TSE** quando os arquivos já estão em `fontes/` (o `preparar` relê o que tem;
   se precisar rebaixar, ≤ 5 req/s e dentro do orçamento abaixo) e servir em `GET /api/v1/nomes` (UFs, municípios,
   candidatos por cargo e circunscrição: nome de urna, partido, vice), com `ETag` fixo por processo.
2. **Mensagem `marco`** (tipo novo, com histórico na conexão como os eventos, e `GET /api/v1/marcos`), gerada na thread
   do motor (que é quem sabe qual BU entrou) e entregue à difusão como os eventos — barata, nunca bloqueante:
   * `primeira_secao` (Brasil; cada UF; cada circunscrição publicada): `{uf, municipio, zona, secao, horario, seq}`;
   * `secao_decisiva`: a seção do BU que fez a circunscrição virar "matematicamente" — **atenção**: o motor avalia por
     lote; registre a **última seção aplicada no lote** que virou o estado (a 1× os lotes têm 1–3 BUs; documente a
     ambiguidade a 60×);
   * `faltam_para_mat` por circunscrição em disputa: **quantos eleitores ainda precisam ser apurados** até o
     "matematicamente" ficar garantido, se o resto votar como no pior caso. Função **nova e pura no motor**
     (`matematica.rs`, sem rede: permitida) a partir de `custo`/`garantido`: o menor `r'` restante com que a
     circunscrição já estaria decidida; `faltam = r − r'`. Com testes de unidade e conferida contra o instante do
     evento real nas noites de 2026 e 2024 (deve chegar a 0 exatamente no `seq` do evento). Mandar quando muda, no
     máximo 1×/s por circunscrição;
   * marcos de contagem simples que só o servidor vê com exatidão (BU nº 100 mil…, última seção do país).
   Os marcos entram também num `marcos.csv` na saída (como o `eventos.csv`), para a conferência.
3. **Servir a página** (`GET /`, `/static/…`) com `Cache-Control` longo nos estáticos versionados e `Content-Security-
   Policy` restrita (`default-src 'self'; connect-src 'self'`).
4. Testes de unidade das rotas novas (só a tabela embutida; caminhos de fora → 404) e do `faltam_para_mat`; os ensaios
   `--ignored` da etapa 4 continuam passando.

## Regras inegociáveis (as da etapa 4 continuam)
- **Rede (TSE):** ≤ 100 req/s somando tudo; depois de **qualquer** 429, parar tudo ≥ 10 min; nada de vários IPs ou
  processos em paralelo. **Esta etapa não precisa do site real:** desenvolva contra o `uv-simulador` e o `uv-servico
  --http`. Uso real permitido: rerrodar `uv-coletor preparar --pleito 3221` (≤ 5 req/s; ~70 requisições; **uma vez por
  dia até 24/10**, ver "Pendências") — orçamento total ~1 mil requisições; para mais, pergunte ao Luciano.
- **Não rodar nada contra o site real em 25/10 e 26/10** sem combinar.
- **O que está medido não pode piorar sem explicação:** critérios A, B, C (etapa 3) e D1–D4 (etapa 4) continuam
  passando; com 0 clientes, BU → publicação no B perto de p50 0,8 ms / p99 4,8 ms; D1 com BU → cliente p99 < 50 ms
  (era 7,7 ms). Os marcos rodam na thread do motor: meça o custo (`custos_ms` na saúde) e mantenha-o abaixo de ~0,05 ms
  por lote no p50.
- **Motor sem rede e sem `tokio`** (`cargo tree -p urna-velox -e normal`). Executáveis começam com `uv-`.
- **Sem regressão e versionamento** exatamente como no `CLAUDE.md` e no handoff da etapa 4: `compilar_dev.sh` sem
  avisos, motor `--ignored` 13/13, ensaios `--ignored` do serviço antes dos commits que mexem no serviço; `scripts/versao.sh bump patch`; CHANGELOG sem `PREENCHER`; `git add` sem `aquisicao/acervo/`; `git commit -F
  D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG` (gravado com a ferramenta de escrita e relido); `git push
  --follow-tags`; **não faça `bump minor`**. Identidade `Luciano Bohnert <luciano.bohnert@gmail.com>`.
- Termos: "**matematicamente eleito**" / "matematicamente 2º turno"; os dados dos candidatos são "**candidatos**".
- Documentos novos com **caminhos absolutos**.

## Ambiente
Igual ao da etapa 4 ([HANDOFF_0.4.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.4.0.md) "Ambiente"): Windows + WSL2,
Desktop Commander com PowerShell (comandos com aspas ou `$` em scripts `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh`),
processos longos iniciados com `start_process` de tempo curto, `pkill -f` com barra final, `ulimit -n 65536`. Roteiro de
ensaio com `--http` + `uv-carga` + `taskset`: `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e4_ensaio.sh`. Para ver a página
de verdade, Chrome no Windows apontando para `http://localhost:<porta>/` (o WSL2 encaminha `localhost`). Testes de
navegador automatizados: Playwright com Chromium no WSL (`npm` funciona), em `site/testes/`.
**Não rode ensaios de desempenho com o navegador aberto animando a página** (divide CPU): meça a página e o serviço em
rodadas separadas, ou registre que estavam juntos.

## O que fazer
1. **Servidor** (seção acima): nomes, `marco` + `faltam_para_mat`, rotas da página; protocolo atualizado (acréscimos).
2. **Página** (`site/`): telas Brasil → UF → município, cargos (Presidente, Governador, Senador, Prefeito, consultas),
   votos, %, brancos, nulos, comparecimento, % de seções apuradas, horário do último BU, selo de matematicamente
   eleito/2º turno com o horário, feed de marcos e curiosidades, curva da apuração, estado da conexão e do serviço
   (pausa por 429 e alertas como `secoes_esperadas_incoerentes` — o % de seções de algumas UFs pode estar errado e a
   página deve dizer isso). Navegação por URL (`#/br/c0001`, `#/sp/c0003`, `#/sp71072/c0011`) e por toque.
3. **Desempenho da página:** medir no Chrome (Performance) durante o D1 reduzido: 60 fps no desktop, sem tarefas
   longas > 50 ms, memória estável por 1 h; CLS = 0.
4. **Critério** (abaixo) e ensaios automatizados (Playwright): a página acompanha um replay sem recarregar; no fim, os
   números na tela = conferência; CLS = 0; reconexão depois de `kill -9` do serviço sem recarregar.
5. **Documentação:** `site/README.md` (como desenvolver, *build*, onde ficam os recursos embutidos), `PROTOCOLO_WS.md`
   (acréscimos), `OPERACAO.md` (a página, o que olhar), ADR novo (página embutida, marcos no servidor), ROADMAP marcado
   com a versão, sem concluir a etapa.

## Critério (ROADMAP + pedidos do Luciano)
- O site acompanha um replay da noite (2026 1º turno a 60× e 2024 2º turno a 10×) **sem recarregar a página** e mostra
  ao final **os mesmos números da conferência oficial** (todas as abrangências que a página exibe; conferido por script
  contra os `-u.json`).
- **CLS = 0** durante toda a noite; nenhum número troca sem animação (hodômetro); nenhuma barra/candidato muda de posição
  sem transição.
- Marcos corretos: primeira seção (Brasil e cada UF) = primeira linha "aplicado" do `secoes.log` na ordem; seção
  decisiva = seção do lote do evento no diário; `faltam_para_mat` chega a 0 no `seq` de cada evento do `eventos.csv`
  e nunca sobe a não ser por reenvio/anulação; primeiro eleito por cargo = primeiro do `eventos.csv` por cargo.
- Celular: 360 px sem rolagem horizontal, toque, conexão caindo e voltando.
- Com a página aberta em 10 abas durante o D1, o p99 de BU → cliente segue < 50 ms.

## Pendências herdadas (para a noite de 25/10)
- **`cs.json` de TO** dava 404 em 13/10: `candidatos.json` do 3221 incompleto (serviço com `--aceitar-incompleto`).
  Rerrodar `uv-coletor preparar --pleito 3221` (≤ 5 req/s) uma vez por dia até 24/10 e registrar.
- **`s.ts` parcial em 9 UFs** para Presidente (alerta `secoes_esperadas_incoerentes`): a página mostra o aviso.
- **Destinação presumida "Válido"** no 3221: conferir cassações/indeferimentos.
- A etapa 4 foi concluída (`v0.4.0`) sem validação independente; se alguém a fizer depois (roteiro no §8 do
  relatório da etapa 4), ela pode acontecer em paralelo: não mude nada incompatível no protocolo v1.

## Fora do escopo
TLS e autenticação no binário; CDN/distribuição massiva (2.0, ADR-021); apps nativos (ADR-025); proporcionais; ensaio
geral e runbook (etapa 6); baixar BUs do TSE.

## Armadilhas conhecidas
- **Redesenhar a cada mensagem** trava o navegador a 60× (100 versões/s do `br-c0001`): desenhe por quadro.
- **Animar `width` das barras** causa reflow em cascata: use `transform: scaleX` com `transform-origin: left`.
- **Hodômetro com número que cresce de dígitos** (999 → 1.000) pula se a largura não estiver reservada.
- **Fonte trocando depois de carregar** (FOUT) gera *layout shift*: embutir e esperar `document.fonts.ready` antes do
  primeiro desenho.
- **`backdrop-filter` em muitas camadas** pesa no celular: limite o número de cartões com desfoque e teste num
  aparelho modesto.
- **Glow girando com `filter: blur` grande** em tela inteira custa GPU: restrinja aos destaques.
- **Ordem dos eventos:** um `evento` pode chegar antes da `atualizacao` que traz o mesmo `seq` (vêm por caminhos
  diferentes); a página deve aceitar as duas ordens.
- **Reconexão:** o `n` dos eventos recomeça depois de reinício do serviço; deduplique por conteúdo (protocolo §5).

## Validação (por outra sessão, independente)
Quem validar vai, sem confiar no relatório: ler o diff; rodar `compilar_dev.sh`, testes e `--ignored`; abrir a página
durante o D1 a 60× e o A a 10×, medir CLS e fps, conferir os números finais contra os oficiais e os marcos contra o
`secoes.log`/`eventos.csv`; derrubar o serviço com `kill -9` e ver a página voltar sozinha; conferir que a página não
pede nada fora do protocolo (aba Network) e nenhum recurso externo; conferir versionamento e a regra `uv-`. Só então
`bump minor` → `0.5.0` (com a `0.4.0` já feita).
