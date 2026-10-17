# Benchmarks do Urna Velox

Mede **memória** e **tempo de totalização** do Urna Velox nas três opções de linguagem do [ADR-001](../docs/ARCHITECTURE_DECISIONS.md#adr-001--linguagem-rust-plano-a) (Rust, Java e JavaScript), com os mesmos BUs sintéticos e a mesma organização dos contadores. A análise da execução de referência está em [docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md](../docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md). Termos técnicos: [docs/GLOSSARIO.md](../docs/GLOSSARIO.md).

> **Com BUs reais:** a alternativa a este benchmark que usa BUs de verdade, baixados do TSE pelo coletor da fase de aquisição, está em [`aquisicao/`](../aquisicao/) (`./aquisicao/teste-real.sh`). Ela mede a mesma coisa (totalização com os BUs já em memória, `real-dense`) e compara com o `manual-dense` sintético daqui; ver [docs/AQUISICAO.md](../docs/AQUISICAO.md).

## Como repetir o teste

Um comando faz tudo: confere/instala as dependências, compila, gera os BUs de teste, confere que todas as versões totalizam igual, mede e apresenta os resultados.

**No Linux** (ou dentro do WSL2):

```bash
./bench/benchmark.sh            # completo: ~7–10 min num desktop de 8+ núcleos
./bench/benchmark.sh --rapido   # curto, só para conferir que funciona: ~1–2 min
```

**No Windows** (PowerShell, usa o WSL2):

```powershell
.\bench\benchmark.ps1
.\bench\benchmark.ps1 -Rapido
.\bench\benchmark.ps1 -Distro Ubuntu-24.04     # se houver mais de uma distribuição
```

Ao final, as tabelas aparecem no terminal e ficam salvas em **`bench/results/RELATORIO.md`**, com os gráficos em `bench/results/graficos/`.

### Pré-requisitos

| Item | Versão | Se faltar |
|---|---|---|
| Linux ou WSL2 | qualquer distribuição recente | no Windows: `wsl --install -d Ubuntu-24.04` |
| Java (JDK) | 21+ | `sudo apt install openjdk-21-jdk` |
| Node.js + npm | 20+ | `sudo apt install nodejs npm` ou https://nodejs.org |
| Python 3 + venv | 3.10+ | `sudo apt install python3 python3-venv` |
| curl, compilador C | — | `sudo apt install curl build-essential` |
| Rust | estável | **instalado automaticamente** pelo `setup.sh`, só para o seu usuário (`~/.cargo`), sem `sudo` |
| Internet | — | só na primeira execução: Rust, Bouncy Castle (Maven Central), `asn1js` (npm) e `matplotlib` (PyPI) |
| Disco | ~1 GB livre em `~/uvbench` | BUs gerados e dependências |
| Memória | 8 GB+ | as variantes "ingênuas" de Java e `asn1js` chegam a 4–5 GB |

Para só conferir as dependências sem instalar nada: `UV_SEM_INSTALAR=1 ./bench/benchmark.sh`.

### O que cada etapa faz

| Etapa | Script | O que faz |
|---|---|---|
| 1 | `scripts/setup.sh` | Confere Java, Node, Python; instala o Rust se faltar; compila o Rust (`cargo build --release --locked`) e o Java; baixa o Bouncy Castle e o `asn1js`; cria o ambiente Python com `matplotlib`. |
| 2 | `scripts/gerar_bus.sh` | Gera 3 conjuntos de BUs sintéticos em `~/uvbench` (eleição geral com e sem assinatura por votável, e municipal) e o layout de contadores. A geração é determinística: sempre os mesmos bytes. |
| 3 | `scripts/verificar.sh` | Totaliza 3.000 BUs em cada variante e exige o **mesmo checksum** e zero BUs inválidos. Para tudo se alguma divergir. |
| 4 | `scripts/medir.sh` | Para cada nº de threads, roda todas as variantes e grava `results/time.csv`; depois mede a memória do estado (`results/mem_*.txt`). |
| 5 | `scripts/relatorio.py` | Calcula medianas, imprime as tabelas, grava `results/RELATORIO.md`, `results/summary.csv` e os gráficos. |

Cada etapa pode ser rodada sozinha, na ordem acima (por exemplo, `./bench/scripts/medir.sh` para medir de novo sem regenerar os BUs).

### Configuração (variáveis de ambiente, todas opcionais)

| Variável | Padrão | Efeito |
|---|---|---|
| `UV_WORK` | `~/uvbench` | pasta dos BUs gerados, binários Java, dependências e venv (fora do repositório) |
| `UV_OUT` | `bench/results` | pasta dos resultados |
| `UV_THREADS` | `1 2 4 8 12 16 … nproc` | números de threads a medir |
| `UV_BUS` | `500000` (rápido: `100000`) | BUs totalizados por medição |
| `UV_NCORPUS` | `20000` | BUs distintos em cada corpus |
| `UV_RAPIDO` | `0` | `1` = modo rápido (o mesmo que `--rapido`) |
| `UV_REGERAR` | `0` | `1` = apaga e gera os BUs de novo |
| `UV_SEM_INSTALAR` | `0` | `1` = não instala nada, só confere |

Exemplo: medir só com 1, 8 e 16 threads, guardando numa pasta separada:

```bash
UV_THREADS="1 8 16" UV_OUT=/tmp/uv-teste ./bench/benchmark.sh
```

### Sobre os resultados versionados

`bench/results/` guarda a **execução de referência** (11/10/2026, i7-14700, WSL2) usada em `docs/ESTIMATIVAS_MEMORIA_E_TEMPO.md`; os gráficos daquele documento são uma cópia em `docs/img/`. Uma nova execução **sobrescreve** `bench/results/`: o `git diff` mostra o que mudou, e você decide se faz commit (nova referência) ou descarta (`git checkout bench/results`). Para não mexer na referência, use `UV_OUT=/outra/pasta`.

## Estrutura

| Caminho | Conteúdo |
|---|---|
| `benchmark.sh` / `benchmark.ps1` | ponto de entrada (Linux / Windows) |
| `scripts/comum.sh` | configuração compartilhada e variáveis |
| `scripts/setup.sh`, `gerar_bus.sh`, `verificar.sh`, `medir.sh`, `relatorio.py` | etapas 1 a 5 |
| `rust/src/main.rs` | `urnabench`: gerador de BUs (`gen`) e totalizador (`run`) — variantes `manual-dense`, `manual-dense-sha512`, `rasn-dense`, `manual-dashmap` |
| `rust/src/bin/membench.rs` | memória do estado por desenho (`DashMap` aninhado, tabela hash, vetor denso) |
| `java/Bench.java` | totalizador Java — `manual-dense`, `bc-dense` (Bouncy Castle), `manual-chm` |
| `java/Mem.java` | memória do estado em Java |
| `js/bench.mjs` | totalizador Node/Bun com workers — `manual-dense`, `asn1js-dense` |
| `js/mem.mjs` | memória do estado em JS e custo de cada worker |
| `results/` | resultados da última execução |

### Uso direto dos programas

```bash
# gerar BUs: tipo geral|municipal, N BUs distintos, --nosig = sem assinatura por votável
bench/rust/target/release/urnabench gen --kind geral --n 20000 --out ~/uvbench
# totalizar: variante, threads, nº de BUs (CHECK=1 imprime o checksum)
CHECK=1 bench/rust/target/release/urnabench run --dir ~/uvbench --kind geral --variant manual-dense --threads 8 --bus 500000
java -cp ~/uvbench/java:~/uvbench/bcprov.jar Bench ~/uvbench geral manual-dense 8 500000
node ~/uvbench/js/bench.mjs ~/uvbench geral manual-dense 8 500000
```

### Formato de `results/time.csv`

`lang, variant, kind, threads, bus, seconds, bus_per_s, consolidate_ms, peak_rss_mb, corpus_mb, invalid`

* `seconds`: tempo de parede da totalização (BUs já em memória; leitura de disco fora da medição).
* `consolidate_ms`: tempo de somar UF e Brasil a partir dos municípios, numa thread.
* `peak_rss_mb`: pico de memória do processo, **incluindo** os BUs de entrada (`corpus_mb`); o benchmark JS mantém duas cópias deles (o relatório desconta as duas).
* `invalid`: cargos cuja soma de votos difere de `comparecimento × vagas`; deve ser sempre 0.
