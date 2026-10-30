# Relatório da etapa 5 — site de exemplo (página de resultados embutida)

*Etapa 5 do [ROADMAP](/mnt/d/git/lukeboh/urna-velox/ROADMAP.md), plano em
[HANDOFF_0.5.0.md](/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.5.0.md). Ponto de partida `v0.4.0` (`ca9352c`). Trabalho
em PATCH (`0.4.1`, `0.4.2`, `0.4.8` a `0.4.11`); **sem `bump minor`**: a `0.5.0` fica para depois da validação independente (§8) ou por
decisão do Luciano. Executado em 13–14/10/2026. Todos os caminhos são absolutos (repositório =
`/mnt/d/git/lukeboh/urna-velox`, dados = `/home/dev/uvreal`, ensaios desta etapa = `/home/dev/uvreal/etapa5`).*

## 1. Resumo

* O `uv-servico --http` serve, **do próprio binário**, uma página de resultados em `http://<end:porta>/` que acompanha a
  apuração só pelo protocolo público. Nada externo em tempo de execução (CSP `default-src 'self'`); fontes em
  `/mnt/d/git/lukeboh/urna-velox/site/`.
* **Números da tela = oficiais** ao fim de um replay, sem recarregar a página: 1º turno de 2026 a 60× — **83/83**
  tópicos (votos de cada candidato, seções, comparecimento, brancos, nulos e a ordem das linhas); 2º turno de 2024 a
  10× com `kill -9` do serviço no meio — **51/51**, com reconexão sozinha.
* **CLS = 0** nos ensaios finais (desktop, celular 360 px, movimento reduzido + tema claro, 10 abas) e, na `0.4.2`,
  no **Chrome do Windows com GPU** (NVIDIA RTX A2000), temas escuro e claro, desktop e celular — medido pela própria
  página (`PerformanceObserver` `layout-shift`). Os pulos achados nos ensaios intermediários foram corrigidos (§4).
* **Com GPU** (`0.4.2`, §6): ~54 fps (o limite da tela da sessão), quadro p99 19–21 ms, no máximo 1 tarefa longa
  (≤ 58 ms, na carga), heap 2–6 MB.
* **Cor do partido** (`0.4.2`): cada candidato leva um tom da cor do seu partido, ajustado à interface (§2.2).
* **Marcos conferidos** com o diário de seções e o `eventos.csv` em todas as noites: primeira seção (2026: 29/29;
  2024: 72/72), seção decisiva (55/55 e 51/51), `faltam_para_mat` chegando a 0 **no `seq` exato** de cada evento
  (55/55 e 51/51) e **nunca subindo** (0 subidas em 10,3 mil mensagens), BU nº 10 mil/50 mil/100 mil/250 mil.
* **Com a página aberta em 10 abas durante o D1** (1.000 clientes `uv-carga`, 2026 a 60×): BU → cliente **p99 8,8 ms**
  (janela final; 11,3 ms na noite inteira; meta < 50 ms), 0 regressões de `seq`, 1.500/1.500 mensagens = arquivo.
* Custo na thread do motor: marcos **p50 0,001–0,003 ms por lote** (meta < 0,05 ms); a conta periódica do
  `faltam_para_mat` roda na verificação de 1 s (p99 0,2–3,6 ms, orçamento de 2 ms por rodada).

## 2. O que foi feito

### 2.1. Servidor (por acréscimo ao protocolo v1)

