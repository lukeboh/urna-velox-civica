# Ciclo diário de simulações com dados reais (18 a 23/10/2026)

*Pedido do Luciano em 17 e 18/10/2026: até o 2º turno de 25/10, o site mostra as eleições já encerradas sendo
totalizadas de novo continuamente, em rotação circular automática por hora fechada:
`452 (1ºT 2024)` → `453 (2ºT 2024)` → `3220 (1ºT 2026)` → `3221 (2ºT 2026, espera 1h)` → `452...`
Operação: `/mnt/d/git/lukeboh/urna-velox/deploy/oci/63_ciclo.sh`; no nó: `/opt/urna-velox/bin/uv-ciclo.sh`.*

## Rotação Contínua e Hora Fechada (hora de Brasília, nos dois nós da OCI)

A sequência é contínua e não depende de horários pré-fixados. Cada nova totalização é disparada na **próxima hora fechada** (17h, 18h, 19h, 20h...), até 10 s depois da virada (intervalo de leitura do vigia); os dois nós arquivam e sobem a eleição seguinte juntos, cerca de 1 a 3 minutos depois do disparo.

1. **Após totalização concluída:** quando `apuracao.encerrada == true` (BUs 100% totalizados e conferência oficial gerada), o resultado permanece no ar em exibição para acompanhamento e auditoria.
2. **Início na próxima hora cheia:** se a totalização encerrar às 20:21, a próxima inicia às 21:00. Se encerrar às 03:45, a próxima inicia às 04:00. Se encerrar a menos de 3 minutos da virada, agenda para a hora seguinte (+1h) com margem de segurança.
3. **Pleito 3221 (2º turno de 2026):** entra em modo `espera` (candidatos pré-carregados para 25/10, sem coleta). Fica em vitrine por 1 hora fechada e na hora seguinte é substituído pelo 452.
4. **Alternância de modo por eleição:** ao invés de alternar por dia, cada eleição simulada alterna o seu próprio modo entre execuções sucessivas:
   - Se o 452 anterior foi rápido, o próximo 452 será cadência (e vice-versa).
   - Se o 453 anterior foi rápido, o próximo 453 será cadência (e vice-versa).
   - Se o 3220 anterior foi rápido, o próximo 3220 será cadência (e vice-versa).
   - O 3221 é sempre tela de espera (sem coleta).

* **Rápido (500×):** totalização em fluxo no limite de processamento dos nós (2 a 10 minutos). Permanece ~50 minutos exibindo o resultado final até a próxima hora.
* **Cadência (1×):** ritmo real do TSE BU a BU. No Nó A, após 3h45 (452 e 3220) ou 2h (453), o simulador acelera automaticamente a 500× para fechar a noite, consolidar a conferência e liberar a troca na hora seguinte.

## Playlist de simulados (0.8.29)

*Pedido do Luciano em 18/10/2026: deixar claro que os dados do simulado são os reais (o que se repete é a
totalização, a cada ciclo, para mostrar a totalização e a auditoria acontecendo) e deixar o público ver a agenda e
escolher o próximo simulado. Suspensa de um dia antes até três dias depois de cada eleição oficial.*

* **Página `/playlist`** (`uv-playlist.html`, servida pelo `uv-playlist.py` no nó A em `127.0.0.1:8090`; o Caddy
  manda `/playlist` e `/api/v1/playlist*` para lá com o IP do visitante no `X-Real-IP`): no ar agora; a seguir (início
  previsto na hora fechada, previsão de fim, eleição e data em que os BUs foram coletados, modo, "★ N votos" ou
  "rotação automática"); as seis opções votáveis — 452, 453 e 3220, cada um em **tempo real** (cadência 1×) ou
  **mesma ordem, acelerado** (rápido 500×). O 3221 (vitrine) não é votável e entra pela rotação.
* **Voto:** um por IP (IPv6 por /64), trocável e retirável; o IP fica só como hash com sal em
  `/srv/urna-velox/ciclo/playlist.json`. Não vota no simulado que está no ar. Só JSON (formulário de outro site não
  vota); até 20 escritas por IP a cada 10 min.
