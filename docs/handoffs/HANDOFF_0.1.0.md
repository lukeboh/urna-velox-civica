# Handoff — Urna Velox: Etapa 1, o motor (`0.1.0`, alvo 14/10/2026)

*Preparado em 12/10/2026 e publicado na `v0.0.21`, sobre a versão `v0.0.20` (ratificada: `VERSION`, manifestos, `Cargo.lock`, tag e `origin/main` coerentes em `7fb4733`; `/mnt/d/git/lukeboh/urna-velox/scripts/versao.sh verificar` → "versão 0.0.20 coerente"). Para iniciar uma nova sessão, cole este documento ou peça para lê-lo.*

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` no Windows (Git, PowerShell) = `/mnt/d/git/lukeboh/urna-velox` no WSL (scripts, cargo); dados = `/home/dev/uvreal` (WSL); acervo de dados abertos = `D:\dados\tse-arquivos-urna`. Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.1.0.md`.

## Contexto
Sou o Luciano. O Urna Velox é um motor de totalização eleitoral em Rust (repo `D:\git\lukeboh\urna-velox`, GitHub `lukeboh/urna-velox`, branch `main`). O protótipo (`/mnt/d/git/lukeboh/urna-velox/aquisicao/`, binário `uv-coletor`) já provou contra a divulgação oficial:
- 1º turno de 2026: Presidente, Governador e Senador idênticos em 83/83 abrangências, 794 situações;
- Prefeito de 2024: 5.569/5.569 municípios no 1º turno e 51/51 no 2º, 15.134 situações; consultas populares 5/5;
- simulação da noite do 2º turno de 2024: **matematicamente eleito** com mediana de 26 min antes da totalização do TSE.

Mas a **situação dos candidatos** (eleito, 2º turno, não eleito) e o "matematicamente eleito" existem só nos scripts Python de conferência (`/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_oficial.py`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_municipal.py`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/simular_noite_municipal.py`), e o núcleo está misturado com o coletor.

**Objetivo desta etapa:** um crate próprio, **`urna-velox`**, com leitor de BU, catálogo, estado, agregador, **situação dos candidatos** e **matematicamente eleito / matematicamente 2º turno**, publicando o resultado num **formato JSON documentado**, coberto por testes automatizados que reproduzem as conferências da etapa 0. É a base das etapas 2 a 6 (catálogo pré-eleição, serviço, WebSocket, site, ensaio) que levam à `1.0.0` em 23/10, para o 2º turno de 2026 (25/10).

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes.
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — seção "Etapa 1 — Motor" e o plano das duas semanas.
3. `/mnt/d/git/lukeboh/urna-velox/docs/processo/VERSIONAMENTO.md` e `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (0.0.10 em diante).
4. `/mnt/d/git/lukeboh/urna-velox/ARCHITECTURE.md` §3 (layout denso, concorrência, consolidação) e `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/ARCHITECTURE_DECISIONS.md` (ADR-002 a ADR-006, ADR-012).
5. Código: `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bu.rs`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/totaliza.rs`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/pacote.rs`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/replay.rs` (consolidação e JSON atual), `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/main.rs` (subcomandos `catalogo`, `totalizar`, `replay`).
6. O oráculo das regras de situação: `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_oficial.py` (`situacao`) e `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_municipal.py` (`situacao`, com o docstring das regras do art. 224); definição de matematicamente eleito em `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/simular_noite_municipal.py` (`definir`).
7. Doc do projeto no Claude (Projects, não está no disco): `claude/aquisicao-bus-tse.md`.

