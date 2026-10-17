# Fixtures dos testes do motor

BUs reais e arquivos oficiais de divulgação do TSE, recortados dos dados locais (`~/uvreal` no WSL) por
[`gerar.sh`](gerar.sh) com `uv-coletor filtrar`. Total ≈ 2,3 MB (limite combinado: 5 MB). Bytes intocados
(`.gitattributes`: binário / sem conversão de fim de linha).

| pasta | conteúdo | usado em |
|---|---|---|
| `bu/` | 5 BUs avulsos: 2026 `-bu.dat` (DF 97012/0001/0003), 2026 `-busa.dat` (MG 50075/0047/0041, Sistema de Apuração), 2024 com consulta popular (RR 03158/0004/0127), 2024 2º turno (BA 34134/0170/0008), seção anulada pela Junta (MA 07315/0023/0354); `.txt` = saída de `uv-motor inspecionar`, idêntica à do leitor da v0.0.22 | `tests/bu_avulsos.rs` |
| `2024-1t/` | 1º turno de 2024, 8 municípios inteiros (333 BUs): AC 01031 (pequeno), RR 03158 e RS 84824 (com consulta popular), MA 07315 (seção anulada), MG 46191 (empate no 1º lugar), SP 62081 (ninguém eleito, art. 224), SE 31313 (totalização não finalizada, `tf = n`), RS 85766 (candidato anulado com votos); `oficial/` com os 10 `-u.json` (8 Prefeito + 2 consultas); `metricas.csv` = linhas desses municípios nas métricas da coleta dos `aux.json` | `tests/municipais.rs` |
| `2024-2t/` | 2º turno de 2024, BA 34134 inteiro (540 BUs), oficial e métricas | `tests/municipais.rs` |

Para refazer: `bash motor/tests/fixtures/gerar.sh` (WSL, depois de `aquisicao/scripts/compilar_dev.sh`).
As conferências completas (todos os BUs) ficam em `tests/integracao.rs`, com `UV_DADOS` e `--ignored`.
