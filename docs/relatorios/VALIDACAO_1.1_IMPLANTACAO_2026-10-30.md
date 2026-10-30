# Validação da 1.1 (multipleito) na implantação — 30/10/2026

Roteiro: testes T1 a T14 da Fase 4 do prompt de 29/10 ([HANDOFF_1.1.0.md](../handoffs/HANDOFF_1.1.0.md) §4). Em 29/10 o
Luciano validou T1–T4 na tela; os demais ficaram pendentes porque dependiam de o pleito certo estar no ar na hora
certa. Em 30/10 (pedido do Luciano: "crie casos de teste, decida, corrija e implante o que falta"), o que dava para
automatizar virou teste contra o site no ar — quase sempre pela **consulta** (slider) dos pleitos encerrados, que
reproduz qualquer instante da noite sem depender da rotação.

Versão no ar durante os testes: `1.0.50`; no T14, também a `1.0.51` e a `1.0.52` (as duas partes da correção do B-081). Testes:
`site/testes/validacao_1_1.mjs` (T1, T7, T8, T9), `site/testes/consulta_desempenho.mjs` (T14), contra
`https://urnavelox.com.br`.

| teste | resultado | observação |
|---|---|---|
| T1 painel "Pleitos do TSE" no computador | passou (29/10, Luciano) e passou de novo (30/10, automático) | agora pelo **seletor de pleito** (1º chip da faixa, `1.0.50`): presente, primeiro chip, abre a lista, faixa dentro da tela a 1366 px |
| T2 o mesmo no celular | passou (29/10, Luciano) e passou de novo (30/10, automático) | a 360 px o seletor é só o ícone (40 px) e a faixa cabe |
| T3 "▶ na playlist" / "replay em preparo" | passou (29/10, Luciano) | coberto por `pleitos_pagina.mjs` |
| T4 suplementares votáveis na /playlist | passou (29/10, Luciano) | `/playlist` responde 200 |
| T5 votado entra na meia hora seguinte | **pendente de observação** | coberto pelos testes da playlist (`deploy/oci/testes/test_playlist.py`); em 30/10 de manhã não houve voto (`total_votos` 0) e o no ar é o 3220 em tempo real (não troca em 30 min). Para observar: votar num suplementar curto quando a rotação estiver em replays rápidos |
| T6 rotação de 30 min sem votos, nós sincronizados | passou, com o B-080 | 29/10: trocas na meia hora redonda nos dois nós às 16h00, 16h30, 17h30, 19h30–21h00; às 21h30 o B saiu da rotação (B-080). Corrigido na `1.0.48` (o vigia realinha com o que a playlist tem no ar em até 10 min); desde 07h34 de 30/10 os dois nós estão na mesma execução (`63_ciclo.sh --estado`: "nós sincronizados") |
| T7 replay de suplementar municipal (3226, Oiapoque/AP) | passou | pela consulta: `#/p/3226` abre em Prefeito de Oiapoque (`ap06092-c0011`), 88/88 seções, resultado final |
| T8 B-070 na tela (3237) | passou (Chromium e Firefox) | pela consulta no fim da noite: estado `sem_eleito`, os três candidatos "Não eleito" (como o `-u.json` oficial), nenhum "Eleito" na tela, selo "Sem eleito (art. 224)" |
| T9 link do art. 224 (Chrome, Safari, Firefox) | passou no Chromium e no Firefox; Safari **não testado** | link para o Código Eleitoral no site do TSE com o fragmento de texto do art. 224, `target=_blank`, `rel="noopener noreferrer"`; a aba nova abre a norma e tem o art. 224; a apuração continua na aba dela. Sem WebKit nesta máquina: o Safari fica para um aparelho real |
| T10 a T12 pleito novo, dia da eleição, fim da noite ao vivo | **pendentes até os suplementares de dezembro** | o que dá para provar sem eleição está em testes: pleito novo no `pleitos.json` (`test_uv_pleitos.py`, `test_pleitos_dinamicos.py`), catálogo pré-eleição dos agendados (`1.0.50`, 4 testes), modo `aovivo` com a série refeita durante a noite e "● ao vivo" na consulta (`test_vigia_serie.py`, cenário "em curso" de `consulta_pagina.mjs`), `no.env` sem `PLEITO` (`test_uv_iniciar.sh`) |
| T13 painel dos nós | **parcial** | `/painel/` responde 200 (página no ar); a operação com login (ver nós, mudar configuração) fica com o Luciano. O campo `PLEITO` do painel passou a opcional (vazio = o ciclo escolhe) |
| T14 desempenho no celular | **falhou na `1.0.50` (B-081); passou na `1.0.52`** | ver abaixo; aparelho real (aquecimento, bateria) fica para o Luciano |