## Regras inegociáveis
- **Esta etapa não precisa de rede.** Tudo o que é necessário já está em `/home/dev/uvreal` (WSL). Não faça requisições ao TSE; se faltar algo, pergunte antes. Se um dia precisar: no máximo 100 req/s somando tudo, parar ≥ 10 min depois de um HTTP 429, nada de vários IPs/proxies/processos, e nunca baixar de novo o que está no acervo local.
- **Sem regressão:** antes de cada commit que mexe em totalização ou situação, rode as três conferências (2026, 2024 1º turno, 2024 2º turno) e registre o resultado no CHANGELOG. Os arquivos oficiais continuam sendo o oráculo até os testes desta etapa existirem, e depois viram os próprios testes.
- **Versionamento a cada commit:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/versao.sh bump patch` (sem resumo na linha de comando), preencher a entrada do `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (o hook rejeita `PREENCHER`), `git add` dos arquivos, `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG`, `git push --follow-tags`. Os commits intermediários são `0.0.23`, `0.0.24`, … (a `0.0.21` publicou este handoff e a `0.0.22`, as decisões abaixo); **o último, que cumpre o critério de pronto, usa `bump minor` → `0.1.0`** e marca a etapa no `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md`.
- **Novo manifesto:** ao criar o crate, acrescente `/mnt/d/git/lukeboh/urna-velox/motor/Cargo.toml` a `MANIFESTOS_CARGO` e `PACOTES_CARGO` em `/mnt/d/git/lukeboh/urna-velox/scripts/versao.sh` e à lista de arquivos do `/mnt/d/git/lukeboh/urna-velox/.githooks/pre-commit`; com workspace, o `Cargo.lock` passa a ser um só na raiz. Rode `/mnt/d/git/lukeboh/urna-velox/scripts/testar_versao.sh` depois.
- Não comitar `/mnt/d/git/lukeboh/urna-velox/aquisicao/acervo/` (é do Luciano, ainda sem commit). Identidade git `Luciano Bohnert <luciano.bohnert@gmail.com>`.

## Ambiente (como trabalhar na máquina)
- Windows + WSL2 `Ubuntu-24.04`, usuário `dev`, i7-14700 (28 threads). Comandos pelo Desktop Commander (`start_process`, PowerShell). **Aspas quebram no PowerShell:** escreva o comando num script em `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_*.sh` e rode `wsl -d Ubuntu-24.04 -- bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_x.sh`. Cada chamada expira em ~60 s; para jobs longos, `start_process` com `wsl -d Ubuntu-24.04 -- bash -c "bash <script> > <log> 2>&1"` e acompanhe o log (o `setsid nohup` dentro do WSL falhou às vezes).
- Compilar: `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` (alvo `/home/dev/uvreal/target-dev`). Testes: `cargo test --release` com `CARGO_TARGET_DIR=/home/dev/uvreal/target-dev`; cobertura: `/mnt/d/git/lukeboh/urna-velox/scripts/cobertura.sh` (acrescente o novo crate à lista dele).
- Git pelo PowerShell (`cd D:\git\lukeboh\urna-velox; git ...`), mensagem em `D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG`.

### Dados locais (fora do repositório)
| conjunto | BUs (`.uvpk`) | oficial (`-u.json`) | conferência atual |
|---|---|---|---|
| 2026, 1º turno (pleito 3220) | `/home/dev/uvreal/dados` (5,9 GB, 28 arquivos) | `/home/dev/uvreal/oficial` (84 arquivos, cargos 1/3/5) | `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/fechar_nacional.sh` (usa `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/totalizar_comparar.sh` + `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_oficial.py`) → 83/83 |
| 2024, 1º turno (pleito 452) | `/home/dev/uvreal/2024-1t/dados` (3,4 GB; só AC: `/home/dev/uvreal/2024-1t/dados-ac`, 15 MB) | `/home/dev/uvreal/2024-1t/oficial` (5.580; só AC: `/home/dev/uvreal/2024-1t/oficial-ac`) | `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/municipal_acervo.sh` / `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_municipal.py --eleicoes 619:11,633:55,634:53,635:51,636:54,637:52` com `--anuladas` → 5.569/5.569 |
| 2024, 2º turno (pleito 453) | `/home/dev/uvreal/2024-2t/dados` (99 MB, 20 UFs) | `/home/dev/uvreal/2024-2t/oficial` | `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_municipal.py --eleicoes 620:11` (turno 2) → 51/51 |

Métricas das coletas (horários `aux.json`, seções anuladas, "Não instalada"): `/home/dev/uvreal/metricas_*.csv`, `/home/dev/uvreal/2024-1t/metricas_aux.csv`, `/home/dev/uvreal/2024-2t/metricas_2t.csv`.

