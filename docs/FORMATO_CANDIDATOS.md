# Formato do arquivo `candidatos.json` (Versão 1)

O arquivo `candidatos.json` é o insumo pré-eleição do Urna Velox, gerado pelo `uv-coletor preparar` e consumido pelo `uv-motor` (`--candidatos candidatos.json`). Ele substitui a necessidade de inferir candidatos a partir dos próprios BUs ou de depender dos arquivos de divulgação oficiais pós-apuração durante a noite da eleição.

## Estrutura Geral

```json
{
  "versao": 1,
  "pleito": 3221,
  "ufs": ["ac", "al", "am", "ap", "ba", "ce", "df", "es", "go", "ma", "mg", "ms", "mt", "pa", "pb", "pe", "pi", "pr", "rj", "rn", "ro", "rr", "rs", "sc", "se", "sp", "to", "zz"],
  "municipios": [
    { "codigo": 97012, "uf": "df" }
  ],
  "secoes": [
    { "municipio": 97012, "zona": 1, "secao": 1 }
  ],
  "destinacao_presumida": true,
  "incompleto": ["se-p003221-cs.json ausente (UF com eleição; seções desconhecidas)"],
  "eleicoes": [
    {
      "codigo": 6258,
      "turno": 2,
      "municipios": ["ac01007", "df97012", "zz29017"],
      "cargos": [
        {
          "cargo": 1,
          "vagas": 1,
          "consulta": false,
          "circunscricoes": [
            {
              "abr": "br",
              "eleitorado": 155823368,
              "candidatos": [
                {
                  "numero": 13,
                  "nome": "LUIZ INÁCIO LULA DA SILVA",
                  "nome_urna": "LULA",
                  "destinacao": "Válido",
                  "nascimento": "06/10/1945"
                },
                {
                  "numero": 22,
                  "nome": "FLAVIO NANTES BOLSONARO",
                  "nome_urna": "FLAVIO BOLSONARO",
                  "destinacao": "Válido",
                  "nascimento": "30/04/1981"
                }
              ]
            }
          ],
          "abrangencias": [
            { "abr": "br", "eleitorado": 155823368 },
            { "abr": "ac", "eleitorado": 600000 }
          ]
        }
      ]
    }
  ]
}
```

## Campos

- **`versao`** (`u32`): versão do formato (`1`; os campos novos abaixo são opcionais na leitura).
- **`pleito`** (`u32`): código do pleito (ex.: `3221`, `3220`, `452`, `453`).
- **`ufs`**: siglas das UFs com eleição, em minúsculas (as dos `mun-e…-cm.json`; `zz` = exterior).
- **`municipios`** (`[{ codigo, uf }]`): todos os municípios das eleições do pleito, dos `cm.json`.
- **`secoes`** (`[{ municipio, zona, secao, agregada? }]`): seções de cada UF, dos `<uf>-p<pleito>-cs.json`.
  **`agregada`** (desde a `0.4.9`, só quando `true`): a seção tem `nsp` no `cs.json` — foi juntada a uma seção principal
  e não tem urna nem BU próprio (os votos vêm no BU da principal). Continua no catálogo, mas fica fora do total de seções
  publicado, como no TSE (`s.ts`): no 1º turno de 2026 eram 17.931 das 517.179 seções listadas (499.248 = `s.ts`).
- **`destinacao_presumida`** (`bool`, padrão `false`): alguma destinação foi presumida "Válido" — os `-u.json`
  pré-apuração não trazem `dvt` (3221) ou o arquivo foi montado com `--ignorar-destinacao` (simulação com dados já
  apurados). Ver "Premissas".
- **Nomes para o site** (desde a `0.4.1`, opcionais; o motor ignora): `nomes_ufs` (`{"ac": "ACRE", …, "zz":
  "EXTERIOR"}`, `abr[].ds` do `cm.json`), `municipios[].nome` (`abr[].mu[].nm` do `cm.json`), e em cada candidato
  `partido` (`par[].sg` do `-u.json`), `vices` (`[{tp, nome, nome_urna, partido}]`, `cand[].vs[]`; `tp` "v" = vice,
  "1"/"2" = suplentes) e `nome_urna` do `nmu` (antes era o nome completo). Saem dos mesmos arquivos que o `preparar` já
  baixava: com o cache em `--fontes`, `--offline` regera sem nenhuma requisição. Servidos em `GET /api/v1/nomes`.
- **`incompleto`** (`[String]`, padrão vazio): fontes que faltaram (HTTP 404). Arquivo incompleto não deve ser usado na
  noite: `uv-coletor preparar` sai com código 2 até tudo estar publicado.
