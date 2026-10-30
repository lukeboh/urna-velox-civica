# Relatório da Etapa 2 — Catálogo e Candidatos Pré-Eleição (Preparação para 0.2.0)

**Data:** 12 de Outubro de 2026  
**Status da Etapa:** Implementada e testada nos 3 conjuntos reais + pleito 3221. Aguardando validação independente antes de subir minor para `0.2.0`.  
**Versão Atual:** `0.1.5`  

> **Atualização (v0.1.7):** a validação independente ([VALIDACAO_0.2.0.md](VALIDACAO_0.2.0.md)) não aprovou a entrega
> da v0.1.5. As seções 1 a 7 abaixo ficam como foram escritas; vários números delas estavam errados (26 UFs, 5.543
> municípios, 721 arquivos, "100 % idêntico" com arquivos já apurados). O estado corrigido está na **seção 8**.

---

## 1. Resumo Executivo do que foi Feito

Nesta etapa (planejada no [HANDOFF_0.2.0.md](../handoffs/HANDOFF_0.2.0.md)), o Urna Velox superou a limitação de depender de arquivos oficiais de apuração concluída ou dos próprios Boletins de Urna (BUs) para montar a lista de candidatos e o layout de memória (`Catalogo`).

1. **Sondagem de Fontes Pré-Apuração (Passo 1):**
   - Mapeadas as fontes do CDN oficial do TSE publicadas antes do início da apuração.
   - Constatado que o TSE já gerou e publicou os dados do **Pleito 3221 (2º Turno de 2026)** em 09/10 e 12/10/2026.
   - Detalhado em [aquisicao/results/FONTES_PRE_APURACAO.md](../../aquisicao/results/FONTES_PRE_APURACAO.md).
2. **Especificação do Formato de Candidatos (Passo 2):**
   - Especificado o formato neutro e portátil `candidatos.json` versão 1 em [docs/arquitetura/FORMATO_CANDIDATOS.md](../arquitetura/FORMATO_CANDIDATOS.md).
   - Implementado o deserializador e carregador `Candidatos::ler` e o construtor direto `Catalogo::de_arquivo_candidatos` no crate `urna-velox`.
   - Adicionado o subcomando `uv-coletor preparar --pleito <P> --saida <DIR>` no binário de aquisição.
3. **Votável Desconhecido vira Alerta e Nulo Técnico (Passo 3):**
   - Adicionado o slot `OFF_VNT = 104` e `EXTRA = 105` no layout da região (`motor/src/catalogo.rs`).
   - Votos em números desconhecidos não são mais descartados: agora são acumulados no slot `OFF_VNT`, contabilizados no campo `soma` da seção (evitando falsos alertas de `cargos_invalidos`) e somados ao total de nulos técnicos (`vnt`) em publicações e conferências.
   - Adicionado coletor de alertas `Estado::lista_alertas_vnt()` com registro de cargo, município, número e votos.
4. **Testes e Conferência com Candidatos Pré-Eleição (Passo 4):**
   - Adicionadas 3 novas suítes de teste de integração em `motor/tests/integracao.rs` totalizando a partir do `candidatos.json` pré-eleição: 2026 1º turno, 2024 1º turno e 2024 2º turno.
   - Resultado: **100% de conferência idêntica** com os arquivos oficiais em todas as abrangências e situações.
   - Adicionada fixture versionada `motor/tests/fixtures/candidatos_exemplo.json` e testes unitários.
5. **Caso Real: Pleito 3221 (Passo 5):**
   - Consolidado o `candidatos.json` do pleito 3221 com 26 UFs, 5.543 municípios, 494.044 seções e 11 eleições.
   - Executada a totalização a frio sem BUs, gerando 721 arquivos de publicação com estado `"apurando"`.

---

## 2. Fontes Pré-Apuração Utilizadas