| item | onde |
|---|---|
| **Nomes**: `candidatos.json` ganha `nomes_ufs`, `municipios[].nome`, `partido` e `vices` de cada candidato (e `nome_urna` do `nmu`), lidos dos `cm.json` e `-u.json` que o `preparar` já baixava; `--offline` regera sem requisição. `GET /api/v1/nomes` (`ETag` fixo por processo, 304) | `aquisicao/src/preparo.rs`, `motor/src/candidatos.rs` (campos opcionais; o motor ignora), `aquisicao/src/difusao.rs` (`montar_nomes`) |
| **`faltam_para_mat`**: função pura nova no motor — o maior restante `r'` com que a circunscrição já estaria decidida, por busca binária sobre `avaliar` (a decisão é monótona em `r`); `faltam = r − r'` | `motor/src/matematica.rs` + 2 testes (`faltam_casos`, `faltam_nunca_sobe`: 400 apurações aleatórias, nunca sobe e chega a 0 no mesmo passo de `avaliar`) |
| **Mensagem `marco`** (`op: marcos`) e `GET /api/v1/marcos`; `marcos.csv` na saída, relido na retomada. Classes: `primeira_secao`, `secao_decisiva`, `faltam_para_mat`, `virada`, `pct_secoes`, `bu_marco`, `ultima_secao` (PROTOCOLO_WS §4.5) | `aquisicao/src/marcos.rs` (novo, 2 testes), `aquisicao/src/servico.rs` (lote, verificação de 1 s, encerramento, saúde `custos_ms.marcos`/`faltam`), `aquisicao/src/difusao.rs` (histórico dos marcos, último `faltam` por circunscrição, entrega por conexão) |
| **Velocidade do replay** (`0.4.8`, pedido do Luciano): `uv-servico --controle-replay` (só contra o site simulado local, com `--http`) — `POST /api/v1/replay/velocidade` (0 = pausa, 1–1000×) repassado ao `uv-simulador` (`POST /controle/velocidade`, relógio sem salto), esperas do serviço seguindo a velocidade atual (inclusive a que já corre), bloco `replay` no `estado`; painel **Replay** na página. Roteiro: `aquisicao/scripts/simular_site.sh` | `aquisicao/src/simulador.rs` (relógio com base móvel, teste `relogio_muda_sem_salto`), `aquisicao/src/difusao.rs` (`Replay`, rotas, `estado`), `aquisicao/src/servico.rs` (`escala` atômica, `dormir_site`), `site/src/replay.js`; PROTOCOLO_WS §11 |
| **Replay como um vídeo** (`0.4.9`, pedido do Luciano): velocidade até **300×** (o máximo em que o serviço acompanha o pico: medido a 200×, 300×, 600× e 1000×) e **posição na noite**: ir a um instante já visto = o serviço retotaliza do disco (diário-mestre + BUs nos pacotes), em ordem cronológica e em blocos de 30 s, sem requisição — 18:30 em ~2 s de retotalização, a noite inteira em ~6 s, e a página volta em 7–16 s com o reinício; conferência 83/83 e 832/832 depois de retotalizar até o fim; os 144 eventos são os mesmos dos ensaios contínuos. Além do visto: busca (300–900×). Barra **Replay** presa acima do letreiro, com o espaço reservado | `aquisicao/src/controle_replay.rs` (novo), `aquisicao/src/servico.rs` (`reconstruir`), `aquisicao/src/simulador.rs` (`/controle/posicao`, alvo, limite do relógio), `aquisicao/src/bin/uv-servico.rs` (reinício em processo), `site/src/replay.js`; PROTOCOLO_WS §11 |
| **Curva da apuração com a noite inteira** (`0.4.10`/`0.4.11`, o Luciano viu as linhas dos candidatos constantes): o serviço guarda um ponto por minuto da noite por tópico e entrega em `GET /api/v1/curva/<topico>`; a página desenha tudo ao abrir, com escala própria para os candidatos (na escala 0–100 os décimos pareciam retas); depois de voltar no tempo a curva vem inteira (a retotalização publica a cada 2 min da noite e a difusão amostra as versões que chegam em rajada), retotalização até 18:30 ainda em 1,8 s | `aquisicao/src/difusao.rs` (+2 testes), `aquisicao/src/servico.rs` (`gravador`, `reconstruir`), `site/src/graficos.js`, `site/src/app.js`; PROTOCOLO_WS §6 |
| **Histórico do feed** (`0.4.10`, pedido do Luciano): painel "O que já passou" com filtro por categoria e texto e linha do tempo própria; CLS 0 no desktop e no celular | `site/src/historico.js` (novo), `site/src/textos.js`, `site/src/feed.js` |
| **Seções agregadas fora do total** (`0.4.9`): o `cs.json` lista 517.179 seções no 1º turno de 2026, das quais 17.931 agregadas (`nsp`: juntadas a uma principal, sem urna nem BU); o total publicado passou de 517.179 a 499.248, o `s.ts` do TSE | `aquisicao/src/tse.rs`, `aquisicao/src/preparo.rs` (`agregada` no `candidatos.json`), `motor/src/candidatos.rs`, `motor/src/resultado.rs` (+1 teste) |
| **Página embutida**: `GET /` e `GET /static/<arquivo>` de uma tabela fixa (`include_bytes!`), CSP, `nosniff`, `no-referrer`, `immutable` nos estáticos | `aquisicao/src/pagina.rs` (novo, teste da tabela: caminhos de fora → nada; nenhuma URL externa nos estáticos) |

### 2.2. Página (`site/`)

JavaScript sem framework (esbuild junta os módulos), CSS e a fonte Archivo variável (pesos e larguras, OFL) embutida.
Telas Brasil → UF → município por URL (`#/br/c0001`, `#/sp/c0003`, `#/sp71072/c0011`) e por seletores; cargos em abas;
para cada tópico: candidatos com número, nome de urna, partido, vice, % e votos (hodômetros), barra; selo de
matematicamente eleito/2º turno com o horário (e a situação final no encerramento); apuração (anel de % de seções,
seções apuradas, horário do último BU, **eleitores que faltam para o matematicamente** em hodômetro decrescente, que vira
"matematicamente decidido às hh:mm:ss"); curva da apuração (% de seções e % dos dois primeiros ao longo da noite);
comparecimento, abstenção, brancos e nulos com %; estado da conexão (cai/volta, com a contagem regressiva da
reconexão), pausa por 429 e os avisos do serviço — inclusive "o % de seções de algumas UFs pode estar errado"
(`secoes_esperadas_incoerentes`); letreiro de marcos e curiosidades no rodapé (pausa no mouse/toque/foco, detalhe ao
tocar, `aria-live` à parte).

