# Handoff — falta de memória (OOM) do `uv-servico` na retomada, nó A da OCI

*17/10/2026, 19:35 (Brasília). Escrito ao fim da `0.8.0` (commit `665867a`, tag `v0.8.0`). Repositório:
`D:\git\lukeboh\urna-velox` = `/mnt/d/git/lukeboh/urna-velox` no WSL (`wsl -d Ubuntu-24.04`). Pendência registrada em
`/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` (fim da etapa 8) e em `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (`0.8.0`).*

## 1. O problema

Na tarde de 17/10 o nó A (`uv-no-a`, Ampere A1, 4 vCPU, ~12 GB, **sem swap**) entrou num laço:

1. O systemd inicia o `uv-servico --federacao` (`Restart=on-failure`, 5 s).
2. Ele retoma o estado do simulado do pleito 3220 (1º turno de 2026). Leva ~190 s: 499.207 BUs do diário, 0 perdidos,
   **241.954 "órfãos no pacote"**, `estado_mb` 7,4.
3. Ele sobe a difusão (`:8080`) e republica 499.207 registros na federação (`federacao_republicados`). Depois sincroniza
   (~245 s desde a partida) e loga `particao` (A ativo, B `so_recebe`).
4. **De 4 a 5 s depois do `particao`**, o kernel mata o processo por falta de memória: `anon-rss` de ~11,75 GB e
   `total-vm` de ~13 GB, sempre os mesmos números. Entre o `particao` e a morte não sai nenhuma linha de log.

No total foram 31 mortes por OOM, a primeira às 20:11:55 UTC (17:11 de Brasília) e a última às 22:26:55 UTC. Isso
aconteceu na `0.7.3` e se repetiu igual na `0.8.0`, que instalei às 22:19 UTC. Como a versão nova não mudou nada,
**a causa já estava no código antes da etapa 8**. O nó A está **parado** desde 22:34 UTC (`systemctl stop`). O nó B
(`uv-no-b`, `0.8.0`) está ativo, com estado `parado`/`encerrada`, e a página dele responde 200.

Isso importa para a noite de 25/10: um nó que reinicia no meio da apuração passa exatamente por esta retomada, com um
estado grande.

## 2. Fatos coletados (nó A)

- `/etc/urna-velox/no.env` foi gerado pelo `10_implantar.sh` em 15/10, com `PLEITO=3220`, `BASE=https://resultados.tse.jus.br`,
  `AMBIENTE=oficial`, `RPS=95`, `CANDIDATOS=/srv/urna-velox/simulado/candidatos.json` (40 MB),
  `ORDEM=/srv/urna-velox/simulado/ordem.csv` (18 MB), `FED_PORTA=7700` e `HTTP_LOCAL=0.0.0.0:8080`. O
  `deploy/oci/servidor/uv-iniciar.sh` monta os argumentos a partir dele, com `--ordem` quando há `ORDEM`.
- O nó está com a trava manual `/etc/urna-velox/controle-manual` (`60_simulado-3220 2026-10-16T17:19:25+00:00`), então
  o `uv-atualizador` não mexe nele (`/srv/urna-velox/saida/atualizador.json`).
- Conteúdo de `/srv/urna-velox/saida` (5,2 GB):
  - `simulador.log`: **5,3 GB**, última escrita às 20:09 UTC de 17/10, dois minutos antes do primeiro OOM.
  - `simulador_resumo.json`, também das 20:09 UTC. Relógio simulado de 04/10 17:07 a 07/10 21:00, com 43,7 milhões de
    requisições, todas de 127.0.0.1: 499.211 `aux` 200, 241.955 `bu` 200, 43.007.463 `cs` 304.
  - `secoes.log` (138 MB, de 16/10), `servico.log`, `stderr.log`, `eventos.csv`, `marcos.csv`, `saude.json` e
    `publicacao/`.
  - **Conferir:** quem rodou um `uv-simulador` contra este nó até 20:09 UTC de 17/10, e se o serviço apontava para ele
    (127.0.0.1) antes de voltar ao `BASE` do TSE.
- Outras pastas em `/srv/urna-velox`: `simulado_local_20261016_184634`, `anteriores/` (saídas de simulados de 16/10,
  incluindo `20261016-141916` e `20261016_221859`), `dados`, `federacao`, `preparo` e `ensaio`.
- O systemd registra várias partidas do serviço entre 19:48 e 19:57 UTC, poucos minutos antes do fim do simulador. Era
  uma operação manual de outra sessão, que não está documentada.
- Coincidência a investigar: **241.954 órfãos na retomada** contra **241.955 BUs servidos pelo simulador**. O que é um
  "órfão no pacote", e o que o serviço faz com eles depois da partição?
- Os números de memória são idênticos em todas as mortes. Isso indica uma alocação determinada pelo estado (uma fila,
  um vetor ou um mapa dimensionado por algo da retomada), não um vazamento gradual. Candidatos:
  - a montagem da fila de coleta das 17.972 seções que faltam (517.179 − 499.207) com o `--ordem`;
  - a sondagem das não instaladas (`0.6.10`);
  - a reconstrução de algo por órfão;
  - o enfileiramento das "delegadas/assumidas" (`aquisicao/src/federacao/no.rs`, função `bater`).

## 3. Regras que valem para esta investigação (do `CLAUDE.md`)

- **No máximo 100 req/s ao TSE**, nunca mais. O nó A, quando roda, aponta para o TSE real (pleito 3220, arquivos
  públicos). **Não religue o nó A contra o TSE em laço.** Para reproduzir, use uma cópia local do estado e o
  `uv-simulador` (servidor local), ou rode o serviço com a coleta desligada.
