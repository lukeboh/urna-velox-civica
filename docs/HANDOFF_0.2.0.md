# Handoff — Urna Velox: Etapa 2, catálogo e candidatos pré-eleição (`0.2.0`, alvo 16/10/2026)

*Preparado em 12/10/2026, publicado na `v0.1.1` e revisto na `v0.1.2` ("cadastro" passou a se chamar **candidatos**, para não confundir com o cadastro de eleitores), sobre a `v0.1.0` (etapa 1 concluída: tag `v0.1.0`, `origin/main` em `1dc1cc8`). Para iniciar a sessão, cole este documento ou peça para lê-lo. **Quem executa este handoff para na `0.1.x` e entrega um relatório; o `bump minor` para `0.2.0` só acontece depois da validação independente descrita no fim.***

**Caminhos:** todos absolutos. Repositório = `D:\git\lukeboh\urna-velox` no Windows (Git, PowerShell) = `/mnt/d/git/lukeboh/urna-velox` no WSL (scripts, cargo); dados = `/home/dev/uvreal` (WSL); acervo de dados abertos = `/mnt/d/dados/tse-arquivos-urna` (= `D:\dados\tse-arquivos-urna`). Este documento: `/mnt/d/git/lukeboh/urna-velox/docs/HANDOFF_0.2.0.md`.

## Contexto
O Urna Velox é um motor de totalização eleitoral em Rust (repo `lukeboh/urna-velox`, branch `main`). Na `v0.1.0` o motor (`/mnt/d/git/lukeboh/urna-velox/motor/`, crate `urna-velox`, binário **`uv-motor`**) totaliza os BUs, calcula a situação dos candidatos e o **matematicamente eleito / matematicamente 2º turno**, publica JSON (`/mnt/d/git/lukeboh/urna-velox/docs/FORMATO_RESULTADO.md`) e reproduz em teste as conferências oficiais (2026 83/83, 2024 5.574/5.574 e 51/51).

**O problema desta etapa:** hoje o motor só funciona **depois** da eleição, porque dois insumos vêm de dados posteriores à apuração:
1. **Catálogo** (`/mnt/d/git/lukeboh/urna-velox/motor/src/catalogo.rs`): os **candidatos** são lidos dos próprios BUs (`Catalogo::dos_pacotes`, `candidatos_dos_bus`; `uv-coletor catalogo`). Um número votado que não é candidato vira "candidato" do catálogo — é assim que os nulos técnicos (`vnt`) aparecem hoje.
2. **Candidatos** (`/mnt/d/git/lukeboh/urna-velox/motor/src/candidatos.rs`): destinação (`dvt`), nascimento, eleitorado, turno, vagas e circunscrições vêm dos arquivos oficiais de resultado já apurados (`Candidatos::dos_oficiais`, a partir dos `-u.json` em `/home/dev/uvreal/oficial`, `/home/dev/uvreal/2024-1t/oficial`, `/home/dev/uvreal/2024-2t/oficial`).

**Objetivo:** montar catálogo e candidatos **só com dados disponíveis antes da apuração**, guardá-los em arquivos locais versionáveis, e provar que o motor, alimentado por eles, reproduz as mesmas conferências da etapa 1. E deixar pronto o catálogo + candidatos do **pleito 3221 (2º turno de 2026, 25/10/2026)**, que é para onde tudo isto vai.

