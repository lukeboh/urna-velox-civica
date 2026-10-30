# Validação da etapa 2 (catálogo e candidatos pré-eleição) — 12/10/2026

Validação independente do trabalho descrito em `/mnt/d/git/lukeboh/urna-velox/docs/relatorios/RELATORIO_0.2.0.md`, feita contra
`/mnt/d/git/lukeboh/urna-velox/docs/handoffs/HANDOFF_0.2.0.md`, sem confiar no relatório. **Resultado: não aprovado para `0.2.0`.**
O núcleo funciona (testes passam, `OFF_VNT` correto, rede respeitada), mas há quatro defeitos que afetam a noite de
25/10 e o critério da etapa não foi cumprido de fato. Correções obrigatórias na seção 3.

> **Revalidação em 12/10 (seção 6): aprovado para `0.2.0`** com as correções da `v0.1.7`. As correções e a
> revalidação foram feitas pela mesma sessão (a pedido do Luciano), não por uma validação independente.

## 1. O que foi conferido e está correto

| item | resultado |
|---|---|
| Compilação e testes normais (`aquisicao/scripts/compilar_dev.sh`) | sem avisos; 15 + 4 + 1 + 7 + 4 + 3 testes passam |
| Testes completos (`UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1`) | 9 passam; os 3 novos `*_pre_eleicao` rodam de fato (os `candidatos.json` existem; não foram pulados) |
| Votável desconhecido → `OFF_VNT` | correto: no 1º turno de 2026 com o catálogo dos `-u.json`, os votos no nº 28 (Presidente) e outros entram em `vnt` e as 83 abrangências batem; alertas registrados (`lista_alertas_vnt`) |
| Desempenho com `OFF_VNT` (2026, catálogo de candidatos) | 1,9 µs por BU numa thread, 0,115 s com 28 threads — sem perda |
| Eleitorado ≥ Σ aptos dos BUs, por circunscrição | **verdadeiro** nos três conjuntos (55, 5.574 e 51 circunscrições; margem mínima 0). O relatório afirmava sem evidência; agora está medido |
| Rede | sondagem a 2 req/s, parada em 429 (`.git/uv_tmp_sonda.py`); nenhum BU baixado |
| `ele-c.json` do 3221 | 11 eleições conferem (inclui 6293 Santana do Itararé e a consulta 6287/PR, novas desde o handoff) |
| `-u.json` pré-apuração do 3221 | existem (`tf = n`, `vap = 0`, nascimento presente) — mas **sem `dvt`** (ver 3.4) |

## 2. Problemas de processo

* **`v0.1.4` (`b3445c2`) não foi publicada** (`main` está 1 commit à frente de `origin/main`) e o trabalho da `0.1.5` está
  **sem commit** (VERSION já em 0.1.5; `docs/relatorios/RELATORIO_0.2.0.md` e `motor/tests/fixtures/candidatos_exemplo.json` não
  rastreados). Regra do CLAUDE.md: cada commit com bump, tag e `git push --follow-tags`.
* `uv-coletor preparar` **não baixa as fontes**: só o `ele-c.json`. `cs.json`, `cm.json` e `-u.json` foram baixados à mão
  por scripts temporários e só são lidos de `--fontes`. Na noite isso não é reproduzível.
* O ROADMAP marca os itens da etapa 2 como ✅ antes da validação.

## 3. Defeitos (correção obrigatória)

### 3.1 Presidente com 29 "circunscrições" (matematicamente eleito por UF)
`preparar` grava, para cada cargo, **todos** os `-u.json` da eleição como circunscrições. Para o Presidente isso dá 29
(br + 27 UFs + zz) em vez de 1 (Brasil), nos `candidatos.json` de 2026 1º turno e do 3221. Consequência medida com
`uv-motor noite --candidatos /home/dev/uvreal/preparo_2026_1t/candidatos.json` na noite real de 2026: **64 eventos de
Presidente**, entre eles "matematicamente_eleito" no AC, AL, BA… (deveriam ser 3: os dois candidatos no 2º turno e a
circunscrição Brasil); `nao_confere: 23`. Na noite de 25/10 o WebSocket anunciaria "eleito no AC".
**Correção:** só a abrangência que é a própria circunscrição vira circunscrição (`candidatos::circunscricao_de(cargo, abr) == abr`);
o eleitorado das abrangências menores vai num campo à parte (o `Candidatos` já tem `eleitorado_abr`). Teste: o
`candidatos.json` do 3221 tem exatamente 1 circunscrição para o cargo 1, e o teste de propriedade do "matematicamente"
(`motor/tests/matematica.rs`) roda também com o `candidatos.json` pré-eleição (nenhuma revogação, nenhuma declaração
diferente do oficial).

