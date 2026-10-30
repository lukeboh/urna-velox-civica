# Handoff da sessão de 14/10/2026 — início da 2.0 (escalabilidade segura)

*Para continuar em outra sessão: cole este documento ou peça para lê-lo, junto com o
[HANDOFF_0.6.0.md](HANDOFF_0.6.0.md) (o plano da 2.0, etapas A a E). Este aqui conta **o que foi decidido e feito
nesta conversa**, onde as coisas estão e o que fazer em seguida. Ponto de parada: **`2.0.0-alpha.6`** na branch
`feature/2.0-escalabilidade`.*

## Onde está cada coisa

| | Caminho | Branch | Versão |
|---|---|---|---|
| 2.0 (este trabalho) | `D:\git\lukeboh\urna-velox-v2` (WSL: `/mnt/d/git/lukeboh/urna-velox-v2`) | `feature/2.0-escalabilidade` | `2.0.0-alpha.6` |
| 1.0 (`main`) | `D:\git\lukeboh\urna-velox` | `main` | `0.4.8` (outra sessão trabalha nela) |

* O diretório da 2.0 é um *git worktree* do mesmo repositório. **Commits e push pelo Git do Windows**
  (`C:\Program Files\Git`); o git do WSL (2.43) lê, mas **nunca rode `git worktree
  prune` nele** (vê o worktree como *prunable*). Não use `--relative-paths`/`extensions.relativeWorktrees` (quebra o git
  do WSL para o repositório inteiro — aconteceu e foi desfeito).
* Compilar/testar no WSL como `dev`, com target próprio: `CARGO_TARGET_DIR=/home/dev/uvreal/target-v2`
  (`~/.cargo/bin/cargo`). Os `scripts/versao.sh` rodam no Git Bash do Windows.

## Decisões do Luciano nesta conversa (todas já refletidas nos documentos)

1. **`2.0.0` = módulo de escalabilidade segura; `3.0.0` = cargos proporcionais.** Corrigido na `main` (0.4.3) — ROADMAP,
   `CLAUDE.md`, `VERSIONAMENTO.md`, CHANGELOG.
2. **Versões alpha** para a 2.0 em paralelo: `2.0.0-alpha.N`, sem tag; `scripts/versao.sh bump alpha` (na `main` desde a
   0.4.3; `definir X.Y.Z` ao sair da alpha). Ao trazer a `main` para a branch, `VERSION`/manifestos/CHANGELOG ficam com a
   alpha e as entradas da `main` entram abaixo dela.
3. **Limite de requisições ao TSE: 100 req/s por órgão, contado por IP de saída** (o que a CDN vê). Órgãos diferentes
   podem colaborar (consórcio); cada nó da federação é de um órgão, no seu IP; nós que saem pelo mesmo IP dividem os
   100 req/s. Está no `CLAUDE.md` da `main` (0.4.5–0.4.7) e no REQ-ESC-08.
4. **Sem storage compartilhado entre os nós.** Os nós trocam *heartbeats* e BUs **diretamente** (cada um serve o seu
   diretório e puxa dos pares; repasse por um terceiro quando não alcança a origem). Cada nó tem todos os BUs e totaliza
   tudo; o mesmo `uv-servico --federacao` sobe a sincronização. ADR-028 revista, REQ-ESC-05 (só na branch).
5. **Etapa B (distribuição aos cidadãos) — direção proposta, a confirmar:** também sem storage — cada nó serve os
   próprios arquivos de resultado e uma CDN só faz cache na frente (sem R2). Registrado no `HANDOFF_0.6.0.md` como "a
   confirmar". **Primeira coisa a perguntar na próxima sessão.**
6. **Por que adiantar a 2.0:** é ela que permite superar a disponibilização oficial. Pelo benchmark, um nó a
   100 req/s fica dezenas de minutos atrás no pico da noite de 2026; com 2 ou 3 órgãos, acompanha o pico.

## O que foi feito (commits na branch, em ordem)

| Versão | O quê |
|---|---|
| `alpha.1` | `HANDOFF_0.6.0.md` (por que, regra de rede, lacunas da especificação, etapas A–E com critérios) |
| `alpha.2` | merge da `main`; regra de 100 req/s por órgão/IP de saída no handoff e no REQ-ESC-08 |
| `alpha.3` | ADR-028; `aquisicao/src/federacao/` — `config` (`federacao.json`), `particao` (HRW ponderado só com inteiros, valores fixos travados por teste), `estado` (quem participa: morto, em pausa, IP divergente; grupos por IP), `armazem` (diretório local); 12 testes |
| `alpha.4` | `uv-servico --federacao` (seções delegadas, troca de resultados, IP de saída medido pelo eco, cota do limitador por IP, republicação depois de `kill -9`); `ConfigRede::ip_local`, `Limitador::definir_cota`; `uv-simulador --limite-ip-rps` (limite e caixa de penalidade por IP de origem, `GET /eco-ip`, IP na 7ª coluna do `simulador.log`, `por_ip` no resumo); `aquisicao/tests/federacao.rs` |
| `alpha.5` | **sincronização direta entre os nós** (`federacao/sincronia.rs`: HTTP só de leitura `GET /fed/v1/...` + sincronizador com repasse); `endereco` por nó no `federacao.json`; `--federacao-escuta`; contador no *heartbeat*; carência no fim para os pares levarem os últimos lotes |

E na `main`: 0.4.3 (2.0/3.0, alpha), 0.4.4 (texto depois substituído), 0.4.5 (por órgão), 0.4.6 (por IP de saída +
correção, ver "Incidente"), 0.4.7 (REQ-ESC-08).