## O que existe hoje (`/mnt/d/git/lukeboh/urna-velox/aquisicao/src`, 2.750 linhas, 5 testes de unidade)
- `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/bu.rs` (259): leitor BER manual, `ler_bu<S: Sink>`; aceita `-bu.dat` e `-busa.dat` de 2024 e 2026.
- `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/totaliza.rs` (604): `Catalogo` (UFs, municípios, cargos, candidatos por UF ou por município — `cargo_municipal(c) = c == 11 || c == 13 || c >= 25`, seções anuladas), `Estado` denso de `AtomicU32` por (município, cargo, votável), `Agregador`, `totalizar_lote`, CSVs de totais por UF e por município. **O catálogo é montado a partir dos próprios BUs** (subcomando `catalogo`) e salvo em texto.
- `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/pacote.rs` (161): `.uvpk` (cabeçalho de 52 bytes + BU), `EmMemoria` via `memmap2`.
- `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/replay.rs` (290): noite reproduzida com horários de chegada, consolidação por UF/município "sujo", JSON mínimo por abrangência.
- `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/coletor.rs`, `tse.rs`, `zip.rs`, `main.rs` (mesma pasta): aquisição — **ficam no `uv-aquisicao`**.

O que **não** está nos BUs e a situação precisa: destinação de cada candidato (`dvt`: "Válido", anulado, sub judice…), data de nascimento (desempate), eleitorado da circunscrição (`e.te`: regra dos 200 mil e eleitorado restante), turno, número de vagas (Senador 1 ou 2) e se a totalização foi finalizada (`tf`). **Nesta etapa esses dados vêm dos arquivos oficiais** já baixados; na etapa 2 virão dos dados publicados antes da apuração. Por isso o motor deve recebê-los por uma estrutura própria (ver "Cadastro" abaixo), sem saber de onde vieram.

## Desenho proposto (ajuste se o código mostrar caminho melhor; registre o que mudar)

### 1. Workspace e crate
```
/mnt/d/git/lukeboh/urna-velox/Cargo.toml   # [workspace] members = ["motor", "aquisicao"]; profile.release único (lto fat, codegen-units 1)
/mnt/d/git/lukeboh/urna-velox/motor/   # crate `urna-velox`: lib + bin `urna-velox`
  src/lib.rs
  src/bu.rs           # movido de aquisicao (sem mudança de comportamento)
  src/pacote.rs       # .uvpk e EmMemoria (o motor lê pacotes; o coletor os escreve)
  src/catalogo.rs     # Catalogo (de totaliza.rs) + Cadastro
  src/estado.rs       # Estado + Agregador + totalizar_lote
  src/situacao.rs     # regras de situação e matematicamente eleito/2º turno (novo)
  src/resultado.rs    # consolidação por abrangência + JSON (novo, substitui json_resultado do replay)
  tests/              # integração com fixtures
/mnt/d/git/lukeboh/urna-velox/aquisicao/   # uv-aquisicao: coletor, tse, zip, replay, main → depende de `urna-velox = { path = "../motor" }`
```
- `/mnt/d/git/lukeboh/urna-velox/bench/rust` fica fora do workspace (é benchmark sintético isolado).
- O binário `urna-velox` nesta etapa só precisa de `totalizar` e `conferir` (ver item 6); `servir` é da etapa 3.
- Mover primeiro, sem mudar nada, e provar que `uv-coletor totalizar` dá o mesmo checksum e as mesmas conferências (commit próprio). Só depois acrescentar código novo.

### 2. Cadastro (dados que não vêm do BU)
Estrutura do motor, preenchida por um carregador. Proposta:
```rust
pub struct Cadastro { pub eleicoes: Vec<EleicaoCad> }
pub struct EleicaoCad { pub codigo: u32, pub turno: u8, pub cargos: Vec<CargoCad> }
pub struct CargoCad {
    pub cargo: u8, pub sistema: Sistema,          // MaioriaAbsoluta | MaioriaSimples { vagas } | Consulta
    pub circunscricoes: Vec<Circunscricao>,       // Brasil, UF ou município, conforme o cargo
}
pub struct Circunscricao { pub abr: Abrangencia, pub eleitorado: u32, pub candidatos: Vec<Candidato>, pub finalizada: bool }
pub struct Candidato { pub numero: u32, pub destinacao: Destinacao, pub nascimento: Option<Data> }
pub enum Destinacao { Valido, Anulado, AnuladoSubJudice, ValidoSubJudice /* … os valores que aparecerem em `dvt` */ }
```
- `Sistema` por cargo e eleitorado: Presidente e Governador → maioria absoluta no 1º turno; Prefeito → maioria absoluta só se o eleitorado da circunscrição passa de **200.000** no 1º turno, senão maioria simples; Senador → maioria simples com 1 ou 2 vagas; qualquer 2º turno → maioria simples com 1 vaga; consultas → opção mais votada.
- Carregador desta etapa: `Cadastro::dos_arquivos_oficiais(pasta, eleicoes)` lendo `cand[].n/dvt/dt`, `e.te`, `t`, `tf` dos `-u.json` (e `perg[].resp[]` das consultas). Liste todos os valores distintos de `dvt` nos três conjuntos antes de fechar o enum.

