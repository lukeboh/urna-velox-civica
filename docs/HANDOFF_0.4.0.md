# Handoff — Urna Velox: Etapa 4, publicação em tempo real (`0.4.0`, alvo 21/10/2026)

*Preparado em 13/10/2026 e publicado na `v0.3.1`, sobre a `v0.3.0` (etapa 3 concluída: tag `v0.3.0`, commit `6e28f76`). Para iniciar a sessão, cole este documento ou peça para lê-lo. **Quem executa este handoff fica na `0.3.x` e entrega um relatório; o `bump minor` para `0.4.0` só acontece depois da validação independente descrita no fim** (ou por decisão explícita do Luciano, como na `0.3.0`).*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` no Windows (Git, PowerShell) = `/mnt/d/git/lukeboh/urna-velox` no WSL (scripts, cargo); dados = `/home/dev/uvreal` (WSL); ensaios da etapa 3 = `/home/dev/uvreal/etapa3`. Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.4.0.md`.

## Contexto
A etapa 3 entregou o **`uv-servico`** (`/mnt/d/git/lukeboh/urna-velox/aquisicao/src/servico.rs`, binário `aquisicao/src/bin/uv-servico.rs`): a partir do `candidatos.json`, acompanha os `cs.json`, coleta `aux.json` → `-bu.dat` dentro dos limites do TSE, totaliza em fluxo com o motor, calcula o **matematicamente eleito / matematicamente 2º turno**, publica JSON em `<saida>/publicacao/`, grava `eventos.csv`, `saude.json`, `secoes.log`, retoma depois de `kill -9` e confere com os oficiais no fim. E o **`uv-simulador`** (`aquisicao/src/simulador.rs`), que serve localmente o site do TSE com relógio acelerado e injeção de falhas. Relatório: `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.3.0.md`; operação: `/mnt/d/git/lukeboh/urna-velox/docs/OPERACAO.md`.

**O problema desta etapa:** hoje o resultado só existe como arquivos numa pasta. Quem quiser acompanhar (o site da etapa 5, um integrador, um jornal) teria de ler o disco ou ficar consultando arquivos. **Objetivo:** o próprio serviço publica o resultado por **WebSocket** (instantâneo na conexão + atualização a cada lote) e por um **REST mínimo**, para muitos clientes ao mesmo tempo, **sem atrasar a totalização**. Critério do ROADMAP: **BU recebido → mensagem no cliente < 50 ms no p99**, noite de 2026 a 60×, **1.000 clientes** conectados.

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes (versionamento, rede, verificação).
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — "Etapa 4", "Etapa 5" (o que o site vai precisar do protocolo) e "Metas de desempenho".
3. `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.3.0.md` — sobretudo §3 (comandos exatos dos critérios A, B e C, que você vai reaproveitar), §4 (desempenho: de onde vem cada milissegundo) e §7 (riscos).
4. `/mnt/d/git/lukeboh/urna-velox/docs/FORMATO_RESULTADO.md` — o conteúdo das mensagens é este formato, não um novo.
5. `/mnt/d/git/lukeboh/urna-velox/docs/ARCHITECTURE_DECISIONS.md` ADR-005 (consultas consistentes), ADR-012 (`tokio` só na camada HTTP), ADR-014, ADR-021 (distribuição massiva por arquivos estáticos na 2.0 — veja "Decisões") e `/mnt/d/git/lukeboh/urna-velox/docs/ESCALABILIDADE_SEGURA.md` §1–2.
6. `/mnt/d/git/lukeboh/urna-velox/ARCHITECTURE.md` §2.6 (serviço) e `/mnt/d/git/lukeboh/urna-velox/docs/OPERACAO.md` (painel, dois ritmos do `cs.json`, saúde).
7. Código: em `aquisicao/src/servico.rs`, a thread do motor (lote → `Publicador` → envio à thread **`uv-gravacao`**), `uv-gravacao`, `uv-sincronia`, `saude()`, `Painel`, os eventos; `motor/src/resultado.rs` (`Publicador`, só o que mudou, cache por UF), `motor/src/acompanhamento.rs`, `motor/src/noite.rs` (formato do `eventos.csv`); `aquisicao/src/simulador.rs` (o servidor `hyper` que já existe, para ver o padrão de `tokio` + threads do crate).
8. Testes: `/mnt/d/git/lukeboh/urna-velox/aquisicao/tests/servico.rs` (ensaios `--ignored` com simulador e serviço como processos filhos em porta aleatória) e `/mnt/d/git/lukeboh/urna-velox/motor/tests/`.