* **Decisão:** na hora fechada, cada vigia pede `POST /api/v1/playlist/decisao` (cabeçalho `X-UV-Segredo`, arquivo
  `/etc/urna-velox/playlist.segredo`) com o ID da execução. A primeira chamada decide (mais votado; empate: quem chegou
  antes à contagem; sem votos, a rotação de sempre) e consome os votos do escolhido; a outra recebe a mesma decisão.
  O A chama `http://127.0.0.1:8090`, o B a URL pública (`PLAYLIST_URL` no `ciclo.env`; a sub-rede da OCI só libera
  7700–7701 e 8080 entre os nós). Sem resposta em ~2 min (12 tentativas), o vigia segue a rotação local. A duração
  medida de cada simulado (início → encerramento) alimenta a previsão da agenda.
* **Suspensão:** `ELEICOES_OFICIAIS` (padrão `2026-10-25`): de 00:00 do dia anterior até 00:00 do 4º dia depois
  (24/10 00:00 a 29/10 00:00) a playlist não aceita votos nem decide, e o vigia não dispara simulado nenhum (o que
  estiver no ar fica até o operador trocar para a produção, como já previsto para 24/10).
* **Página da apuração (até a 0.8.37):** chip "Dados reais do TSE · retotalização · …" (botão: abre a explicação, que
  abre sozinha na primeira visita do dia) e "▶ Playlist" ao lado. **Desde a 0.8.39, ver "Replay" abaixo.**
* **Instalar / atualizar:** `./63_ciclo.sh --playlist [domínio]` (padrão `urnavelox.com.br`; gera
  `gerado/playlist.segredo` na primeira vez; instala o serviço `uv-playlist` no A, a rota no Caddy dos dois nós —
  validada antes de recarregar, com cópia `Caddyfile.antes-playlist` —, `PLAYLIST_URL`/`ELEICOES_OFICIAIS` no
  `ciclo.env` e os scripts do ciclo; reinicia só o vigia). Página nova da apuração: binários (`--binarios`).
* **Limites conhecidos:** com a Cloudflare na frente, o IP visto seria o da Cloudflare (hoje o DNS aponta direto
  para o A); o Caddy do B não tem a playlist (responde o 503 do serviço fora do ar), o que não importa enquanto o
  DNS apontar para o A.

## Replay, selo único e a próxima eleição ao vivo (0.8.39)

*Pedido do Luciano em 20/10/2026 (M-004 e B-022 no [BUGS.md](BUGS.md)): "simulado" não serve, porque os dados são
reais; o nome passa a ser **replay**, como a reprise de um jogo. Nos textos para o público, "simulado"/"simulação"
virou "replay" (página da apuração, `/playlist` — agora "Playlist de Replays" —, mensagens do `uv-playlist` e o
painel); nos nomes internos (`uv-simulador`, `--simulacao`, campo `simulacao` do protocolo, `tse_simulado.rs`) nada
mudou, para não mexer em implantação e ensaios a cinco dias do 2º turno.*

* **Selo do topo = modo + playlist:** **REPLAY** (âmbar) quando o serviço manda `simulacao.rotulo`; **AO VIVO**
  (vermelho) com BUs aplicados ou apuração encerrada sem rótulo; **EM BREVE** sem rótulo e sem BU nenhum — a vitrine
  do 3221 e a noite da eleição antes das 17h (antes da 0.8.39 essa tela dizia "ao vivo" e não tinha a playlist: o
  B-022). O ponto do selo continua dizendo a conexão (caiu, pausa do TSE, encerrada). Tocar no selo abre o aviso, com o
  link "▶ Playlist" quando há playlist.