### 3. Situação final (o mesmo que os scripts, agora em Rust)
Por circunscrição, com `votos[c]` totalizados pelos BUs e `apto(c) = destinação começa com "Válido"`:
1. Ordem: votos decrescentes; empate → **o mais idoso** (`nascimento` menor; sem data vai por último).
2. **Prefeito (art. 224, observado nos oficiais de 2024):** ninguém eleito se o mais votado de **todos** (inclusive anulados) não for apto, ou se os votos de candidatos não aptos passam de 50 % da votação nominal. A maioria absoluta do Prefeito é medida sobre a **votação nominal inteira** (inclusive anulados).
3. **Presidente/Governador:** maioria absoluta sobre os votos dos **aptos** (`/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/comparar_oficial.py`).
4. Maioria absoluta atingida → "Eleito"; senão os 2 primeiros aptos → "2º turno". Maioria simples → os `vagas` primeiros aptos "Eleito". Demais → "Não eleito".
5. `tf = "n"` (totalização não finalizada) → situação vazia/pendente, como no oficial.
6. Seção anulada (aux "Anulada", BU "Excluído") → votos do BU contam como nulos (já implementado: `Catalogo::anuladas`, `--anuladas`).

Atenção à diferença de denominador entre 2 e 3: as duas regras foram as que deram 100 % nos oficiais. Não "unifique" sem rodar as três conferências.

### 4. Matematicamente eleito / matematicamente 2º turno (novo no motor)
Recalculado a cada atualização da circunscrição. Seja `R` = eleitorado ainda não apurado = `eleitorado − Σ aptos das seções já totalizadas` (mínimo 0; seções anuladas e não instaladas contam como apuradas), `V` = denominador da maioria absoluta (regra 2 ou 3 acima), `v_i` = votos do i-ésimo apto. O pior caso é todo o `R` comparecer e votar contra o candidato analisado. **Empate final é decidido pela idade (o mais idoso fica à frente), como na lei e na situação final.** Para um adversário `j` alcançar o candidato `i`, ele precisa de `custo(j, i) = v_i − v_j` votos se `j` é mais idoso que `i`, e `v_i − v_j + 1` se não é; sem data de nascimento de algum dos dois, use o caso do adversário mais idoso (custo menor).
- **Maioria simples, `k` vagas** (Prefeito ≤ 200 mil, Senador, 2º turnos): o candidato na posição `i ≤ k` está **matematicamente eleito** se `R < Σ custo(j, i)` sobre os `k − i + 1` aptos de menor custo abaixo dele. Para `k = 1`: `v_1 − v_2 > R`, ou `v_1 − v_2 = R` com o líder mais idoso (o critério de `/mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/simular_noite_municipal.py` é só o primeiro; difere apenas na igualdade exata).
- **Maioria absoluta, 1º turno:** o líder está **matematicamente eleito** se `2·v_1 > V + R` (maioria absoluta é mais da metade; aqui a idade não entra). A circunscrição está **matematicamente em 2º turno** se nenhum apto pode mais chegar à maioria: `2·v_c + R ≤ V` para todo `c`. Cada candidato está **matematicamente no 2º turno** quando a circunscrição está matematicamente em 2º turno e ele está garantido entre os 2 primeiros (regra de maioria simples com `k = 2`).
- **Prefeito, art. 224:** só declarar "matematicamente" se o líder for apto e os anulados não puderem mais passar de 50 %: `2·A + R ≤ N` (`A` = votos de não aptos, `N` = votação nominal apurada).
- Registrar, por candidato e por circunscrição, a **sequência e o horário** (do BU que disparou) em que cada estado foi atingido; esses são os eventos que a etapa 4 publica por WebSocket.
- Teste de propriedade obrigatório: em todos os municípios de 2024 (1º e 2º turnos) e abrangências de 2026, quem fica "matematicamente eleito" durante o replay é o eleito oficial, e nenhum "matematicamente" é revogado depois.