## Leia antes de começar (nesta ordem)
1. `/mnt/d/git/lukeboh/urna-velox/CLAUDE.md` — regras permanentes (versionamento, rede, verificação).
2. `/mnt/d/git/lukeboh/urna-velox/ROADMAP.md` — seção "Etapa 2" (e a etapa 1, já concluída).
3. `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` — entradas 0.0.23 a 0.1.0 (o que o motor faz e as decisões tomadas).
4. `/mnt/d/git/lukeboh/urna-velox/ARCHITECTURE.md` §2.5 (o motor) e `/mnt/d/git/lukeboh/urna-velox/docs/AQUISICAO.md` §1 (endereços da API de resultados).
5. Código: `/mnt/d/git/lukeboh/urna-velox/motor/src/catalogo.rs`, `candidatos.rs`, `estado.rs` (Agregador: o que acontece com votável desconhecido), `oficial.rs`, `conferencia.rs`, `main.rs`; `/mnt/d/git/lukeboh/urna-velox/aquisicao/src/tse.rs` (URLs, `ele-c.json`, `cs.json`) e `aquisicao/src/main.rs` (subcomandos `catalogo`, `secoes`, `eleicoes`).
6. Testes: `/mnt/d/git/lukeboh/urna-velox/motor/tests/` (`comum/mod.rs`, `integracao.rs`, `matematica.rs`, `municipais.rs`, `publicacao.rs`) e `/mnt/d/git/lukeboh/urna-velox/motor/tests/fixtures/README.md`.

## Regras inegociáveis
- **Rede (TSE):** no máximo **100 req/s** somando tudo; nesta etapa use **≤ 5 req/s** para sondagens e ≤ 20 req/s para baixar listas; depois de **qualquer HTTP 429**, pare tudo por ≥ 10 min (o IP de saída pode ser compartilhado); nada de vários IPs, proxies ou processos em paralelo para contornar. Nunca baixe de novo o que já está em `/home/dev/uvreal` ou no acervo. **Não baixe BUs** nesta etapa. Orçamento total da etapa: alguns milhares de requisições; se uma ideia exigir mais, pare e pergunte ao Luciano.
- Downloads de **dados abertos** (CKAN/CDN, ex.: `consulta_cand`, eleitorado por seção) vão para `/mnt/d/dados/tse-arquivos-urna/candidatos/<ano>/` com uma linha no `manifesto.csv` do acervo (URL, ETag, SHA-256, data), uma requisição por arquivo.
- **Separação:** o **motor não faz rede**. Quem baixa é o `uv-coletor` (`/mnt/d/git/lukeboh/urna-velox/aquisicao/`); o motor lê arquivos locais.
- **Todos os executáveis começam com `uv-`** (`uv-motor`, `uv-coletor`); um executável novo segue a regra.
- **Sem regressão:** antes de cada commit rode `cargo test --release -p urna-velox` e `UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1` (6 testes, ~15 s) e registre o resultado no CHANGELOG. Os testes existentes continuam passando sem mudança de expectativa; se um número mudar, explique por quê no CHANGELOG antes de mudar o teste.
- **Versionamento a cada commit:** `bash /mnt/d/git/lukeboh/urna-velox/scripts/versao.sh bump patch` (sem resumo na linha de comando), preencher a entrada do `/mnt/d/git/lukeboh/urna-velox/CHANGELOG.md` (o hook rejeita `PREENCHER`), `git add` (nunca `aquisicao/acervo/`, que é do Luciano), `git commit -F D:\git\lukeboh\urna-velox\.git\UV_COMMIT_MSG`, `git push --follow-tags`. Commits `0.1.2`, `0.1.3`, … **Não faça `bump minor`.** Identidade git `Luciano Bohnert <luciano.bohnert@gmail.com>`. Mensagens `área: resumo`, em português.
- Termos: "**matematicamente eleito**" / "matematicamente 2º turno".
- Handoffs, relatórios e documentos novos com **caminhos absolutos**.

## Ambiente
- Windows + WSL2 `Ubuntu-24.04`, usuário `dev`, i7-14700 (28 threads), 15 GB no WSL. Comandos pelo Desktop Commander (`start_process`, PowerShell). **Aspas quebram no PowerShell:** escreva o comando num script `/mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh` (ou `.py`) e rode `wsl -d Ubuntu-24.04 -- bash /mnt/d/git/lukeboh/urna-velox/.git/uv_tmp_<nome>.sh`. Cada chamada expira em ~60 s; para jobs longos, `wsl -d Ubuntu-24.04 -- bash -c "bash <script> > <log> 2>&1"` em segundo plano e acompanhe o log. Para editar arquivos com acentos, prefira scripts Python com `encoding="utf-8"` ou o `edit_block`.
- Compilar e testar o workspace: `bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh` (alvo `/home/dev/uvreal/target-dev`; binários `/home/dev/uvreal/target-dev/release/uv-motor` e `uv-coletor`). Cobertura: `bash /mnt/d/git/lukeboh/urna-velox/scripts/cobertura.sh`.
- Git pelo PowerShell (`cd D:\git\lukeboh\urna-velox; git …`).