**Design pedido** (detalhes em `site/README.md`): glassmorfismo "tátil" (vidro com `backdrop-filter`, borda clara,
sombra, inclinação e reflexo sob o ponteiro/dedo só por `transform`/`opacity`); **brilho arco-íris** por
`conic-gradient` girando (`@property --angulo`), só no selo, na linha do matematicamente eleito/2º turno e nos dois
marcos mais novos do feed; gráficos SVG à mão com valores interpolados (mola redirecionável); troca de posição dos
candidatos por `translateY`; hodômetros com largura do eleitorado reservada; tema escuro padrão e claro opcional;
`prefers-reduced-motion` sem giro, sem rolagem de dígitos (fade) e feed item a item; celular com nomes estreitados pelo
eixo de largura da fonte. **Cor do partido** (`0.4.2`, `site/src/cores.js`): a cor de referência de cada um dos 31
partidos dos dois conjuntos (Wikipédia, `Module:Political party`, consultada em 14/10/2026; partido sem cor própria usa
a da federação) dá o **matiz**; luminosidade e croma são os da interface em cada tema (OKLCH), para todos os
candidatos terem o mesmo peso e contraste com o vidro; numa disputa com matizes parecidos (PT, PSTU, PCB, PCO; PL, DC,
DEMOCRATA…) cada candidato recebe o primeiro tom da própria cor — mais claro ou mais escuro, giro de matiz ≤ 10° — a
pelo menos 0,075 em OKLab de todos os outros. A cor é um `light-dark()` (resolvido pelo tema) e é fixa na noite
(escolhida na ordem do número). UP (preto na fonte) vira grafite. `site/testes/cores.mjs`: todo partido dos
`candidatos.json` de 2026 e 2024 tem cor e nenhuma das 55 + 51 disputas repete cor. **Ritmo de desenho ≠ ritmo de mensagens**: a mensagem de resultado só guarda o texto (o JSON
é interpretado quando a tela precisa) e o desenho é por quadro.

**Curiosidades que entraram** (pedido + sugestões escolhidas): do servidor — primeira seção do Brasil/UF/circunscrição
municipal, seção decisiva, eleitores que faltam, viradas de liderança e quantas, 25/50/75/90/99/100 % das seções do
Brasil e de cada UF, BU nº 10 mil/50 mil/100 mil/250 mil/500 mil…, última seção da noite; da página — todo
"matematicamente" de candidato, primeiro governador/senador/prefeito matematicamente eleito do país e primeiro prefeito
de cada UF, primeira UF 100 % apurada, disputa mais apertada e mais folgada no momento, maior e menor comparecimento
por UF, ritmo (BUs por minuto), apuração encerrada. **Ficaram de fora**: primeira capital 100 % (precisaria da lista das
capitais), antecedência sobre a totalização do TSE (só existe depois da conferência; está na `conferencia.md`).

## 3. Critérios