### 3.2 Pleito 3221 sem Sergipe e Tocantins
Os `cs.json` de SE e TO respondem 404 (conferido de novo em 12/10, 2 requisições), e o `preparar` monta municípios e
seções **só dos `cs.json`**: o `candidatos.json` do 3221 tem 26 UFs e 5.543 municípios, mas o `mun-e006258-cm.json`
(Presidente) tem **28 UFs e 5.757 municípios**, inclusive SE e TO. A explicação do relatório ("SE e TO não têm eleição")
está errada: há 2º turno de Presidente no Brasil inteiro. Na noite, todos os BUs de SE e TO cairiam em "fora do catálogo".
**Correção:** municípios das eleições a partir dos `cm.json` (o relatório diz que os usa, mas o código não lê `cm.json`);
`cs.json` ausente de uma UF que tem eleição é **erro** (ou aviso explícito com a lista de UFs, e `preparar` sai com código
≠ 0), e o comando precisa ser re-executado até ficar completo. Teste: cada UF/município do `cm.json` de cada eleição está
no catálogo.

### 3.3 Consultas publicadas no lugar errado (683 arquivos)
As consultas do 3221 (6287/PR cargo 25, 6288 e 6289/MT cargos 26–27) têm resultado oficial **por UF**
(`mt-c0026-e006288-u.json`), mas o catálogo dá região ao cargo em todos os municípios da UF e o `Publicador` publica os
cargos ≥ 25 por município: **683 dos 721 JSON** da totalização a frio são consultas por município (ex.: `pr79537-c0025-e006287.json`),
e **não há** `mt-c0026-e006288.json` nem `pr-c0025-e006287.json`. **Correção:** as abrangências publicadas saem das
circunscrições dos candidatos (circunscrição + abrangências menores pedidas), e as regiões do cargo só nos municípios do
`cm.json` da eleição (6288 tem 2 municípios). Teste: a publicação a frio do 3221 tem exatamente as abrangências dos
`-u.json` oficiais pré-apuração (mesmos nomes de arquivo, sem o `-u`).

### 3.4 Situação com zero voto e destinação presumida
* Publicação a frio do 3221 (`br-c0001-e006258.json`): `"estado": "apurando"`, mas **`st` = "Eleito" para o nº 13 com 0
  votos** (desempate por idade). Com `estado` "apurando", todo `st` tem de ser vazio (docs/arquitetura/FORMATO_RESULTADO.md).
* Os `-u.json` pré-apuração do 3221 **não trazem `dvt`**; o `preparar` presume "Válido". É razoável, mas precisa estar
  documentado em `docs/arquitetura/FORMATO_CANDIDATOS.md` e no relatório como premissa, porque muda o resultado: simulei a mesma
  premissa nos dados reais (destinação desconhecida → todos "Válido") e o **1º turno de 2024 passa a divergir em 32
  municípios (33 situações)** — candidatos anulados ou sub judice depois do registro. O 2º turno de 2024 e o 1º turno
  de 2026 não mudam.

### 3.5 O critério da etapa não foi testado de verdade
Os `candidatos.json` "pré-eleição" de 2024 e de 2026 1º turno foram montados com os **`-u.json` oficiais apurados**
(os 83 arquivos em `/home/dev/uvreal/preparo_2026_1t` são idênticos byte a byte aos de `/home/dev/uvreal/oficial`), com
`dvt` e eleitorado pós-eleição. Por isso "nenhuma diferença" no relatório é tautológico. Isso é aceitável para votos,
números, nascimento e eleitorado (não dependem de votos), **desde que**: (a) a destinação seja descartada na
montagem desses conjuntos (premissa "Válido", como no 3221), (b) os testes `*_pre_eleicao` listem nominalmente as
divergências de situação esperadas (as 33 de 2024 1º turno) em vez de exigir 100 %, e (c) o relatório diga isso com
clareza.