### 5. Resultado por abrangência (JSON documentado)
Documente o formato em `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/FORMATO_RESULTADO.md` (é o contrato do WebSocket da etapa 4 e do site da etapa 5). Proposta mínima, por (eleição, cargo, abrangência) — valores ilustrativos:
```json
{ "v": 1, "eleicao": 620, "cargo": 11, "abr": "xx00000", "seq": 1000, "ultimo_bu": "2024-10-27T18:40:00-03:00",
  "secoes": { "total": 1000, "apuradas": 1000 }, "eleitorado": 300000, "comparecimento": 240000,
  "vb": 4000, "vn": 11000, "vnt": 0, "validos": 225000,
  "estado": "definido",             // apurando | matematicamente_eleito | matematicamente_2t | definido | sem_eleito
  "cand": [ { "n": 10, "v": 130000, "pct": 57.78, "dvt": "Válido", "st": "Eleito",
              "mat": { "estado": "eleito", "seq": 700, "em": "2024-10-27T17:50:00-03:00" } } ] }
```
Campos que **têm** de bater com o oficial: votos por candidato (`vap`), `vb`, `vn`, `vnt`, comparecimento (`e.c`), seções totalizadas (`s.st`), situação (`st`). Abrangências: Brasil (Presidente), UF (Presidente, Governador, Senador; Presidente também `zz`), município (Prefeito, consultas; e, de brinde, os cargos gerais por município).

### 6. Testes
- **Unidade** (`/mnt/d/git/lukeboh/urna-velox/motor/src/situacao.rs`): uma tabela de casos para cada regra do item 3 e do item 4 — maioria absoluta atingida/não atingida, 200 mil exatos, 2 vagas, empate decidido pela idade, líder anulado, anulados > 50 %, `tf = "n"`, `R = 0`, empate exato no limite do "matematicamente" com o líder mais idoso e mais novo, candidato sem data de nascimento.
- **Fixtures versionadas** em `/mnt/d/git/lukeboh/urna-velox/motor/tests/fixtures/` (alvo: poucos MB no total, nada de UF inteira):
  - BUs avulsos extraídos dos `.uvpk`: um `-bu.dat` de 2026, um `-busa.dat` de 2026, um BU de 2024 com consulta popular, um BU de 2º turno de 2024 e o da seção anulada MA 07315/0023/0354 (2024, 1º turno);
  - um ou dois municípios pequenos inteiros de 2024 (de `/home/dev/uvreal/2024-1t/dados-ac`) e um município do 2º turno (o `/home/dev/uvreal/2024-2t/dados/ba.uvpk` tem 0,5 MB, com o município 34134), com os `-u.json` correspondentes e os totais esperados;
  - escreva um subcomando ou script para gerar os fixtures (`.uvpk` filtrado por município) a partir de `/home/dev/uvreal`, para que sejam reproduzíveis.
- **Integração completa** (`#[ignore]`, roda com `UV_DADOS=/home/dev/uvreal cargo test -- --ignored`): as três conferências inteiras (2026 83/83, 2024 5.569, 2024 2º turno 51/51) em Rust, comparando com os `-u.json` campo a campo e a situação de cada candidato. Quando passarem, os scripts Python viram conferência secundária (mantenha-os).
- **Replay com matematicamente:** o teste de propriedade do item 4 sobre o replay em ordem de chegada (2024 2º turno: `/home/dev/uvreal/2024-2t/metricas_2t.csv`; 2026: `/home/dev/uvreal/metricas_nacional.csv`).
- **Desempenho:** `< 10 µs` por BU numa thread (hoje 2,3 µs em 2026 e 1,2 µs em 2024) **com** situação e matematicamente recalculados por lote; eleição geral inteira em memória `< 1 s` com 28 threads (hoje 0,125 s). Meça antes e depois e registre no CHANGELOG.