| critério (handoff/ROADMAP) | resultado | onde |
|---|---|---|
| acompanha o replay **sem recarregar** e mostra ao final **os números oficiais** | 2026 1º turno a 60×: **83/83**; 2024 2º turno a 10× com `kill -9`: **51/51** (`testes/conferir.py`: números lidos da tela × `-u.json`) | E1, E2 (§5) |
| **CLS = 0** a noite inteira | **0** nas 9 abas desktop do E1 e nas duas abas do E2 (desktop e celular, com o código final); o celular do E1 teve 0,0005 no encerramento, pela pílula de largura variável, corrigida antes do E2 | §4 |
| nenhum número troca sem animação; nenhuma barra/candidato muda de posição sem transição | todo número da tela é hodômetro (votos, %, seções, horário do último BU, eleitores que faltam, ritmo); posições por `translateY` com transição; barras por `scaleX` | código + fotos |
| primeira seção = primeira linha `aplicado` do `secoes.log` | 29/29 (2026), 72/72 (2024) | `testes/conferir_marcos.py` |
| seção decisiva = seção do lote do evento | 55/55, 51/51 | idem |
| `faltam_para_mat` chega a 0 no `seq` de cada evento e nunca sobe | 55/55 e 51/51; 0 subidas (também depois do `kill -9`) | idem |
| primeiro eleito por cargo = primeiro do `eventos.csv` por cargo | calculado pela página a partir dos eventos (ordem de `seq`), com histórico na conexão | `site/src/textos.js` (`primeiros`) |
| celular: 360 px sem rolagem horizontal, toque, conexão caindo e voltando | 0 rolagem horizontal; 4 tentativas de reconexão e volta sozinha no `kill -9`; toque pausa o feed e inclina os cartões | E1, E2 |
| 10 abas durante o D1: BU → cliente p99 < 50 ms | **8,8 ms** (janela), 11,3 ms (noite inteira) | E1 |
| D1 com BU → cliente p99 < 50 ms e B perto de p50 0,8 / p99 4,8 ms com 0 clientes | D1 (com 10 abas): p99 8,8 ms ✅. B: p50 igual (0,78–0,85 ms); p99 de hoje 15–18 ms **também com o binário da `v0.4.0`** (máquina mais ruidosa que em 13/10); `0.4.1` ~2,5 ms acima da `0.4.0` nas rodadas alternadas, dentro da variação — a repetir na validação | §5.1 |
| marcos na thread do motor < ~0,05 ms por lote no p50 | **0,001 ms** (p99 0,002–0,022) | §5 |
| 60 fps no desktop, sem tarefas longas > 50 ms, memória estável por 1 h | **com GPU** (Chrome 150 no Windows, RTX A2000, 2026 a 10×, `0.4.2`): **~54 fps em todas as rodadas, que é a taxa da tela da sessão** (quadro p50 18,3–18,8 ms; a sessão RDP estava desconectada, sem monitor de 60 Hz), quadro p99 19–21 ms, 1–5 quadros > 34 ms em ~5.400; 0–1 tarefa longa, na carga (≤ 58 ms); heap 2–6 MB. Sem GPU: celular 60 fps, desktop 31–48 fps; heap sem crescer em 25 min (E2). **Falta** 1 h contínua com GPU e um monitor de 60 Hz | §6 |
| motor sem rede e sem `tokio`; executáveis `uv-` | `cargo tree -p urna-velox -e normal`: sem `tokio`/`hyper`/`axum`; nenhum executável novo | — |

## 4. Pulos achados e corrigidos (por isso os ensaios intermediários não deram 0)

1. **Faixa de estado montada antes da fonte** (fumaça 1: CLS 6·10⁻⁶ no primeiro segundo): a faixa passou a ser montada
   depois de `document.fonts.ready`.
2. **`.brilho { position: relative }`** sobrescrevia o `position: absolute` das linhas dos candidatos quando uma delas
   ganhava o brilho (fumaça 2, Presidente matematicamente 2º turno: as duas primeiras linhas se sobrepuseram; CLS 0,005
   no desktop e 0,016 no celular). O brilho não mexe mais no posicionamento.
3. **Pílula de largura variável** ("2º turno às 21:00:20" → "2º turno" no encerramento; E1 celular, CLS 0,0005): largura
   fixa.

4. **Barra de rolagem vertical** (`0.4.2`, só no Chrome com GPU — o Chromium dos ensaios usa barra sobreposta): ao
   aparecer, quando o painel cresce, ela empurrava o cabeçalho 15 px para a esquerda (CLS 8·10⁻⁵ no desktop, temas
   escuro e claro). `scrollbar-gutter: stable` reserva a calha desde o início.
5. **Pílula vazia reservada** (`0.4.2`, fotos do celular): a pílula de "eleito" tinha 104 px reservados em toda linha
   e cortava os nomes ("FLAV…", "RON…"). Agora sai do fluxo enquanto vazia (`display: none`) e entra com *fade*
   (`@starting-style`); ao aparecer só encurta o nome da própria linha — CLS 0 conferido.
6. **Aviso ilegível no tema claro** (`0.4.2`, fotos): o amarelo/verde/rosa de alerta, ok e erro foram escurecidos no
   tema claro (contraste ≥ 4,5:1).

Também corrigido nos ensaios: classe `.votos` repetida entre o cartão de votos e a coluna de votos das linhas
(a linha ficava com 196 px de altura); preload da fonte com URL diferente da do CSS.

## 5. Ensaios

Roteiro: `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e5_ensaio.sh` (simulador + `uv-servico --http 127.0.0.1:18080
--continuar` + `uv-carga` opcional + N abas Playwright/Chromium com `site/testes/rodar.mjs`, `kill -9` opcional);
conferências: `uv_tmp_e5_conf2.sh` (`conferencia.md`, `conferir_marcos.py`, `conferir.py`, resumo das abas, saúde,
`uv-carga`). `candidatos.json` com nomes regerados `--offline` (0 requisições) em
`/home/dev/uvreal/etapa5/preparo-{2024-2t,2026-1t,3221}` — iguais aos anteriores fora os campos novos (conferido).

