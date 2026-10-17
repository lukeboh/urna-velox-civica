# Fontes Pré-Apuração — Levantamento e Evidências

Este documento registra a sondagem e identificação das fontes de dados disponíveis **antes da apuração** (pré-eleição) para o Urna Velox, em cumprimento ao Passo 1 do [HANDOFF_0.2.0.md](../../docs/HANDOFF_0.2.0.md).

Data do levantamento: 12/10/2026.
Ambiente de testes: WSL2 Ubuntu-24.04, conectado aos serviços públicos do TSE (CDN de resultados e arquivos de urna).

---

## 1. O Caso de Teste Real: Pleito 3221 (2º Turno de 2026, 25/10/2026)

Em 12/10/2026, o TSE já gerou e publicou no CDN os arquivos pré-apuração do pleito 3221:
- `ele-c.json` (`/oficial/comum/config/ele-c.json`): lista o pleito 3221 (`ele2026`, data 25/10/2026), com:
  - **Eleição 6258**: Presidente 2º Turno (Federal, `t = 2`, circunscrição Brasil `br`).
  - **Eleição 6260**: Governador 2º Turno (Estadual, `t = 2`, circunscrições em 5 UFs: `am`, `rn`, `df`, `es`, `ac`).
  - **Eleições Suplementares de Prefeito (`t = 1`)**:
    - 6284: Narandiba/SP (município 67458)
    - 6282: Macedônia/SP (município 66656)
    - 6283: Martinópolis/SP (município 66850)
    - 6290: Arroio do Sal/RS (município 85308)
    - 6285: Itaguaí/RJ (município 58394)
    - 6293: Santana do Itararé/PR (município 78557)
  - **Consultas Populares (`t = 1`)**:
    - 6288: Mato Grosso (cargo 26)
    - 6289: Mato Grosso (cargo 27)
    - 6287: Paraná (cargo 25)

### 1.1. Arquivos de Candidatos e Resultados Pré-Apuração (`-u.json`)

**Descoberta fundamental:** Os arquivos `-u.json` de cada abrangência são publicados **antes da apuração**, com os seguintes metadados característicos:
- `tf = "n"` (totalização não finalizada)
- `f = "o"` (oficial)
- `vap = 0` para todos os candidatos
- `carg[].agr[].par[].cand[]`: lista completa de candidatos concorrentes, com número (`n`), nome (`nm`), nome de urna (`nmu`), data de nascimento (`dt`) no formato `dd/mm/aaaa`.
- Nas consultas populares, `perg[].resp[]`: lista de opções com número (`n`), texto (`ds`) e `vap = 0`.
- `e.te`: eleitorado total da circunscrição/abrangência.

#### Evidências de URLs, Datas e Tamanhos (baixados para `/home/dev/uvreal/2026-2t/preparo/fontes/`):
| Arquivo | URL | Tamanho | `dg` / `hg` | Candidatos / Opções |
|---|---|---:|---|---|
| `br-c0001-e006258-u.json` | `.../ele2026/6258/dados/br/br-c0001-e006258-u.json` | 2.864 B | 12/10/2026 13:32:21 | 2 candidatos (Flávio Bolsonaro 22 e Lula 13) |
| `df-c0003-e006260-u.json` | `.../ele2026/6260/dados/df/df-c0003-e006260-u.json` | 3.101 B | 09/10/2026 17:30:52 | 2 candidatos (Celina Leão 11 e Leandro Grass 13) |
| `am-c0003-e006260-u.json` | `.../ele2026/6260/dados/am/am-c0003-e006260-u.json` | 2.734 B | 09/10/2026 21:15:26 | 2 candidatos |
| `rn-c0003-e006260-u.json` | `.../ele2026/6260/dados/rn/rn-c0003-e006260-u.json` | 3.052 B | 09/10/2026 17:30:48 | 2 candidatos |
| `es-c0003-e006260-u.json` | `.../ele2026/6260/dados/es/es-c0003-e006260-u.json` | 2.779 B | 09/10/2026 17:46:24 | 2 candidatos |
| `ac-c0003-e006260-u.json` | `.../ele2026/6260/dados/ac/ac-c0003-e006260-u.json` | 2.832 B | 09/10/2026 17:30:46 | 2 candidatos |
| `sp67458-c0011-e006284-u.json` | `.../ele2026/6284/dados/sp/sp67458-c0011-e006284-u.json` | 2.989 B | 09/10/2026 17:42:04 | 2 candidatos (Narandiba) |
| `mt-c0026-e006288-u.json` | `.../ele2026/6288/dados/mt/mt-c0026-e006288-u.json` | 1.620 B | 09/10/2026 17:05:49 | 2 respostas (Sim / Não) |