## Plano sugerido (um commit verificado por passo)
1. **Workspace + mover** `bu`, `pacote`, `catalogo`, `estado` para `/mnt/d/git/lukeboh/urna-velox/motor/` sem mudança de comportamento; ajustar `/mnt/d/git/lukeboh/urna-velox/scripts/versao.sh`, `/mnt/d/git/lukeboh/urna-velox/.githooks/pre-commit` e `/mnt/d/git/lukeboh/urna-velox/scripts/cobertura.sh`; três conferências idênticas e checksum igual. → `0.0.23`
2. **Fixtures + testes de integração** do que já existe (totais), incluindo o gerador de fixtures. → `0.0.24`
3. **Cadastro + situação final** em Rust, com testes de unidade; integração completa passa com a situação do motor (sem Python). → `0.0.25`
4. **Matematicamente eleito / 2º turno** no motor e no replay, com o teste de propriedade; refazer a simulação do 2º turno de 2024 com o motor e confirmar a mediana de 26 min. → `0.0.26`
5. **Formato JSON** (`/mnt/d/git/lukeboh/urna-velox/motor/src/resultado.rs` + `/mnt/d/git/lukeboh/urna-velox/docs/arquitetura/FORMATO_RESULTADO.md`), usado pelo replay no lugar do JSON atual; binário `urna-velox totalizar|conferir`. → `0.0.27`
6. **Fechamento:** desempenho medido, cobertura medida (`/mnt/d/git/lukeboh/urna-velox/scripts/cobertura.sh`), `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` (etapa 1 ✅ e tabela de cobertura de cargos: situação "no motor"), `/mnt/d/git/lukeboh/urna-velox/ARCHITECTURE.md` (crate), docs; `bump minor` → **`0.1.0`**; atualizar o doc do projeto `claude/aquisicao-bus-tse.md` (Projects).

## Decisões tomadas (Luciano, 12/10/2026)
- Crate em `/mnt/d/git/lukeboh/urna-velox/motor/`, pacote e binário `urna-velox`; `uv-coletor` continua existindo para a aquisição até a etapa 3.
- Fixtures no repositório: até **5 MB** no total.
- No "matematicamente", o empate é decidido **pela idade, como manda a lei** (o mais idoso fica à frente), e não tratado como ultrapassagem (item 4).

## Fora do escopo desta etapa
Catálogo a partir do site antes da apuração (etapa 2), serviço automatizado (etapa 3), HTTP/WebSocket (etapa 4), site (etapa 5), cargos proporcionais e distribuição de vagas (`2.0.0`), verificação do `.vsc` e formato de 2022 (`1.x`).

## Armadilhas conhecidas
- Votos em número sem candidato apto vêm **nominais** no BU e o TSE os soma em `vnt`; candidatos "anulado sub judice" têm `dvt` próprio.
- Seções "Não instalada" entram em `s.st` sem BU (vêm das métricas da coleta); seção anulada pela Junta vira nulos.
- Arquivo oficial de abrangência menor repete a situação da circunscrição do cargo.
- `dt`/`ht` dos oficiais estão na **hora local da UF** (AC −2 h, AM/MT/MS/RO/RR −1 h em relação a Brasília); `aux.json` e `cs.json` em Brasília. Recebimento do `aux.json` pode ser posterior à disponibilização no `cs.json`; vale o menor.
- Prefeito: o catálogo precisa de candidatos **por município** (`Catalogo::listas_municipais`); a busca é binária no agregador — preserve o desempenho ao mover.
- `cs.json` de 2024 foi republicado em 29/09/2026 (horários de disponibilização do 1º turno de 2024 não são os da noite).

## Critério de pronto (`0.1.0`)
- Crate `urna-velox` no workspace; `uv-coletor` depende dele; versão e hooks coerentes.
- Testes automatizados (unidade + fixtures no CI local; integração `--ignored` com `/home/dev/uvreal`) reproduzem **83/83 abrangências de 2026, 5.569/5.569 e 51/51 municípios de 2024**, com a situação de cada candidato calculada **pelo motor**.
- Matematicamente eleito / 2º turno no motor, nunca revogado e sempre igual ao eleito oficial nos replays.
- Formato JSON documentado e usado pelo replay.
- `< 10 µs` por BU numa thread; desempenho e cobertura registrados; ROADMAP, CHANGELOG e doc do projeto atualizados; tag `v0.1.0` publicada.