### O que já existe localmente (não baixe de novo)
| conjunto | o quê | onde |
|---|---|---|
| 2026, 1º turno (pleito 3220; eleições 6257 Presidente, 6259 Governador/Senador/Deputados) | BUs `.uvpk`, `cs.json` das UFs, oficiais `-u.json` (cargos 1/3/5), métricas da coleta, `catalogo.txt` | `/home/dev/uvreal/dados`, `/home/dev/uvreal/config` e `/home/dev/uvreal/dados/config`, `/home/dev/uvreal/oficial`, `/home/dev/uvreal/metricas_*.csv`, `/home/dev/uvreal/resultado/catalogo.txt` |
| 2024, 1º turno (pleito 452; 619 Prefeito/Vereador; consultas 633–637 = cargos 51–55) | BUs, oficiais por município, `cm.json` das eleições, métricas dos `aux.json` | `/home/dev/uvreal/2024-1t/{dados,oficial,metricas_aux.csv}`, `/home/dev/uvreal/config/mun-e000619-cm.json` (e 633–637), `/home/dev/uvreal/2024-1t/descoberta/` (amostras de endereços, inclusive `ele-c.json` de 10/10/2026) |
| 2024, 2º turno (pleito 453; 620 Prefeito, 51 municípios) | BUs, oficiais, métricas, `catalogo.txt` | `/home/dev/uvreal/2024-2t/{dados,oficial,metricas_2t.csv,resultado/catalogo.txt}` |
| acervo de dados abertos | zips originais de BUs 2022/2024/2026 | `/mnt/d/dados/tse-arquivos-urna/{originais,bus}` |

O `ele-c.json` (`/home/dev/uvreal/2024-1t/descoberta/ele-c.json`) já mostra o **pleito 3221 (25/10/2026)**: 6258 Presidente 2º turno (verifique), **6260 Governador 2º turno em AM, RN, DF, ES, AC**, suplementares de Prefeito (6282 Macedônia, 6283 Martinópolis, 6284 Narandiba, 6285 Itaguaí, 6290 Arroio do Sal) e consultas de MT (6288 cargo 26, 6289 cargo 27). O campo `arq` lista os tipos de arquivo da divulgação: `ft` (fotos), `cm` (config/municípios), `e`, `cs`, `ab`, `u` e `aux`, com os diretórios-modelo (`<base>/<ambiente>/<ciclo>/<cd_eleicao>/dados/<uf>` etc.).

## O que fazer