| ensaio | pasta | o quê | resultado |
|---|---|---|---|
| fumaça 1 | `fumaca1` | 2024 2º turno a 60×, 1 aba | conferência 51/51; tela 51/51; marcos 0 erros; 48 fps (Chromium sem GPU); CLS 6·10⁻⁶ (corrigido, §4.1) |
| fumaça 2 | `fumaca2` | 2026 1º turno a 60×, desktop + celular | conferência 83/83 e 832/832; tela 83/83; marcos 0 erros; celular 60 fps, desktop 31 fps (sem GPU); CLS 0,005/0,016 (corrigido, §4.2) |
| **E1** | `E1-d1-10abas` | 2026 a 60×, **1.000 clientes + 10 abas** (desktop com conferência, celular, movimento reduzido + tema claro, 7 desktop) | conferência 83/83 e 832/832; **tela 83/83**; marcos 0 erros (primeira 29, decisiva 55, faltam 55, 0 subidas, 4 `bu_marco`, 116 `pct_secoes`, 19 viradas); **BU → cliente p50 2,7 / p99 8,8 ms** (janela), p99 11,3 / máx 66,9 ms (noite); 0 regressões; 1.500/1.500 idênticas; eventos = `eventos.csv` em 1.000/1.000; CLS 0 em 9 abas, 0,0005 no celular (corrigido, §4.3); 0 recargas, 0 requisições externas |
| **E2** | `E2-kill9-2024` | 2024 2º turno a **10×**, desktop + celular, **`kill -9` do serviço aos 300 s** e reinício 10 s depois (retomada de 43.644 BUs) | as duas abas reconectaram sozinhas (4 tentativas com espera crescente), **sem recarregar**; **tela 51/51**; conferência 51/51 e 102/102; marcos 0 erros (histórico relido do `marcos.csv`; `faltam` nunca subiu); **CLS 0** nas duas; desktop 48 fps, celular 60 fps; heap 2–3 MB |
| E3 | `E3-B-http0`, `E3b-B-http0`, `E3c-B-semhttp`, `base-{v040a,atual-a,v040b,atual-b}` | 2026 a 60×, 0 clientes, sem navegador (o B4 da etapa 4) | conferência 83/83 e 832/832 em todos; marcos 0 erros; detalhes do BU → publicação em §5.1 |
| E4 | `E4-navegador` | 2026 a 10×, página no **Chrome 150 do Windows com GPU** por CDP (`.git/uv_tmp_e5_gpu.mjs`): desktop 1440×900 (90 s) e celular 360×780 (45 s), temas escuro e claro, fotos de Presidente/Brasil e Governador/SP | ver §6 |

### 5.1. BU → publicação com 0 clientes (o que está medido não pode piorar)

Na etapa 4 (13/10, `v0.3.3`) o B deu p50 0,805 / p99 4,80 ms com `--http` (`p50_total`/`p99_total` da `saude.json`).
Hoje (14/10, madrugada) a mesma máquina está mais ruidosa: **o binário da própria `v0.4.0`**, recompilado de um
`git worktree` e rodado agora com o mesmo roteiro (`uv_tmp_e4_ensaio.sh`, núcleos separados), deu p99 de 15,2 e 16,2 ms.
Rodadas alternadas, mesmas condições (`uv_tmp_e5_base.sh`):

| rodada | binário | p50 | p99 | máx. | lote p50 | marcos p50 por lote |
|---|---|---|---|---|---|---|
| v040a | `v0.4.0` | 0,785 | 15,2 | 136 | 0,055 | — |
| atual-a | `0.4.1` | 0,775 | 18,3 | 238 | 0,055 | 0,001 |
| v040b | `v0.4.0` | 0,665 | 16,2 | 111 | 0,045 | — |
| atual-b | `0.4.1` | 0,845 | 18,4 | 111 | — | 0,001 |

(ms; BU → publicação `p50_total`/`p99_total`/`max_total`.) Rodadas isoladas antes disso: `E3-B-http0` 0,985/8,8;
`E3b-B-http0` 0,785/7,8; `E3c-B-semhttp` (sem `--http`) 0,815/11,8. Leitura: o p50 não mudou e o lote do motor também
não; o p99 oscila 8–18 ms entre rodadas do **mesmo** binário hoje (o WSL estava com o cache de página cheio — 14 de 15 GB
— depois de uma noite de ensaios). A diferença média de ~2,5 ms no p99 entre `0.4.1` e `0.4.0` nas rodadas alternadas
está dentro dessa variação, mas pode ter uma parte real: a verificação de 1 s agora faz a conta do `faltam_para_mat`,
das viradas e (a cada 5 s) das % de seções — p50 0,13 ms, p99 0,9 ms por rodada, depois de duas reduções feitas aqui
(orçamento de 2 → 0,5 ms em rodízio, % de seções só a cada 5 s e só das UFs incompletas, e janelas pequenas nas medidas
novas da saúde: a `saude()` ordena cada janela a cada 2 s na thread do motor). **Recomendação para a validação**: repetir
o B das duas versões numa máquina recém-iniciada.

Observações: os erros de console das abas no E2 são as tentativas de reconexão recusadas enquanto o serviço estava
fora ("ERR_CONNECTION_REFUSED"), esperados. Com 10 abas Chromium sem GPU em 6 núcleos, cada aba desenha a ~8 fps (a CPU
é o limite) — essas abas servem para a carga sobre o servidor, não para medir a página.