## Decisões já tomadas (mude só se o Luciano pedir)
- **Onde fica:** a camada HTTP/WebSocket entra **no `uv-servico`** (crate `uv-aquisicao`), num módulo novo (ex.: `aquisicao/src/difusao.rs`), ligada por `--http <endereço:porta>` (desligada sem o parâmetro; documente `127.0.0.1:8080` como padrão de exemplo). **O motor continua sem rede e sem `tokio`** (ADR-012/014): o runtime `tokio` da camada HTTP roda nas suas próprias threads e recebe os resultados **por canal**, como a `uv-gravacao` já recebe.
- **Biblioteca:** `axum` com WebSocket (ADR-012), sobre o `hyper 1`/`tokio` que o crate já usa. Se houver motivo forte para `hyper` + `tokio-tungstenite` direto, registre num ADR.
- **Mesmos bytes do disco:** cada lote serializa o JSON de cada abrangência × cargo **uma vez** (o `Publicador` já faz isso); o mesmo buffer (`Bytes`/`Arc`) vai para o arquivo e para todos os clientes inscritos. Nada de serializar por cliente. A mensagem final de cada tópico tem de ser **idêntica byte a byte** ao arquivo publicado.
- **A totalização nunca espera um cliente:** o motor e a `uv-gravacao` entregam o lote sem bloquear (canal que guarda só o último valor por tópico, ex.: `tokio::sync::watch`, ou fila limitada que descarta). Cliente lento recebe o **valor mais recente** (atualizações intermediárias são juntadas, com aviso de `seq` pulado), nunca uma fila sem limite; acima de um limite de memória por cliente, a conexão cai com motivo.
- **Escala desta etapa × ADR-021:** WebSocket serve o site de exemplo e integradores (milhares de conexões numa máquina). A distribuição para centenas de milhares de navegadores continua sendo a da 2.0 (arquivos estáticos numa CDN). Por isso o REST desta etapa já deve se comportar como arquivo estático (`ETag` = `seq`, `If-None-Match` → 304, `Cache-Control` curto), para que um proxy/CDN na frente funcione sem mudar nada.
- **Hora para medir:** cada mensagem leva o instante (relógio de parede, µs Unix) em que chegou o BU mais recente do lote (`t_bu`) e o instante de envio, para o cliente de carga medir BU → cliente na mesma máquina.