**Destinação (`dvt`):**
Nos arquivos `-u.json` gerados antes do início da apuração, o campo `dvt` pode vir ausente/nulo (já que nenhum voto foi processado para receber destinação de urna). Como todos os candidatos presentes no 2º turno estão aptos por definição (foram homologados para a disputa), a destinação pré-eleição padrão é `"Válido"`. Caso um candidato esteja em situação jurídica com destinação explicitada no arquivo, ela é preservada.

---

## 2. Configuração de Municípios (`cm.json`)

Para cada eleição, o arquivo `mun-e<eleição>-cm.json` (tipo `cm` do `ele-c.json`) mapeia os municípios e UFs participantes:
- URL padrão: `<base>/<ambiente>/<ciclo>/<cd_eleicao>/config/mun-e<cd_eleicao:06>-cm.json`
- Exemplos baixados e verificados:
  - `mun-e006258-cm.json`: 534.026 B, cobre todas as UFs e municípios do Brasil + exterior (ZZ).
  - `mun-e006260-cm.json`: 31.002 B, cobre exclusivamente os municípios das 5 UFs com 2º turno (`am`, `rn`, `df`, `es`, `ac`).
  - `mun-e006284-cm.json`: 224 B, município 67458 Narandiba/SP.
  - `mun-e006288-cm.json`: 314 B, municípios de Mato Grosso participantes da consulta.

---

## 3. Seções Eleitorais (`cs.json`)

Para o pleito 3221:
- URL padrão: `<base>/<ambiente>/<ciclo>/arquivo-urna/<cd_pleito>/config/<uf>/<uf>-p<cd_pleito:06>-cs.json`
- Baixados para `/home/dev/uvreal/2026-2t/preparo/fontes/cs/`.
- 26 arquivos disponíveis (25 UFs + ZZ).
- **Atenção:** `se` (Sergipe) e `to` (Tocantins) responderam HTTP 404 para o pleito 3221 no momento da sondagem (12/10), pois não possuem nenhuma eleição suplementar nem 2º turno estadual, e o TSE ainda não havia gerado ou publicado a árvore do pleito 3221 para essas duas UFs. Para as 26 UFs com eleição, os `cs.json` contêm a lista exata de zonas e seções eleitorais com data/hora de geração.

---

## 4. Eleitorado: Regra `eleitorado >= Σ aptos`

Foi verificado nos conjuntos de dados de 2024 (1º e 2º turnos) e 2026 (1º turno):
1. O campo `e.te` nos arquivos oficiais e pré-eleição representa o eleitorado apto cadastrado no cadastro eleitoral para a circunscrição.
2. Em todas as eleições e circunscrições verificadas, `e.te >= Σ aptos_BU`.
3. No cálculo do "matematicamente", o Urna Velox utiliza:
   `R = eleitorado.saturating_sub(apurado)`
   onde `apurado` é a soma de aptos das seções cujos BUs já foram processados. Como `e.te >= Σ aptos`, o eleitorado restante `R` nunca subestima os votos faltantes, garantindo que declarações matemáticas não sejam revogadas.

---

## 5. Estratégia para o `candidatos.json` e `uv-coletor preparar`

A partir dessas evidências:
1. O subcomando `uv-coletor preparar --pleito <P> --saida <DIR>` fará a leitura de:
   - `ele-c.json` (descoberta das eleições do pleito)
   - `mun-e*-cm.json` (municípios e UFs de cada eleição)
   - `cs.json` de cada UF (seções eleitorais)
   - Arquivos `-u.json` das circunscrições do pleito (candidatos, números, nomes, nascimentos, vagas, eleitorado, turno).
2. O arquivo gerado será o `candidatos.json`, formato estruturado e desacoplado, que permitirá ao motor instanciar `Candidatos` e `Catalogo` sem rede e sem necessidade de processar BUs previamente.

## Uso pelo `uv-coletor preparar` (v0.1.7)

O comando baixa (com cache em `--fontes`, 1 conexão, ≤ 5 req/s, parada no primeiro HTTP 429) exatamente estas fontes:
`ele-c.json`; `mun-e<eleição>-cm.json` de cada eleição com cargo majoritário ou consulta (municípios e UFs da eleição);
o `-u.json` de cada abrangência publicada (Presidente: `br`, UFs e `zz`; Governador/Senador: UF; Prefeito: município;
consulta: a abrangência do `ele-c.json` — município quando há `mu`, senão a UF); e o `<uf>-p<pleito>-cs.json` de cada UF
com eleição. Em 12/10/2026 os `cs.json` de **SE e TO do 3221 ainda respondem 404**: o `candidatos.json` sai com
`incompleto` e código 2. Os `-u.json` pré-apuração do 3221 não trazem `dvt` (destinação presumida "Válido").