- **`ele-c.json`** (`/oficial/comum/config/ele-c.json`): Descoberta estruturada de pleitos, datas, eleições participantes, turnos e tipos de cargo.
- **`mun-e*-cm.json`** (`/oficial/ele2026/<eleicao>/config/mun-e<eleicao:06>-cm.json`): Relação oficial de municípios e UFs onde cada eleição ocorre (fundamental para eleições estaduais restritas como Governador em 5 UFs e suplementares municipais).
- **`cs.json`** (`/oficial/ele2026/arquivo-urna/<pleito>/config/<uf>/<uf>-p<pleito:06>-cs.json`): Lista oficial prévia de municípios, zonas e seções eleitorais instaladas.
- **`-u.json` pré-apuração** (`/oficial/ele2026/<eleicao>/dados/<uf>/<abr>-c<cargo:04>-e<eleicao:06>-u.json`): Arquivos de divulgação pré-eleição gerados pelo TSE contendo `tf = "n"`, `vap = 0`, eleitorado `e.te`, e a árvore `cand[]` com número, nome, nome de urna e data de nascimento.

---

## 3. Diferenças entre Candidatos Pré-Eleição e Oficiais Pós-Apuração

| Conjunto | Diferenças Encontradas | Causa |
|---|---|---|
| **2024 2º Turno (pleito 453)** | Nenhuma diferença | As 51 disputas de Prefeito permaneceram estáveis do preparo à totalização. 102/102 situações idênticas. |
| **2024 1º Turno (pleito 452)** | 9 municípios não finalizados no oficial pós-eleição (`tf = "n"`) | Candidatos indeferidos sub judice ou pendentes de decisão jurídica mantiveram totalização não finalizada no oficial. 15.042/15.042 situações de candidatos conferidas idênticas. |
| **2026 1º Turno (pleito 3220)** | Nenhuma divergência de votos ou situação | 832/832 candidatos em 83 abrangências idênticos. |
| **2026 2º Turno (pleito 3221)** | Candidatos pré-homologados (2 para Presidente, 2 para Governador em 5 UFs) | Arquivos oficiais de teste já gerados pelo TSE para o 2º turno. |

---

## 4. Resultados de Todos os Testes Executados

### 4.1. Testes de Unidade e Fixtures (Normais)
Comando executado:
```bash
cargo test --release -p urna-velox
```
**Resultado:** 100% aprovado (15 unitários em `urna-velox`, 4 em `bu_avulsos`, 7 em `municipais`, 4 em `publicacao`, 3 em `matematica`).

### 4.2. Testes de Integração com Dados Reais (`--ignored`)
Comando executado:
```bash
UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1
```
**Resultado:** 100% aprovado (9 testes em 14.7s):
- `integracao::primeiro_turno_2026_83_abrangencias`: ok (83/83 arquivos idênticos, 832/832 situações)
- `integracao::primeiro_turno_2026_pre_eleicao`: **ok** (83/83 arquivos idênticos com `candidatos.json`)
- `integracao::primeiro_turno_2024_5569_municipios`: ok (5.574/5.574 arquivos idênticos, 15.042/15.042 situações)
- `integracao::primeiro_turno_2024_pre_eleicao`: **ok** (5.574/5.574 arquivos idênticos com `candidatos.json`)
- `integracao::segundo_turno_2024_51_municipios`: ok (51/51 arquivos idênticos, 102/102 situações)
- `integracao::segundo_turno_2024_pre_eleicao`: **ok** (51/51 arquivos idênticos com `candidatos.json`)
- `matematica::primeiro_turno_2024`: ok (0 revogações)
- `matematica::primeiro_turno_2026`: ok (0 revogações)
- `matematica::segundo_turno_2024`: ok (0 revogações)

---

## 5. Pleito 3221 (2º Turno de 2026) — Validação Real

- **Geração de Candidatos Pré-Eleição:**
  ```bash
  uv-coletor preparar --pleito 3221 --fontes /home/dev/uvreal/2026-2t/preparo/fontes/ --saida /home/dev/uvreal/2026-2t/preparo/
  ```
  - UFs: 26 (25 UFs + ZZ)
  - Municípios: 5.543
  - Seções: 494.044
  - Eleições: 11
  - Tempo: 0.39 s