## 6. Desempenho da página

**Chromium sem GPU (Playwright no WSL, renderização por software)** — é o pior caso e o que os ensaios automáticos medem:

| situação | fps (mediana) | tarefas longas > 50 ms | heap JS | CLS |
|---|---|---|---|---|
| 2024 2º turno a 10×, desktop 1440×900 (E2) | 48 | 0 | 2–3 MB | 0 |
| 2024 2º turno a 10×, celular 360×780 (E2) | 60 | 0 | 2–3 MB | 0 |
| 2026 a 60×, desktop (fumaça 2; ~3.500 mensagens/s na página) | 31 | 2 (na carga inicial, ~60 ms) | 2–12 MB | — |
| 2026 a 60×, celular (fumaça 2) | 60 | 5 (carga inicial, ≤ 65 ms) | 3–14 MB | — |

No desktop sem GPU o custo é o `backdrop-filter` em ~4 cartões grandes desenhado por software; o celular (área menor e
desfoque menor) fica em 60 fps. Desde a fumaça 2 a página só interpreta o JSON do tópico da tela (as outras mensagens
guardam só o texto). A memória não cresce: o letreiro tem no máximo 70 itens, a curva 720 pontos, e os resultados
guardados são um por tópico.

**Navegador de verdade com GPU** (E4, `0.4.2`): Chrome 150 do Windows, **NVIDIA RTX A2000** (ANGLE/D3D11, conferido
pelo `WEBGL_debug_renderer_info`), 2026 1º turno a 10×, roteiro `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_e5_gpu.mjs`
(CDP direto; cada rodada num contexto isolado, janela própria, descartada no fim). O Chrome do usuário (porta 9222)
está numa sessão RDP desconectada e lá toda janela fica `hidden` (sem quadros); as medidas são de uma segunda instância
do mesmo Chrome (porta 9333, perfil temporário, `--disable-features=CalculateNativeWinOcclusion`,
`--disable-backgrounding-occluded-windows`, `--force_high_performance_gpu`), com a página `visible` o tempo todo.

| rodada (código final, `uv_gpu3`) | fps (mediana) | quadro p50 / p99 | quadros > 34 ms | tarefas longas | heap JS | CLS |
|---|---|---|---|---|---|---|
| desktop 1440×900, escuro, 98 s | 53 | 18,8 / 20,5 ms | 1 de 5.330 | 1 (51 ms, carga) | 2–3 MB | 0 |
| celular 360×780, escuro, 52 s | 53 | 18,6 / 20,4 ms | 1 de 2.950 | 0 | 2–3 MB | 0 |
| desktop, claro, 98 s | 54 | 18,4 / 20,0 ms | 1 de 5.443 | 0 | 3 MB | 0 |
| celular, claro, 52 s | 55 | 18,3 / 19,2 ms | 1 de 3.027 | 0 | 3–5 MB | 0 |

O teto de ~54 fps é o da tela da sessão desconectada (o quadro p50 de 18,4 ms é o intervalo do vsync, e o p99 fica
a 1–2 ms dele): a página não perde quadros. Antes das correções do §4 (`uv_gpu`, iGPU Intel UHD 770; `uv_gpu2`, RTX
A2000) os números de fps e quadro foram os mesmos e o CLS do desktop foi 8·10⁻⁵ (barra de rolagem). **Falta**: 1 h
contínua com GPU num monitor de 60 Hz (recomendado na validação, com a sessão conectada).

## 7. Riscos e limitações

* **Seção decisiva a 60×**: é a última seção do lote dentro da circunscrição; nos ensaios os lotes do evento tiveram
  1 BU (`lote_bus`), mas pode haver ambiguidade com lotes grandes (documentado no protocolo).
* **Curva da apuração** começa quando a página abre (ou reconecta): o protocolo não tem histórico de resultados.
* **Disputa apertada/folgada, comparecimento e ritmo** são "agora" por natureza; os marcos com histórico vêm do
  servidor.
* **Horário depois de uma retomada**: o serviço reavalia tudo com o relógio do site ainda zerado e usa o relógio do
  sistema (comportamento anterior a esta etapa: os eventos repetidos no `eventos.csv` e a antecedência na
  `conferencia.md` do E2 saem com o horário de 2026). Os marcos com histórico não se repetem (relidos do
  `marcos.csv`), mas o `faltam_para_mat` de uma circunscrição sem BU novo depois da retomada sai com esse horário.
  Correção sugerida para a etapa 6: na retomada, usar o horário do último BU do diário.
* **% de seções no 3221**: `pct_secoes` e o anel usam o `s.ts` pré-apuração, parcial em 9 UFs (alerta
  `secoes_esperadas_incoerentes`, que a página mostra).