### 1. Descobrir as fontes pré-apuração (sondagem, relatório antes de codar)
Responda com evidência (URL, tamanho, campos, data `dg`/`hg`), num relatório `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/FONTES_PRE_APURACAO.md`:
- **Candidatos por circunscrição** com número, nome, **destinação** (`dvt`) e **nascimento**. **O TSE já gerou os arquivos de candidatos do 2º turno de 2026 (pleito 3221)** (informação do Luciano, 12/10): eles são a fonte principal e o caso de teste real desta etapa — descubra quais são (endereço, tipo no `ele-c.json`, campos), baixe-os uma vez para `/home/dev/uvreal/2026-2t/preparo/fontes/` e guarde o formato no relatório. Verifique também se o mesmo tipo de arquivo existe para 2024 e 2026 1º turno; hipótese: os `-u.json` de cada abrangência são publicados **antes** da apuração (com `vap` 0) — confira no pleito 3221 (ex.: `df-c0003-e006260-u.json`, `br-c0001-e006258-u.json`, um município de suplementar, uma consulta de MT). Hipótese alternativa: tipos `e`/`ab` do `ele-c.json`. Fonte de reserva: dados abertos `consulta_cand_<ano>.zip` (CSV com número, situação da candidatura, data de nascimento) — se usar, documente o mapeamento situação da candidatura → destinação e onde ele diverge do `dvt` oficial.
- **Eleitorado** por circunscrição e, se possível, **por seção** (aptos por seção permitem `R` = aptos das seções ainda não apuradas, que é exato e seguro). Candidatos: `e.te` dos `-u.json` pré-apuração, tipo `e`, dados abertos de eleitorado por seção/local de votação. **Atenção:** para o "matematicamente" ser seguro, o eleitorado usado precisa ser **≥ Σ aptos dos BUs** da circunscrição (senão `R` subestima o que falta e uma declaração pode ser revogada). Meça isso em 2024 e 2026.
- **Seções e municípios:** `cs.json` por UF do pleito (já usado) e `mun-e<eleição>-cm.json`; quais municípios e UFs têm cada eleição (no 3221, Governador só em 5 UFs; suplementares em municípios avulsos).
- **Vagas** (Senador 2 em 2026; 1 no resto) e **turno** por eleição (`t` no `ele-c.json`).
- Para 2024 e 2026 1º turno, a sondagem só pode usar **fontes que por natureza são pré-apuração** (o que é publicado antes do primeiro BU). Os `-u.json` desses pleitos hoje já estão apurados: deles só vale tirar o que não depende de votos (lista, números, nascimento, eleitorado), e a destinação de hoje pode ter mudado depois da eleição — registre as diferenças, não as esconda.

### 2. Formato e carregamento dos candidatos + catálogo pré-eleição
- Um arquivo local por pleito (proposta: `candidatos.json`, documentado em `/mnt/d/git/lukeboh/urna-velox/docs/FORMATO_CANDIDATOS.md`) com eleições, turno, cargos, vagas, circunscrições (abrangência, eleitorado, aptos por seção se houver, candidatos com número/destinação/nascimento/nome), municípios e seções. O motor ganha `Candidatos::ler(caminho)` e um construtor de `Catalogo` a partir dos candidatos + seções (sem ler BUs). `Candidatos::dos_oficiais` continua existindo (é o oráculo dos testes).
- `uv-coletor` ganha um subcomando que monta esse arquivo a partir das fontes do passo 1 (com cache local e respeitando os limites de rede), ex.: `uv-coletor preparar --pleito 3221 --saida /home/dev/uvreal/2026-2t/preparo`.
- `uv-motor totalizar | conferir | noite` aceitam `--candidatos <arquivo>` no lugar de `--oficial` para os candidatos (o `--oficial` continua sendo o oráculo da conferência).

### 3. Votável desconhecido vira alerta, e nulo técnico deixa de depender do catálogo dos BUs
Com o catálogo montado pelos candidatos, um número votado que não é candidato **não pode** sumir: hoje o `Agregador` só incrementa `desconhecidos` e descarta o voto. Faça esses votos contarem como **nulos técnicos** na região (ex.: um slot `OFF_VNT` no layout de `/mnt/d/git/lukeboh/urna-velox/motor/src/catalogo.rs`, ou equivalente), publique `vnt` a partir dele e registre o alerta (número, cargo, abrangência, quantidade) num relatório/contador consultável. Mudança de layout muda o checksum: documente no CHANGELOG; os totais publicados e as conferências não podem mudar.

### 4. Testes e conferência com os candidatos pré-eleição
- Gere os candidatos pré-eleição de **2024 2º turno (453)**, **2024 1º turno (452)** e **2026 1º turno (3220)** e compare-os campo a campo com `Candidatos::dos_oficiais` (números, destinação, nascimento, eleitorado, vagas, turno, circunscrições): um relatório com as diferenças e a causa de cada uma.
- Testes `--ignored` novos (em `/mnt/d/git/lukeboh/urna-velox/motor/tests/`): totalizar com **catálogo e candidatos pré-eleição** e conferir com os oficiais — votos (`vap`, `vb`, `vn`, `vnt`, `e.c`, `s.st`) **idênticos** nos três conjuntos (83/83, 5.574/5.574, 51/51); situação idêntica, exceto em candidatos cuja destinação mudou depois da eleição, listados nominalmente no teste com a justificativa; teste de propriedade do "matematicamente" (nenhuma revogação, nenhuma declaração diferente do oficial) com os candidatos pré-eleição.
- Fixtures pequenas (o total de `/mnt/d/git/lukeboh/urna-velox/motor/tests/fixtures` deve continuar ≤ 5 MB) com um `candidatos.json` de exemplo, e testes de unidade do carregador, do alerta de votável desconhecido e do `vnt`.

