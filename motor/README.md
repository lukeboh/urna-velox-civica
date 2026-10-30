# urna-velox — o motor

Crate do motor de totalização (`urna-velox`; binário **`uv-motor`**) do Urna Velox: leitor de BU, catálogo, estado denso, agregador, candidatos, situação dos
candidatos, matematicamente eleito / 2º turno e publicação do resultado em JSON
([docs/arquitetura/FORMATO_RESULTADO.md](../docs/arquitetura/FORMATO_RESULTADO.md)). Arquitetura: [ARCHITECTURE.md §2.5](../ARCHITECTURE.md).
A aquisição dos BUs (rede, ≤ 100 req/s) fica em [`aquisicao/`](../aquisicao) (`uv-coletor`); o motor não faz rede.

## Binário

```bash
# compilar (WSL): aquisicao/scripts/compilar_dev.sh  → ~/uvreal/target-dev/release/uv-motor
uv-motor totalizar --dados ~/uvreal/dados --oficial ~/uvreal/oficial --metricas <csv,…> --threads 28 --saida json/
uv-motor conferir  --dados ~/uvreal/dados --oficial ~/uvreal/oficial --metricas <csv,…> --vagas 5=2 --threads 28
uv-motor conferir-publicacao --publicacao <replay>/publicacao --oficial ~/uvreal/oficial
uv-motor noite --dados ~/uvreal/2024-2t/dados --oficial ~/uvreal/2024-2t/oficial --chegadas <csv> --saida noite/
uv-motor inspecionar o03220df9701200010003-bu.dat
# previsão (etapa 8): backtest na noite reproduzida, uma previsão a cada 30 s do relógio da noite, e gabaritos da noite
uv-motor previsao --dados … --oficial … --chegadas <csv> --nome 2024-2t --cargos 11 --threads 16 --saida prev/
```

* `previsao` (módulo `previsao`): estimativa, com faixa P10–P90, do horário do "matematicamente" e do fim da apuração;
  as três noites de uma vez: `scripts/backtest_previsao.sh` (relatório em `docs/relatorios/RELATORIO_0.8.0.md`).

* `--oficial`: arquivos de divulgação (`-u.json`), de onde vêm os **candidatos** nesta etapa (destinação, nascimento,
  eleitorado, turno, vagas); na etapa 2 virá dos dados publicados antes da apuração.
* `--metricas`: métricas da coleta (`uv-coletor baixar --metricas`): seções anuladas (votos viram nulos) e não instaladas.
* Sem `--catalogo`, o catálogo é montado com os próprios BUs.

## Testes

```bash
cargo test --release -p urna-velox                                   # unidade + fixtures (tests/fixtures, 2,3 MB)
UV_DADOS=/home/dev/uvreal cargo test --release -p urna-velox -- --ignored --test-threads 1   # conferências completas
```

As conferências completas reproduzem a etapa 0: 1º turno de 2026 (83 abrangências, 832 situações), Prefeito de 2024
(5.569 + 51 municípios, 15.042 + 102 situações, 5 consultas), pelo estado e pelas publicações JSON, e o teste de
propriedade do "matematicamente" nas três noites (nenhuma revogação, sempre igual ao oficial).