* **`cs.json` de TO** continua 404 (preparar de 13/10 às 22:57, 29 requisições, sem 429).
* Ensaios automáticos em Chromium **sem GPU** (o WSL não tem): fps do desktop subestimado; a medida com GPU (§6) foi de
  poucos minutos e limitada a ~54 fps pela sessão RDP desconectada.
* **Atraso dos ensaios contínuos a 60×** (achado ao conferir a retotalização, `0.4.9`): nos ensaios da etapa 5, os
  eventos saíam em média 11 min da noite (≈ 11 s reais) depois da disponibilização do BU que os decidiu (até 46 min no
  pico: Governador de SP). Os roteiros desses ensaios usavam `--intervalo-cs 30/aceleração` — dividido de novo pela
  aceleração dentro do serviço, o que relê os 28 `cs.json` (31 MB) sem parar e disputa a CPU com a coleta. Na operação
  real (1×, intervalo de 30 s) não se aplica; o `simular_site.sh` usa o intervalo padrão (a 200× a fila fica em até
  4,5 mil seções). Vale medir o atraso disponibilização → BU aplicado num ensaio a 1×.
* **Cores dos partidos**: referência da Wikipédia (não há cor oficial no TSE); partido novo ou renomeado sem entrada
  em `site/src/cores.js` cai na cor da federação ou num matiz de reserva (o `testes/cores.mjs` aponta). PCdoB está com
  o vermelho da fonte, embora tenha adotado verde e amarelo em 2025.

## 8. Como validar (sugestão)

Sem confiar neste relatório: ler o diff (`git diff v0.4.0`); `compilar_dev.sh`; motor `--ignored`; ensaios
`--ignored` do serviço; rodar `uv_tmp_e5_ensaio.sh` (E1 e E2 acima) e as conferências; abrir
`http://localhost:18080/` no Chrome do Windows durante um replay, medir CLS e fps (DevTools → Performance), olhar a aba
Network (só `/`, `/static/*`, `/api/v1/nomes`, `/api/v1/topicos` e `/ws`), derrubar o serviço com `kill -9` e ver a
página voltar sozinha. Só então `bump minor` → `0.5.0`.

## 9. Commits desta etapa

| versão | assunto |
|---|---|
| `0.4.1` | site: página de resultados embutida no `uv-servico`, marcos da noite (`faltam_para_mat` no motor) e nomes por acréscimo ao protocolo |
| `0.4.2` | site: medida com GPU no Chrome do Windows, cor do partido nos candidatos, CLS da barra de rolagem, nomes cortados no celular e contraste do tema claro |
| `0.4.8` | replay com velocidade controlada pela página (pausa, 1× a 1000×) e roteiro `simular_site.sh` (`0.4.3`–`0.4.7` foram de documentação, em outra sessão) |
| `0.4.9` | replay como um vídeo (linha do tempo: retotalização local, busca, até 300×), barra do replay sem cobrir a página, seções agregadas fora do total |
| `0.4.10` | checkpoint: curva da apuração com o histórico da noite (`GET /api/v1/curva`) e escala própria dos candidatos, histórico do feed com filtro e linha do tempo |
| `0.4.11` | curva completa depois de voltar no tempo no replay (publicação a cada 2 min da retotalização, que um estouro de `i64` impedia), legenda da curva no celular |

## 10. Fechamento (`0.5.0`, 15/10/2026)

A etapa foi concluída na `0.5.0` **por decisão do Luciano**, sem a validação independente do §8 e sem repetir todas as
medições pedidas no handoff de fechamento (`/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.4.10.md` §3): a máquina estava
sendo usada pelos ensaios da 2.0 e um reinício do WSL interrompeu a cadeia de medições no meio.

**Entrou na `0.5.0` — hodômetro num sentido só** (pedido do Luciano): cada dígito gira sempre para a frente, como num
hodômetro de verdade (de 7 para 2 passa por 8, 9, 0, 1, também quando o número diminui); os contadores regressivos —
eleitores que faltam para o matematicamente, no cartão da apuração e no feed (`regressivo: true`) — giram no sentido
oposto. A coluna de cada dígito tem 0–9 três vezes; quando o dígito sairia dela, volta 10 ou 20 posições sem transição
(o mesmo dígito na cópia vizinha), medindo onde a coluna está no meio da transição; se a coluna ficou muito para trás
(versões a cada quadro), descarta voltas inteiras do caminho, sem inverter o sentido
(`/mnt/d/git/lukeboh/urna-velox/site/src/hodometro.js`). Teste `/mnt/d/git/lukeboh/urna-velox/site/testes/hodometro.mjs`
(Chromium do Playwright, posição de cada coluna a cada quadro): **6/6 casos** — subida, % que cai, regressivo,
regressivo que sobe (troca de disputa), rajadas de uma versão por quadro nos dois sentidos — com **0 passos contra o
sentido**, 0 saídas da coluna e o dígito final igual ao valor.

