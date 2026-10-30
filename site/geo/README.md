# Geografia do mapa das seções

`gerar_mapa.py` gera `site/src/mapa.json`, que o build copia para `site/dist/mapa.json` e o `uv-servico` embute em
`/static/mapa.json` (`aquisicao/src/pagina.rs`). A página (`site/src/mapa.js`) só busca o arquivo quando o cartão
"Mapa das seções" aparece na tela.

O servidor não sabe de geografia: numera as seções do catálogo (`GET /api/v1/mapa/secoes`) e diz quais já acenderam
(`docs/arquitetura/PROTOCOLO_WS.md` §4.8). Este arquivo diz onde desenhar cada uma.

## O que tem no `mapa.json`

| campo | o quê |
|---|---|
| `plano` | projeção do mapa plano: equirretangular, `x = (lon − lon0) · largura / span`, `y = (lat1 − lat) · largura / span` (largura 1000) |
| `ufs` | contorno de cada UF no plano (caminho SVG simplificado), para a silhueta do Brasil |
| `centros` | centro aproximado de cada UF (seção de município sem coordenada) |
| `mun` | `[código TSE, x·10, y·10, …]`: centro de cada município (sem local de votação conhecido, a seção fica em volta dele) |
| `locais` | com `locais_v: 2` (desde a 1.0.0): por zona, `[município, zona, m, m × (Δs0, x, y)]` — a partir da seção s0 (em diferença da anterior) a posição é (x, y) em 1/`locais_div` do plano (5 ≈ 0,9 km); (0, 0) = local sem coordenada (a seção fica em volta do município). As seções de locais diferentes vêm intercaladas na numeração, então só se grava onde a posição muda. Sem `locais_v`: o formato antigo `[município, zona, 1ª seção, nº, x·10, y·10, …]` |
| `fonte_locais` | ano, data da eleição e da geração da base dos locais |
| `mundo` | `[lon·10, lat·10, …]`: malha de pontos dos continentes (o globo) |
| `exterior` | nome da cidade (sem acento, maiúsculas) → `[lat·100, lon·100]`, para a UF `zz` |

## Fontes (consultadas em 24/10/2026)

* Centro dos municípios: [kelvins/municipios-brasileiros](https://github.com/kelvins/municipios-brasileiros) (`csv/municipios.csv`, licença MIT).
* Código do TSE × código do IBGE: [betafcc/Municipios-Brasileiros-TSE](https://github.com/betafcc/Municipios-Brasileiros-TSE) (`municipios_brasileiros_tse.csv`; a tabela de correspondência vem dos dados públicos do TSE e do IBGE).
* Contorno das UFs: [giuliano-oliveira/geodata-br-states](https://github.com/giuliano-oliveira/geodata-br-states) (`geojson/br_states`, licença MIT).
* Continentes: [johan/world.geo.json](https://github.com/johan/world.geo.json) (`countries.geo.json`, domínio público).
* Cidades do exterior: tabela no próprio `gerar_mapa.py` (coordenadas aproximadas do centro de cada cidade). Cidade da
  UF `zz` que não estiver nela fica fora do globo; a página lista as que faltaram em `window.__uv.mapa.semPosicao`.
* Locais de votação (opcional): arquivo "Eleitorado — local de votação" dos dados abertos do TSE (uma linha por
  seção, com `NR_LATITUDE`/`NR_LONGITUDE`;
  `https://cdn.tse.jus.br/estatistica/sead/odsele/eleitorado_locais_votacao/eleitorado_local_votacao_<ano>.zip`), ou a
  mesma base já agregada por local que o mcp-eleitoral guarda em cache (`~/.cache/mcp-eleitoral/locais-votacao-<ano>.v1.json`).
  Na 1.0.0: base de **2026** gerada pelo TSE em 09/10/2026 — 95.570 locais, 517.179 seções, 1.195 fronteiras sem
  coordenada; os 458 locais do exterior vêm sem coordenada (o exterior continua pelo nome da cidade, tabela EXTERIOR,
  que cobre as 186 cidades da base). Sem a base, cada seção fica em volta do centro do seu município.

## Gerar de novo

```bash
git clone --depth 1 https://github.com/kelvins/municipios-brasileiros /tmp/mb
git clone --depth 1 https://github.com/betafcc/Municipios-Brasileiros-TSE /tmp/tse
git clone --depth 1 https://github.com/giuliano-oliveira/geodata-br-states /tmp/st
git clone --depth 1 https://github.com/johan/world.geo.json /tmp/wg
python3 site/geo/gerar_mapa.py --municipios /tmp/mb/csv/municipios.csv --tse-ibge /tmp/tse/municipios_brasileiros_tse.csv \
  --ufs /tmp/st/geojson/br_states --mundo /tmp/wg/countries.geo.json \
  [--locais /mnt/d/dados/eleitorado_local_votacao_2026.csv]
cd site && npm run build
```

Só trocar os locais de votação (e a tabela do exterior) num `mapa.json` já gerado, sem as fontes de geografia:

```bash
python3 site/geo/gerar_mapa.py --so-locais --locais /mnt/c/Users/<você>/.cache/mcp-eleitoral/locais-votacao-2026.v1.json \
  [--exigir-exterior]   # sai com erro se alguma cidade do exterior da base ficar sem posição
cd site && npm run build
```

O `mapa.json` com os locais tem ~3,6 MB; o `uv-servico` manda os estáticos em gzip (~790 KB) quando o navegador
aceita (`aquisicao/src/pagina.rs`, teste `difusao::testes::estaticos_em_gzip`).