## Regras inegociáveis
- **Rede (TSE):** no máximo **100 req/s** somando tudo; depois de **qualquer HTTP 429**, parar tudo ≥ 10 min (o IP de saída pode ser compartilhado); nada de vários IPs, proxies ou processos em paralelo. **Esta etapa não precisa do site real:** tudo roda contra o `uv-simulador`. Não baixe BUs do TSE. O único uso real permitido é repetir `uv-coletor preparar --pleito 3221` (≤ 5 req/s) para tentar completar o `candidatos.json` (ver "Pendências herdadas"); orçamento total ~1 mil requisições; se precisar de mais, pare e pergunte ao Luciano.
- **Não rodar nada contra o site real em 25/10 e 26/10** sem combinar (noite e dia seguinte da eleição).
- **O que já está medido não pode piorar sem explicação:** os critérios A, B e C da etapa 3 continuam passando **com e sem `--http`**; com 0 clientes, BU → publicação no B fica próximo de 0,675 ms no p50 e 4,3 ms no p99 (RELATORIO_0.3.0 §4). Os limites de taxa travados no código (`coletor::limites`, `base_local`) não mudam.
- **Separação:** motor sem rede e sem `tokio`; quem faz rede (de saída e de entrada) é o `uv-aquisicao`. Nada que chega de um cliente vai além da inscrição: cliente não muda estado, não força conferência, não lê arquivo arbitrário.
- **Todos os executáveis começam com `uv-`** (o cliente de carga desta etapa é `uv-carga`).
- **Sem regressão:** antes de cada commit, `bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` (sem avisos) e `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (13 testes, ~20 s); os ensaios `--ignored` do `uv-aquisicao` antes dos commits que mexem no serviço; registre no CHANGELOG. Um número que mudar exige explicação no CHANGELOG antes de mudar o teste.
- **Versionamento a cada commit:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/versao.sh bump patch` (sem resumo na linha de comando), preencher a entrada do `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (o hook rejeita `PREENCHER`), `git add` (nunca `aquisicao/acervo/`), `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG`, `git push --follow-tags`. Commits `0.3.2`, `0.3.3`, … **Não faça `bump minor`.** **Só as versões `X.Y.0` ganham tag** (o hook `post-commit` cria; os PATCH ficam sem tag — desde a `0.3.1`, ver `/mnt/d/git/lukeboh/urna-velox/docs/VERSIONAMENTO.md`). Identidade git `Luciano Bohnert <luciano.bohnert@gmail.com>`. Mensagens `área: resumo`, em português. **Grave o `UV_COMMIT_MSG` com a ferramenta de escrita de arquivo e releia antes do commit.**
- Termos: "**matematicamente eleito**" / "matematicamente 2º turno"; os dados dos candidatos são "**candidatos**" (nunca "cadastro").
- Documentos novos com **caminhos absolutos**.

## Ambiente
- Windows + WSL2 `Ubuntu-24.04`, usuário `dev`, i7-14700 (28 threads), 15 GB no WSL. Comandos pelo Desktop Commander (`start_process`, PowerShell). **Aspas e `$` quebram no PowerShell:** escreva o comando num script `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh` (ou `.py`) e rode `wsl -d Ubuntu-24.04 -- bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh`. Cada chamada expira em ~60 s.
- **Processos longos** (simulador, serviço, carga): `nohup … &` dentro de `wsl … bash -c` **morre quando o `wsl` sai**. Inicie o script longo com `start_process` e timeout curto (o processo continua) e acompanhe o log em chamadas curtas. Para matar, `pkill -f` com **barra final** no padrão (`pkill -f etapa4/D1/`), senão você mata o processo de outro ensaio com prefixo parecido. Dê notícias ao Luciano a cada passo; não fique em `sleep` longo calado.
- Binários de desenvolvimento: `/home/dev/uvreal/target-dev/release/{uv-motor,uv-coletor,uv-servico,uv-simulador}` (+ `uv-carga`). Ensaios novos em `/home/dev/uvreal/etapa4/`.
- Git pelo PowerShell (`cd D:\git\lukeboh\urna-velox; git …`).
- **Descritores:** 1.000 clientes + o servidor na mesma máquina passam de 2.000 sockets; o `ulimit -n` padrão do WSL (1024) não basta. Suba nos scripts (`ulimit -n 65536`) e registre no OPERACAO.md.

### O que já existe localmente (não baixe de novo)
| conjunto | uso nesta etapa | onde |
|---|---|---|
| 2026, 1º turno (pleito 3220) | critério D1/D2 (= B da etapa 3 com clientes) | `/home/dev/uvreal/dados`, `/home/dev/uvreal/config`, `/home/dev/uvreal/metricas_*.csv`, `/home/dev/uvreal/oficial`, `/home/dev/uvreal/preparo_2026_1t/candidatos.json` |
| 2024, 2º turno (pleito 453) | critério D3/D4 e ensaios automatizados (= A) | `/home/dev/uvreal/2024-2t/{dados,oficial,metricas_2t.csv,preparo/candidatos.json}` |
| ensaios da etapa 3 | comandos e números de referência | `/home/dev/uvreal/etapa3/` (A, B, C*, `painel_br.py`) |
| 2026, 2º turno (pleito 3221) | publicação a frio para testar o protocolo sem BU | `/home/dev/uvreal/2026-2t/preparo/candidatos.json` (incompleto: TO) |

## O que fazer

### 1. Protocolo (documente antes de codificar)
Escreva `/mnt/d/git/lukeboh/urna-velox/docs/PROTOCOLO_WS.md` e implemente exatamente o que ele diz. Ponto de partida (ajuste com motivo):
- **Tópico** = abrangência × cargo, com o mesmo nome do arquivo publicado (`br-c0001`, `sp-c0003`, `<mun>-c0011`…). `GET /api/v1/topicos` lista os disponíveis.
- **Cliente → servidor** (texto JSON, ≤ 4 KB): `{"op":"inscrever","topicos":[…]}`, `{"op":"cancelar","topicos":[…]}`, `{"op":"eventos"}` (só eventos). Limite de tópicos por conexão (ex.: 200) e de mensagens por segundo; erro = mensagem `erro` com motivo, não queda do serviço.
- **Servidor → cliente:**
  - `instantaneo` (na inscrição: o último resultado do tópico, ou "sem dados" antes do primeiro lote);
  - `atualizacao` (a cada lote que mudou o tópico);
  - `evento` (matematicamente eleito, matematicamente 2º turno, revogação com motivo, apuração encerrada; mesmo conteúdo do `eventos.csv`, sem repetir eventos já enviados na mesma conexão e com o histórico enviado na inscrição em `eventos`);
  - `estado` (apurando/encerrada, pausa por 429 e retomada, conferência concluída, de tempos em tempos);
  - `aviso` (`seq` pulado por cliente lento, desligamento iminente).
  Toda mensagem de resultado leva `seq` (BUs aplicados, como na publicação), `secoes_apuradas` (consistentes com os totais — ADR-005), `t_bu` e `t_envio` (µs Unix) e o resultado no **formato da etapa 1** (`FORMATO_RESULTADO.md`), sem campo novo dentro dele. `seq` nunca diminui numa conexão.
- **Vivacidade:** ping do servidor a cada ~20 s; conexão sem pong por ~60 s cai. Reconexão é do cliente (o `instantaneo` resolve o estado). Documente o que um cliente deve fazer ao reconectar e ao receber `seq` pulado.
- **REST mínimo:** `GET /api/v1/estado` (subconjunto estável da `saude.json`: versão, apurando/encerrada, seções, último BU, pausa, alertas por tipo), `GET /api/v1/resultado/<topico>` (o mesmo JSON do arquivo, `ETag` = `seq`, 304 com `If-None-Match`), `GET /api/v1/eventos`, `GET /api/v1/topicos`. Só leitura, CORS `*`. Nada de listar pastas ou servir caminho vindo do cliente.

### 2. Camada HTTP/WebSocket no serviço
- `--http <end:porta>`, `--max-clientes` (padrão 5.000), threads do runtime limitadas (`--http-threads`, padrão 4) para não roubar CPU dos leitores de BU; `TCP_NODELAY`; sem compressão por mensagem no começo (meça antes de ligar).
- Ponto de saída: onde o motor entrega o lote à `uv-gravacao` (ou dentro dela, **antes** de gravar o arquivo, não depois). Um canal "último valor" por tópico + um canal de eventos; tópico sem inscritos não custa nada além do envio do `Arc`.
- Na partida e na retomada (`kill -9` + reinício), o estado inicial dos tópicos sai da publicação recarregada, para um cliente que conecta logo depois receber o mesmo que está no disco.
- Parada limpa (`SIGTERM`): avisa os clientes, fecha as conexões e sai com código 0, sem atrasar a gravação final.
- Saúde: na `saude.json` e no `/api/v1/estado`, clientes conectados, inscrições, mensagens/s e bytes/s enviados, juntadas por cliente lento, desconexões por motivo, e a latência BU → envio (p50/p99/máx) medida no servidor.
- O custo da `saude()` na thread do motor (21 ms a cada 2 s, RELATORIO_0.3.0 §4) é a maior cauda que sobrou: se aparecer no p99 de D1, tire da thread do motor (instantâneo barato + cálculo em outra thread) e meça de novo.

### 3. Cliente de carga (`uv-carga`)
Binário novo no `uv-aquisicao`: abre N conexões WebSocket a um endereço, inscreve cada uma num conjunto de tópicos (parâmetro: ex. 50 % só `br-c0001`, 50 % `br-c0001` + uma UF aleatória `c0001`/`c0003`, semente fixa), mede BU → cliente (`agora − t_bu`) e servidor → cliente (`agora − t_envio`), conta `seq` pulados e conexões perdidas, e no fim confere que a última mensagem de cada tópico é **idêntica byte a byte** ao arquivo em `<saida>/publicacao/`. Modo `--lentos K`: K conexões que não leem nada (testam a juntada e o corte). Grava um relatório JSON com histogramas. Rode com `taskset` em núcleos diferentes do serviço e registre a CPU de cada um, para a carga não distorcer a medida do serviço.

### 4. Critérios com o site simulado
Reaproveite os comandos exatos de RELATORIO_0.3.0 §3, com `--http` e o `uv-carga`. Registre no relatório o comando, a duração, as contagens do `simulador.log`, a `saude.json` final e o relatório do `uv-carga`:
- **D1 — 1º turno de 2026 a 60× com 1.000 clientes:** **BU → mensagem no cliente < 50 ms no p99** (e p50, máx.); 0 conexões perdidas; última mensagem = arquivo em todos os tópicos; a conferência automática segue com 83/83 e 832/832; 3 eventos de Presidente recebidos por todos os inscritos em `br-c0001`, iguais ao `eventos.csv`; BU → publicação, CPU e RSS do serviço comparados com o B da etapa 3 (explique a diferença).
- **D2 — clientes lentos:** D1 com mais 100 conexões `--lentos`: p99 dos outros 1.000 e BU → publicação no serviço não pioram de forma significativa; os lentos são juntados e, acima do limite, cortados com motivo; RSS limitado (registre o pico).
- **D3 — falhas** (2024 2º turno a 60×, com 200 clientes): `kill -9` do serviço no meio e reinício → os clientes reconectam e recebem `instantaneo` com `seq` ≥ o último que tinham, resultado final igual ao de A; rajada de 429 no simulador → mensagem `estado` com a pausa e a retomada; reenvio idêntico e anulação depois de publicada → `atualizacao` com os totais corrigidos e `evento`/`aviso` coerentes com os alertas.
- **D4 — A pelo WebSocket:** 2024 2º turno a 10×, um cliente inscrito em todos os tópicos: a sequência de `seq` de cada tópico só sobe, a última mensagem = arquivo (51 municípios + UFs), eventos = `eventos.csv`, situação final = conferência (102/102).
- **Ensaios automatizados `--ignored`** em `aquisicao/tests/`: D4 reduzido (algumas UFs, ~1 min) com cliente WebSocket no próprio teste; cliente lento que não lê não atrasa o serviço; `kill -9` + reconexão. Testes de unidade do protocolo (mensagens válidas e inválidas, limites, `seq` monotônico, REST com `ETag`/304) rodam sempre.

### 5. Exposição segura
O serviço vai ser exposto a clientes que não controlamos. Mínimo desta etapa: escuta em `127.0.0.1` nos exemplos (expor é decisão de quem opera, via proxy reverso com TLS — documente um exemplo `nginx`/`caddy` no OPERACAO.md, sem implementar TLS no binário); limites de conexões, de tópicos, de tamanho e de taxa de mensagens recebidas; tempo máximo para o *handshake*; nenhuma rota que escreva ou leia caminho vindo do cliente; uma conexão que viola um limite cai sem afetar as outras. Registre no relatório o teste de cada limite (ex.: 6.000 conexões com `--max-clientes 5000`, mensagem de 1 MB, 1.000 mensagens/s de um cliente).

### 6. Documentação
- `/mnt/d/git/lukeboh/urna-velox/docs/PROTOCOLO_WS.md` (novo; é o contrato da etapa 5 e de integradores): mensagens com exemplos reais tirados de D4, tópicos, REST, limites, reconexão, como medir a latência.
- `/mnt/d/git/lukeboh/urna-velox/docs/OPERACAO.md`: como ligar o `--http`, `ulimit`, proxy reverso, o que olhar na saúde nova, o que fazer quando clientes lentos começam a ser cortados.
- `ARCHITECTURE.md` (camada de difusão: motor → `uv-gravacao` → canais → `tokio`/`axum` → clientes), `docs/AQUISICAO.md` se o serviço mudar, **ADR novo** (camada HTTP/WS no serviço, último valor por tópico, relação com ADR-021) e ROADMAP com os itens da etapa 4 marcados com a versão (sem marcar a etapa como concluída).

## Pendências herdadas da etapa 3 (para a noite de 25/10)
- **`cs.json` de TO** ainda dava 404 em 13/10: o `candidatos.json` do 3221 segue incompleto (o serviço exige `--aceitar-incompleto`). Repita `uv-coletor preparar --pleito 3221` (≤ 5 req/s) **uma vez por dia, até 24/10**; se completar, registre no CHANGELOG e no relatório.
- **`s.ts` parcial em 9 UFs** para Presidente (alerta `secoes_esperadas_incoerentes`): o % de seções apuradas dessas UFs fica errado até o TSE corrigir. O protocolo não pode esconder isso: o `estado` repassa o alerta.
- **Destinação presumida "Válido"** no 3221 (os `-u.json` pré-apuração não trazem `dvt`): continua o aviso na partida; repasse no `estado`.
- O orçamento de releitura dos `cs.json` no modo ocioso é de **4 UFs por ciclo** (≤ 4 req/s; RELATORIO_0.3.0 e OPERACAO.md): não mude.

## Entregáveis
- Código (`uv-aquisicao`: camada HTTP/WS no `uv-servico`, `uv-carga`), testes, `docs/PROTOCOLO_WS.md`, documentação atualizada, ADR, CHANGELOG a cada commit, ROADMAP marcado.
- **Relatório final** `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.4.0.md`: o que foi feito; D1–D4 e os limites do item 5 com comandos exatos e números medidos (histogramas do `uv-carga`, `saude.json`, `simulador.log`); comparação com a etapa 3 (com e sem `--http`); quantos clientes a máquina aguenta antes de passar de 50 ms no p99 (suba até achar o limite ou 10.000); riscos para 25/10; o que ficou pendente. Termine com a lista de commits.

## Fora do escopo
Site e página estática (etapa 5), TLS e autenticação no binário, distribuição por CDN e federação (2.0, ADR-021/022), ensaio geral e runbook final (etapa 6), proporcionais, verificação do `.vsc`, baixar BUs do TSE.

## Armadilhas conhecidas
- **Bloquear a thread do motor** é o erro mais caro: qualquer `send` que espere, `Mutex` disputado com o `tokio` ou serialização por cliente aparece direto no p99 de BU → publicação. Meça BU → publicação com 0 clientes e com 1.000.
- `tokio::sync::broadcast` com receptor lento devolve `Lagged` e descarta: trate como `seq` pulado e mande o valor atual, não feche a conexão em silêncio.
- O `br-c0001` de 2026 é o maior tópico e vai para todos: veja o tamanho e quanto custa mandar 1.000 cópias a cada lote (bytes/s na saúde). Lotes chegam várias vezes por segundo a 60×; se o p99 vier da quantidade de envios, junte lotes por tópico num intervalo curto (ex.: 50–100 ms) e registre a escolha no protocolo.
- O cliente de carga na mesma máquina divide CPU com o serviço e com o simulador; sem `taskset`, a medida mede a carga, não o serviço.
- `ulimit -n` (ver "Ambiente"); portas efêmeras do WSL e o `somaxconn` também podem limitar a abertura de 1.000 conexões de uma vez — abra em rampa.
- Relógios: `t_bu` e o cliente usam o relógio de parede da mesma máquina; não compare com horários do site simulado (que corre a 60×).
- A publicação a frio do 3221 (43 tópicos, "apurando") é um bom teste do `instantaneo` sem nenhum BU.

## Validação (será feita por outra sessão, independente)
Quem validar vai, sem confiar no relatório: ler o diff desde `v0.3.0`; rodar `compilar_dev.sh`, os testes normais e os `--ignored`; rodar de novo D1 com 1.000 clientes e comparar com o relatório; repetir D2 (clientes lentos) e o `kill -9` com reconexão de D3; conferir byte a byte a última mensagem × arquivo; tentar quebrar os limites do item 5; conferir que o motor continua sem `tokio` (`cargo tree -p urna-velox`) e que A, B e C da etapa 3 passam com e sem `--http`; conferir versionamento (`scripts/versao.sh verificar`, só tags `X.Y.0`, CHANGELOG) e a regra `uv-`. Só então sobe `bump minor` → `0.4.0`.