## T14 — desempenho da consulta num celular modesto

`consulta_desempenho.mjs`: Chromium a 360 px com a CPU 4× mais lenta (CDP), abre a consulta e toca a 300× por 20 s.
Critérios (decididos agora, revisáveis): abre em ≤ 10 s; tocando, tarefas longas ≤ 35 % do tempo e a maior ≤ 1 s;
memória JS ≤ 200 MB; CLS < 0,01.

Na `1.0.50`:

| pleito | abre | tarefas longas tocando | maior | memória JS | baixado (consulta, comprimido) | CLS |
|---|---|---|---|---|---|---|
| 3237 | 0,5 s | 4 % | 166 ms | 6 MB | 0,01 MB | 0,0000 |
| 3221 | 1,1 s | 4 % | 405 ms | 33 MB | 0,67 MB | 0,0003 |
| 3220 | 1,2 s | 36 % | 421 ms | 31 MB | 0,71 MB | 0,0003 |
| 452 | 1,6 s | **97 %** | **31.615 ms** | 100 MB | 2,06 MB | 0,0008 |

O 452 travava a página (B-081): o letreiro media a largura de cada marco novo, centenas por tarefa tocando acelerado.
Corrigido em duas partes ([B-081](../itens/B-081.md)). Na `1.0.51` (o letreiro recebe em lote), só o 452:

| pleito | abre | tarefas longas tocando | maior | memória JS | baixado (consulta, comprimido) | CLS |
|---|---|---|---|---|---|---|
| 452 | 1,6 s | 83 % | 808 ms | 96 MB | 2,06 MB | 0,0008 |

A página não travava mais, mas ainda desenhava demais: cada "primeira seção" de município pedia uma onda grande ao
mapa. Na `1.0.52` (no máximo 8 ondas grandes ao mesmo tempo; letreiro menor no celular), com o serviço já de pé depois
do reinício (uma primeira rodada logo depois do reinício deu "abre em 30 s" no 3237 e no 3221: a página de
indisponível enquanto o serviço retomava; não é da página):

| pleito | abre | tarefas longas tocando | maior | memória JS | baixado (consulta, comprimido) | CLS |
|---|---|---|---|---|---|---|
| 3237 | 0,5 s | 2 % | 170 ms | 7 MB | 0,01 MB | 0,0000 |
| 3221 | 1,2 s | 4 % | 401 ms | 36 MB | 0,67 MB | 0,0003 |
| 3220 | 1,1 s | 27 % | 420 ms | 31 MB | 0,71 MB | 0,0003 |
| 452 | 1,6 s | 9 % | 363 ms | 86 MB | 2,06 MB | 0,0008 |

Todos os critérios passam. No mesmo instante, `validacao_1_1.mjs` (T1, T7, T8, T9) passou de novo na `1.0.52`.

## Cache da consulta na Cloudflare

`dados.urnavelox.com.br/consulta/*` (Caddy da `1.0.49`, implantado em 30/10): índice geral, índice de um pleito e um
tópico, três pedidos seguidos cada, todos `HTTP/2 200`, `content-encoding: gzip`, `access-control-allow-origin: *` e
`cf-cache-status: HIT` (`cache-control` 300 s no índice geral, 3600 s no resto). Num pleito em curso o `uv-servico`
manda `max-age=30` (`1.0.50`).

## Operação

* `no.env` sem `PLEITO` nos dois nós (cópias em `/etc/urna-velox/no.env.antes-sem-pleito-*`). O `uv-iniciar.sh` sem o
  pleito do ciclo sai com 78 e o `urna-velox.service` não reinicia nesse código — conferido nos dois nós.
* Nenhuma requisição nova ao TSE nesta validação (a consulta e o site são servidos pelos nós e pela Cloudflare).