**Medições de 15/10** (roteiro `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_c50_base.sh`, mesmas condições do §5.1,
máquina recém-iniciada, saídas em `/home/dev/uvreal/etapa5/c50-*`):

| rodada | binário | BU → publicação p50 | p99 | máx. | lote p50 | marcos p50 | conferência |
|---|---|---|---|---|---|---|---|
| v040a | `v0.4.0` | 1,13 ms | 17,2 ms | 222 ms | 0,085 ms | — | 83/83, 832/832 |
| atual-a | `0.4.15` (código da `0.4.11`) | 1,02 ms | **62,3 ms** | 313 ms | 0,065 ms | 0,005 ms | 83/83, 832/832 |

As rodadas v040b e atual-b foram interrompidas pelo reinício do WSL. Com uma rodada de cada não dá para separar ruído
de regressão (no §5.1 o p99 do mesmo binário variou de 8 a 18 ms), mas a diferença é grande e coincide com a mudança do
gravador na `0.4.11` (todas as versões publicadas vão para a difusão, com cópia do nome e dos `Bytes`). **Pendente para
a etapa 6** (registrado no ROADMAP): repetir as 4 rodadas com a máquina só para isso; se confirmar, mandar as versões à
difusão só durante a retotalização (ou amostrar a curva no próprio `reconstruir`); medir a retotalização da noite
inteira depois da `0.4.11`; repetir o E2 com o código atual. Os demais critérios do §3 seguem os medidos nos ensaios
desta etapa (o código de Rust não mudou desde a `0.4.11`, verificada com motor `--ignored` 13/13 e serviço 9/9).

### 10.1. Medições do fechamento, completas (`0.5.1`, 15/10/2026, à tarde)

Com a sessão da 2.0 parada, as medições foram refeitas com a máquina só para isso (roteiros
`/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_c50_b3.sh` e `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_c50_pos.sh`; saídas
em `/home/dev/uvreal/etapa5/c50f-*`, `/home/dev/uvreal/etapa5/c50-E2` e `/home/dev/uvreal/simulacao-site`).

**B, 6 rodadas alternadas** (2026 a 60×, `--http`, 0 clientes; carga média da máquina < 1,2):

| rodada | binário | p50 | p99 | máx. | conferência |
|---|---|---|---|---|---|
| v040a | `v0.4.0` | 0,79 ms | 15,3 ms | 177 ms | 83/83, 832/832 |
| fixa | `0.5.0` + correção do gravador | 0,84 ms | 11,6 ms | 87 ms | 83/83, 832/832 |
| atuala | `0.5.0` | 0,78 ms | 10,0 ms | 115 ms | 83/83, 832/832 |
| v040b | `v0.4.0` | 0,83 ms | 10,7 ms | 62 ms | 83/83, 832/832 |
| fixb | `0.5.0` + correção do gravador | 0,88 ms | 11,7 ms | 71 ms | 83/83, 832/832 |
| atualb | `0.5.0` | 0,93 ms | 13,6 ms | 99 ms | 83/83, 832/832 |

**Não há regressão**: as três versões ficam na mesma faixa (10–15 ms de p99, p50 < 1 ms). Os 62,3 e 16,1 ms da manhã
eram ruído (máquina recém-reiniciada depois da compactação do WSL; depois, ensaios da 2.0 rodando). A correção do
gravador (versões superadas para a difusão só na retotalização do replay; na coleta ao vivo só a mais nova, como na
`0.4.0`) é neutra no desempenho e entrou na `0.5.1` porque tira da noite real cópias que ninguém usa e é o binário em que
todos os ensaios abaixo rodaram.

**Retotalização** (`0.5.1`): noite inteira (até 01:59) em 7,7–8,1 s, 489.315 BUs (5,8 s antes da `0.4.11`; o acréscimo
vem das publicações a cada 2 min da noite que a `0.4.11` introduziu para a curva); até 18:30 em 2,4 s (1,8 s antes).
Curva do `br-c0001` completa depois de voltar: 124 pontos até 18:30, 185 na noite inteira. Conferência ao fim 83/83 e
832/832, 55/55 matematicamente, 0 revogações.

**E2 de novo** (`0.5.1`; 2024 2º turno a 10×, desktop + celular, `kill -9` aos 300 s e reinício 10 s depois):
conferência 51/51 e 102/102, 51/51 matematicamente, 0 revogações; **tela × oficial 51/51**; **CLS 0** nas duas abas;
4 quedas e reconexão sozinha, 0 recargas; 0 recursos externos; marcos 0 erros (primeira seção 72, decisiva 51,
`faltam_para_mat` zero no evento 51/51, 0 subidas); BU → publicação p50 0,2 / p99 5,1 ms; BU → difusão p99 0,48 ms;
desktop 40 fps, celular 60 fps (Chromium sem GPU). A antecedência da `conferencia.md` continua com valores negativos
depois da retomada (horário do sistema, §7: correção para a etapa 6).