* **A próxima eleição ao vivo, sempre à vista:** o `uv-playlist` publica `proxima_ao_vivo` (`{eleicao, nome, inicio,
  fim, ao_vivo}`; início às 17h de Brasília de cada data em `ELEICOES_OFICIAIS`, `UV_INICIO_APURACAO` troca a hora;
  a noite vai até as 8h do dia seguinte). A página busca a playlist na partida e a cada ~15 min e mostra um chip
  vermelho/âmbar na faixa ("Próxima eleição AO VIVO · 2º turno · Eleições Gerais 2026 · domingo (25/10), 17h · faltam
  4d 02h 13min"; no celular, "AO VIVO (25/10), 17h 4d 02h") e um item fixo no letreiro, os dois abrindo o aviso. Com
  um `uv-playlist` antigo (sem o campo), a data sai da suspensão + 17h. Some quando a noite oficial está na tela (modo
  AO VIVO dentro da janela).
* **Aviso do dia:** abre sozinho na primeira entrada do dia **do visitante** (B-023: antes o "dia" era o de UTC e o
  aviso reabria depois das 21h), num replay ou com a próxima eleição anunciada; nunca com a noite oficial no ar.
* **Teste:** `site/testes/modo_pagina.mjs` (sem servidor: página de `dist/`, WebSocket e playlist falsos; replay,
  vitrine, ao vivo e dia local a 1366, 390 e 360 px). `deploy/oci/testes/test_playlist.py` confere o
  `proxima_ao_vivo`.
* **Implantar:** binários novos (`--binarios`, a página está embutida no `uv-servico`) e `./63_ciclo.sh --playlist`
  (o `uv-playlist` com o campo novo; o Caddy com o 301 do endereço antigo, abaixo).

## Endereço antigo: 301 para urnavelox.com.br (0.8.39)

*M-005: oficializar a mudança de `urnavelox.duckdns.org` para `urnavelox.com.br`.* O `Caddyfile.template` tem um bloco
`urnavelox.duckdns.org { redir https://urnavelox.com.br{uri} permanent }` (HTTP 301, também para `/robots.txt` e
`/sitemap.xml`); o `servidor/gerar_caddyfile.sh` o põe só no nó A, para onde o DuckDNS aponta (no B o Caddy nunca
obteria o certificado), e é o mesmo gerador no `02_configurar_borda.sh` e no `63_ciclo.sh --playlist`.
`DOMINIO_ANTIGO` no `nos.env` troca o endereço; vazio desliga. A página tem `<link rel="canonical"
href="https://urnavelox.com.br/">`. Teste: `deploy/oci/testes/test_caddyfile.py` (com `UV_CADDY`, passa pelo
`caddy adapt` e confere o 301 e o `Location`). **O DuckDNS precisa continuar apontando para o A** enquanto o Google
não terminar de migrar (semanas); depois do 301, acompanhar no Search Console a indexação do domínio novo.

## Regras atendidas

1. **Selo de simulação:** `uv-servico --simulacao "<eleição> · <modo>"` põe `simulacao.rotulo` na mensagem `estado`
   (e na `saude.json`); a página acrescenta "SIMULAÇÃO" ao selo de situação do topo (com a explicação no `title`) e
   um chip na faixa: "Simulação · dados reais do TSE · 1º turno de 2024 · na cadência real do TSE". O 3221 das 17h
   não é simulação e não leva o selo.
2. **Os dois nós ao mesmo tempo:** o `uv-simulador` roda no nó A; o B chega a ele por uma ponte `socat` na rede
   privada (como no `40_ensaio.sh`). Os dois coletam (federação `/etc/urna-velox/federacao-ciclo.json`, IPs de saída
   127.0.0.2 e 127.0.0.1 vistos pelo simulador) e trocam os BUs entre si; os dois servem a página.
3. **Telemetria:** `uv-telemetria.py` grava a cada 15 s uma linha em `telemetria.jsonl` (a `saude.json` inteira,
   carga, CPU, memória, disco, RSS e CPU do `uv-servico`/`uv-simulador`/Caddy, conexões na porta 8080, relógio do
   simulador). No fim de cada eleição, `uv-ciclo-resumo.py` gera o `RESUMO.json` com o veredito (aprovado = apuração
   encerrada, conferência idêntica ao oficial, nenhuma situação divergente, nenhuma revogação, nenhum duplicado, nenhum
   429, nenhum reinício), o tempo até encerrar, a ordem de totalização × recebimento e os picos. As evidências ficam em
   `/srv/urna-velox/ciclo/historico/<id>/` e vão para o bucket (`ciclo/<id>/<nó>.tar.gz`, apagadas pelo bucket em
   7 dias); `./63_ciclo.sh --telemetria` baixa para `D:\dados\urna-velox\ciclo-diario`. Log das trocas:
   `/srv/urna-velox/ciclo/ciclo.log`.
4. **Dados persistentes:** o acervo `/srv/urna-velox/acervo/<pleito>` (BUs em `.uvpk`, `cs.json`, `-u.json` oficiais,
   `candidatos.json` e linha do tempo) foi enviado uma vez ao nó A (o B recebe só candidatos e linha do tempo). Os BUs
   são os já baixados do TSE: **nenhuma requisição ao TSE nas simulações**. Cada eleição começa do zero numa pasta
   própria (`/srv/urna-velox/ciclo/atual`), apagada na troca seguinte depois de arquivadas as evidências; o acervo
   nunca é apagado. O `candidatos.json` das simulações leva a destinação final (`dvt`) dos `-u.json` oficiais
   (`deploy/oci/candidatos_simulacao.sh`, B-005): as eleições já acabaram, então nada é presumido e a conferência
   compara também as situações (cassados e indeferidos inclusive). Na noite real o `preparar` segue sem `dvt` quando
   o TSE não o publica, com o alerta `destinacao_presumida`.
5. **Ordem do TSE:** o simulador publica cada seção no horário de recebimento do BU no TSE (`--horario recebido`) e
   os nós enfileiram pela mesma ordem (`--ordem` com a linha do tempo). Com dois nós e requisições em paralelo, a
   ordem exata não é possível; o `RESUMO.json` mede o desvio (atraso no recebimento p50/p99, deslocamento de posição).
   Teste local do 2º turno de 2024 no modo rápido: atraso p50 4 s e p99 148 s da noite (no simulado de 16/10 contra a
   CDN: 1 s e 291 s).
6. **Modos alternados:** acima.

**Limite de 100 req/s:** é só para buscar arquivos no TSE (regra de 17/10/2026). Contra o simulador local, o
`uv-servico` aceita `--rps` acima de 100 (`coletor::limites`) e a federação do ciclo declara teto de 5.000 req/s por
nó com `acordo` "simulação local". Contra o TSE nada mudou: teto de 100, pausa de 10 min depois de um 429.

## Execuções de prova na OCI (17/10/2026, noite)

| execução | tempo até encerrar | conferência (nós A e B) | observação |
|---|---|---|---|
| 2º turno de 2024, rápido | 1,3 / 1,8 min | 51/51 idênticos, 102/102 situações | 0 revogações, 0 duplicados, 0 HTTP 429 |
| 2º turno de 2024, cadência (com `acelerar` às 23:33) | 4,1 / 5 min | 51/51, 102/102 | o `uv-ciclo.sh acelerar` funcionou; TSE simulado ligado |
| 1º turno de 2024, rápido | 7,8 min | 5.542/5.574: **nenhuma divergência de votos**; 33 situações e 41 "matematicamente" diferentes do oficial | candidatos depois indeferidos/cassados: o `candidatos.json` pré-eleição não tem a destinação final ("destinação presumida Válido"); 9 oficiais não finalizados |
| 1º turno de 2026, rápido | 9,8 min | 83/83 idênticos, 832/832 situações | 499.207 BUs, 41 não instaladas, 0 revogações, 0 HTTP 429 |
| 2º turno de 2026 (3221), espera | — | — | o `candidatos.json` de 16/10 estava incompleto (`cs.json` de TO ausente): regerado em 18/10 00:11 (completo, 28 UFs) e o serviço do 3221 aceita incompleto se voltar a faltar |

No modo rápido o gargalo é a troca de lotes entre os nós (no 1º turno de 2026, cada um coleta ~250 mil BUs e espera
os do outro). **Ordem no modo rápido:** cada nó aplica os seus BUs na ordem do TSE, mas os do par chegam em lotes, com
atraso; no 1º turno de 2026 o atraso de recebimento ficou em p50 ~10 min da noite (a noite inteira passa em ~1 min
real a 500×), e o p99 é dominado pelos BUs recebidos dias depois (entram no corte, 02:00). Na cadência real a ordem é a
do TSE com a defasagem de um ciclo de `cs.json`. Para ordem estrita no modo rápido seria preciso um nó só coletando
(o outro só recebendo) ou relógio mais lento. O `RESUMO.json` separa divergência de votos (falha) de situação
divergente pela destinação presumida (observação).

## Linha do tempo das eleições (`D:\dados\urna-velox\pleitos\<AAAA-MM-DD-código>\linha-do-tempo.csv`)

Desde 26/10/2026 cada pleito tem a sua pasta em `D:\dados\urna-velox\pleitos`, com o nome `AAAA-MM-DD-código` (data da
eleição e código do pleito: `2024-10-06-452`, `2024-10-27-453`, `2026-10-04-3220`, `2026-10-25-3221`); antes, as
linhas do tempo ficavam juntas em `D:\dados\urna-velox\linha-do-tempo\<pleito>.csv`.

`linha-do-tempo.csv`, uma linha por seção, no formato das métricas da coleta (lido pelo `uv-simulador --metricas` e pelo
`uv-servico --ordem`), ordenado pelo recebimento no TSE: `recebido_tse` = `dr`/`hr` do hash do `aux.json` (o
recebimento do BU no TSE). 452: 478.026 seções com recebimento (todas as não agregadas); 453: 97.392 (todas); 3220:
499.207 (todas as instaladas; 6.969 completadas com o `ordem_3220.csv` do simulado de 16/10).

**Hora de totalização do BU:** o `aux.json` não traz. A sondagem de 200 `aux.json` do 3220 (17/10/2026, 5 req/s,
`sondagem-aux/` na pasta `2026-10-04-3220`) mostrou que o `dg`/`hg` do arquivo é a hora da **última regeração** do arquivo pelo TSE (126 de
200 em 05/10 entre 12:53 e 12:56; as outras em 06, 10, 14, 15 e 16/10), não a da totalização da seção; no 2024 é
29/09/2026 (republicação). O `disponivel_cs` do `cs.json` também não serve (mediana de 4,5 h depois do recebimento no
3220). Por isso a extração completa não foi feita.

## Arquitetura da Rotação Contínua (`uv-ciclo-vigia` e `uv-ciclo`)

Para suportar o ciclo contínuo sem horários pré-fixados e com alternância de modos por pleito, o sistema implementa uma máquina de estados autônoma distribuída nos nós:

```
[ Início da Execução ] ──> [ Totalização / Espera ]
                                    │
                                    ▼ (apuracao.encerrada == true ou 1h do 3221)
                             [ Manter Resultado em Exibição ]
                                    │
                                    ▼ (virada da hora cheia HH:00:00)
                             [ Disparo uv-ciclo.sh avancar ]
                                    │
                                    ├── Arquivamento e telemetria (RESUMO.json)
                                    ├── Limpeza de /srv/urna-velox/ciclo/atual
                                    ├── Alternância do modo do pleito (rapido <-> cadencia)
                                    └── Subida da próxima eleição da fila circular
```

### Componentes

1. **`uv-ciclo-vigia.py` (daemon de orquestração local):**
   - Executa como unidade transitória do systemd (`uv-ciclo-vigia.service`), com privilégios de `root`.
   - Lê a cada 10 s o arquivo `/srv/urna-velox/ciclo/atual/saida/saude.json`.
   - Quando `apuracao.encerrada == true`, marca a execução como concluída, registra o horário exato e calcula o próximo alvo horário (`HH:00:00`), garantindo margem mínima de 3 minutos (se terminar às 17:58, agenda para 19:00).
   - Mantém o arquivo `/srv/urna-velox/ciclo/rotacao.json` com o histórico de modos executados por pleito (`historico_modos`), a execução em curso (`atual`, gravada assim que o vigia começa a acompanhá-la) e o próximo pleito, modo e horário (`proximo`, gravado na conclusão).
   - Se o vigia reiniciar depois da conclusão (`Restart=on-failure` ou `63_ciclo.sh --scripts`), retoma a decisão salva em `rotacao.json` em vez de recalcular o horário.
   - Na hora programada (até 10 s depois), dispara `/opt/urna-velox/bin/uv-ciclo.sh avancar <prox_pleito> <prox_modo>` **numa unidade transitória própria** (`systemd-run --collect --unit uv-ciclo-avancar-<epoch>`), com `CICLO_CARIMBO=<AAAAMMDD-HHMM do disparo>`, e sai. Se o disparo falhar, tenta de novo a cada minuto.
   - Sem histórico de um pleito, o modo padrão é `rapido` (o mesmo no vigia e no `uv-ciclo.sh`).

2. **`uv-ciclo.sh` (orquestrador no nó):**
   - Subcomando `avancar`: lê `rotacao.json`, obtém a próxima eleição e modo, para as unidades ativas, gera o `RESUMO.json`, envia evidências ao bucket OCI, limpa a pasta de trabalho temporária e sobe a nova rodada.
   - Mantém a trava `/etc/urna-velox/controle-manual` ativa para blindar os nós contra o `uv-atualizador`.
   - Subcomando `estado`: exibe os serviços ativos do systemd, a execução corrente, o conteúdo de `rotacao.json` (atual e próximo disparo) e as últimas linhas dos logs.

3. **`63_ciclo.sh` (gerenciador operacional do operador):**
   - `--scripts`: empacota e distribui os scripts operacionais (`uv-ciclo.sh`, `uv-ciclo-vigia.py`, `uv-telemetria.py`, `uv-ciclo-resumo.py`) para os Nós A e B, ajusta permissões e recarrega os daemons sem tocar nos binários Rust nem reiniciar a apuração em curso.
   - `--avancar` e `--iniciar`: disparam a transição nos dois nós em paralelo via `systemd-run`, com o mesmo `CICLO_CARIMBO` (minuto do comando na máquina do operador), para A e B gravarem a execução com o mesmo ID.
   - `--estado`: consulta o status da rotação e métricas de apuração nos dois nós e compara os IDs das execuções: "nós sincronizados", "carimbos diferentes" (mesmo pleito e modo) ou "nós DESSINCRONIZADOS".

### Sincronia entre os nós e risco residual

Cada nó tem o seu vigia e decide sozinho a hora do disparo, a partir do momento em que vê `apuracao.encerrada`. Com a federação, os dois encerram com segundos de diferença e chegam à mesma hora fechada. Se o encerramento cair justamente na fronteira da margem (cerca de HH:57:00), um nó pode agendar para a hora seguinte e o outro para a próxima; nesse caso o `--estado` acusa "nós DESSINCRONIZADOS" e o realinhamento é `./63_ciclo.sh --iniciar <pleito> <modo>` (os dois nós no mesmo pleito e carimbo). A coordenação entre os vigias (um nó decidindo pelos dois) fica para depois do ciclo, se o caso aparecer.

---

## Histórico de Problemas Identificados e Correções Aplicadas

Durante a implantação inicial do modelo contínuo em 18/10/2026, foram identificados e corrigidos três problemas (o 3º na revisão das 19h, antes da primeira virada automática com o vigia como root):

### 1. Falha de Permissão em `/srv/urna-velox/ciclo` (`rotacao.json` e `ciclo.log`)

* **Sintoma:** O daemon vigia emitia erros de `Permission denied` ao tentar gravar `/srv/urna-velox/ciclo/rotacao.json` e anexar mensagens a `/srv/urna-velox/ciclo/ciclo.log`.
* **Causa:** O diretório pai `/srv/urna-velox/ciclo` havia sido criado pelo root durante o provisionamento com propriedade `root:root` e máscara `0755`. Como os daemons e scripts de ciclo rodavam com o usuário não-privilegiado `urnavelox`, não tinham permissão de criar ou atualizar arquivos diretamente na raiz do ciclo.
* **Correção aplicada:**
  1. Alterado o ownership recursivo de `/srv/urna-velox/ciclo` para `urnavelox:urnavelox` via `sudo chown -R urnavelox:urnavelox /srv/urna-velox/ciclo` (embutido na ação `--scripts` do `63_ciclo.sh` e executado no início do `uv-ciclo.sh`).
  2. Implementado mecanismo de resiliência e fallback no `uv-ciclo-vigia.py`: caso a gravação em `/srv/urna-velox/ciclo/rotacao.json` falhe por erro de E/S ou permissão, o script tenta gravar e ler de `/srv/urna-velox/ciclo/atual/rotacao.json` e `/srv/urna-velox/ciclo/atual/vigia.log`.

### 2. Transição Bloqueada por Execução Não-Root do `uv-ciclo-vigia`

* **Sintoma:** Às 18:00:06 de 18/10/2026, o vigia registrou no log que o tempo de exibição do Pleito 3221 havia expirado e executou a chamada para avançar (`uv-ciclo.sh avancar 452 cadencia`), porém os nós continuaram na tela de espera do 3221 sem realizar a transição.
* **Causa:** O serviço do vigia havia sido instanciado via `systemd-run --uid urnavelox`. Contudo, o script `uv-ciclo.sh avancar` (e o fluxo `iniciar`) executa tarefas que requerem privilégios de superusuário (`root`):
  - Parada e inicialização de serviços do sistema (`systemctl stop uv-ciclo-*`, `systemctl reset-failed`);
  - Criação de unidades efêmeras do systemd (`systemd-run`);
  - Manipulação de regras de firewall no `iptables` (liberação da porta da ponte socat `PPONTE`);
  - Criação e atualização da trava `/etc/urna-velox/controle-manual`.
  Como o usuário `urnavelox` não possuía privilégios administrativos/sudo irrestritos para essas ações, a invocação falhava silenciosamente e o nó não trocava de eleição.
  *Revisão (19h):* a falta de root explica só parte. No nó B o `uv-ciclo.sh` registrou "avançando para próximo pleito… 452" às 18:00:09 e mais nada: nem arquivamento, nem erro. Isso é compatível também com o problema 3, abaixo (o avancar morrendo junto com o vigia).
* **Correção aplicada:**
  1. Removida a flag `--uid urnavelox` na criação do `uv-ciclo-vigia` em `uv-ciclo.sh`, garantindo que o daemon vigia rode com `uid=0` (root), herdando a capacidade de invocar o `uv-ciclo.sh avancar` com sucesso total.
  2. No `uv-ciclo-vigia.py`, a função `executar_avancar` foi reformulada para registrar `stdout` e `stderr` do subprocesso no log, registrar o código de retorno (`rc`) e, caso seja executada por usuário não-root, tentar automaticamente `sudo -n`.
  3. Scripts atualizados em produção com `./63_ciclo.sh --scripts`.
  4. Realizada transição assistida com `./63_ciclo.sh --avancar`, colocando com sucesso o Pleito 452 (1ºT 2024) em modo Cadência (1×) em ambos os nós, federados, com o vigia ativo sob `uid=0`. (Ainda não pronto para as viradas automáticas: ver o problema 3.)

### 3. O `avancar` disparado pelo vigia morria junto com o vigia (corrigido na 0.8.24)

* **Sintoma esperado:** na primeira virada automática com o vigia como root, os dois nós ficariam com tudo parado (`uv-ciclo-no`, `uv-ciclo-sim`, `uv-ciclo-tel`), sem arquivar a execução e sem subir a eleição seguinte, até intervenção manual. Não chegou a acontecer: a correção foi implantada às 19h de 18/10, com o 452 em cadência no ar (virada prevista para 23h).
* **Causa:** o vigia chamava `uv-ciclo.sh avancar` como subprocesso, dentro da própria unidade `uv-ciclo-vigia`. O `avancar` → `iniciar` → `parar_unidades` executa `systemctl stop uv-ciclo-vigia ...`; com `KillMode=control-group` (padrão, conferido nos nós), o systemd mata todo o cgroup da unidade, inclusive o `uv-ciclo.sh` filho, logo depois de ter enfileirado a parada das outras unidades. Como `urnavelox`, o `systemctl stop` não tinha efeito; como root, a parada passou a acontecer de verdade. O `63_ciclo.sh --avancar` funcionava porque já usava `systemd-run` (outra unidade, outro cgroup).
* **Correção aplicada:**
  1. `executar_avancar` dispara `systemd-run --quiet --collect --unit uv-ciclo-avancar-<epoch> --setenv=CICLO_CARIMBO=<hora do disparo> uv-ciclo.sh avancar <pleito> <modo>` e o vigia sai; o `avancar` sobrevive à parada do vigia. Se o `systemd-run` falhar, nova tentativa a cada minuto.
  2. **ID igual nos dois nós:** o `iniciar` usa `CICLO_CARIMBO` (do vigia ou do `63_ciclo.sh`) no lugar do relógio de cada nó. Na troca manual das 18:34 o A gravou `20261018-1834-452-cadencia` e o B `20261018-1835-452-cadencia`, e as evidências caíram em prefixos diferentes do bucket.
  3. `rotacao.json` passa a mostrar a execução em curso desde o início (o `--estado` mostrava "atual 3221 · disparo 18:00" com o 452 no ar) e o vigia retoma a decisão salva se reiniciar.
  4. Padrão de modo sem histórico unificado em `rapido` (o vigia usava `cadencia`, o `uv-ciclo.sh`, `rapido`).
  5. `63_ciclo.sh --estado` compara os IDs das execuções nos dois nós.
  6. Testado localmente (vigia com `systemd-run` simulado: execução nova, conclusão, retomada após reinício, 3221) e implantado com `./63_ciclo.sh --scripts`.

**Nota de versionamento:** a correção do problema 2 (0.8.22) foi publicada dentro do commit `76814ac`, cuja mensagem diz "0.8.21: M-002" mas cujo `VERSION` já é 0.8.22; não há commit próprio da 0.8.22. O histórico não foi reescrito (já estava no remoto).

---

## Operação

```bash
cd /mnt/d/git/lukeboh/urna-velox/deploy/oci
./63_ciclo.sh --estado                  # o que está rodando em cada nó e próximo disparo
./63_ciclo.sh --scripts                 # envia scripts atualizados aos dois nós sem reiniciar binários
./63_ciclo.sh --avancar                 # força avanço imediato para a próxima eleição da rotação
./63_ciclo.sh --iniciar 453 rapido      # troca agora para um pleito específico (os dois nós)
./63_ciclo.sh --telemetria              # evidências do bucket para D:\dados\urna-velox\ciclo-diario
./63_ciclo.sh --remover                 # desliga o ciclo
./63_ciclo.sh --binarios                # só troca os executáveis nos dois nós, sem parar nada
./63_ciclo.sh --reiniciar [A|B]         # reinicia o uv-servico do nó, retomando de onde parou
```

**Bug no meio de uma totalização.** O `uv-servico` retoma da pasta da execução (`/srv/urna-velox/ciclo/atual`: pacotes,
diário de seções e storage da federação). Para trocar só o binário e seguir de onde parou:
`./compilar_local_arm64.sh && ./63_ciclo.sh --binarios && ./63_ciclo.sh --reiniciar`. Sem o `--reiniciar`, o binário
novo entra no horário seguinte. Se o processo cair sozinho, o systemd o reinicia em 10 s (retomada automática). Nunca
reiniciar o `uv-ciclo-sim` (o relógio da noite voltaria ao começo); se a correção muda o resultado (totalização,
situação), recomece a eleição com `--iniciar <pleito> <modo>` em vez de retomar. Um reinício conta no `RESUMO.json`
(`reinicios_servico`) e reprova a execução, mesmo intencional. **Provado em 18/10 às 07:06** (1º turno de 2024, rápido):
`--reiniciar B` com 265.838 BUs aplicados; retomada de 265.838 BUs do diário em 9,2 s, 0 perdidos, 0 órfãos, e a
totalização seguiu — resultado abaixo.

**Antes de qualquer implantação durante o ciclo:** não usar `10_implantar.sh` (reescreve `no.env`/`federacao.json`,
religa a produção e regera o `candidatos.json` do 3221 contra o TSE), `20_iniciar.sh` (sobe a produção: disputa as
portas 8080/7700 e, com o `no.env` atual, coletaria do TSE), `21_parar.sh` (tira a trava manual), `50_atualizar.sh` e
`60_simulado.sh`. O `70_publicar.sh` fica inerte (a trava faz o `uv-atualizador` ignorar o manifesto). Em 18/10 o
`urna-velox.service` foi **desabilitado** nos dois nós (`systemctl disable`): se um nó reiniciasse, a produção voltaria
coletando o 3220 do TSE e brigaria com o ciclo.

**Depois do ciclo (24/10):** `./63_ciclo.sh --remover`, conferir o `no.env` de cada nó (pleito 3221, B com
`NO_B_COLETA=false`: contra o TSE os dois nós são do mesmo órgão e só o A coleta), **religar o serviço de produção**
(`sudo systemctl enable urna-velox` nos dois nós) e voltar à produção com `./20_iniciar.sh` quando combinado. A trava manual (`/etc/urna-velox/controle-manual`) fica ligada durante o ciclo:
o `uv-atualizador` não mexe nos nós.