- BUs que já estão no acervo local (`aquisicao/acervo`) não são baixados de novo.
- **Não citar arquivos ou dados confidenciais** em documento público. Um hook de pré-commit bloqueia vazamento de informação interna
  do TSE.
- Cada commit sobe a versão com `bash scripts/versao.sh bump patch` (no WSL) e preenche a entrada do `CHANGELOG.md`
  (o bump cria `- PREENCHER`). Commit com o git do Windows (PowerShell, em `D:\git\lukeboh\urna-velox`), identidade
  `Luciano Bohnert <luciano.bohnert@gmail.com>`, e publicação com `git push --follow-tags`. O MINOR (`0.9.0`) só sobe
  com a etapa 9 fechada.
- Ensaios ficam em `~/uvreal/ensaios/<nome>` e depois são arquivados com `bash scripts/disco.sh arquivar <pasta>`, que
  manda para `D:\dados\urna-velox\ensaios`. Arquivos grandes, mesmo temporários, vão para `D:\dados` (o disco C: da
  distro está quase cheio). Os scripts auxiliares podem ficar em `D:\dados\uv-tmp`.
- Executáveis novos começam com `uv-`. Use "matematicamente eleito / 2º turno" e "candidatos" (não "cadastro").

## 4. Como operar os nós (sem porta 22)

- `deploy/oci/nos.env` tem `TRANSPORTE=oci`. As primitivas de `/mnt/d/git/lukeboh/urna-velox/deploy/oci/lib.sh` são
  `em <A|B> "<comando>"` (Run Command), `para <nó> <local> <remoto>` e `de <nó> <remoto> <local>` (pelo bucket
  `urna-velox-implantacao`, que apaga objetos depois de 7 dias). Escreva um script e rode com
  `cd /mnt/d/git/lukeboh/urna-velox/deploy/oci && source ./lib.sh && em A "..."`.
- O Run Command leva de 15 a 30 s por chamada e **perde a saída de comandos longos** (um laço de 5 min voltou vazio).
  Faça chamadas curtas, ou grave num arquivo no nó e leia depois.
- O `sudo` é restrito por `/etc/sudoers.d/urna-velox-ocarun`: `systemctl start|stop urna-velox` funciona,
  `sudo journalctl` não. Em compensação, `journalctl -u urna-velox` e `journalctl -k` funcionam sem sudo.
- Situação dos dois nós: `bash deploy/oci/30_verificar.sh`. Troca de binários: `bash deploy/oci/compilar_local_arm64.sh`
  e depois `bash deploy/oci/50_atualizar.sh --sem-compilar`, que só religa o nó que estava ativo.
- Ponte da máquina do Luciano: os comandos do WSL que passam de 60 s precisam rodar em segundo plano, com a saída
  num arquivo, e ser acompanhados por leitura.

## 5. Caminho sugerido

1. **Achar onde fica o estado** do nó A: diário, BUs e lotes da federação. Na listagem da `saida/` não aparece o
   diário. Procure em `/srv/urna-velox/dados`, `federacao/` e `publicacao/`, e confira contra
   `aquisicao/src/servico.rs` (retomada) e `aquisicao/src/federacao/`.
2. **Trazer o estado para a máquina local**, sem o `simulador.log` de 5,3 GB. Use `em A "tar czf /tmp/…"` e depois
   `de A`, para `D:\dados\urna-velox\oom-no-a\`. Se o estado for grande demais para o bucket, reproduza com o simulado
   local do 3220 (`D:\dados\urna-velox\simulado-3220`, que tem o `candidatos.json` e o `ordem_3220.csv`).
3. **Reproduzir localmente** com o `uv-servico` da `0.8.0` (x86_64, `aquisicao/scripts/compilar_dev.sh`), apontando para
   um `uv-simulador` local ou com a coleta desligada, e com o limite de memória do `systemd-run --user -p MemoryMax=…`
   ou um `ulimit -v`, para não derrubar o WSL. Meça a memória ao longo do tempo (`/proc/<pid>/status`, `VmRSS`) e use
   `heaptrack` ou um `RUST_LOG`/instrumentação temporária em torno do que acontece logo depois do `particao`.
4. **Corrigir a alocação** e escrever um teste que retome um estado com os mesmos números: 499.207 aplicadas, 241.954
   órfãos, 17.972 pendentes, `--ordem` e a federação com um par `so_recebe`.
5. **Validar no nó A**, só depois de combinar com o Luciano. A retomada tem de passar do `particao` com memória estável
   (alvo: menos de 2 GB) e, se for contra o TSE real, respeitar o teto de 95 req/s, sem baixar de novo os BUs que já
   estão no estado. Depois, decidir com ele se o nó A volta ao `no.env` de produção do 3221
   (`deploy/oci/nos.env`: `PLEITO=3221`) e se a trava manual sai (`21_parar.sh` / `60_simulado.sh --restaurar`).

## 6. Critério de pronto

- A causa foi identificada: qual estrutura cresce até ~11,7 GB e por quê, com o caminho no código.
- Correção com teste automatizado. `cargo test --release --workspace` passa.
- Retomada do mesmo estado sem OOM, localmente e, se combinado, no nó A, com o pico de memória relatado.
- `ROADMAP.md` (pendência da etapa 9) e `CHANGELOG.md` atualizados, com versão PATCH nova e commit/push.