- **`eleicoes`**: eleições do pleito que têm cargo majoritário ou consulta (proporcionais ficam de fora nesta versão).
  - **`codigo`**, **`turno`**: do `ele-c.json`.
  - **`municipios`** (`["<uf><mun:05>"]`, padrão: todos): municípios desta eleição (do seu `cm.json`). No catálogo, um
    cargo municipal (Prefeito, consultas) só tem região nestes municípios — inclusive a consulta cuja circunscrição é
    a UF (6288/MT: 2 municípios).
  - **`cargos`**:
    - **`cargo`** (`u8`): 1 Presidente, 3 Governador, 5 Senador, 11 Prefeito, 25–99 consultas.
    - **`vagas`** (`u8`), **`consulta`** (`bool`, `tp` = 3 no `ele-c.json`).
    - **`circunscricoes`**: só a abrangência que **é** a circunscrição do cargo (onde se decide quem é eleito):
      Presidente `br`; Governador e Senador a UF; Prefeito o município; consulta a abrangência do `ele-c.json` (município
      ou UF). Entrada com abrangência menor (ex.: Presidente numa UF) é ignorada na leitura.
      - **`abr`**: `br`, `<uf>` ou `<uf><municipio:05>`; **`eleitorado`**: eleitores aptos.
      - **`candidatos`**: `numero`, `nome`, `nome_urna`, `destinacao` ("Válido", "Válido sub judice", "Anulado",
        "Anulado sub judice"), `nascimento` (`dd/mm/aaaa`, desempate por idade).
    - **`abrangencias`** (`[{ abr, eleitorado, secoes? }]`, padrão: as circunscrições): todas as abrangências publicadas do
      cargo — as dos arquivos de divulgação (Presidente: `br`, as 27 UFs e `zz`) — com o eleitorado de cada uma. São
      os nomes dos arquivos que o `uv-motor` publica (`<abr>-c<cargo>-e<eleição>.json`). **`secoes`** (opcional, desde a
      `0.2.2`): seções a totalizar na abrangência (`s.ts` do `-u.json` pré-apuração = BUs + não instaladas, sem as
      agregadas); o `uv-servico` encerra a apuração quando todas as abrangências chegam a esse número. Ausente ou 0 =
      desconhecido (o fim só por comando, arquivo `CONFERIR`).

## Como gerar

```text
uv-coletor preparar --pleito 3221 --saida /home/dev/uvreal/2026-2t/preparo --fontes /home/dev/uvreal/2026-2t/preparo/fontes
```

Fontes (todas publicadas antes da apuração), com cache em `--fontes` (`<fontes>/<sub>/<nome>` ou `<fontes>/<nome>`):
`ele-c.json`; `mun-e<eleição>-cm.json` (`cm/`); `<abr>-c<cargo>-e<eleição>-u.json` de cada abrangência (`u/`);
`<uf>-p<pleito>-cs.json` (`cs/`). O que está no cache não é baixado de novo (`--atualizar` força `ele-c.json` e
`cs.json`, que mudam até a eleição; `--offline` não usa a rede). Rede: 1 conexão, ≤ 5 req/s, para no primeiro HTTP 429.
`--cargos 1,3,5` restringe os cargos; `--ignorar-destinacao` presume "Válido" para todos (simulação com arquivos já
apurados, como nos conjuntos de 2024 e de 2026 1º turno).

## Premissas

* **Destinação presumida.** Os `-u.json` pré-apuração do 3221 têm `tf` = n, `vap` = 0 e nascimento, mas **não têm
  `dvt`**: o `preparar` grava "Válido" e marca `destinacao_presumida`. Cassações e indeferimentos posteriores mudam o
  resultado: com essa premissa, o 1º turno de 2024 diverge do oficial em 32 municípios (33 situações; votos idênticos)
  — justamente onde o oficial tem candidato "Anulado"/"Anulado sub judice" (entre eles os 5 municípios das eleições
  suplementares do 3221). Antes da noite, conferir as destinações (ou regerar o arquivo perto da eleição).
* **Circunscrições "finalizadas".** `Candidatos::do_arquivo` marca toda circunscrição como finalizada: a situação
  (`st`) só aparece na publicação encerrada e só quando o `estado` não é "apurando".
* **Contador "votáveis desconhecidos"** (`Estado::desconhecidos`, linha "conferência" do `uv-motor totalizar`). Conta
  **ocorrências, não votos**: cada votável nominal em número sem candidato no catálogo (o voto vai para `vnt`, nulo
  técnico, e fica em `Estado::lista_alertas_vnt()` com cargo, município, número e quantidade) e cada cargo de um BU que
  não tem região no catálogo (município fora, cargo fora). No 1º turno de 2026 com o `candidatos.json` de
  `--cargos 1,3,5` são 1.165.919 ocorrências, sem nenhuma divergência nos 83 arquivos: é informativo, não erro.
  Erros de verdade estão nos outros contadores da mesma linha (fora do catálogo, divergentes, inválidos).