### 5. Pleito 3221 (o 2º turno de 2026) — o teste real
Os arquivos de candidatos do 3221 já estão publicados: use-os desde o início (passos 1 a 3), não só no fim. Monte os candidatos + catálogo do pleito 3221 com `uv-coletor preparar` e guarde em `/home/dev/uvreal/2026-2t/preparo/` (fora do repositório). Sanidade: Presidente com 2 candidatos aptos e circunscrição Brasil; Governador nas 5 UFs, 2 candidatos cada; suplementares e consultas com suas abrangências; nº de seções por UF; eleitorado; `uv-motor totalizar --candidatos …` rodando sem BUs (tudo zero, `estado` "apurando"). Registre os números no relatório.

## Entregáveis
- Código (motor + `uv-coletor`), `/mnt/d/git/lukeboh/urna-velox/docs/FORMATO_CANDIDATOS.md`, `/mnt/d/git/lukeboh/urna-velox/aquisicao/results/FONTES_PRE_APURACAO.md`, testes e fixtures; CHANGELOG preenchido a cada commit; ROADMAP com os itens da etapa 2 marcados com a versão (sem marcar a etapa como concluída).
- **Relatório final** `/mnt/d/git/lukeboh/urna-velox/docs/RELATORIO_0.2.0.md`: o que foi feito, fontes usadas, diferenças candidatos pré-eleição × oficial (com causa), resultados de todos os testes (com os comandos exatos), riscos para a noite de 25/10, e o que ficou pendente. Termine com a lista de commits/tags produzidos.

## Fora do escopo
Serviço automatizado (etapa 3), HTTP/WebSocket (etapa 4), site (etapa 5), proporcionais (2.0), baixar BUs, verificação do `.vsc`.

## Armadilhas conhecidas
- `cs.json` de 2024 foi republicado em 29/09/2026 (datas de disponibilização não são as da noite).
- Hora dos oficiais (`dt`/`ht`) na hora local da UF; `aux.json`/`cs.json` em Brasília.
- Cargos municipais (Prefeito, consultas 25–99): o mesmo número é um candidato diferente em cada município (`cargo_municipal`).
- Exterior (`zz`) só vota para Presidente; 41 seções não instaladas no 1º turno de 2026 (sem BU, contam em `s.st`).
- Destinações vistas: "Válido", "Anulado", "Anulado sub judice" (há anulações posteriores à eleição em 2024).
- Senador 2026 tem 2 vagas (cada eleitor vota duas vezes); consultas usam `perg[0].resp` em vez de `carg`.
- Uma seção anulada pela Junta (MA 07315/0023/0354, 2024) só é conhecida pelo `aux.json` ("Excluído") — continua vindo das métricas.

## Validação (será feita por outra sessão, independente)
Quem validar vai, sem confiar no relatório: ler o diff desde `v0.1.0`; rodar `bash /mnt/d/git/lukeboh/urna-velox/aquisicao/scripts/compilar_dev.sh`, os testes normais e os `--ignored`; regenerar os candidatos pré-eleição de 2024 2º turno com o `uv-coletor` (se precisar de rede, dentro dos limites) e conferir com `uv-motor conferir --candidatos … --oficial …`; comparar os candidatos pré-eleição com o dos oficiais; checar a regra eleitorado ≥ Σ aptos; inspecionar os candidatos do pleito 3221 e conferi-los com os arquivos publicados pelo TSE; conferir versionamento (`scripts/versao.sh verificar`, tags, CHANGELOG) e a regra `uv-`. Só então sobe `bump minor` → `0.2.0`.
