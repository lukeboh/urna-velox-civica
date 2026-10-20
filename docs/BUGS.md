# Bugs do Urna Velox

Lista viva (regra no [CLAUDE.md](../CLAUDE.md), seção "Bugs"). Cada bug entra aqui assim que é visto, **só com o
sintoma**: sem análise nem correção até o Luciano definir a ordem de ataque. Na correção, primeiro o teste que comprova o
bug (falhando), depois a correção (teste passando e suíte inteira verde).

Estados: **registrado** (só o sintoma) → **na fila** (ordem definida) → **teste vermelho** (teste comprova o bug) →
**corrigido** (teste verde, versão) · ou **descartado** (com o motivo).

## Ordem de ataque

Definida em 18/10/2026 (ponderação e motivos em [HANDOFF_BUGS_2026-10-18.md](HANDOFF_BUGS_2026-10-18.md), §3): **1** B-008 → **2** B-009 → **3** B-014 → **4** B-013 → **5** B-011 → **6** B-010 → **7** B-012 → **8** M-001 → **9** B-004 → **10** B-005 → **11** M-002 → **12** B-003 → **13** B-002 → **14** B-006 → **15** B-001 → **16** B-007.

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

## Melhorias

Pedidos que não são bug (mesma regra de registro: só o pedido, sem desenho nem análise até a ordem de ataque).

| id | registrado em | quem pediu | onde | pedido | estado | ordem | versão |
|---|---|---|---|---|---|---|---|
| M-001 | 18/10/2026 | Luciano | site público, topo da página | **Identificação clara e permanente da eleição que está sendo vista.** Ele fica meio perdido ao olhar a página. Referência: o aplicativo oficial da Justiça Eleitoral deixa claro no topo qual é a eleição ("Eleições 2026 · Geral Ordinária"), a abrangência ("Anamã, AM") e o turno (1º/2º) — [captura](bugs/M-001-referencia-app-je.jpg). O que ele pediu: (1) eleição, abrangência e turno **nunca somem** ao rolar a tela; (2) a marca Urna Velox **pode sumir** ao rolar; (3) o usuário pode esconder essa faixa se quiser; (4) algo **simples, que ocupe pouco espaço e seja informativo**, sem perder a qualidade e a identidade visual atuais (que ele considera muito boas); (5) os controles do topo (incluindo o selo de simulado) estão muito bons, mas a disposição dessas informações e a identificação correta da eleição precisam melhorar. | feito | 8 | 0.8.16 |
| M-002 | 18/10/2026 | Luciano | serviço, federação e nós (topologia) | **Medir a vazão de ponta a ponta para achar uma topologia que escale.** Contexto: o motor soma os ~500 mil BUs em menos de 1 s (prova de conceito), mas o ciclo rápido de 18/10 levou ~8 min (coleta HTTP de `aux.json` + BU por seção, troca de lotes entre os nós — o nó A esperou o B com a fila zerada —, publicação das abrangências e WebSocket, diário em disco, nós ARM de 2 núcleos). Objetivo: medir onde vai o tempo em cada etapa (incluindo o I/O para obter os dados, do TSE e do disco) e avaliar uma topologia que permita **aumentar a vazão quando precisar** (mais nós, outra divisão de papéis etc.); se houver possibilidade de melhorar, ele quer avaliar. | medido; decisão de topologia com o Luciano | 11 | 0.8.21 |
| M-003 | 18/10/2026 | Luciano | site público (horários na tela e card "Previsão") | **Separar o horário que vem do TSE do horário do próprio sistema.** Hoje a página mostra datas e horas de totalização tiradas do TSE (dos BUs e dos arquivos de divulgação); ele quer que fique distinto, na tela, o que é horário do TSE e o que é horário do Urna Velox (quando o sistema de fato totalizou). E **toda a previsão de horário** (matematicamente, fim da apuração) deve ser baseada no que o sistema está fazendo, **também no simulado**. Esclarecimento (18/10, 20h29): no simulado valem a data e a hora reais em que o sistema totalizou; para comparar com o TSE (eleição real, em outro dia), ajustar a diferença de data entre a eleição real e o simulado, para medir a distância de tempo entre as duas. | feito (teste `horarios_do_sistema_no_simulado`; saída abaixo) | pedida em 18/10 (antes da fila) | 0.8.28 |

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