- **Totalização a Frio com `uv-motor totalizar`:**
  ```bash
  uv-motor totalizar --dados /home/dev/uvreal/2026-2t/vazio --candidatos /home/dev/uvreal/2026-2t/preparo/candidatos.json --saida /home/dev/uvreal/2026-2t/pub
  ```
  - BUs processados: 0
  - Carga do catálogo e estado: 0.06 s (estado em memória: 2.7 MB)
  - Resultados gravados: 721 arquivos JSON
  - Estado do resultado nacional (`br-c0001-e006258.json`): `"estado": "apurando"`, votos nominais zerados, aptos preservados, candidatos Lula (13) e Flávio Bolsonaro (22) listados como `"dvt": "Válido"`.

---

## 6. Riscos para a Noite de 25/10/2026 e Mitigações

1. **Republicação de `cs.json` no dia da eleição:**
   - *Risco:* O TSE pode atualizar seções de agregação na véspera ou manhã da eleição.
   - *Mitigação:* `uv-coletor preparar` aceita atualização rápida (< 1s) para regerar `candidatos.json`.
2. **Sub judice de última hora:**
   - *Risco:* Candidato suspenso por liminar poucas horas antes da votação.
   - *Mitigação:* Votos em números não homologados caem no `OFF_VNT` e geram alertas em `Estado::lista_alertas_vnt()`, sem travar a totalização.
3. **Limites de requisição HTTP (CDN do TSE):**
   - *Risco:* Bloqueio 429 se exceder a taxa combinada.
   - *Mitigação:* Mantido o limitador GCRA/AIMD com teto rígido de 100 req/s e disjuntor de proteção.

---

## 7. Commits e Tags Produzidos nesta Sessão

- `v0.1.4` (`b3445c2`): `motor: votavel desconhecido vira alerta e nulo tecnico (OFF_VNT)`
- `v0.1.5`: `motor/coletor: catalogo e candidatos pre-eleicao (candidatos.json), uv-coletor preparar e testes`

> **Nota:** Não foi realizado bump para `0.2.0`. A versão `0.2.0` será gerada pela validação independente conforme definido na especificação.

---

## 8. Correções após a validação (v0.1.7)