## Estado verificado no ponto de parada

* `cargo test --release -p uv-aquisicao --lib`: **37/37**.
* `UV_DADOS=/home/dev/uvreal cargo test --release -p uv-aquisicao --test federacao -- --ignored --test-threads 1`:
  **6/6** (271 s) — três órgãos; dois no mesmo IP; IP divergente; `kill -9` sem e com reinício; repasse. Cada nó 2/2
  idêntico ao oficial, 0 HTTP 429, no máximo 601 req num segundo por IP (teto 600). Conjunto: 2º turno de 2024, AM e RO,
  5.102 BUs, 120×, vazão reduzida a 5 req/s da noite por IP para exercitar o limite. Números no CHANGELOG (`alpha.5`).
* `--test servico -- --ignored` (nó único, sem federação): **9/9** (479 s) — nada mudou para a 1.0.
* Motor sem nenhuma mudança na 2.0 até aqui (as conferências `--ignored` do motor não precisaram ser refeitas).

## Próximos passos (nesta ordem)

1. **Confirmar com o Luciano a direção da etapa B** (cada nó serve + CDN só cache, sem storage). Se confirmada,
   rever ADR-021 e REQ-ESC-01/04 e reescrever o critério B no `HANDOFF_0.6.0.md` (já esboçado lá).
2. **Fechar o critério A** (`HANDOFF_0.6.0.md`, etapa A): 3 nós com a noite de 2026 a 60× (499 mil BUs; dados e
   comandos do critério B da `0.3.0` em `docs/relatorios/RELATORIO_0.3.0.md`), `kill -9` de um nó no pico, e o 2º turno de 2024
   inteiro a 10×. Medir: tamanho do diretório da federação por nó (os lotes guardam o BU inteiro — ~16 KB por seção em
   2026, ~8 GB por nó), atraso da sincronização, req/s por IP. Rodar **em segundo plano** (ver "Ambiente") e escrever
   `docs/RELATORIO_2.0.0-A.md`.
3. Etapa C: verificação da assinatura da urna (pré-requisito para aceitar BU de outro nó em produção), lotes e
   *heartbeats* assinados com Ed25519, autenticação entre os nós, amostragem cruzada, quórum.
4. Depois: etapas D (Wasm/SPA) e E (ensaio e comparação com o TSE).
5. Nada da 2.0 contra o site real do TSE antes de 26/10 (2º turno em 25/10); qualquer teste real combinado antes.

## Ambiente e armadilhas desta sessão

* **A ponte com o computador corta chamadas em 60 s.** Ensaios longos rodam em segundo plano com saída em arquivo:
  scripts auxiliares (fora do repositório) em `D:\dados\uv2-scripts\` — `ensaio_bg.sh` (unidade + federação →
  `ensaio.log`), `servico_bg.sh` (ensaios do nó único → `servico.log`), `status.sh`/`numeros.sh` (resumo da
  `saude.json` de cada nó e do simulador), `teste.sh` (cargo com o target da 2.0). Lançar com
  `Start-Process -WindowStyle Hidden wsl -ArgumentList "-d","Ubuntu-24.04","-u","dev","--","bash","<script>"` e
  consultar o log em chamadas curtas.
* **Finais de linha misturados (LF/CRLF)** em arquivos editados por partes (ex.: `ARCHITECTURE_DECISIONS.md`): edições de
  vários parágrafos podem falhar por diferença invisível; `D:\dados\uv2-scripts\substituir.py <arquivo> <pares.json>`
  troca trechos aceitando LF ou CRLF.
* Ensaios deixam pastas em `/tmp/uv-federacao-<pid>-*` e `/tmp/uv-ensaio-<pid>-*` (podem ser apagadas).
* **Disco do WSL (incidente de 15/10):** o `ext4.vhdx` do `Ubuntu-24.04` fica em `C:\_PRJ\WSL\Ubuntu-24.04` (238 GB),
  e o C: estava quase cheio. O critério A com 3 nós e a noite de 2026 grava vários GB por nó em `/home/dev`; aos ~100
  mil BUs o vhdx encheu o C: (0 GB livres) e o Ubuntu passou a dar `I/O error` até para abrir processos (WSL parado;
  os nós morreram; a memória não era o problema: 14 GB livres, ~200 MB por nó). Resolvido no mesmo dia: apagadas as `dados/` de 58 ensaios antigos e `~/uvreal/v2` (164 GB; uso
  da distro de 234 para 70 GB) e o Luciano compactou o vhdx (C: com 184 GB livres). Regra nova no `CLAUDE.md` da branch.
  `D:\dados\uv2-scripts\criterio_a.sh` agora recusa começar com menos de 60 GB livres no disco do vhdx e aborta abaixo
  de 15 GB (`HOST_VHDX`, `HOST_MIN_GB`, `HOST_PISO_GB`), além da trava de memória. Antes de qualquer ensaio grande,
  conferir `df -h /mnt/c`.

## Incidente (para não repetir)

Na `main`, o commit 0.4.5 usou `git add -u` e levou junto **alterações de código de outra sessão** ainda sem commit
(`servico.rs`, `difusao.rs`, `simulador.rs`, `uv-servico.rs`, `site/`). A 0.4.6 desfez isso no repositório sem reescrever
o histórico, e as alterações voltaram intactas ao diretório de trabalho. **Na `main`, adicionar só os arquivos que a
própria sessão mudou** (nunca `git add -u`/`-A`); conferir `git status` antes. A outra sessão continua ativa lá.
