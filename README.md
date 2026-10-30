# Urna Velox

![Urna Velox: totalização eleitoral independente, BU por BU](docs/img/capa.png)

> ### 🌐 Acompanhe a apuração ao vivo: [urnavelox.com.br](https://urnavelox.com.br)

[![Rust](https://img.shields.io/badge/rust-stable%20%7C%20edition%202021%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-production%20ready%20%7C%202.0%20in%20dev-brightgreen.svg)](#roadmap)
[![Engine](https://img.shields.io/badge/engine-zero--copy%20%7C%20concurrent-purple.svg)](#arquitetura)

> **Motor de totalização eleitoral independente de alta performance, projetado em Rust para processamento contínuo de Boletins de Urna (BUs) do TSE com agregação analítica instantânea em nível Municipal, Estadual e Nacional.**

---

## ✨ Funcionalidades e Inovações

O Urna Velox transforma centenas de milhares de Boletins de Urna brutos do TSE em uma experiência de apuração analítica, transparente e instantânea, direto no navegador do celular ou computador, sem necessidade de cadastro ou instalação.

### ⚡ Apuração e Análise em Tempo Real
- **Resultado milissegundo a milissegundo:** Cada Boletim de Urna é processado assim que liberado pelo TSE. O placar atualiza em tempo real na sua tela, sem aguardar os ciclos de atualização em lote do portal oficial.
- **🏁 Momento Matemático da Decisão:** Cálculo exato, BU a BU, do instante em que a eleição está matematicamente definida (*eleito* ou *2º turno garantido*), indicando o horário preciso e a **seção eleitoral decisiva**.
- **⏳ Contador Regressivo de Decisão:** Hodômetro decrescente que informa exatamente quantos votos faltam ser apurados para fechar a disputa matematicamente. Dá para ver a disputa afunilando ao vivo.
- **🗺️ Mapa Interativo das Seções:** Visualização geográfica onde cada uma das ~500.000 seções eleitorais totalizadas acende um ponto luminoso em tempo real — com alternância fluida entre a visão do **Brasil** e o **Globo 3D do Exterior**.
- **🔮 Previsão Inteligente e Honesta:** Modelo estatístico que estima com faixas horárias (P10–P90 e P50) quando a eleição será decidida e quando a apuração terminará. Se a transmissão de BUs do TSE sofrer lentidão ou paradas, o sistema suspende e adia a estimativa em vez de simular falsa precisão. Depois da definição, informa com honestidade se o resultado saiu dentro da faixa prevista.
- **⚖️ Comparação Lado a Lado com o TSE:** Seletor de fontes (*Urna Velox · Comparar · TSE Oficial*) para auditar diferenças de votos, percentuais e ordem dos candidatos, destacando quanto tempo antes o Urna Velox atingiu cada marca.
- **📰 Feed de Notícias e Marcos da Noite:** Letreiro dinâmico com as viradas de liderança, marcos de 25%, 50%, 90% e 100%, primeiras seções e cidades apuradas, e painel histórico pesquisável (*"O que já passou"*) com filtros por categoria e linha do tempo própria.
- **📱 Dashboard Modular e Fluido (CLS = 0):** Interface tátil em glassmorfismo com zero pulo de tela (*Cumulative Layout Shift = 0*), hodômetros mecânicos digitais que giram sempre no sentido correto, suporte completo a temas claro/escuro e **cartões que podem ser reorganizados e maximizados** livremente.
- **🕐 Dois relógios, sem confusão:** Separação clara entre o horário do Urna Velox ("Totalizado pelo Urna Velox às…") e o horário de recebimento pelo TSE ("último BU no TSE: …"), garantindo transparência temporal.
- **📈 A curva da noite:** Gráfico temporal com o avanço das seções e a evolução percentual dos concorrentes desde o primeiro BU.
- **🗺️ Do Brasil ao seu município:** Presidente no Brasil, Governador e Senador no seu estado, Prefeito na sua cidade: cada abrangência e cada cargo com sua tela, votos nominais, legenda, brancos e nulos.
- **🔍 Aberto e conferível:** Todos os números vêm dos BUs públicos do TSE. Em todas as eleições já totalizadas (2024 e 2026), o resultado final foi rigorosamente idêntico ao oficial, candidato por candidato.

---

### 🎬 Playlist Interativa e Replays 24/7 com Dados Reais
Fora dos dias de eleição oficial, o Urna Velox opera como uma plataforma contínua de auditoria e estudo eleitoral:
- **Dados 100% Reais:** Reprocessamento contínuo dos BUs originais das eleições de **2024 (1º e 2º turnos)** e **2026 (1º e 2º turnos)**, totalizando quase 500 mil BUs por ciclo sem nenhum voto inventado.
- **Dois Modos de Reprodução:**
  - *Cadência Real (1×):* Reproduz a noite exatamente no ritmo, ordem e pausas em que os dados chegaram da Justiça Eleitoral.
  - *Acelerado (500×):* Totalização massiva de quase 500 mil BUs em poucos minutos.
- **▶ Playlist Democrática ([/playlist](https://urnavelox.com.br/playlist)):** O público acompanha a agenda das próximas exibições pontuais por hora cheia e vota na eleição que deseja assistir a seguir (um voto por conexão IP com proteção anti-abuso).
- **Player de Replay Integrado:** Barra de controle tipo mídia para pausar, acelerar (1× a 500×) e navegar livremente na linha do tempo da apuração (*scrubbing*) com retotalização transparente em memória.
- **Selo e Aviso de Transmissão:** O selo do topo identifica claramente o modo (**AO VIVO**, **REPLAY** ou **EM BREVE**). Em períodos de eleições oficiais, a playlist é suspensa e o sistema acompanha exclusivamente a eleição de verdade.

Como funciona por dentro (rotação, vigias dos nós, votação e suspensão): [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md).

---

## 🎯 Visão e Missão

O **Urna Velox** nasceu para resolver o desafio de auditoria cívica e totalização em escala do processo eleitoral brasileiro. Em eleições gerais e municipais, a Justiça Eleitoral (TSE) disponibiliza centenas de milhares de arquivos brutos de Boletim de Urna (`.bu`) codificados em ASN.1/BER.

Diferente de abordagens baseadas em scripts em lote (*batch*) sequenciais ou cargas pesadas em bancos de dados relacionais que demoram horas para consolidar resultados, o **Urna Velox** opera como um **motor analítico in-memory contínuo e concorrente**:

- **Agregação Virtualmente Instantânea:** À medida que cada arquivo `.bu` ou lote compactado é lido do disco ou recebido via rede, os totais consolidados para **Município**, **UF** e **Brasil** são atualizados em microssegundos.
- **Zero-Copy & Parser ASN.1 Nativo:** Decodificação de baixo nível dos formatos oficiais do TSE sem alocações supérfluas de memória.
- **Eficiência de Recursos Extrema:** O estado completo de uma eleição geral (~500.000 BUs) ocupa ~36 MB num vetor denso de contadores pré-dimensionado a partir dos dados públicos de candidatura; o motor inteiro fica na casa de 50 MB, saturando todos os núcleos de CPU disponíveis ([estimativas medidas](docs/arquitetura/ESTIMATIVAS_MEMORIA_E_TEMPO.md)).
- **Auditoria Cívica Aberta:** Código-fonte público e determinístico, viabilizando a conferência pública e independente dos resultados das urnas eletrônicas brasileiras por pesquisadores, partidos, entidades fiscalizadoras e a sociedade civil.

---

## 🏛️ Granularidade e Modelo de Agregação

O Urna Velox mantém os totais municipais num vetor denso de contadores atômicos; UF e Brasil são somas derivadas dos municípios, calculadas sob demanda:

```
[ Boletim de Urna (.bu) ] ──> [ Parser ASN.1/BER ] ──> [ Validador & Deduplicador ]
                                                                │
                                      ┌─────────────────────────┴─────────────────────────┐
                                      ▼                                                   ▼
                          [ Dimensão Eleitoral ]                               [ Dimensão Geográfica ]
                     (Eleição / Cargo / Candidato)                        (Município ──> UF ──> Brasil)
                                      │                                                   │
                                      └─────────────────────────┬─────────────────────────┘
                                                                ▼
                                                [ Tabela Agregada em Memória ]
                                             (Totalização Instantânea Multiescopo)
```

1. **Nível Município:** Consolidação local imediata de votos nominais, votos de legenda, brancos e nulos por cargo.
2. **Nível Unidade da Federação (UF):** Soma dos municípios de cada estado (e Distrito Federal / Votação no Exterior).
3. **Nível Brasil (Nacional):** Soma das UFs para cargos nacionais (Presidência da República) e indicadores gerais de comparecimento e abstenção. A consolidação completa leva poucos milissegundos.

---

## 🛠️ Stack Tecnológica

| Componente | Tecnologia | Justificativa |
|---|---|---|
| **Linguagem Principal** | **Rust** (Edição 2021+) | Compilação nativa, previsibilidade total de latência, ausência de coletor de lixo (GC) e garantias formais de segurança de memória e ausência de *data races*. |
| **Parser ASN.1 / BER** | Parser BER manual zero-copy + `rasn` como referência | Caminho rápido lê os bytes do `.bu` sem alocar; o esquema oficial compilado com `rasn` confere o resultado nos testes (ADR-007). |
| **Paralelismo Massivo** | threads nativas / `rayon` | Uma thread por núcleo lógico, divisão do trabalho em blocos de BUs. |
| **Estado de Totalização** | `Vec<AtomicU32>` denso | Um contador por (município, cargo, votável), layout pré-computado; incremento atômico relaxado, sem travas (ADR-002, ADR-003). |
| **Motor** | crate `urna-velox` (`motor/`) | Leitor de BU, catálogo, estado denso, situação dos candidatos, matematicamente eleito / 2º turno, publicação em JSON e conferência com o oficial ([ARCHITECTURE §2.5](ARCHITECTURE.md)). |
| **Aquisição** | `tokio` + `reqwest` (HTTP/2, `rustls`), crate `aquisicao/` | Coleta dos BUs no site de resultados e nos dados abertos do TSE, com teto de 100 req/s e controle adaptativo; entrega ao núcleo por canal e pacotes `.uvpk` (ADR-013 a ADR-016). |
| **I/O & Streaming** | `zip` & `notify` (Fases 1–3); `tokio` + `axum` (Fase 4) | Leitura direta dos pacotes compactados do TSE; runtime assíncrono só na camada HTTP e na aquisição (ADR-010, ADR-012, ADR-014). |
| **Interface CLI** | `clap` (v4) | CLI robusta, tipada e ergonômica para operações, benchmarks e diagnósticos via terminal. |
| **Serialização de Saída** | `serde` (`serde_json`, `csv`) | Exportação instantânea de snapshots e relatórios consolidados em formatos abertos. |

---

## 🚀 Guia de Início Rápido

### Pré-requisitos
- **Rust Toolchain**: Rust 1.82+ com Cargo instalado ([rustup.rs](https://rustup.rs/)).

### Compilação

Clone o repositório e compile o binário com otimizações de release:

```bash
git clone https://github.com/lukeboh/urna-velox-civica.git
cd urna-velox
cargo build --release
```

O binário otimizado estará disponível em `target/release/uv-motor` (ou `target/release/uv-motor.exe` no Windows); todos os executáveis do projeto começam com `uv-`.

### Comandos do motor (`v0.1.0`)

O workspace tem dois binários: **`uv-motor`** (o motor, crate `urna-velox` em `motor/`) e **`uv-coletor`** (a aquisição, `aquisicao/`).
Os dados (pacotes `.uvpk` de BUs, arquivos oficiais `-u.json`, métricas da coleta) ficam fora do repositório.

```bash
# totaliza em lote e grava o resultado de cada cargo e abrangência (JSON, docs/arquitetura/FORMATO_RESULTADO.md)
./target/release/uv-motor totalizar --dados ~/uvreal/dados --oficial ~/uvreal/oficial --threads 28 --saida resultado/

# confere votos e situação de cada candidato com a divulgação oficial do TSE (código 1 se divergir)
./target/release/uv-motor conferir --dados ~/uvreal/dados --oficial ~/uvreal/oficial --metricas ~/uvreal/metricas_nacional.csv --vagas 5=2

# reproduz a noite em ordem de chegada dos BUs: quando cada circunscrição fica matematicamente decidida
./target/release/uv-motor noite --dados ~/uvreal/2024-2t/dados --oficial ~/uvreal/2024-2t/oficial \
  --chegadas ~/uvreal/2024-2t/metricas_2t.csv --saida noite/

# conteúdo de um BU
./target/release/uv-motor inspecionar o03220df9701200010003-bu.dat
```

Coleta dos BUs, replay da noite com publicação contínua e os demais utilitários: [aquisicao/README.md](aquisicao/README.md).
Testes: `cargo test --release -p urna-velox` (unidade + fixtures) e, com os dados locais,
`UV_DADOS=~/uvreal cargo test --release -p urna-velox -- --ignored` (as conferências completas). Detalhes em
[motor/README.md](motor/README.md).

---

## 📚 Documentação Técnica

- [docs/arquitetura/AQUISICAO.md](docs/arquitetura/AQUISICAO.md): Fase de aquisição: como os BUs são obtidos do TSE (API de resultados e dados abertos), arquitetura do coletor, limites do CDN, medições reais e gráficos. Código em [aquisicao/](aquisicao/) (`uv-coletor`); para repetir: `./aquisicao/teste-real.sh` ou `.\aquisicao\teste-real.ps1`.
- [docs/arquitetura/FORMATO_RESULTADO.md](docs/arquitetura/FORMATO_RESULTADO.md): Formato JSON do resultado por cargo e abrangência (situação, matematicamente eleito), contrato do WebSocket e do site.
- [docs/operacao/CICLO_DIARIO.md](docs/operacao/CICLO_DIARIO.md): Replays contínuos com dados reais nos dois nós da OCI: rotação por hora cheia, modos tempo real e acelerado, Playlist de Replays (votação por IP, decisão única nos dois nós, suspensão nas eleições oficiais), telemetria e conferência de cada ciclo.
- [ARCHITECTURE.md](ARCHITECTURE.md): Detalhamento do pipeline de parsing, modelo de concorrência, estruturas em memória e garantias de idempotência e integridade.
- [docs/arquitetura/ARCHITECTURE_DECISIONS.md](docs/arquitetura/ARCHITECTURE_DECISIONS.md): Registro de decisões (ADR-000 a ADR-031): linguagem (Rust vs. JavaScript vs. Java), modelo de estado, concorrência, deduplicação, validação, orçamento de memória, critérios de pivot, federação e licenciamento (ADR-031: Source-Available / PolyForm Noncommercial com dupla licença).
- [docs/arquitetura/ESTIMATIVAS_MEMORIA_E_TEMPO.md](docs/arquitetura/ESTIMATIVAS_MEMORIA_E_TEMPO.md): Cálculos de memória e tempo de totalização de uma eleição real nas três linguagens, medidos com o mesmo corpus sintético, com gráficos.
- [bench/](bench/): Benchmarks reproduzíveis (Rust, Java, Node.js) e resultados brutos. Para repetir tudo: `./bench/benchmark.sh` (Linux/WSL2) ou `.\bench\benchmark.ps1` (Windows).
- [docs/arquitetura/GLOSSARIO.md](docs/arquitetura/GLOSSARIO.md): Significado de todos os termos técnicos usados na documentação (ADR, BER, zero-copy, operação atômica, CDN, HTTP 429, AIMD etc.).
- [ROADMAP.md](ROADMAP.md): Planejamento de engenharia por etapas e versões (motor, catálogo, operação automatizada, WebSocket, site, federação, comparação com o TSE, previsão, ensaio geral → `1.0.0`; proporcionais → `2.0.0`).

---

## 🤝 Contribuição e Transparência Cívica

O **Urna Velox** é um projeto de soberania informacional, auditoria cidadã e pesquisa eleitoral. Contribuições da comunidade técnica são muito bem-vindas para fins cívicos e acadêmicos!

1. Faça um Fork do projeto.
2. Crie uma branch para sua funcionalidade ou correção: `git checkout -b feature/minha-feature`.
3. Garanta que o código passa no linter e testes: `cargo fmt --check && cargo clippy && cargo test`.
4. Envie suas alterações via Pull Request com descrição clara dos objetivos técnicos.

Ao contribuir, valem as condições do [licenciamento](docs/legal/LICENCIAMENTO.md#contribuições).

---

## ⚖️ Licença e Termos de Uso

O Urna Velox é **livre para qualquer uso não comercial** — pesquisa, auditoria cidadã, ensino e uso pessoal —, sem custo e sem pedir autorização. Detalhes em [docs/legal/LICENCIAMENTO.md](docs/legal/LICENCIAMENTO.md).

Privacidade, acesso e retenção de dados do site (fazemos o mínimo; não vendemos nem compartilhamos informação pessoal dos visitantes): [docs/legal/PRIVACIDADE.md](docs/legal/PRIVACIDADE.md), publicada em [urnavelox.com.br/privacidade](https://urnavelox.com.br/privacidade).