| defeito ([VALIDACAO_0.2.0.md](VALIDACAO_0.2.0.md) §3) | correção | evidência |
|---|---|---|
| 3.1 Presidente com 29 circunscrições | `preparar` grava como circunscrição só a abrangência que é a circunscrição do cargo (`circunscricao_de(cargo, abr) == abr`); as demais vão para `cargos[].abrangencias` (nome do arquivo e eleitorado). `Candidatos::do_arquivo` ignora circunscrição menor que a do cargo | `uv-motor noite --candidatos /home/dev/uvreal/preparo_2026_1t/candidatos.json` na noite de 2026: **3 eventos de Presidente** (2 × "2t" e "matematicamente_2t" no `br`), 0 revogações, `nao_confere` 0, 55 circunscrições decididas; `matematica::*_candidatos` |
| 3.2 Pleito 3221 sem SE e TO | municípios e UFs de cada eleição vêm do `mun-e<eleição>-cm.json` (novo campo `eleicoes[].municipios`); `cs.json` ausente de UF com eleição vai para `incompleto` e o comando sai com **código 2** | `candidatos.json` do 3221: **28 UFs, 5.757 municípios**, 494.044 seções; `incompleto` = `se-p003221-cs.json`, `to-p003221-cs.json` (404 em 12/10); teste `segundo_turno_2026_candidatos_pre_apuracao` confere todo município de cada `cm.json` no catálogo |
| 3.3 Consultas por município (683 arquivos) | cargo municipal só tem região nos municípios da eleição; consulta sem `mu` no `ele-c.json` é publicada na abrangência (UF); o `Publicador` usa as abrangências do `candidatos.json`; o nome do arquivo usa a eleição da circunscrição (`Candidatos::eleicao_de`: Prefeito está em 6 eleições suplementares) | publicação a frio do 3221: **43 arquivos, nomes idênticos aos 43 `-u.json` pré-apuração** (sem o `-u`), entre eles `pr-c0025-e006287.json`, `mt-c0026-e006288.json`, `mt-c0027-e006289.json`; a frio do 3220 (BUs de 2026): 83 nomes idênticos aos oficiais |
| 3.4 `st` com zero voto | com `estado` "apurando", todo `st` é vazio | 3221 a frio: 43 × "apurando", 89 × `"st":""` |
| 3.4 destinação presumida | documentada em [FORMATO_CANDIDATOS.md](../arquitetura/FORMATO_CANDIDATOS.md) (Premissas) e marcada no arquivo (`destinacao_presumida`) | — |
| 3.5 teste tautológico | conjuntos de 2024 e de 2026 1º turno regerados pelo próprio `preparar` com `--ignorar-destinacao` (todos "Válido"); os testes `*_pre_eleicao` falham se o arquivo não existir e exigem votos idênticos; em 2024 1º turno admitem só divergência de situação em arquivo cujo oficial tem candidato não apto | 2026 1º turno 83/83 e 832/832; 2024 2º turno 51/51 e 102/102; **2024 1º turno: 32 municípios / 33 situações divergem**, todos com candidato "Anulado"/"Anulado sub judice" no oficial (entre eles `rj58394`, `rs85308`, `sp66656`, `sp66850`, `sp67458`, que têm eleição suplementar no 3221); `matematica::primeiro_turno_2024_candidatos`: 63 declarações diferentes do oficial, todas nesses municípios, 0 revogações |
| §2 `preparar` não baixava as fontes | baixa `ele-c.json`, `cm.json`, `-u.json` e `cs.json` com cache (`--fontes`), 1 conexão, ≤ 5 req/s, para no 1º HTTP 429; `--offline`, `--atualizar`, `--cargos`, `--ignorar-destinacao` | 3221: 3 requisições (ele-c + 2 × 404); 3220: 2 (`cm.json` de 6257 e 6259); 2024: 0 (cache) |
| §4 pendências menores | `ARCHITECTURE.md` §3.1 com `OFF_VNT`; `de_arquivo_candidatos` devolve erro com UF inválida (teste unitário); `finalizada: true` e o contador "votáveis desconhecidos" (ocorrências, não votos; 1.165.919 em 2026) documentados em FORMATO_CANDIDATOS.md | teste `test_leitura_candidatos_json` |

Regerar os quatro conjuntos:

```bash
U=/home/dev/uvreal; C=$U/target-dev/release/uv-coletor
$C preparar --ciclo ele2024 --pleito 452 --saida $U/2024-1t/preparo --fontes $U/2024-1t/preparo_fontes --offline --ignorar-destinacao
$C preparar --ciclo ele2024 --pleito 453 --saida $U/2024-2t/preparo --fontes $U/2024-2t/preparo_fontes --offline --ignorar-destinacao
$C preparar --pleito 3220 --cargos 1,3,5 --saida $U/preparo_2026_1t --fontes $U/preparo_2026_1t --ignorar-destinacao
$C preparar --pleito 3221 --saida $U/2026-2t/preparo --fontes $U/2026-2t/preparo/fontes      # código 2 enquanto faltar SE/TO
```

(`preparo_fontes` de 2024 recebeu links para o `ele-c.json` e os `mun-e…-cm.json` já guardados em `descoberta/` e
`oficial/`.)

Testes: `compilar_dev.sh` (sem avisos; 15 + 4 + 1 + 7 + 4 + 3 passam) e
`UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1`: **13 passam**
(7 em `integracao`, 6 em `matematica`).

**Pendente para a noite de 25/10:** os `cs.json` de SE e TO (404 em 12/10). Rodar o `preparar` de novo até sair com
código 0; antes disso, os BUs de SE e TO cairiam em "fora do catálogo".