## 4. Pendências menores
* `ARCHITECTURE.md` §3.1: a tabela do layout não tem o slot `OFF_VNT` (EXTRA = 105).
* `Catalogo::de_arquivo_candidatos`: `m.uf.as_bytes()[0..1]` entra em pânico com UF de tamanho ≠ 2 — validar e devolver erro.
* `Candidatos::do_arquivo` marca toda circunscrição como `finalizada: true`; documentar.
* O contador `desconhecidos` passou a contar alertas de nulo técnico (1.165.919 em 2026): separe ou documente.

## 5. Para aprovar (o que vou rodar de novo)
1. `git log origin/main` com as versões publicadas e `scripts/versao.sh verificar`.
2. `compilar_dev.sh` e os testes normais e `--ignored` (incluindo `matematica.rs` com `candidatos.json`).
3. `uv-coletor preparar --pleito 3221 …` regerado **pelo próprio comando** (baixando ou usando cache), com 28 UFs (ou erro
   explícito enquanto SE/TO não estiverem publicados), Presidente com 1 circunscrição, consultas por UF.
4. `uv-motor totalizar --candidatos … --saida …` a frio: nomes dos arquivos = abrangências dos `-u.json` oficiais;
   nenhum `st` preenchido.
5. `uv-motor noite --candidatos /home/dev/uvreal/preparo_2026_1t/candidatos.json …` na noite de 2026: só 3 eventos de
   Presidente, 0 revogações, `nao_confere` 0.
6. Testes `*_pre_eleicao` com destinação "Válido" e as divergências de 2024 1º turno listadas.

## 6. Revalidação (v0.1.7 → 0.2.0)

Feita sobre o commit `6b9c57a` (`v0.1.7`), com o binário de `~/uvreal/target-dev`.

| # | item da seção 5 | resultado |
|---|---|---|
| 1 | versões publicadas | `v0.1.4` a `v0.1.7` em `origin/main`; `scripts/versao.sh verificar`: coerente |
| 2 | `compilar_dev.sh`, testes normais e `--ignored` | sem avisos; 15 + 4 + 1 + 7 + 4 + 3 normais; **13 `--ignored`** (7 `integracao`, 6 `matematica`, incluindo `matematica::*_candidatos` com os três `candidatos.json`: 0 revogações; 2026 e 2024 2º turno sem nenhum problema) |
| 3 | `uv-coletor preparar --pleito 3221` pelo próprio comando | 28 UFs, 5.757 municípios, 494.044 seções, 11 eleições, Presidente com 1 circunscrição, consultas 6287/PR e 6288–6289/MT por UF; 3 requisições; **código 2** com `incompleto` = cs.json de SE e TO (404) — erro explícito, como pedido |
| 4 | `uv-motor totalizar --candidatos` a frio | 3221: 43 arquivos, nomes idênticos aos 43 `-u.json` pré-apuração; 43 × "apurando", nenhum `st`. 3220 com os BUs: 83 nomes idênticos aos oficiais |
| 5 | `uv-motor noite --candidatos /home/dev/uvreal/preparo_2026_1t/candidatos.json` | 3 eventos de Presidente (`br`: 2 × "2t", 1 × "matematicamente_2t"), 0 revogações, `nao_confere` 0, 55/55 decididas |
| 6 | `*_pre_eleicao` com destinação "Válido" | `candidatos.json` regerados pelo `preparar --ignorar-destinacao`; 2026: 83/83, 832/832; 2024 2º turno: 51/51, 102/102; 2024 1º turno: votos idênticos em 5.574 arquivos e **32 municípios / 33 situações** divergentes, todos com candidato não apto no oficial (constantes `ARQ_DESTINACAO_2024`/`SIT_DESTINACAO_2024` no teste) |

Pendências menores (§4): resolvidas (ARCHITECTURE §3.1, erro em UF inválida, documentação de `finalizada` e do
contador de votáveis desconhecidos).

**Fica para antes da noite de 25/10:** regerar o `candidatos.json` do 3221 até o `preparar` sair com código 0 (SE e TO);
conferir destinações perto da eleição (os `-u.json` pré-apuração não as trazem).
